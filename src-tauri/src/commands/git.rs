use serde::Serialize;
use crate::process::cmd;

// ---- Helpers ----

fn run_git(args: &[&str], repo_path: &str, op: &str) -> Result<String, String> {
    let output = cmd("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("{} failed: {}. Is git installed?", op, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        return Err(format!("{} failed: {}", op, msg.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_git_simple(args: &[&str], repo_path: &str, op: &str) -> Result<(), String> {
    run_git(args, repo_path, op)?;
    Ok(())
}

// ---- Types ----

#[derive(Debug, Clone, Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub files: Vec<GitFileStatus>,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub conflicted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitBranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

// ---- Commands ----

#[tauri::command]
pub fn git_status(project_dir: String) -> Result<GitStatus, String> {
    // Get branch name
    let branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"], &project_dir, "Get branch")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    // Get porcelain status
    let status_out = run_git(&["status", "--porcelain"], &project_dir, "Status")?;
    let files: Vec<GitFileStatus> = status_out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let chars: Vec<char> = line.chars().collect();
            let idx = chars.get(0).copied().unwrap_or(' ');
            let wt = chars.get(1).copied().unwrap_or(' ');
            let pair: String = chars.iter().take(2).collect();
            let conflicted = matches!(
                pair.as_str(),
                "UU" | "AA" | "DD" | "AU" | "UA" | "DU" | "UD"
            ) || idx == 'U' || wt == 'U';

            let (status, staged) = if idx != ' ' && idx != '?' {
                (idx.to_string(), true)
            } else {
                (
                    if wt != ' ' { wt.to_string() } else { "?".to_string() },
                    false,
                )
            };

            let path = line.chars().skip(3).collect::<String>();
            GitFileStatus { path, status, staged, conflicted }
        })
        .collect();

    // Get ahead/behind
    let (ahead, behind) = cmd("git")
        .args(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .current_dir(&project_dir)
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let parts: Vec<&str> = s.trim().split('\t').collect();
            if parts.len() == 2 {
                Some((
                    parts[0].parse::<u32>().unwrap_or(0),
                    parts[1].parse::<u32>().unwrap_or(0),
                ))
            } else {
                None
            }
        })
        .unwrap_or((0, 0));

    Ok(GitStatus { branch, files, ahead, behind })
}

#[tauri::command]
pub fn git_diff_staged(project_dir: String) -> Result<String, String> {
    run_git(&["diff", "--cached"], &project_dir, "Staged diff")
}

#[tauri::command]
pub fn git_diff(project_dir: String, file_path: String, staged: bool) -> Result<String, String> {
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(&file_path);
    run_git(&args, &project_dir, "Diff")
}

#[tauri::command]
pub fn git_stage_file(project_dir: String, file_path: String) -> Result<(), String> {
    run_git_simple(&["add", &file_path], &project_dir, "Stage")
}

#[tauri::command]
pub fn git_unstage_file(project_dir: String, file_path: String) -> Result<(), String> {
    run_git_simple(&["reset", "HEAD", &file_path], &project_dir, "Unstage")
}

#[tauri::command]
pub fn git_stage_all(project_dir: String) -> Result<(), String> {
    run_git_simple(&["add", "-A"], &project_dir, "Stage all")
}

#[tauri::command]
pub fn git_unstage_all(project_dir: String) -> Result<(), String> {
    run_git_simple(&["reset", "HEAD"], &project_dir, "Unstage all")
}

#[tauri::command]
pub fn git_discard_file(project_dir: String, file_path: String) -> Result<(), String> {
    run_git_simple(&["checkout", "--", &file_path], &project_dir, "Discard")
}

#[tauri::command]
pub fn git_commit(project_dir: String, message: String, files: Vec<String>) -> Result<String, String> {
    if files.is_empty() {
        // Commit whatever is staged
        run_git_simple(&["commit", "-m", &message], &project_dir, "Commit")?;
    } else {
        // Stage specified files then commit
        let mut add_args: Vec<&str> = vec!["add", "--"];
        let refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        add_args.extend(&refs);
        run_git_simple(&add_args, &project_dir, "Stage files")?;
        run_git_simple(&["commit", "-m", &message], &project_dir, "Commit")?;
    }

    // Return the new commit hash
    let hash = run_git(&["rev-parse", "HEAD"], &project_dir, "Get hash")?;
    Ok(hash.trim().to_string())
}

#[tauri::command]
pub fn git_log(project_dir: String, count: usize) -> Result<Vec<GitCommitInfo>, String> {
    let n = count.to_string();
    let output = run_git(
        &["log", "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%s%x1e", "-n", &n],
        &project_dir,
        "Log",
    )?;

    let mut commits = Vec::new();
    for entry in output.split('\x1e') {
        if entry.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = entry.split('\x1f').collect();
        if parts.len() < 6 {
            continue;
        }
        commits.push(GitCommitInfo {
            hash: parts[0].trim().to_string(),
            short_hash: parts[1].to_string(),
            author: parts[2].to_string(),
            author_email: parts[3].to_string(),
            timestamp: parts[4].parse::<i64>().unwrap_or(0),
            message: parts[5].trim().to_string(),
        });
    }
    Ok(commits)
}

#[tauri::command]
pub fn git_branch_list(project_dir: String) -> Result<Vec<GitBranchInfo>, String> {
    let output = run_git(&["branch", "-a"], &project_dir, "Branch list")?;
    let branches: Vec<GitBranchInfo> = output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let trimmed = line.trim();
            let is_current = trimmed.starts_with("* ");
            let name = trimmed.trim_start_matches("* ").to_string();
            let is_remote = name.starts_with("remotes/");
            GitBranchInfo { name, is_current, is_remote }
        })
        .collect();
    Ok(branches)
}

#[tauri::command]
pub fn git_branch_current(project_dir: String) -> Result<String, String> {
    let out = run_git(&["rev-parse", "--abbrev-ref", "HEAD"], &project_dir, "Current branch")?;
    Ok(out.trim().to_string())
}

#[tauri::command]
pub fn git_checkout(project_dir: String, branch: String) -> Result<(), String> {
    run_git_simple(&["checkout", &branch], &project_dir, "Checkout")
}

#[tauri::command]
pub fn git_create_branch(project_dir: String, branch_name: String) -> Result<(), String> {
    run_git_simple(&["checkout", "-b", &branch_name], &project_dir, "Create branch")
}

#[tauri::command]
pub fn git_stash(project_dir: String) -> Result<(), String> {
    run_git_simple(&["stash", "push", "-u"], &project_dir, "Stash")
}

#[tauri::command]
pub fn git_stash_pop(project_dir: String) -> Result<(), String> {
    run_git_simple(&["stash", "pop"], &project_dir, "Stash pop")
}

#[tauri::command]
pub fn git_push(project_dir: String) -> Result<(), String> {
    run_git_simple(&["push"], &project_dir, "Push")
}

#[tauri::command]
pub fn git_pull(project_dir: String) -> Result<(), String> {
    run_git_simple(&["pull"], &project_dir, "Pull")
}

/// Close a GitHub pull request without merging it.
///
/// Uses the authenticated `gh` CLI (same account the worker uses for reviews
/// and merges). `gh pr close <full-pr-url>` resolves the owner/repo/number
/// from the URL itself, so no working directory is required — but when the
/// task carries a `repo_path` we run from there too so any repo-scoped `gh`
/// config is picked up.
///
/// Closing an already-closed PR or a merged PR is treated as success so the
/// "Close PR & Mark Done" button still completes the task in those cases.
#[tauri::command]
pub fn close_pr(pr_url: String, repo_path: Option<String>) -> Result<(), String> {
    let url = pr_url.trim();
    if url.is_empty() {
        return Err("No PR URL to close.".to_string());
    }

    let mut gh = crate::process::cmd("gh");
    gh.args(["pr", "close", url]);
    if let Some(dir) = repo_path.as_deref() {
        if !dir.is_empty() {
            gh.current_dir(dir);
        }
    }

    let output = gh
        .output()
        .map_err(|e| format!("spawn gh pr close: {}. Is the GitHub CLI installed and logged in?", e))?;

    if output.status.success() {
        return Ok(());
    }

    // gh prints a human message on stdout for "already closed/merged" cases.
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    let combined = format!("{} {}", stdout, stderr);

    // Already closed / already merged → the PR is not open, so the goal of the
    // button (PR is no longer open + mark task done) is already satisfied.
    if combined.contains("already closed") || combined.contains("already merged") {
        log::info!("[close_pr] PR already closed/merged, treating as success: {}", url);
        return Ok(());
    }

    let msg = if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    };
    Err(if msg.is_empty() {
        format!("gh pr close failed for {}", url)
    } else {
        format!("gh pr close failed: {}", msg)
    })
}
