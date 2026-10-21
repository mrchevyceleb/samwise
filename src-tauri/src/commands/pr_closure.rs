//! Read-only GitHub evidence for a PR that was closed without merging.
//! A reference alone is not replacement evidence. Require a maintainer's
//! explicit supersession statement and a real, still-open or merged PR.
use crate::process::async_cmd;
use regex::Regex;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SupersedingPr {
    pub url: String,
    pub evidence_url: String,
    pub merged: bool,
}

impl SupersedingPr {
    pub fn note(&self) -> String {
        format!(
            "This PR was superseded by {} ({}). Work is tracked there; this task is complete as superseded, not as a separate merge or deployment. Evidence: {}",
            self.url, if self.merged { "merged" } else { "open" }, self.evidence_url
        )
    }

    pub fn receipt(&self) -> Value {
        json!({"pr_url": self.url, "evidence_url": self.evidence_url, "merged": self.merged})
    }
}

fn pr_identity(raw: &str) -> Option<(String, u64)> {
    let url = url::Url::parse(raw).ok()?;
    if url.scheme() != "https" || url.host_str()? != "github.com" { return None; }
    let parts: Vec<_> = url.path_segments()?.collect();
    if parts.len() != 4 || parts[2] != "pull" { return None; }
    Some((format!("{}/{}", parts[0], parts[1]), parts[3].parse().ok()?))
}

fn trusted_author(value: &Value) -> bool {
    matches!(value["author_association"].as_str(), Some("OWNER" | "MEMBER" | "COLLABORATOR"))
}

async fn github(endpoint: &str, paginate: bool, token: Option<&str>) -> Result<Value, String> {
    let mut command = async_cmd("gh");
    command.kill_on_drop(true).args(["api", endpoint]);
    if let Some(token) = token {
        command.env("GH_TOKEN", token).env_remove("GITHUB_TOKEN");
    }
    if paginate { command.arg("--paginate"); }
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await.map_err(|_| "GitHub closure evidence timed out".to_string())?
        .map_err(|_| "Could not read GitHub closure evidence".to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("HTTP 404") || stderr.contains("Not Found") {
            return Err("GitHub closure evidence was not found".into());
        }
        // stderr may contain auth details. Retry later without classifying a
        // transport/auth failure as either a supersession or a real blocker.
        return Err("GitHub closure evidence is unavailable; retrying later".into());
    }
    if paginate {
        // Older supported gh versions have --paginate but no --slurp. Parse
        // its consecutive JSON page documents without losing later pages.
        serde_json::Deserializer::from_slice(&output.stdout).into_iter::<Value>()
            .collect::<Result<Vec<_>, _>>().map(Value::Array)
            .map_err(|_| "Invalid GitHub closure evidence".into())
    } else {
        serde_json::from_slice(&output.stdout).map_err(|_| "Invalid GitHub closure evidence".into())
    }
}

fn pages(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten().flat_map(|page| page.as_array().into_iter().flatten())
}

pub async fn is_unmerged_closed(original: &str) -> Result<bool, String> {
    is_unmerged_closed_using(original, None).await
}

pub async fn is_unmerged_closed_with_token(original: &str, token: &str) -> Result<bool, String> {
    is_unmerged_closed_using(original, Some(token)).await
}

async fn is_unmerged_closed_using(original: &str, token: Option<&str>) -> Result<bool, String> {
    let (repo, number) = pr_identity(original).ok_or("Invalid closed PR URL")?;
    let pr = github(&format!("repos/{}/pulls/{}", repo, number), false, token).await?;
    Ok(pr["state"] == "closed" && pr["merged"] == false)
}

fn explicit_successors(body: &str, repo: &str) -> Vec<String> {
    let pattern = Regex::new(r"(?im)^\s*(?:closing|closed)?\s*,?\s*(?:this (?:pr|pull request) (?:is|was)\s+)?(?:superseded by|replaced by|folded into|rolled into|consolidated into|duplicate of|(?:closing|closed)\s+in favou?r of|in favou?r of)\s*:?\s*(?:\[)?(https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/[0-9]+|#[0-9]+)").unwrap();
    pattern.captures_iter(body).filter_map(|capture| {
        let reference = capture.get(1)?.as_str();
        Some(if let Some(number) = reference.strip_prefix('#') {
            format!("https://github.com/{}/pull/{}", repo, number)
        } else { reference.to_string() })
    }).collect()
}

fn replacement_cross_reference(body: &str, repo: &str, number: u64, source_url: &str) -> bool {
    let same_repo = pr_identity(source_url).map(|(source_repo, _)| source_repo == repo).unwrap_or(false);
    let pattern = if same_repo {
        format!(r"(?:#{}\b|https://github\.com/{}/pull/{}\b)", number, regex::escape(repo), number)
    } else {
        format!(r"https://github\.com/{}/pull/{}\b", regex::escape(repo), number)
    };
    let reference = Regex::new(&pattern).unwrap();
    // Keep the relation and original PR in the same sentence. A generic
    // mention, closing keyword or unrelated cross-reference is insufficient.
    body.lines().any(|line| {
        let lower = line.to_lowercase();
        reference.is_match(line) && !lower.contains("not supersed")
            && !lower.contains("not incorporated") && !lower.contains("isn't incorporated")
            && !lower.contains("wasn't incorporated") && !lower.contains("doesn't incorporate")
            && (lower.contains("supersedes ") || lower.contains("replaces ")
                || lower.contains("incorporated here") || lower.contains("folded into this"))
    })
}

async fn verify_successor(original: &str, candidate: &str, evidence_url: &str, token: Option<&str>) -> Result<Option<SupersedingPr>, String> {
    let Some((repo, number)) = pr_identity(candidate) else { return Ok(None); };
    if pr_identity(original) == Some((repo.clone(), number)) { return Ok(None); }
    let pr = match github(&format!("repos/{}/pulls/{}", repo, number), false, token).await {
        Ok(pr) => pr,
        Err(error) if error == "GitHub closure evidence was not found" => return Ok(None),
        Err(error) => return Err(error),
    };
    let merged = pr["merged"].as_bool() == Some(true);
    if !merged && pr["state"] != "open" { return Ok(None); }
    Ok(Some(SupersedingPr {
        url: format!("https://github.com/{}/pull/{}", repo, number),
        evidence_url: evidence_url.to_string(), merged,
    }))
}

/// None means a genuinely closed PR, so the normal failure notification stays.
/// Err means GitHub could not establish the outcome; do not alert or finish it.
pub async fn superseding_pr(original: &str) -> Result<Option<SupersedingPr>, String> {
    superseding_pr_using(original, None).await
}

pub async fn superseding_pr_with_token(original: &str, token: &str) -> Result<Option<SupersedingPr>, String> {
    superseding_pr_using(original, Some(token)).await
}

async fn superseding_pr_using(original: &str, token: Option<&str>) -> Result<Option<SupersedingPr>, String> {
    let (repo, number) = pr_identity(original).ok_or("Invalid closed PR URL")?;
    let timeline = github(&format!("repos/{}/issues/{}/timeline?per_page=100", repo, number), true, token).await?;
    let events: Vec<_> = pages(&timeline).collect();
    // Prefer the closer's explicit note, including comments on later pages.
    for event in events.iter().rev().filter(|event| event["event"] == "commented" && trusted_author(event)) {
        let body = event["body"].as_str().unwrap_or("");
        for url in explicit_successors(body, &repo) {
            if let Some(successor) = verify_successor(original, &url, event["html_url"].as_str().unwrap_or(original), token).await? {
                return Ok(Some(successor));
            }
        }
    }
    for event in events.iter().rev().filter(|event| event["event"] == "cross-referenced") {
        let source = &event["source"]["issue"];
        let url = source["html_url"].as_str().unwrap_or("");
        if source["pull_request"].is_null() || !trusted_author(source)
            || !replacement_cross_reference(source["body"].as_str().unwrap_or(""), &repo, number, url) { continue; }
        if let Some(successor) = verify_successor(original, url, url, token).await? { return Ok(Some(successor)); }
    }
    Ok(None)
}
