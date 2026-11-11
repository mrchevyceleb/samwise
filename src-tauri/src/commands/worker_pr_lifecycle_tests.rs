//! Opt-in end-to-end check: the real worker, local Postgres/PostgREST and
//! read-only GitHub PRs. Never connects to the production task database.
use super::*;

#[tokio::test]
#[ignore = "requires an isolated local PostgREST and read-only GitHub access"]
async fn live_pr_lifecycle() {
    let base = std::env::var("AUTOSAM_QA_REST_URL").expect("local REST URL required");
    let url = url::Url::parse(&base).unwrap();
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let repo = std::env::var("AUTOSAM_QA_REPO").expect("isolated TARDIS checkout required");
    let open_pr = std::env::var("AUTOSAM_QA_OPEN_PR").expect("real open PR required");
    let config = SupabaseConfig { url: base, ..Default::default() };
    let client = reqwest::Client::new();
    let merged = "https://github.com/mrchevyceleb/TARDIS/pull/349";
    let closed = "https://github.com/mrchevyceleb/TARDIS/pull/347";
    let unmerged = "https://github.com/mrchevyceleb/TARDIS/pull/267";
    let missing = "https://github.com/mrchevyceleb/TARDIS/pull/999999999";
    assert_eq!(pr_closure::state(merged).await.unwrap(), PrState::Merged);
    assert_eq!(pr_closure::state(closed).await.unwrap(), PrState::Closed);
    assert_eq!(pr_closure::state(&open_pr).await.unwrap(), PrState::Open);
    assert!(pr_closure::state(missing).await.is_err());
    // This check must never deploy anything: #349 has no native deploy plan.
    let files = review::fetch_pr_files(merged, &repo).await.unwrap();
    assert!(build_deploy_plan(&repo, &files).await.unwrap().commands.is_empty());
    assert!(!deploy_green_wait_required(&repo, &files).await);
    let mut ids = Vec::new();
    for (label, pr, status, cycles, context) in [
        ("merged review", merged, "review", 0, serde_json::json!({})),
        ("merged capped fix", merged, "fixes_needed", 5, serde_json::json!({})),
        ("closed fix", closed, "fixes_needed", 1, serde_json::json!({})),
        ("unavailable", missing, "fixes_needed", 1, serde_json::json!({})),
        ("active repair", merged, "in_progress", 2, serde_json::json!({"samwise_auto_fix_status":"running"})),
        ("closed unmerged", unmerged, "fixes_needed", 2, serde_json::json!({})),
    ] {
        let id = uuid::Uuid::new_v4().to_string();
        let row = serde_json::json!({"id":id,"title":format!("Lifecycle QA: {}",label),
            "status":status,"source":"manual","pr_url":pr,"repo_path":repo,
            "repo_url":"https://github.com/mrchevyceleb/TARDIS","context":context,
            "review_cycle_count":cycles,"on_hold":false});
        let response = client.post(format!("{}/rest/v1/ae_tasks",config.url)).json(&row).send().await.unwrap();
        assert!(response.status().is_success(), "seed {}: {}",label,response.text().await.unwrap());
        ids.push((id,label,cycles));
    }
    let settings = Some(serde_json::json!({"autoPrReviewEnabled":true,"autoFixFromFixesNeededEnabled":true}));
    sweep_pr_merged_cards(&config).await;
    sweep_pr_review_queue(&config, &settings).await;
    sweep_stale_fixes_needed_cards(&config, &settings).await;
    assert!(!task_is_live(&config,&ids[4].0).await, "active repair must stop on a merged PR");
    let active = supabase::fetch_task(&config,&ids[4].0).await.unwrap().unwrap();
    assert!(!pr_task_is_open(&config,&active).await);
    // The spawn entrypoint itself must reject unavailable state, without a claim.
    spawn_auto_fix_task(config.clone(),ids[3].0.clone(),missing.into(),repo.clone(),
        "## Blockers\n- Must not run".into(),1,5,"sam/absent".into());
    for _ in 0..60 {
        let mut done = true;
        for (id,label,_) in &ids {
            if *label == "unavailable" { continue; }
            let task = supabase::fetch_task(&config,id).await.unwrap().unwrap();
            done &= task["status"] == if *label == "closed unmerged" { "failed" } else { "done" };
        }
        if done { break; }
        sweep_merge_deploy_requests(&config).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    for (id,label,cycles) in &ids {
        let task = supabase::fetch_task(&config,id).await.unwrap().unwrap();
        assert_eq!(task["review_cycle_count"], *cycles, "{} spent a fix cycle", label);
        assert_eq!(task["status"],if *label == "unavailable" {"fixes_needed"} else if *label == "closed unmerged" {"failed"} else {"done"}, "{}: {:?}",label,task["failure_reason"]);
        println!("PASS {}: status={}, cycles={}",label,task["status"],task["review_cycle_count"]);
    }
    // Repeated sweeps are idempotent; terminal cards never re-enter repair.
    sweep_pr_review_queue(&config,&settings).await;
    sweep_stale_fixes_needed_cards(&config,&settings).await;
    for (id,label,cycles) in &ids {
        let task = supabase::fetch_task(&config,id).await.unwrap().unwrap();
        assert_eq!(task["review_cycle_count"], *cycles, "repeat {}", label);
        let _ = client.delete(format!("{}/rest/v1/ae_tasks?id=eq.{}",config.url,id)).send().await.unwrap();
        if let Some(path) = deploy_worktree_path(&repo,id) {
            if path.exists() { run_git(&["worktree","remove",path.to_str().unwrap()],&repo).await.unwrap(); }
        }
    }
    assert!(pr_closure::require_open(&open_pr).await.is_ok());
    println!("PASS open PR remains eligible; terminal repeats spend no cycles; local rows/worktrees removed");
}

/// Real native coder cancellation on this job's own draft PR. Reopens it and
/// removes its local task even on assertion failure. No production task DB.
#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires explicit owned draft PR and local database; starts one bounded native coder"]
async fn live_repair_cancellation() {
    use futures_util::FutureExt;
    let base = std::env::var("AUTOSAM_QA_REST_URL").unwrap();
    let parsed = url::Url::parse(&base).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    let pr = std::env::var("AUTOSAM_QA_OWNED_DRAFT_PR").unwrap();
    assert!(pr.starts_with("https://github.com/mrchevyceleb/samwise/pull/"));
    let info = async_cmd("gh").args(["pr","view",&pr,"--json","state,isDraft,headRefName"]).output().await.unwrap();
    assert!(info.status.success());
    let info: Value = serde_json::from_slice(&info.stdout).unwrap();
    assert_eq!(info["state"], "OPEN");
    assert_eq!(info["isDraft"], true);
    assert_eq!(info["headRefName"], "fix/stop-work-on-terminal-pr");
    let repo = std::env::var("AUTOSAM_QA_REPAIR_REPO").unwrap();
    assert!(std::fs::canonicalize(&repo).unwrap().starts_with("/var/tmp/nca-autosam-terminal-pr/"));
    let config = SupabaseConfig { url: base, ..Default::default() };
    let client = reqwest::Client::new();
    let id = uuid::Uuid::new_v4().to_string();
    let response = client.post(format!("{}/rest/v1/ae_tasks",config.url)).json(&serde_json::json!({
        "id":id,"title":"Native cancellation QA","source":"manual","status":"in_progress",
        "pr_url":pr,"repo_path":repo,"review_cycle_count":1,"on_hold":false,
        "context":{"samwise_auto_fix_status":"running"}})).send().await.unwrap();
    assert!(response.status().is_success());
    let _owner = safety::continuation_owner(&id);
    let pid_slot = Arc::new(tokio::sync::Mutex::new(None));
    let cfg = config.clone(); let tid = id.clone(); let slot = pid_slot.clone();
    let child = tokio::spawn(async move {
        run_claude_code_streaming(&repo,
            "This is a cancellation lifecycle check. Do not change files, commit, push, use network tools, or run reviewers. Execute sleep 90 in the foreground and wait for cancellation.",
            2, 120, &cfg, &tid, slot, Some("claude-opus-5-5")).await
    });
    let result = std::panic::AssertUnwindSafe(async {
        let mut started = None;
        for _ in 0..60 {
            started = *pid_slot.lock().await;
            if started.is_some() { break; }
            if child.is_finished() { panic!("native coder exited before its PID was recorded"); }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let pid = started.expect("native coder did not start");
        assert!(Path::new(&format!("/proc/{}",pid)).exists());
        println!("Native coder running: pid={}",pid);
        let closed = async_cmd("gh").args(["pr","close",&pr]).output().await.unwrap();
        assert!(closed.status.success());
        let output = tokio::time::timeout(std::time::Duration::from_secs(60), child).await.unwrap().unwrap();
        assert!(matches!(output, Err(ref error) if error == "TASK_CANCELLED"), "native coder did not stop through cancellation");
        assert!(!Path::new(&format!("/proc/{}",pid)).exists(), "coder was not reaped");
        assert!(!finish_auto_fix_run(&config,&id,&pr,1).await);
        let task = supabase::fetch_task(&config,&id).await.unwrap().unwrap();
        assert_eq!(task["status"],"failed");
        assert_eq!(task["review_cycle_count"],1);
        assert_eq!(task["context"]["samwise_auto_fix_status"],"stopped");
        sweep_stale_fixes_needed_cards(&config,&Some(serde_json::json!({"autoFixFromFixesNeededEnabled":true}))).await;
        let task = supabase::fetch_task(&config,&id).await.unwrap().unwrap();
        assert_eq!(task["review_cycle_count"],1);
        println!("PASS real native coder reaped after closure; post-child gate prevented push; task failed once; no new cycle");
    }).catch_unwind().await;
    if let Some(pid) = *pid_slot.lock().await { crate::process::stop_process_group(pid).await; }
    let reopened = async_cmd("gh").args(["pr","reopen",&pr]).output().await.unwrap();
    assert!(reopened.status.success(), "owned draft PR must be reopened");
    client.delete(format!("{}/rest/v1/ae_tasks?id=eq.{}",config.url,id)).send().await.unwrap();
    if let Err(panic) = result { std::panic::resume_unwind(panic); }
}
