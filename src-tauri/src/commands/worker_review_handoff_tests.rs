use super::*;

/// Opt-in native acceptance: the actual webhook must first insert both
/// incident handoffs into an isolated local database. GitHub is read-only;
/// these terminal PRs must finish before any model process is launched.
#[tokio::test]
#[ignore = "requires a local task-webhook/DB replay of TARDIS #347 and #349"]
async fn live_original_review_handoffs() {
    let url = std::env::var("AUTOSAM_QA_REST_URL").expect("local replay URL");
    let parsed = url::Url::parse(&url).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    let config = SupabaseConfig { url, anon_key: "local".into(), ..Default::default() };
    let initial = supabase::fetch_tasks(&config, None).await.unwrap();
    let rows = initial.as_array().unwrap();
    assert_eq!(rows.len(), 2, "dedicated local replay only");
    for task in rows {
        let number = task["pr_number"].as_u64().unwrap();
        assert!(matches!(number, 347 | 349));
        let original = format!("https://github.com/mrchevyceleb/TARDIS/pull/{number}");
        assert_eq!(task["pr_url"], original);
        assert_eq!(task["context"]["pr_url"], original);
        assert_eq!(task["context"]["simple_full_pr_review"], true);
        assert_eq!(task["context"]["full_pr_review_task"], true);
        assert!(is_full_pr_review_owned(task), "normal coding queue must skip this handoff");
        assert!(is_plant_full_pr_review_task(task));
    }

    // Check new webhook handoffs, then the original incident shape: flags
    // missing and pr_url overwritten by a stray coding PR.
    for legacy in [false, true] {
        if legacy {
            for task in rows {
                let stray = if task["pr_number"] == 347 { 349 } else { 350 };
                let mut misrouted = task.clone();
                misrouted["context"] = serde_json::json!({});
                misrouted["pr_url"] = serde_json::json!(format!("https://github.com/mrchevyceleb/TARDIS/pull/{stray}"));
                assert_eq!(full_pr_review_pr_url(&misrouted), task["pr_url"].as_str().unwrap());
                // Reproduce the old webhook's initial row, before the coding
                // pipeline attached the stray PR. Never reopen terminal work.
                supabase::create_task(&config, &serde_json::json!({
                    "title": task["title"], "description": task["description"],
                    "status": "queued", "source": "webhook", "context": {},
                    "pr_number": stray, "repo_url": task["repo_url"],
                    "repo_path": task["repo_path"], "project": task["project"],
                })).await.unwrap();
            }
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
        let finished = loop {
            sweep_plant_full_pr_review_queue(&config).await;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let tasks = supabase::fetch_tasks(&config, None).await.unwrap();
            if tasks.as_array().unwrap().iter().all(|task| task["status"] == "done") { break tasks; }
            assert!(std::time::Instant::now() < deadline, "handoffs did not finish: {tasks}");
        };
        for task in finished.as_array().unwrap() {
            let original = format!("https://github.com/mrchevyceleb/TARDIS/pull/{}", task["pr_number"]);
            assert_eq!(task["pr_url"], original);
            assert_eq!(task["context"]["pr_url"], original);
            assert!(task["branch"].is_null());
            assert_eq!(task["review_cycle_count"].as_i64().unwrap_or(0), 0);
            let comments = supabase::fetch_comments(&config, task["id"].as_str().unwrap()).await.unwrap();
            assert!(comments.to_string().contains("No review was started."));
            println!("PASS legacy={legacy} original={original} status=done branch=null cycles=0; no model review started");
        }
        sweep_plant_full_pr_review_queue(&config).await;
        assert_eq!(supabase::fetch_tasks(&config, None).await.unwrap(), finished, "terminal handoffs must stay untouched");
    }
}
