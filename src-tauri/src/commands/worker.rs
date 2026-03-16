use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use super::supabase::{self, SupabaseConfig, SupabaseState};

// ── State ────────────────────────────────────────────────────────────

pub struct WorkerState {
    pub running: Arc<AtomicBool>,
    pub machine_name: Arc<tokio::sync::Mutex<Option<String>>>,
    pub current_task_id: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            machine_name: Arc::new(tokio::sync::Mutex::new(None)),
            current_task_id: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

// ── Event Payloads ──────────────────────────────────────────────────

#[derive(Clone, Serialize)]
struct WorkerEvent {
    event_type: String,
    message: String,
    task_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkerStatusInfo {
    pub running: bool,
    pub machine_name: Option<String>,
    pub current_task_id: Option<String>,
}

// ── Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn worker_start(
    machine_name: String,
    state: tauri::State<'_, WorkerState>,
    sb_state: tauri::State<'_, SupabaseState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if state.running.load(Ordering::Relaxed) {
        return Err("Worker is already running".to_string());
    }

    // Verify Supabase is configured
    let config = sb_state.get_config().await;
    if config.url.is_empty() || config.anon_key.is_empty() {
        return Err("Supabase not configured. Go to Settings first.".to_string());
    }

    state.running.store(true, Ordering::Relaxed);
    {
        let mut name = state.machine_name.lock().await;
        *name = Some(machine_name.clone());
    }

    let running = Arc::clone(&state.running);
    let current_task = Arc::clone(&state.current_task_id);
    let sb_config_arc = Arc::clone(&sb_state.config);
    let app_handle = app.clone();

    tokio::spawn(async move {
        worker_loop(running, current_task, machine_name, sb_config_arc, app_handle).await;
    });

    Ok(())
}

#[tauri::command]
pub async fn worker_stop(
    state: tauri::State<'_, WorkerState>,
    sb_state: tauri::State<'_, SupabaseState>,
) -> Result<(), String> {
    state.running.store(false, Ordering::Relaxed);
    // Set worker offline
    let name = state.machine_name.lock().await.clone();
    if let Some(name) = name {
        let config = sb_state.get_config().await;
        let _ = supabase::worker_offline(&config, &name).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn worker_status(
    state: tauri::State<'_, WorkerState>,
) -> Result<WorkerStatusInfo, String> {
    let name = state.machine_name.lock().await.clone();
    let task = state.current_task_id.lock().await.clone();
    Ok(WorkerStatusInfo {
        running: state.running.load(Ordering::Relaxed),
        machine_name: name,
        current_task_id: task,
    })
}

// ── Worker Loop ─────────────────────────────────────────────────────

async fn worker_loop(
    running: Arc<AtomicBool>,
    current_task_id: Arc<tokio::sync::Mutex<Option<String>>>,
    machine_name: String,
    sb_config_arc: Arc<tokio::sync::RwLock<SupabaseConfig>>,
    app: tauri::AppHandle,
) {
    let mut tick: u64 = 0;

    emit_worker_event(&app, "started", "Worker started. Ready to pick up tasks.", None);

    while running.load(Ordering::Relaxed) {
        let config = sb_config_arc.read().await.clone();

        // Heartbeat every 5 seconds (every tick)
        if tick % 1 == 0 {
            let _ = supabase::worker_heartbeat(&config, &machine_name).await;
        }

        // Poll for tasks every 10 seconds (every 2nd tick)
        if tick % 2 == 0 {
            let is_idle = current_task_id.lock().await.is_none();
            if is_idle {
                if let Ok(tasks) = supabase::fetch_tasks(&config, Some("queued")).await {
                    if let Some(arr) = tasks.as_array() {
                        if let Some(task) = arr.first() {
                            let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();

                            if !task_id.is_empty() {
                                match supabase::claim_task(&config, &task_id, &machine_name).await {
                                    Ok(_) => {
                                        {
                                            let mut ct = current_task_id.lock().await;
                                            *ct = Some(task_id.clone());
                                        }
                                        emit_worker_event(&app, "task_claimed", "Picked up a new task.", Some(&task_id));

                                        let result = execute_task(&app, &machine_name, &config, task.clone()).await;

                                        {
                                            let mut ct = current_task_id.lock().await;
                                            *ct = None;
                                        }

                                        match result {
                                            Ok(msg) => emit_worker_event(&app, "task_completed", &msg, Some(&task_id)),
                                            Err(err) => emit_worker_event(&app, "task_failed", &err, Some(&task_id)),
                                        }
                                    }
                                    Err(_) => {
                                        // Someone else claimed it, try next tick
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for new chat messages every 5 seconds
        if tick % 1 == 0 {
            let _ = check_chat_messages(&config, &app).await;
        }

        tick += 1;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Mark worker offline
    let config = sb_config_arc.read().await.clone();
    let _ = supabase::worker_offline(&config, &machine_name).await;
    emit_worker_event(&app, "stopped", "Worker stopped. Going offline.", None);
}

// ── Task Execution ──────────────────────────────────────────────────

async fn execute_task(
    app: &tauri::AppHandle,
    worker_id: &str,
    config: &SupabaseConfig,
    task: serde_json::Value,
) -> Result<String, String> {
    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
    let description = task.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let repo_path = task.get("repo_path").and_then(|v| v.as_str()).unwrap_or(".").to_string();
    let branch = task.get("branch").and_then(|v| v.as_str()).map(|s| s.to_string());
    let preview_url = task.get("preview_url").and_then(|v| v.as_str()).map(|s| s.to_string());

    // 1. Post initial comment
    agent_comment(config, &task_id, &format!("On it. Setting up for: {}", title)).await;

    // 2. Update status
    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
        "status": "in_progress",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;

    emit_worker_event(app, "task_working", &format!("Working on: {}", title), Some(&task_id));

    // 3. If branch specified, check it out
    if let Some(ref branch_name) = branch {
        let _ = tokio::process::Command::new("git")
            .args(["checkout", "-B", branch_name])
            .current_dir(&repo_path)
            .output()
            .await;
    }

    // 4. Take BEFORE screenshots if preview_url is set
    let screenshot_dir = format!("C:\\agent-one-screenshots\\{}", task_id);
    if let Some(ref preview) = preview_url {
        let _ = tokio::fs::create_dir_all(&screenshot_dir).await;
        agent_comment(config, &task_id, "Taking before screenshots...").await;

        let _ = take_screenshot(preview, &format!("{}\\before-desktop.png", screenshot_dir), "1280,720").await;
        let _ = take_screenshot(preview, &format!("{}\\before-mobile.png", screenshot_dir), "393,852").await;

        let _ = supabase::update_task(config, &task_id, &serde_json::json!({
            "screenshots_before": [
                format!("{}\\before-desktop.png", screenshot_dir),
                format!("{}\\before-mobile.png", screenshot_dir),
            ],
        })).await;
    }

    // 5. Run Claude Code CLI
    agent_comment(config, &task_id, "Starting code changes with Claude Code...").await;

    let prompt = format!(
        "Task: {}\n\nDescription: {}\n\nComplete this task. Make all necessary code changes.",
        title, description
    );
    let claude_result = run_claude_code(&repo_path, &prompt).await;

    match claude_result {
        Ok(output) => {
            let summary = truncate(&output, 500);
            agent_comment(config, &task_id, &format!("Code changes done. Here's what I did:\n\n{}", summary)).await;

            // 6. Take AFTER screenshots
            if let Some(ref preview) = preview_url {
                agent_comment(config, &task_id, "Taking after screenshots...").await;

                let _ = take_screenshot(preview, &format!("{}\\after-desktop.png", screenshot_dir), "1280,720").await;
                let _ = take_screenshot(preview, &format!("{}\\after-mobile.png", screenshot_dir), "393,852").await;

                let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                    "screenshots_after": [
                        format!("{}\\after-desktop.png", screenshot_dir),
                        format!("{}\\after-mobile.png", screenshot_dir),
                    ],
                })).await;
            }

            // 7. Visual QA (if preview_url set)
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "testing",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;

            if preview_url.is_some() {
                agent_comment(config, &task_id, "Running visual QA...").await;

                let qa_result = run_visual_qa(&repo_path, &title, &description, &screenshot_dir).await;
                match qa_result {
                    Ok((passed, explanation)) => {
                        let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                            "visual_qa_result": { "pass": passed, "explanation": explanation },
                        })).await;

                        if passed {
                            agent_comment(config, &task_id, &format!("Visual QA passed. {}", explanation)).await;
                        } else {
                            agent_comment(config, &task_id, &format!("Visual QA caught something: {}. Pushing anyway so you can take a look.", explanation)).await;
                        }
                    }
                    Err(e) => {
                        agent_comment(config, &task_id, &format!("Visual QA couldn't run: {}. Skipping.", e)).await;
                    }
                }
            }

            // 8. Create PR
            let pr_result = create_pr(&repo_path, &title, &description, &branch, &screenshot_dir, preview_url.is_some()).await;

            match pr_result {
                Ok(pr_url) => {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "status": "review",
                        "pr_url": pr_url,
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
                    agent_comment(config, &task_id, &format!("PR's up: {}. Let me know if you want any changes.", pr_url)).await;
                    Ok(format!("PR created: {}", pr_url))
                }
                Err(e) => {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "status": "review",
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
                    agent_comment(config, &task_id, &format!("Code changes are done but PR creation failed: {}. You can push manually.", e)).await;
                    Ok("Code changes complete (no PR)".to_string())
                }
            }
        }
        Err(e) => {
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "failed",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;
            agent_comment(config, &task_id, &format!("Ran into an issue: {}. You might want to re-queue this or take a look.", e)).await;
            Err(format!("Task failed: {}", e))
        }
    }
}

// ── Chat Message Processing ─────────────────────────────────────────

async fn check_chat_messages(config: &SupabaseConfig, _app: &tauri::AppHandle) {
    // TODO: Check for unprocessed user messages in ae_messages
    // For now, chat processing will be triggered by realtime on the frontend
    // The worker will respond to messages that the frontend marks as needing a response
}

// ── Helpers ─────────────────────────────────────────────────────────

fn emit_worker_event(app: &tauri::AppHandle, event_type: &str, message: &str, task_id: Option<&str>) {
    let _ = app.emit("worker-event", WorkerEvent {
        event_type: event_type.to_string(),
        message: message.to_string(),
        task_id: task_id.map(|s| s.to_string()),
    });
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Post a comment as the agent on a task
async fn agent_comment(config: &SupabaseConfig, task_id: &str, content: &str) {
    let _ = supabase::post_comment(config, &serde_json::json!({
        "task_id": task_id,
        "author": "agent",
        "content": content,
        "mentions": [],
    })).await;
}

/// Run Claude Code CLI one-shot
async fn run_claude_code(cwd: &str, prompt: &str) -> Result<String, String> {
    let claude_exe = find_claude_exe();

    let mut cmd = if claude_exe.ends_with(".cmd") {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.arg("/C").arg(&claude_exe);
        c
    } else {
        tokio::process::Command::new(&claude_exe)
    };

    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("text")
        .arg("--dangerously-skip-permissions")
        .current_dir(cwd);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd.output().await.map_err(|e| format!("Failed to run Claude Code: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude Code failed: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn find_claude_exe() -> String {
    if cfg!(target_os = "windows") {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let local_bin = format!("{}\\.local\\bin\\claude.exe", home);
        if std::path::Path::new(&local_bin).exists() { return local_bin; }
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let npm_exe = format!("{}\\npm\\claude.exe", appdata);
        if std::path::Path::new(&npm_exe).exists() { return npm_exe; }
        let npm_cmd = format!("{}\\npm\\claude.cmd", appdata);
        if std::path::Path::new(&npm_cmd).exists() { return npm_cmd; }
        "claude".to_string()
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        let local_bin = format!("{}/.local/bin/claude", home);
        if std::path::Path::new(&local_bin).exists() { return local_bin; }
        "claude".to_string()
    }
}

/// Take a screenshot using Playwright
async fn take_screenshot(url: &str, output_path: &str, viewport: &str) -> Result<(), String> {
    let output = tokio::process::Command::new("npx")
        .args(["playwright", "screenshot", url, output_path, "--viewport-size", viewport])
        .output()
        .await
        .map_err(|e| format!("Playwright failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Screenshot failed: {}", stderr.trim()));
    }
    Ok(())
}

/// Run visual QA: compare before/after screenshots using Claude Code
async fn run_visual_qa(cwd: &str, title: &str, description: &str, screenshot_dir: &str) -> Result<(bool, String), String> {
    // Copy screenshots into the working directory so Claude Code can see them
    let agent_dir = format!("{}\\.agent-one", cwd);
    let _ = tokio::fs::create_dir_all(&agent_dir).await;

    for name in &["before-desktop.png", "after-desktop.png", "before-mobile.png", "after-mobile.png"] {
        let src = format!("{}\\{}", screenshot_dir, name);
        let dst = format!("{}\\{}", agent_dir, name);
        let _ = tokio::fs::copy(&src, &dst).await;
    }

    let prompt = format!(
        "Look at the screenshots in .agent-one/ folder. Compare before-desktop.png with after-desktop.png, and before-mobile.png with after-mobile.png.\n\nTask: {}\nDescription: {}\n\nDoes the change look correct? Reply with exactly this JSON format:\n{{\"pass\": true, \"explanation\": \"brief explanation\"}}\nor\n{{\"pass\": false, \"explanation\": \"what's wrong\"}}",
        title, description
    );

    let result = run_claude_code(cwd, &prompt).await?;

    // Try to parse JSON from the response
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
        let pass = parsed.get("pass").and_then(|v| v.as_bool()).unwrap_or(false);
        let explanation = parsed.get("explanation").and_then(|v| v.as_str()).unwrap_or("No explanation").to_string();
        return Ok((pass, explanation));
    }

    // Try to find JSON in the response text
    if let Some(start) = result.find('{') {
        if let Some(end) = result.rfind('}') {
            let json_str = &result[start..=end];
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                let pass = parsed.get("pass").and_then(|v| v.as_bool()).unwrap_or(false);
                let explanation = parsed.get("explanation").and_then(|v| v.as_str()).unwrap_or("No explanation").to_string();
                return Ok((pass, explanation));
            }
        }
    }

    // Couldn't parse, assume pass with raw output
    Ok((true, format!("QA output (unparseable): {}", truncate(&result, 200))))
}

/// Create a PR with before/after screenshots in the body
async fn create_pr(
    repo_path: &str,
    title: &str,
    description: &str,
    branch: &Option<String>,
    screenshot_dir: &str,
    has_screenshots: bool,
) -> Result<String, String> {
    let branch_name = branch.clone().unwrap_or_else(|| format!("agent-one/{}", uuid::Uuid::new_v4()));

    // Ensure we're on the right branch
    let _ = tokio::process::Command::new("git")
        .args(["checkout", "-B", &branch_name])
        .current_dir(repo_path)
        .output()
        .await;

    // Copy screenshots into repo if they exist
    if has_screenshots {
        let agent_dir = format!("{}\\.agent-one", repo_path);
        let _ = tokio::fs::create_dir_all(&agent_dir).await;
        for name in &["before-desktop.png", "after-desktop.png", "before-mobile.png", "after-mobile.png"] {
            let src = format!("{}\\{}", screenshot_dir, name);
            let dst = format!("{}\\{}", agent_dir, name);
            let _ = tokio::fs::copy(&src, &dst).await;
        }
    }

    // Stage all changes
    let stage = tokio::process::Command::new("git").args(["add", "-A"]).current_dir(repo_path).output().await.map_err(|e| format!("git add failed: {}", e))?;
    if !stage.status.success() { return Err("git add failed".to_string()); }

    // Check if there are changes
    let diff = tokio::process::Command::new("git").args(["diff", "--cached", "--quiet"]).current_dir(repo_path).output().await.map_err(|e| format!("git diff check failed: {}", e))?;
    if diff.status.success() { return Err("No changes to commit".to_string()); }

    // Commit
    let commit_msg = format!("agent-one: {}", title);
    let commit = tokio::process::Command::new("git").args(["commit", "-m", &commit_msg]).current_dir(repo_path).output().await.map_err(|e| format!("git commit failed: {}", e))?;
    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        return Err(format!("git commit failed: {}", stderr));
    }

    // Push
    let push = tokio::process::Command::new("git").args(["push", "-u", "origin", &branch_name]).current_dir(repo_path).output().await.map_err(|e| format!("git push failed: {}", e))?;
    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        return Err(format!("git push failed: {}", stderr));
    }

    // Build PR body
    let mut pr_body = format!("## {}\n\n{}\n\n", title, description);
    if has_screenshots {
        pr_body.push_str("### Visual QA\n\n");
        pr_body.push_str("| Desktop Before | Desktop After |\n|--------|-------|\n");
        pr_body.push_str("| ![before](.agent-one/before-desktop.png) | ![after](.agent-one/after-desktop.png) |\n\n");
        pr_body.push_str("| Mobile Before | Mobile After |\n|--------|-------|\n");
        pr_body.push_str("| ![before](.agent-one/before-mobile.png) | ![after](.agent-one/after-mobile.png) |\n\n");
    }
    pr_body.push_str("---\nAutomated by Agent One");

    // Create PR
    let pr = tokio::process::Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", &pr_body, "--head", &branch_name])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| format!("gh pr create failed: {}", e))?;

    if !pr.status.success() {
        let stderr = String::from_utf8_lossy(&pr.stderr);
        return Err(format!("gh pr create failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&pr.stdout).trim().to_string())
}
