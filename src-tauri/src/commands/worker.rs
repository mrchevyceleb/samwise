use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};

use super::dev_server;
use super::review;
use super::supabase::{self, SupabaseConfig, SupabaseState};
use crate::process::async_cmd;

// ── State ────────────────────────────────────────────────────────────

pub struct WorkerState {
    pub running: Arc<AtomicBool>,
    pub machine_name: Arc<tokio::sync::Mutex<Option<String>>>,
    pub current_task_id: Arc<tokio::sync::Mutex<Option<String>>>,
    pub last_telegram_update_id: Arc<tokio::sync::Mutex<Option<i64>>>,
    /// PID of the currently running Claude Code process (for stop functionality)
    pub current_process_id: Arc<tokio::sync::Mutex<Option<u32>>>,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            machine_name: Arc::new(tokio::sync::Mutex::new(None)),
            current_task_id: Arc::new(tokio::sync::Mutex::new(None)),
            last_telegram_update_id: Arc::new(tokio::sync::Mutex::new(None)),
            current_process_id: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

fn join_path(dir: &str, name: &str) -> String {
    std::path::Path::new(dir).join(name).to_string_lossy().into_owned()
}

const MERGE_DEPLOY_REQUESTED_AT_KEY: &str = "samwise_merge_deploy_requested_at";
const MERGE_DEPLOY_STARTED_AT_KEY: &str = "samwise_merge_deploy_started_at";
const MERGE_DEPLOY_COMPLETED_AT_KEY: &str = "samwise_merge_deploy_completed_at";
const MERGE_DEPLOY_STATUS_KEY: &str = "samwise_merge_deploy_status";
const MERGE_DEPLOY_ERROR_KEY: &str = "samwise_merge_deploy_error";
const MERGE_DEPLOY_PLAN_KEY: &str = "samwise_merge_deploy_plan";

/// External edge function that closes the upstream ticket (Operly triage,
/// Banana triage, Sentry issue) after Sam ships and the deploy is green.
/// Lives outside the Samwise Supabase project; URL is the source of truth.
const CLOSE_ORIGIN_TICKET_URL: &str =
    "https://iycloielqcjnjqddeuet.supabase.co/functions/v1/close-origin-ticket";

async fn run_git(args: &[&str], repo_path: &str) -> Result<String, String> {
    let output = async_cmd("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| format!("git {:?}: {}", args, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {:?}: {}", args, stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn detect_default_branch(repo_path: &str) -> String {
    if let Ok(out) = run_git(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"], repo_path).await {
        if let Some(name) = out.rsplit('/').next() {
            if !name.is_empty() { return name.to_string(); }
        }
    }
    // Fall back to main, then master.
    for candidate in ["main", "master"] {
        if run_git(&["rev-parse", "--verify", &format!("origin/{}", candidate)], repo_path).await.is_ok() {
            return candidate.to_string();
        }
    }
    "main".to_string()
}

/// Returns true if the repo has any evidence of work: uncommitted changes, staged
/// changes, untracked files, or new commits on the current branch vs the base branch.
async fn worker_made_changes(repo_path: &str) -> bool {
    if let Ok(out) = run_git(&["status", "--porcelain"], repo_path).await {
        if !out.trim().is_empty() { return true; }
    }
    let base = detect_default_branch(repo_path).await;
    if let Ok(count) = run_git(&["rev-list", "--count", &format!("origin/{}..HEAD", base)], repo_path).await {
        if count.trim().parse::<u32>().unwrap_or(0) > 0 { return true; }
    }
    false
}

/// Path where Sam keeps his worktrees, one subdirectory per repo, one leaf per task.
fn worktrees_root() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("samwise")
        .join("worktrees")
}

/// Returns the short form of a task id used for worktree paths and branch names.
fn short_task_id(task_id: &str) -> String {
    task_id.chars().filter(|c| c.is_ascii_hexdigit()).take(8).collect()
}

/// Compute the task branch name from a short id. Single source of truth so sweep
/// and task-create agree.
fn task_branch_name(short_id: &str) -> String {
    format!("sam/{}", short_id)
}

/// Fetch latest origin state into Matt's main repo, then create a worktree for Sam
/// to work in. Matt's checkout is never touched. Returns (worktree_path, base_branch,
/// task_branch).
///
/// `base_branch_override`: when Some, use that branch as the base instead of the
/// repo's default (main/master). Used when a task wants to stack on a feature branch.
/// If the requested base doesn't exist on origin, returns Err so the caller can set
/// the task to pending_confirmation.
///
/// If a worktree for this task already exists (follow-up work on an open PR), reuse
/// it rather than failing. The caller is expected to have git fetch'd fresh refs.
async fn create_task_worktree(
    main_repo_path: &str,
    task_id: &str,
    base_branch_override: Option<&str>,
) -> Result<(String, String, String), String> {
    if tokio::fs::metadata(main_repo_path).await.is_err() {
        return Err(format!("repo_path does not exist: {}", main_repo_path));
    }
    if tokio::fs::metadata(join_path(main_repo_path, ".git")).await.is_err() {
        return Err(format!("repo_path is not a git repo: {}", main_repo_path));
    }

    run_git(&["fetch", "origin", "--prune"], main_repo_path).await?;
    let base_branch = match base_branch_override {
        Some(b) if !b.trim().is_empty() => {
            let b = b.trim().to_string();
            // Verify origin/<b> resolves before proceeding.
            if run_git(&["rev-parse", "--verify", &format!("origin/{}", b)], main_repo_path).await.is_err() {
                return Err(format!("base branch `origin/{}` doesn't exist on remote. Push it or pick a different base.", b));
            }
            b
        }
        _ => detect_default_branch(main_repo_path).await,
    };

    let repo_name = std::path::Path::new(main_repo_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".to_string());
    let short_id = short_task_id(task_id);
    let task_branch = task_branch_name(&short_id);
    let worktree_path = worktrees_root().join(&repo_name).join(&short_id);
    let worktree_str = worktree_path.to_string_lossy().into_owned();

    if let Some(parent) = worktree_path.parent() {
        tokio::fs::create_dir_all(parent).await
            .map_err(|e| format!("create worktree parent dir: {}", e))?;
    }

    // Existing worktree for this task (follow-up task on an open PR)? Reuse it.
    if tokio::fs::metadata(&worktree_path).await.is_ok() {
        // Make sure it really is a git worktree and the branch matches.
        if run_git(&["rev-parse", "--git-dir"], &worktree_str).await.is_ok() {
            let _ = run_git(&["checkout", &task_branch], &worktree_str).await;
            let _ = run_git(&["fetch", "origin", "--prune"], &worktree_str).await;
            return Ok((worktree_str, base_branch, task_branch));
        }
        // Directory is there but it isn't a worktree (stale junk). Blow it away.
        let _ = tokio::fs::remove_dir_all(&worktree_path).await;
    }

    // Create a fresh worktree off origin/<base>. Using `--force` lets us recover from
    // a dangling worktree registration where the dir is gone but git still lists it.
    let origin_ref = format!("origin/{}", base_branch);
    run_git(
        &[
            "worktree", "add", "--force",
            "-b", &task_branch,
            &worktree_str,
            &origin_ref,
        ],
        main_repo_path,
    ).await?;

    Ok((worktree_str, base_branch, task_branch))
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

/// Backend-only worker bootstrap. Runs at app startup so task pickup is
/// independent of whether the Tauri window ever hydrates. Safe to call even
/// if the worker is already running or Supabase isn't configured (no-op in
/// both cases).
/// Read settings.json and populate the in-memory SupabaseState if it is
/// empty. Called synchronously during app setup() so the Tauri frontend sees
/// a populated config on its very first `supabase_get_config` invoke.
pub async fn hydrate_supabase_from_disk(app: &tauri::AppHandle) {
    use tauri::Manager;
    let sb_state: tauri::State<'_, SupabaseState> = app.state();
    let config = sb_state.get_config().await;
    if !config.url.is_empty() && !config.anon_key.is_empty() {
        return;
    }
    if let Some(loaded) = load_supabase_config_from_disk(app).await {
        eprintln!(
            "[hydrate] loaded Supabase config from settings.json (url_len={}, anon_len={})",
            loaded.url.len(), loaded.anon_key.len()
        );
        let mut w = sb_state.config.write().await;
        *w = loaded;
    } else {
        eprintln!("[hydrate] no Supabase config in settings.json; frontend will need Settings modal");
    }
}

pub async fn autostart_worker(app: tauri::AppHandle) {
    use tauri::Manager;

    let worker_state: tauri::State<'_, WorkerState> = app.state();
    if worker_state.running.load(Ordering::Relaxed) {
        eprintln!("[worker autostart] already running, skipping");
        return;
    }

    let sb_state: tauri::State<'_, SupabaseState> = app.state();
    let config = sb_state.get_config().await;

    let machine_name = std::env::var("SAMWISE_MACHINE_NAME")
        .unwrap_or_else(|_| hostname_or_default());

    worker_state.running.store(true, Ordering::Relaxed);
    {
        let mut name = worker_state.machine_name.lock().await;
        *name = Some(machine_name.clone());
    }

    let running = Arc::clone(&worker_state.running);
    let current_task = Arc::clone(&worker_state.current_task_id);
    let last_tg_update = Arc::clone(&worker_state.last_telegram_update_id);
    let current_pid = Arc::clone(&worker_state.current_process_id);
    let sb_config_arc = Arc::clone(&sb_state.config);
    let app_handle = app.clone();

    log::info!("[worker] autostart: launching worker_loop as {}", machine_name);
    tokio::spawn(async move {
        worker_loop(running, current_task, last_tg_update, current_pid, machine_name, sb_config_arc, app_handle).await;
    });
}

async fn load_supabase_config_from_disk(app: &tauri::AppHandle) -> Option<SupabaseConfig> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().ok()?;
    let path = dir.join("settings.json");
    let raw = tokio::fs::read_to_string(&path).await.ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let get_str = |k: &str| -> String {
        v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
    };
    let url = get_str("supabaseUrl");
    let anon = get_str("supabaseAnonKey");
    let service = {
        let s = get_str("supabaseServiceRoleKey");
        if s.is_empty() { None } else { Some(s) }
    };
    if url.is_empty() || anon.is_empty() { return None; }
    Some(SupabaseConfig {
        url,
        anon_key: anon,
        service_role_key: service,
        telegram_bot_token: {
            let t = get_str("telegramBotToken");
            if t.is_empty() { None } else { Some(t) }
        },
        telegram_chat_id: {
            let t = get_str("telegramChatId");
            if t.is_empty() { None } else { Some(t) }
        },
    })
}

fn hostname_or_default() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "agent-one".to_string())
}

#[tauri::command]
pub async fn worker_start(
    machine_name: String,
    state: tauri::State<'_, WorkerState>,
    sb_state: tauri::State<'_, SupabaseState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Idempotent: if the backend already autostarted the worker, treat a
    // second call (from frontend onMount) as success so the UI can surface
    // the live status instead of a confusing error toast.
    if state.running.load(Ordering::Relaxed) {
        log::info!("[worker] worker_start called but worker already running (likely autostarted); no-op");
        return Ok(());
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
    let last_tg_update = Arc::clone(&state.last_telegram_update_id);
    let current_pid = Arc::clone(&state.current_process_id);
    let sb_config_arc = Arc::clone(&sb_state.config);
    let app_handle = app.clone();

    tokio::spawn(async move {
        worker_loop(running, current_task, last_tg_update, current_pid, machine_name, sb_config_arc, app_handle).await;
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

/// Stop the currently running task by killing its Claude Code process.
/// The task is marked as "failed" with a comment explaining it was manually stopped.
#[tauri::command]
pub async fn stop_current_task(
    state: tauri::State<'_, WorkerState>,
    sb_state: tauri::State<'_, SupabaseState>,
) -> Result<String, String> {
    let task_id = state.current_task_id.lock().await.clone();
    let Some(task_id) = task_id else {
        return Err("No task is currently running".to_string());
    };

    // Kill the Claude Code process
    let pid = state.current_process_id.lock().await.take();
    if let Some(pid) = pid {
        #[cfg(target_os = "windows")]
        {
            // On Windows, kill the process tree (claude spawns child processes)
            let _ = async_cmd("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .output()
                .await;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = async_cmd("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .await;
        }
        log::info!("[worker] Killed Claude Code process {} for task {}", pid, task_id);
    }

    // Mark task as failed with explanation
    let config = sb_state.get_config().await;
    let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
        "status": "failed",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;
    agent_comment(&config, &task_id, "Task stopped manually.").await;

    Ok(task_id)
}

/// Restart a failed/stopped task by setting it back to queued.
#[tauri::command]
pub async fn restart_task(
    task_id: String,
    sb_state: tauri::State<'_, SupabaseState>,
) -> Result<(), String> {
    let config = sb_state.get_config().await;
    let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
        "status": "queued",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;
    agent_comment(&config, &task_id, "Task restarted. Back in the queue.").await;
    Ok(())
}

// ── Worker Loop ─────────────────────────────────────────────────────

async fn worker_loop(
    running: Arc<AtomicBool>,
    current_task_id: Arc<tokio::sync::Mutex<Option<String>>>,
    last_telegram_update_id: Arc<tokio::sync::Mutex<Option<i64>>>,
    current_process_id: Arc<tokio::sync::Mutex<Option<u32>>>,
    machine_name: String,
    sb_config_arc: Arc<tokio::sync::RwLock<SupabaseConfig>>,
    app: tauri::AppHandle,
) {
    let mut tick: u64 = 0;
    let mut idle_ticks: u64 = 0; // Track how long the worker has been idle

    emit_worker_event(&app, "started", "Worker started. Ready to pick up tasks.", None);

    // Greet Matt on startup
    {
        let config = sb_config_arc.read().await.clone();
        agent_chat(&config, "Hey, Sam here. I'm online and ready. Drop a task or just tell me what you need.").await;
    }

    // Recover tasks stuck in `in_progress` from a prior crash. Samwise is
    // single-active (one worker row, enforced by heartbeat), so any in_progress
    // row at startup is orphaned and safe to re-queue.
    {
        let config = sb_config_arc.read().await.clone();
        let recovered = recover_stuck_tasks(&config).await;
        if recovered > 0 {
            log::info!("[worker] startup recovered {} stuck task(s)", recovered);
            agent_chat(&config, &format!(
                "Picked up {} task{} that got stuck mid-run before I came back online. Re-queued.",
                recovered, if recovered == 1 { "" } else { "s" }
            )).await;
        }
    }

    // Sweep merged/closed PR worktrees on startup, and periodically thereafter.
    // Tick is ~5s; 4320 ticks = 6h cadence.
    const SWEEP_TICKS: u64 = 4320;
    {
        let config = sb_config_arc.read().await.clone();
        let (removed, kept) = sweep_worktrees_with_config(&config).await;
        if removed > 0 {
            log::info!("[worker] startup sweep removed {} worktree(s), kept {}", removed, kept);
            agent_chat(&config, &format!(
                "Cleaned up {} worktree{} whose PRs were merged/closed or tasks failed while I was away. {} still in flight.",
                removed, if removed == 1 { "" } else { "s" }, kept
            )).await;
        }
    }

    while running.load(Ordering::Relaxed) {
        let config = sb_config_arc.read().await.clone();

        // Heartbeat every tick
        let _ = supabase::worker_heartbeat(&config, &machine_name).await;

        // Periodic worktree sweep (every 6h). Only runs when idle to avoid racing
        // with an in-flight task touching the same branch/worktree.
        if tick > 0 && tick % SWEEP_TICKS == 0 {
            let is_idle = current_task_id.lock().await.is_none();
            if is_idle {
                let (removed, kept) = sweep_worktrees_with_config(&config).await;
                if removed > 0 {
                    log::info!("[worker] periodic sweep removed {} worktree(s), kept {}", removed, kept);
                }
            }
        }

        // Poll for tasks every 10 seconds (every 2nd tick)
        if tick % 2 == 0 {
            let is_idle = current_task_id.lock().await.is_none();
            if is_idle {
                idle_ticks += 1;

                // Proactive idle messages (every ~5 min = 60 ticks at 5s each)
                if idle_ticks == 60 {
                    agent_chat(&config, "Been idle for a few minutes. Got anything for me? I can pick up coding tasks, run reviews, or just chat.").await;
                }
                if idle_ticks == 360 {
                    // 30 min idle
                    agent_chat(&config, "Still here, still idle. Queue's empty. Let me know when you've got something.").await;
                }

                if let Ok(tasks) = supabase::fetch_tasks(&config, Some("queued")).await {
                    if let Some(arr) = tasks.as_array() {
                        // Sort by priority: critical=0, high=1, medium=2, low=3, then created_at asc
                        let priority_order = |p: &str| match p {
                            "critical" => 0u8,
                            "high" => 1,
                            "medium" => 2,
                            "low" => 3,
                            _ => 4,
                        };
                        let mut sorted = arr.clone();
                        sorted.sort_by(|a, b| {
                            let pa = priority_order(a.get("priority").and_then(|v| v.as_str()).unwrap_or("medium"));
                            let pb = priority_order(b.get("priority").and_then(|v| v.as_str()).unwrap_or("medium"));
                            pa.cmp(&pb).then_with(|| {
                                let ta = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                let tb = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                ta.cmp(tb)
                            })
                        });
                        if let Some(task) = sorted.first() {
                            let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            let task_title = task.get("title").and_then(|v| v.as_str()).unwrap_or("a task").to_string();

                            if !task_id.is_empty() {
                                match supabase::claim_task(&config, &task_id, &machine_name).await {
                                    Ok(_) => {
                                        idle_ticks = 0; // Reset idle counter
                                        {
                                            let mut ct = current_task_id.lock().await;
                                            *ct = Some(task_id.clone());
                                        }
                                        emit_worker_event(&app, "task_claimed", "Picked up a new task.", Some(&task_id));

                                        // Proactive chat: tell Matt what we're doing
                                        agent_chat(&config, &format!(
                                            "Picked up \"{}\" from the queue. I'll post updates as I go.", task_title
                                        )).await;

                                        let result = execute_task(&app, &machine_name, &config, task.clone(), current_process_id.clone()).await;

                                        {
                                            let mut ct = current_task_id.lock().await;
                                            *ct = None;
                                        }

                                        match &result {
                                            Ok(msg) => {
                                                emit_worker_event(&app, "task_completed", msg, Some(&task_id));
                                                // Proactive chat: announce completion.
                                                // If a Codex review was just kicked off on the freshly-opened PR,
                                                // don't ask "want me to pick up something else?" — the card is
                                                // still being reviewed async. Signal that instead.
                                                let completion_settings: Option<serde_json::Value> = if let Ok(data_dir) = app.path().app_data_dir() {
                                                    let p = data_dir.join("settings.json");
                                                    tokio::fs::read_to_string(&p).await.ok().and_then(|s| serde_json::from_str(&s).ok())
                                                } else { None };
                                                let auto_merge_on = completion_settings.as_ref()
                                                    .and_then(|s| s.get("autoMergeEnabled"))
                                                    .and_then(|v| v.as_bool()).unwrap_or(false);
                                                let pr_review_on = completion_settings.as_ref()
                                                    .and_then(|s| s.get("autoPrReviewEnabled"))
                                                    .and_then(|v| v.as_bool()).unwrap_or(true);

                                                if msg.contains("PR created") && pr_review_on && !auto_merge_on {
                                                    agent_chat(&config, &format!(
                                                        "PR's up for \"{}\": {}. Running Codex review now. I'll post the verdict and route the card in a minute — no need to pick up something new yet.",
                                                        task_title, msg
                                                    )).await;
                                                } else if msg.contains("PR created") {
                                                    agent_chat(&config, &format!(
                                                        "Done with \"{}\". {} Want me to pick up something else?", task_title, msg
                                                    )).await;
                                                } else {
                                                    agent_chat(&config, &format!(
                                                        "Finished \"{}\". {} Anything else?", task_title, msg
                                                    )).await;
                                                }
                                            }
                                            Err(err) => {
                                                emit_worker_event(&app, "task_failed", err, Some(&task_id));
                                                // Proactive chat: explain failure
                                                agent_chat(&config, &format!(
                                                    "Ran into trouble on \"{}\": {}. You might want to take a look or re-queue it.",
                                                    task_title, truncate(err, 200)
                                                )).await;
                                            }
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
            } else {
                idle_ticks = 0; // Working on something, reset idle counter
            }
        }

        // Evaluate crons every 60 seconds (every 12th tick)
        if tick % 12 == 0 {
            let _ = evaluate_crons(&config, &app).await;
        }

        // Evaluate triggers every 30 seconds (every 6th tick)
        if tick % 6 == 0 {
            let _ = evaluate_triggers(&config, &app).await;
        }

        // Check Telegram messages every 50 seconds (every 10th tick)
        if tick % 10 == 0 {
            check_telegram_messages(&config, &last_telegram_update_id, &machine_name).await;
        }

        // Check remote chat messages every 10 seconds (every 2nd tick)
        if tick % 2 == 0 {
            check_remote_chat_messages(&config, &machine_name).await;
        }

        // Expire pending_confirmation tasks older than 30 min (every 5 min, skip startup)
        if tick > 0 && tick % 60 == 0 {
            expire_pending_confirmations(&config).await;
        }

        // Sweep the PR-review queue every ~30s (6 ticks). Picks up cards that
        // just entered review (fresh PRs) and cards you dragged back from
        // fixes_needed -> review so they get re-reviewed automatically.
        if tick % 6 == 0 {
            let settings: Option<serde_json::Value> = if let Ok(data_dir) = app.path().app_data_dir() {
                let settings_path = data_dir.join("settings.json");
                tokio::fs::read_to_string(&settings_path).await
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
            } else { None };
            sweep_pr_review_queue(&config, &settings).await;
        }

        // Pick up Merge + Deploy requests from either the desktop UI or the
        // web UI. The browser only writes a context flag; this local worker
        // owns GitHub/Railway/Supabase credentials and executes the workflow.
        if tick % 2 == 0 {
            sweep_merge_deploy_requests(&config).await;
        }

        // Sweep approved/review/fixes_needed cards whose GitHub PRs got merged
        // or closed outside Sam's pipeline. Merged PRs run the same post-merge
        // deploy plan before the card moves to Done. Every ~60s (12 ticks).
        if tick % 12 == 0 {
            sweep_pr_merged_cards(&config).await;
        }

        tick += 1;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Mark worker offline
    let config = sb_config_arc.read().await.clone();
    let _ = supabase::worker_offline(&config, &machine_name).await;
    emit_worker_event(&app, "stopped", "Worker stopped. Going offline.", None);
}

// ── Cron Evaluation ─────────────────────────────────────────────────

async fn evaluate_crons(config: &SupabaseConfig, app: &tauri::AppHandle) -> Result<(), String> {
    let crons = supabase::fetch_crons(config).await?;
    let Some(arr) = crons.as_array() else { return Ok(()); };

    let now = chrono::Utc::now();

    for cron_entry in arr {
        let enabled = cron_entry.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        if !enabled { continue; }

        let cron_id = cron_entry.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let schedule_str = cron_entry.get("schedule").and_then(|v| v.as_str()).unwrap_or_default();
        let cron_name = cron_entry.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed cron");

        // Parse next_run (if set)
        let next_run = cron_entry.get("next_run")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        // If next_run is in the future, skip
        if let Some(nr) = next_run {
            if nr > now { continue; }
        }

        // Parse cron schedule - convert 5-field standard cron to 7-field (sec min hour dom month dow year)
        let cron_expr = {
            let parts: Vec<&str> = schedule_str.trim().split_whitespace().collect();
            match parts.len() {
                5 => format!("0 {} *", schedule_str),  // standard 5-field: prepend sec=0, append year=*
                6 => format!("0 {}", schedule_str),     // 6-field (with year): prepend sec=0
                7 => schedule_str.to_string(),           // already 7-field
                _ => schedule_str.to_string(),           // let parser handle invalid
            }
        };
        let schedule = match cron_expr.parse::<cron::Schedule>() {
            Ok(s) => s,
            Err(e) => {
                log::warn!("[worker] Invalid cron schedule '{}' for '{}': {}", schedule_str, cron_name, e);
                continue;
            }
        };

        // Get task template
        let template = match cron_entry.get("task_template") {
            Some(t) if t.is_object() => t.clone(),
            _ => {
                log::warn!("[worker] Cron '{}' has no valid task_template", cron_name);
                continue;
            }
        };

        // Create task from template
        let mut task = template.clone();
        if let Some(obj) = task.as_object_mut() {
            obj.insert("source".to_string(), serde_json::json!("cron"));
            obj.insert("cron_id".to_string(), serde_json::json!(cron_id));
            if !obj.contains_key("status") {
                obj.insert("status".to_string(), serde_json::json!("queued"));
            }
            if !obj.contains_key("priority") {
                obj.insert("priority".to_string(), serde_json::json!("medium"));
            }
        }

        match supabase::create_task(config, &task).await {
            Ok(_) => {
                log::info!("[worker] Cron '{}' created task", cron_name);
                emit_worker_event(app, "cron_fired", &format!("Cron '{}' created a new task", cron_name), None);
            }
            Err(e) => {
                log::error!("[worker] Cron '{}' failed to create task: {}", cron_name, e);
            }
        }

        // Compute next run from schedule
        let next = schedule.upcoming(chrono::Utc).next();
        let next_run_str = next.map(|dt| dt.to_rfc3339());

        // Update cron last_run and next_run
        let update = serde_json::json!({
            "last_run": now.to_rfc3339(),
            "next_run": next_run_str,
        });
        let _ = supabase::update_cron(config, cron_id, &update).await;
    }

    Ok(())
}

// ── URL Normalization ───────────────────────────────────────────────

/// Normalize a GitHub repo URL for comparison: lowercase, strip .git suffix and trailing slashes.
fn normalize_repo_url(url: &str) -> String {
    url.trim().to_lowercase()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/')
        .to_string()
}

// ── Trigger Evaluation ──────────────────────────────────────────────

async fn evaluate_triggers(config: &super::supabase::SupabaseConfig, app: &tauri::AppHandle) -> Result<(), String> {
    let triggers = supabase::fetch_triggers(config).await?;
    let Some(arr) = triggers.as_array() else { return Ok(()); };

    // Fetch projects once for all triggers (avoids N+1 queries per event)
    let cached_projects = supabase::fetch_projects(config).await.ok();

    for trigger in arr {
        let enabled = trigger.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        if !enabled { continue; }

        let trigger_id = trigger.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let trigger_name = trigger.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed trigger");
        let _source_type = trigger.get("source_type").and_then(|v| v.as_str()).unwrap_or_default();

        // Check for unprocessed trigger events
        let events = match supabase::fetch_trigger_events(config, trigger_id).await {
            Ok(e) => e,
            Err(e) => {
                // Table might not exist yet, just skip silently
                log::debug!("[worker] Trigger event fetch failed for '{}': {}", trigger_name, e);
                continue;
            }
        };

        let Some(event_arr) = events.as_array() else { continue; };

        for event in event_arr {
            let event_id = event.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let payload = event.get("payload").cloned().unwrap_or(serde_json::json!({}));

            // Get task template and merge with event payload
            let template = match trigger.get("task_template") {
                Some(t) if t.is_object() => t.clone(),
                _ => {
                    log::warn!("[worker] Trigger '{}' has no valid task_template", trigger_name);
                    continue;
                }
            };

            let mut task = template.clone();
            if let Some(obj) = task.as_object_mut() {
                obj.insert("source".to_string(), serde_json::json!("trigger"));
                obj.insert("trigger_id".to_string(), serde_json::json!(trigger_id));
                if !obj.contains_key("status") {
                    obj.insert("status".to_string(), serde_json::json!("queued"));
                }

                // Allow payload to override template fields (string values only)
                if let Some(payload_obj) = payload.as_object() {
                    for field in &["title", "description", "priority", "project", "repo_url"] {
                        if let Some(val) = payload_obj.get(*field) {
                            if val.is_string() {
                                obj.insert(field.to_string(), val.clone());
                            } else {
                                log::warn!("[worker] Trigger '{}': ignoring non-string payload field '{}' = {}", trigger_name, field, val);
                            }
                        }
                    }
                }

                // Resolve repo_url -> project registry fields (repo_path, preview_url, project name)
                let task_repo_url = obj.get("repo_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let task_project = obj.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if !task_repo_url.is_empty() && task_project.is_empty() {
                    if let Some(ref projects) = cached_projects {
                        if let Some(proj_arr) = projects.as_array() {
                            let normalized = normalize_repo_url(&task_repo_url);
                            if let Some(proj) = proj_arr.iter().find(|p| {
                                let purl = p.get("repo_url").and_then(|v| v.as_str()).unwrap_or("");
                                normalize_repo_url(purl) == normalized
                            }) {
                                if let Some(name) = proj.get("name").and_then(|v| v.as_str()) {
                                    obj.insert("project".to_string(), serde_json::json!(name));
                                }
                                for field in &["repo_path", "preview_url"] {
                                    if let Some(v) = proj.get(*field).filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                                        obj.insert(field.to_string(), v.clone());
                                    }
                                }
                            } else {
                                log::warn!("[worker] Trigger '{}': repo_url '{}' not found in project registry", trigger_name, task_repo_url);
                                obj.insert("status".to_string(), serde_json::json!("pending_confirmation"));
                            }
                        }
                    }
                }

                if !obj.contains_key("priority") {
                    obj.insert("priority".to_string(), serde_json::json!("medium"));
                }
                // Merge event payload into task context
                obj.insert("context".to_string(), payload);
            }

            match supabase::create_task(config, &task).await {
                Ok(_) => {
                    log::info!("[worker] Trigger '{}' created task from event {}", trigger_name, event_id);
                    emit_worker_event(app, "trigger_fired", &format!("Trigger '{}' created a new task", trigger_name), None);
                    // Only mark processed on success - failed events retry next cycle
                    let _ = supabase::mark_trigger_event_processed(config, event_id).await;
                }
                Err(e) => {
                    log::error!("[worker] Trigger '{}' failed to create task: {}", trigger_name, e);
                }
            }
        }

        // Update last_checked on the trigger
        let now = chrono::Utc::now();
        let _ = supabase::update_trigger(config, trigger_id, &serde_json::json!({
            "last_checked": now.to_rfc3339(),
        })).await;
    }

    Ok(())
}

// ── Task Execution ──────────────────────────────────────────────────

async fn execute_task(
    app: &tauri::AppHandle,
    worker_id: &str,
    config: &SupabaseConfig,
    task: serde_json::Value,
    process_id_slot: Arc<tokio::sync::Mutex<Option<u32>>>,
) -> Result<String, String> {
    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
    let description = task.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let task_type = task.get("task_type").and_then(|v| v.as_str()).unwrap_or("code").to_string();
    let is_research = task_type == "research";

    // Read settings.json once and cache for both notification prefs and worker rules
    let cached_settings: Option<serde_json::Value> = if let Ok(data_dir) = app.path().app_data_dir() {
        let settings_path = data_dir.join("settings.json");
        if let Ok(settings_json) = tokio::fs::read_to_string(&settings_path).await {
            serde_json::from_str::<serde_json::Value>(&settings_json).ok()
        } else { None }
    } else { None };

    // Extract notification preferences (default to true if settings unavailable)
    let mut notify_task_started = true;
    let mut notify_task_completed_code = true;
    let mut notify_task_completed_research = true;
    let mut notify_task_failed = true;
    if let Some(ref settings) = cached_settings {
        let master_enabled = settings.get("telegramNotificationsEnabled")
            .and_then(|v| v.as_bool()).unwrap_or(true);
        if !master_enabled {
            notify_task_started = false;
            notify_task_completed_code = false;
            notify_task_completed_research = false;
            notify_task_failed = false;
        } else {
            notify_task_started = settings.get("telegramNotifyTaskStarted")
                .and_then(|v| v.as_bool()).unwrap_or(true);
            notify_task_completed_code = settings.get("telegramNotifyTaskCompletedCode")
                .and_then(|v| v.as_bool()).unwrap_or(true);
            notify_task_completed_research = settings.get("telegramNotifyTaskCompletedResearch")
                .and_then(|v| v.as_bool()).unwrap_or(true);
            notify_task_failed = settings.get("telegramNotifyTaskFailed")
                .and_then(|v| v.as_bool()).unwrap_or(true);
        }
    }

    // Resolve repo_path and preview_url: if task has a project name but no repo_path,
    // look it up from the ae_projects registry. Tasks created from chat often only have
    // a project name and no paths, which previously defaulted to "." (the Tauri process
    // directory), causing Claude Code to run in the wrong location.
    let mut repo_path = task.get("repo_path").and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && *s != ".")
        .map(|s| s.to_string());
    let mut preview_url = task.get("preview_url").and_then(|v| v.as_str()).map(|s| s.to_string());

    let mut project_name = task.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let task_repo_url = task.get("repo_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut project_dev_command: Option<String> = None;

    // Resolve project from registry: by project name, or by repo_url if no name is set
    if repo_path.is_none() || preview_url.is_none() || project_name.is_empty() {
        if let Ok(projects) = supabase::fetch_projects(config).await {
            if let Some(arr) = projects.as_array() {
                // Try matching by project name first, then fall back to repo_url
                let matched_proj = if !project_name.is_empty() {
                    let name_lower = project_name.to_lowercase();
                    arr.iter().find(|p| {
                        p.get("name").and_then(|v| v.as_str())
                            .map(|n| n.to_lowercase() == name_lower)
                            .unwrap_or(false)
                    })
                } else if !task_repo_url.is_empty() {
                    let normalized = normalize_repo_url(&task_repo_url);
                    arr.iter().find(|p| {
                        let purl = p.get("repo_url").and_then(|v| v.as_str()).unwrap_or("");
                        normalize_repo_url(purl) == normalized
                    })
                } else {
                    None
                };

                if let Some(proj) = matched_proj {
                    // Backfill project name if resolved via repo_url
                    if project_name.is_empty() {
                        if let Some(name) = proj.get("name").and_then(|v| v.as_str()) {
                            project_name = name.to_string();
                            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                                "project": &project_name,
                            })).await;
                        }
                    }
                    if repo_path.is_none() {
                        repo_path = proj.get("repo_path").and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());
                    }
                    if preview_url.is_none() {
                        preview_url = proj.get("preview_url").and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());
                    }
                    project_dev_command = proj.get("dev_command").and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    // Backfill repo_url on the task if missing
                    if task_repo_url.is_empty() {
                        if let Some(url) = proj.get("repo_url").and_then(|v| v.as_str()) {
                            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                                "repo_url": url,
                            })).await;
                        }
                    }
                }
            }
        }
    }

    let mut repo_path = match repo_path {
        Some(p) => p,
        None => {
            // Pause the task and ask Matt to wire up repo_path rather than failing.
            // Matt can answer in chat to move it back to `queued`.
            let msg = if project_name.is_empty() {
                "I need to know which repo to work in. Tag a project with @name in the task, or set repo_path directly and I'll pick this back up.".to_string()
            } else {
                format!("Project \"{}\" doesn't have a repo_path configured yet. Add it in Projects settings and I'll pick this back up.", project_name)
            };
            agent_comment(config, &task_id, &msg).await;
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "pending_confirmation",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;
            return Err(format!("No repo_path for task {}", task_id));
        }
    };

    // Matt's main clone of the repo. We never modify this — we create worktrees off it.
    let main_repo_path = repo_path.clone();
    // Branch is set after the worktree is created for code tasks; research leaves it None.
    let mut branch: Option<String> = None;
    // The actual base branch used for the worktree, so create_pr targets the right one
    // (feature/foo if we stacked on it, not main).
    let mut resolved_base_branch: Option<String> = None;

    // 1. Post initial comment
    agent_comment(config, &task_id, &format!("On it. Setting up for: {}", title)).await;

    // 1b. Extract and display active worker rules for transparency
    let active_rules: Vec<String> = cached_settings.as_ref()
        .and_then(|s| s.get("workerRules"))
        .and_then(|v| v.as_array())
        .map(|rules| {
            rules.iter()
                .filter_map(|r| r.as_str())
                .filter(|r| !r.trim().is_empty())
                .map(|r| r.to_string())
                .collect()
        })
        .unwrap_or_default();

    if !active_rules.is_empty() {
        let rules_display: Vec<String> = active_rules.iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}", i + 1, r))
            .collect();
        agent_comment(config, &task_id, &format!(
            "Keeping {} rule{} in mind on this one:\n{}",
            active_rules.len(),
            if active_rules.len() == 1 { "" } else { "s" },
            rules_display.join("\n")
        )).await;
    }

    // 2. Update status
    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
        "status": "in_progress",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;
    notify_callback(config, &task_id, "in_progress", None, None);

    emit_worker_event(app, "task_working", &format!("Working on: {}", title), Some(&task_id));
    if notify_task_started {
        send_telegram(config, &format!("Working on: *{}*", escape_markdown_v2(&title))).await;
    }

    // 3. Create a git worktree for this task off a fresh origin/<base>. Matt's main
    // checkout at main_repo_path is untouched (he can keep editing it while Sam works).
    // The worktree lives at ~/samwise/worktrees/<repo>/<short_id> and persists through
    // the PR lifecycle so follow-up tasks can reuse it. A daily sweep removes it once
    // the PR is merged or closed.
    if !is_research {
        // Optional base_branch from the task row — supports stacking on feature branches
        // instead of always basing off the default branch.
        let base_override = task.get("base_branch")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        match create_task_worktree(&main_repo_path, &task_id, base_override).await {
            Ok((worktree_path, base_branch, task_branch)) => {
                agent_comment(config, &task_id, &format!(
                    "Worktree ready at `{}` on branch `{}` off `origin/{}`.",
                    worktree_path, task_branch, base_branch
                )).await;
                repo_path = worktree_path;
                branch = Some(task_branch);
                resolved_base_branch = Some(base_branch);
            }
            Err(e) => {
                agent_comment(config, &task_id, &format!(
                    "Can't prepare the workspace at `{}`: {}. Fix the repo_path or base_branch and I'll pick this back up.",
                    main_repo_path, e
                )).await;
                let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                    "status": "pending_confirmation",
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })).await;
                return Err(format!("create_task_worktree failed: {}", e));
            }
        }
    }

    // 3b. Start dev server if no preview_url and repo has a package.json (code tasks only)
    let mut dev_server_handle: Option<dev_server::DevServerHandle> = None;
    if !is_research && preview_url.is_none() {
        let pkg_json = std::path::Path::new(&repo_path).join("package.json");
        if tokio::fs::metadata(&pkg_json).await.is_ok() {
            agent_comment(config, &task_id, "No preview URL set. Starting a dev server...").await;

            // Ensure node_modules exists before trying to start the dev server
            if let Err(e) = dev_server::ensure_deps_installed(&repo_path).await {
                agent_comment(config, &task_id, &format!("npm install failed: {}. Proceeding without screenshots or visual QA.", e)).await;
            } else {
                match dev_server::start_dev_server(&repo_path, project_dev_command.as_deref()).await {
                    Ok(handle) => {
                        // 60s timeout: Next.js and large projects can take 30-60s on first start
                        match dev_server::wait_for_ready(&handle.url, 60).await {
                            Ok(()) => {
                                agent_comment(config, &task_id, &format!("Dev server running at {}", handle.url)).await;
                                preview_url = Some(handle.url.clone());
                                dev_server_handle = Some(handle);
                            }
                            Err(e) => {
                                agent_comment(config, &task_id, &format!("Dev server started but not responding: {}. Proceeding without screenshots or visual QA.", e)).await;
                                let _ = dev_server::kill_dev_server(handle).await;
                            }
                        }
                    }
                    Err(e) => {
                        agent_comment(config, &task_id, &format!("Couldn't start dev server: {}. Proceeding without screenshots or visual QA.", e)).await;
                    }
                }
            }
        }
    }

    // 4. Take BEFORE screenshots if preview_url is set (code tasks only)
    let screenshot_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("agent-one-screenshots")
        .join(&task_id)
        .to_string_lossy()
        .into_owned();
    if !is_research {
        if let Some(ref preview) = preview_url {
            let _ = tokio::fs::create_dir_all(&screenshot_dir).await;
            agent_comment(config, &task_id, "Taking before screenshots...").await;

            for (label, name, viewport) in [
                ("desktop", "before-desktop.png", "1280,720"),
                ("mobile",  "before-mobile.png",  "393,852"),
            ] {
                if let Err(e) = take_screenshot(preview, &join_path(&screenshot_dir, name), viewport).await {
                    agent_comment(config, &task_id, &format!("Before-{} screenshot failed: {}", label, e)).await;
                    log::warn!("[worker] before-{} screenshot failed: {}", label, e);
                }
            }
        }
    }

    // 5. Run Claude Code CLI
    let action_label = if is_research { "Running analysis with Claude Code..." } else { "Starting code changes with Claude Code..." };
    agent_comment(config, &task_id, action_label).await;

    // Build a context-aware prompt with repo info
    let mut prompt_parts: Vec<String> = Vec::new();

    // Read CLAUDE.md if it exists in the repo
    let claude_md_path = join_path(&repo_path, "CLAUDE.md");
    if let Ok(claude_md) = tokio::fs::read_to_string(&claude_md_path).await {
        let claude_md_truncated = truncate(&claude_md, 2000);
        prompt_parts.push(format!("## Project Instructions (from CLAUDE.md)\n{}\n", claude_md_truncated));
    }

    // Inject worker rules into prompt (reuse the rules extracted earlier for the comment)
    if !active_rules.is_empty() {
        let rule_strings: Vec<String> = active_rules.iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}", i + 1, r))
            .collect();
        prompt_parts.push(format!(
            "## Worker Rules (MUST follow these)\n{}\n\nAt the end of your response, briefly mention any rules that affected your approach (one line max).\n",
            rule_strings.join("\n")
        ));
    }

    // Add subtask context if present
    if let Some(subtasks_val) = task.get("subtasks") {
        if let Some(arr) = subtasks_val.as_array() {
            if !arr.is_empty() {
                let subtask_lines: Vec<String> = arr.iter().enumerate().map(|(i, s)| {
                    let done = s.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
                    let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("?");
                    format!("  {} {}. {}", if done { "[x]" } else { "[ ]" }, i + 1, title)
                }).collect();
                prompt_parts.push(format!(
                    "## Subtasks (checklist)\nWork on the FIRST unchecked item only. Do not skip ahead:\n{}\n",
                    subtask_lines.join("\n")
                ));
            }
        }
    }

    // Get recent git log for context
    if let Ok(git_log) = async_cmd("git")
        .args(["log", "--oneline", "-10"])
        .current_dir(&repo_path)
        .output()
        .await
    {
        if git_log.status.success() {
            let log_str = String::from_utf8_lossy(&git_log.stdout);
            if !log_str.trim().is_empty() {
                prompt_parts.push(format!("## Recent git history\n```\n{}\n```\n", log_str.trim()));
            }
        }
    }

    // Inject webhook/trigger context if present (capped at 8KB to avoid blowing up the prompt)
    if let Some(context) = task.get("context") {
        if !context.is_null() {
            let ctx_str = if context.is_string() {
                context.as_str().unwrap_or("").to_string()
            } else {
                serde_json::to_string_pretty(context).unwrap_or_default()
            };
            if !ctx_str.is_empty() {
                let capped = truncate(&ctx_str, 8000);
                prompt_parts.push(format!("## Task Context\n```json\n{}\n```\n", capped));
            }
        }
    }

    // Materialize attachments: download each task.attachments[].url into a
    // /tmp/samwise-attachments/<task_id>/ directory so Claude Code can read
    // them as local files. Append the paths to the prompt so Claude knows
    // to consult them (images carry info text can't).
    let attachment_paths = materialize_task_attachments(&task_id, &task).await;
    if !attachment_paths.is_empty() {
        let lines: Vec<String> = attachment_paths
            .iter()
            .map(|p| format!("- {}", p.display()))
            .collect();
        prompt_parts.push(format!(
            "## Attached files\nMatt attached {} file(s) to this task. Read them before doing anything else — they almost always contain the bug repro, design reference, or error screenshot that the description alone does not convey:\n{}\n",
            attachment_paths.len(),
            lines.join("\n")
        ));
        agent_comment(config, &task_id, &format!(
            "Downloaded {} attachment(s) for this task.",
            attachment_paths.len()
        )).await;
    }

    // The actual task
    if is_research {
        prompt_parts.push(format!(
            "## Task\n**{}**\n\n{}\n\n## Instructions\nThis is a RESEARCH/ANALYSIS task. Do NOT make any code changes, do NOT commit, do NOT create files. Read, analyze, and provide a thorough written report. Be detailed and specific.",
            title, description
        ));
    } else {
        prompt_parts.push(format!(
            "## Task\n**{title}**\n\n{description}\n\n\
## Instructions\n\
Make the code changes required by this task. You have approval to edit \
any file in this repo \u{2014} do not ask for confirmation before writing.\n\n\
Explore only what the task needs. Do not read the whole codebase or add \
unrelated cleanup \u{2014} those will bloat the diff and slow review.\n\n\
When you are done making changes, stage everything and write a structured commit. \
Use a HEREDOC so the body is multi-line:\n\
```\n\
git add -A && git commit -m \"$(cat <<'EOF'\n\
{title}\n\
\n\
What was fixed:\n\
- <plain-English description of the bug or feature, from the user/customer POV>\n\
\n\
How it was fixed:\n\
- <technical summary of the change: files/functions touched, approach, why>\n\
\n\
Deployment required:\n\
- Railway server: <yes/no/unknown> - <plain reason, including service name if yes>\n\
- Supabase migrations: <yes/no/unknown> - <plain reason, including migration filenames if yes>\n\
- Supabase Edge Functions: <yes/no/unknown> - <plain reason, including function names if yes>\n\
\n\
For Customer Success:\n\
- <one or two sentences CS can paste to the customer explaining that it's fixed and what to expect now, in non-technical language>\n\
EOF\n\
)\"\n\
```\n\
Fill in every section concretely. Do not use placeholders. If the task is a \
non-customer-facing refactor, still write the \"For Customer Success\" line \
but mark it as \"internal only, no customer message needed\". The deployment \
section must be crystal clear: use \"no\" when the PR does not require that \
deployment path, \"yes\" when it does, and \"unknown\" only when the codebase \
does not provide enough evidence.\n\n\
Then stop. Do not open the PR yourself \u{2014} that is handled after this step.\n\n\
If the task is genuinely ambiguous and you cannot proceed without a decision \
from Matt, stop without making changes and explain specifically what you need clarified."
        ));
    }

    let prompt = prompt_parts.join("\n");

    // 30-minute timeout, UNLIMITED turns. A hard cap just surfaces as
    // error_max_turns mid-run on complex tasks — the timeout is the real
    // guard. Pass 0 so `--max-turns` is omitted entirely.
    let claude_result = run_claude_code_streaming(&repo_path, &prompt, 0, 1800, config, &task_id, process_id_slot.clone()).await;
    // Clear PID after process completes
    { let mut pid = process_id_slot.lock().await; *pid = None; }

    // Honor cancellation from the streaming heartbeat: task was deleted or
    // cancelled by Matt mid-run. Tear down the worktree helpers and stop
    // without posting failure comments on a task that no longer exists.
    if matches!(&claude_result, Err(e) if e == "TASK_CANCELLED") {
        log::info!("[worker] execute_task aborting: task {} was cancelled/deleted", task_id);
        if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
        return Ok("Task was cancelled".to_string());
    }

    let task_result = match claude_result {
        Ok(output) if !is_research && !worker_made_changes(&repo_path).await => {
            // Code task finished with zero changes (nothing committed, nothing staged,
            // nothing untracked). Distinct from a task failure: Claude read the code and
            // concluded there was nothing to do, or the task was unclear. Route to
            // review so Matt can decide if it's really done or needs more context.
            let summary = truncate(&output, 800);
            let msg = if summary.trim().is_empty() {
                "I looked at this but didn't change anything. Either the task is already done or I wasn't sure what to do. Mark it done, or reply with more context and requeue.".to_string()
            } else {
                format!("I looked at this but didn't change anything. What I considered:\n\n{}\n\nMark it done, or reply with more context and requeue.", summary)
            };
            agent_comment(config, &task_id, &msg).await;
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "review",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;
            notify_callback(config, &task_id, "review", None, None);
            if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
            // Leave the worktree in place; daily sweep will reap it after 48h if no PR shows up.
            return Ok("No changes made; routed to review".to_string());
        }
        Ok(output) => {
            let summary = truncate(&output, 500);

            // Research tasks: save full output as artifact, post short comment, mark done
            if is_research {
                // Save the full report as an artifact
                let artifact_result = supabase::create_artifact(config, &serde_json::json!({
                    "task_id": task_id,
                    "title": title,
                    "content": output,
                    "artifact_type": "report",
                })).await;

                match artifact_result {
                    Ok(_) => {
                        agent_comment(config, &task_id, "Analysis complete. Full report saved. Click the Report tab above to read it.").await;
                    }
                    Err(e) => {
                        log::warn!("[worker] Failed to save artifact: {}", e);
                        // Fallback: post truncated summary in comment
                        agent_comment(config, &task_id, &format!("Analysis complete. (Failed to save full report, showing summary)\n\n{}", summary)).await;
                    }
                }

                let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                    "status": "done",
                    "completed_at": chrono::Utc::now().to_rfc3339(),
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })).await;
                notify_callback(config, &task_id, "done", None, None);
                if notify_task_completed_research {
                    send_telegram(config, &format!(
                        "Finished analysis on *{}*\\. Full report saved as an artifact\\.",
                        escape_markdown_v2(&title)
                    )).await;
                }
                if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
                return Ok("Analysis complete".to_string());
            }

            // Post full output but cap at 10KB to avoid Supabase/UI issues with massive comments
            let comment_output = truncate(&output, 10_000);
            agent_comment(config, &task_id, &format!("Code changes done. Here's what I did:\n\n{}", comment_output)).await;

            // 5b. Run /codex-fix in the worktree to get a Codex review of the diff and auto-apply
            // any must-fix/should-fix edits. Runs BEFORE screenshots + QA so QA validates the
            // final state. Any edits codex-fix makes are committed separately so the PR shows
            // a clear "task commit" + "codex-fix commit" history.
            // Cancellation check before starting another long phase.
            if !task_is_live(config, &task_id).await {
                log::info!("[worker] Task {} cancelled before codex-fix; stopping", task_id);
                if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
                return Ok("Task was cancelled".to_string());
            }

            agent_comment(config, &task_id, "Running /codex-fix for a review pass before QA...").await;
            let codex_result = run_claude_code_streaming(
                &repo_path, "/codex-fix", 0, 1200, config, &task_id, process_id_slot.clone()
            ).await;
            { let mut pid = process_id_slot.lock().await; *pid = None; }
            // If codex-fix itself was cancelled mid-run, bail out.
            if matches!(&codex_result, Err(e) if e == "TASK_CANCELLED") {
                log::info!("[worker] Task {} cancelled during codex-fix; stopping", task_id);
                if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
                return Ok("Task was cancelled".to_string());
            }
            match codex_result {
                Ok(_) => {
                    let porcelain = run_git(&["status", "--porcelain"], &repo_path).await.unwrap_or_default();
                    if porcelain.trim().is_empty() {
                        agent_comment(config, &task_id, "codex-fix found nothing to change.").await;
                    } else {
                        let _ = run_git(&["add", "-A"], &repo_path).await;
                        match run_git(&["commit", "-m", "codex-fix: apply review feedback"], &repo_path).await {
                            Ok(_) => {
                                agent_comment(config, &task_id, "Applied codex-fix suggestions in a follow-up commit.").await;
                            }
                            Err(e) => {
                                log::warn!("[worker] codex-fix commit failed: {}", e);
                                agent_comment(config, &task_id, &format!("codex-fix edited files but the commit failed ({}). Changes still staged.", e)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("[worker] /codex-fix failed: {}", e);
                    agent_comment(config, &task_id, &format!("codex-fix didn't complete cleanly ({}). Proceeding to QA.", e)).await;
                }
            }

            // 5c. Build check. Runs the project's build command (npm run build / cargo
            // build, auto-detected) so logic bugs that break compilation or the bundle
            // get caught before a PR is opened. One auto-fix pass with codex if the
            // first build fails; if the second build still fails, bail out clearly
            // instead of pushing broken code.
            match run_build_check(&repo_path).await {
                Ok(Some(cmd)) => {
                    agent_comment(config, &task_id, &format!("Build passed ({}).", cmd)).await;
                }
                Ok(None) => {
                    // No build command detected; nothing to gate on.
                }
                Err((cmd, log_tail)) => {
                    agent_comment(config, &task_id, &format!(
                        "Build failed ({}). Trying one codex-fix pass with the build output as context.", cmd
                    )).await;
                    let fix_prompt = format!(
                        "/codex-fix\n\nThe project's build just failed. Fix the build errors only. Don't refactor anything else.\n\nBuild command: {}\n\nBuild output (tail):\n{}",
                        cmd, log_tail
                    );
                    let retry_fix = run_claude_code_streaming(
                        &repo_path, &fix_prompt, 0, 1200, config, &task_id, process_id_slot.clone()
                    ).await;
                    if matches!(&retry_fix, Err(e) if e == "TASK_CANCELLED") {
                        log::info!("[worker] Task {} cancelled during build-retry codex-fix; stopping", task_id);
                        if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
                        return Ok("Task was cancelled".to_string());
                    }
                    if let Err(e) = retry_fix {
                        log::warn!("[worker] codex-fix (build retry) failed: {}", e);
                    }
                    // Stage and commit whatever codex-fix produced, even if stream errored.
                    let _ = run_git(&["add", "-A"], &repo_path).await;
                    let status_out = run_git(&["diff", "--cached", "--quiet"], &repo_path).await;
                    if matches!(status_out, Err(_)) {
                        // Exit non-zero from diff --quiet means there are staged changes.
                        let _ = run_git(&["commit", "-m", "codex-fix: repair failing build"], &repo_path).await;
                    }

                    match run_build_check(&repo_path).await {
                        Ok(_) => {
                            agent_comment(config, &task_id, "Build passed on second try after codex-fix. Proceeding.").await;
                        }
                        Err((cmd2, log_tail2)) => {
                            agent_comment(config, &task_id, &format!(
                                "Build still failing ({}) after a codex-fix pass. Not opening a PR. Last output:\n\n```\n{}\n```",
                                cmd2, log_tail2
                            )).await;

                            if let Some(h) = dev_server_handle.take() {
                                let _ = dev_server::kill_dev_server(h).await;
                            }
                            let reason = format!("build failed: {}", cmd2);
                            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                                "status": "failed",
                                "failure_reason": &reason,
                                "updated_at": chrono::Utc::now().to_rfc3339(),
                            })).await;
                            notify_callback(config, &task_id, "failed", None, Some(&reason));
                            return Err(reason);
                        }
                    }
                }
            }

            // 6. Take AFTER screenshots (may be re-taken inside the QA retry loop below)
            if let Some(ref preview) = preview_url {
                agent_comment(config, &task_id, "Taking after screenshots...").await;

                for (label, name, viewport) in [
                    ("desktop", "after-desktop.png", "1280,720"),
                    ("mobile",  "after-mobile.png",  "393,852"),
                ] {
                    if let Err(e) = take_screenshot(preview, &join_path(&screenshot_dir, name), viewport).await {
                        agent_comment(config, &task_id, &format!("After-{} screenshot failed: {}", label, e)).await;
                        log::warn!("[worker] after-{} screenshot failed: {}", label, e);
                    }
                }
            }

            // Mark first unchecked subtask as done (re-fetch fresh subtasks to avoid stale data)
            let fresh_subtasks = {
                let all_tasks = supabase::fetch_tasks(config, None).await.ok();
                all_tasks.and_then(|tasks| {
                    tasks.as_array().and_then(|arr| {
                        arr.iter()
                            .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(&task_id))
                            .and_then(|t| t.get("subtasks").cloned())
                    })
                })
            };
            if let Some(subtasks_val) = fresh_subtasks {
                if let Some(arr) = subtasks_val.as_array() {
                    let mut updated: Vec<serde_json::Value> = arr.clone();
                    let mut marked = false;
                    for item in updated.iter_mut() {
                        if let Some(done) = item.get("done").and_then(|v| v.as_bool()) {
                            if !done {
                                if let Some(obj) = item.as_object_mut() {
                                    obj.insert("done".to_string(), serde_json::json!(true));
                                    marked = true;
                                    break;
                                }
                            }
                        }
                    }
                    if marked {
                        let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                            "subtasks": updated,
                            "updated_at": chrono::Utc::now().to_rfc3339()
                        })).await;
                    }

                    // Check if unchecked subtasks remain -> re-queue for next subtask
                    let remaining = updated.iter().any(|s| {
                        s.get("done").and_then(|v| v.as_bool()) == Some(false)
                    });
                    if remaining {
                        // Push branch so next iteration has the commits
                        if let Some(ref b) = branch {
                            let _ = async_cmd("git")
                                .args(["push", "-u", "origin", b])
                                .current_dir(&repo_path)
                                .output()
                                .await;
                        }

                        // Kill dev server before early return
                        if let Some(h) = dev_server_handle.take() {
                            let _ = dev_server::kill_dev_server(h).await;
                        }

                        agent_comment(config, &task_id,
                            &format!("Subtask done. {} more to go. Re-queuing for the next one.",
                                updated.iter().filter(|s| s.get("done").and_then(|v| v.as_bool()) != Some(true)).count()
                            )
                        ).await;
                        let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                            "status": "queued",
                            "worker_id": serde_json::Value::Null,
                            "claimed_at": serde_json::Value::Null,
                            "updated_at": chrono::Utc::now().to_rfc3339()
                        })).await;
                        return Ok("Subtask completed, re-queued for next subtask".to_string());
                    }
                }
            }

            // 7. Visual QA with self-correct loop.
            // If QA flags a problem, feed the explanation back to Claude Code as a fix-it
            // prompt, re-screenshot, re-QA. Up to MAX_QA_ATTEMPTS (3) total attempts so
            // Sam can't spiral. On exhaustion we push anyway with an honest note.
            const MAX_QA_ATTEMPTS: u32 = 3;
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "testing",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;

            let mut qa_attempts: u32 = 0;
            let mut final_qa: Option<(bool, String)> = None;

            if preview_url.is_some() {
                loop {
                    qa_attempts += 1;
                    let starting_msg = if qa_attempts == 1 {
                        "Running visual QA...".to_string()
                    } else {
                        format!("Re-running visual QA (attempt {}/{})...", qa_attempts, MAX_QA_ATTEMPTS)
                    };
                    agent_comment(config, &task_id, &starting_msg).await;

                    match run_visual_qa(&repo_path, &title, &description, &screenshot_dir).await {
                        Ok((passed, explanation)) => {
                            if passed {
                                let msg = if qa_attempts == 1 {
                                    format!("Visual QA passed. {}", explanation)
                                } else {
                                    format!("Visual QA passed on attempt {}/{}. {}", qa_attempts, MAX_QA_ATTEMPTS, explanation)
                                };
                                agent_comment(config, &task_id, &msg).await;
                                final_qa = Some((true, explanation));
                                break;
                            }

                            final_qa = Some((false, explanation.clone()));

                            if qa_attempts >= MAX_QA_ATTEMPTS {
                                agent_comment(config, &task_id, &format!(
                                    "QA still flagging after {} attempts: {}. Pushing anyway with a note so you can review.",
                                    MAX_QA_ATTEMPTS, explanation
                                )).await;
                                break;
                            }

                            agent_comment(config, &task_id, &format!(
                                "QA flagged: {}. Having a go at fixing it (attempt {}/{}).",
                                explanation, qa_attempts + 1, MAX_QA_ATTEMPTS
                            )).await;

                            let fix_prompt = format!(
                                "## Fix visual QA feedback\n\n\
The visual QA reviewer looked at before/after screenshots of your last changes \
and flagged this problem:\n\n\
> {explanation}\n\n\
## Original task\n\
**{title}**\n\n\
{description}\n\n\
## Instructions\n\
Your previous changes are already committed on this branch. Make additional edits \
to fix *only* the issue QA described. Don't add unrelated changes or touch files \
that weren't involved.\n\n\
When done, run `git add -A && git commit -m \"fix: address QA feedback\"` and stop."
                            );

                            let fix_result = run_claude_code_streaming(
                                &repo_path, &fix_prompt, 0, 900, config, &task_id, process_id_slot.clone()
                            ).await;
                            { let mut pid = process_id_slot.lock().await; *pid = None; }
                            if matches!(&fix_result, Err(e) if e == "TASK_CANCELLED") {
                                log::info!("[worker] Task {} cancelled during QA-retry fix; stopping", task_id);
                                if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
                                return Ok("Task was cancelled".to_string());
                            }

                            if let Err(e) = fix_result {
                                agent_comment(config, &task_id, &format!(
                                    "Fix attempt {} errored: {}. Pushing what we have.", qa_attempts, e
                                )).await;
                                break;
                            }

                            // Re-take AFTER screenshots so the next QA round sees the fixed app.
                            if let Some(ref preview) = preview_url {
                                let _ = take_screenshot(preview, &join_path(&screenshot_dir, "after-desktop.png"), "1280,720").await;
                                let _ = take_screenshot(preview, &join_path(&screenshot_dir, "after-mobile.png"), "393,852").await;
                            }
                        }
                        Err(e) => {
                            agent_comment(config, &task_id, &format!("Visual QA couldn't run: {}. Skipping.", e)).await;
                            break;
                        }
                    }
                }

                if let Some((passed, explanation)) = &final_qa {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "visual_qa_result": { "pass": passed, "explanation": explanation, "attempts": qa_attempts },
                    })).await;
                }
            }

            // 7b. Upload the (possibly revised) screenshots to Supabase Storage now that
            // the QA loop has settled. Task record points at the final images only.
            if preview_url.is_some() {
                let (urls, _) = upload_screenshots_to_storage(config, &task_id, &screenshot_dir).await
                    .unwrap_or_default();
                let before_urls: Vec<&String> = urls.iter().filter(|u| u.contains("before-")).collect();
                let after_urls: Vec<&String> = urls.iter().filter(|u| u.contains("after-")).collect();
                let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                    "screenshots_before": before_urls,
                    "screenshots_after": after_urls,
                })).await;
            }

            let qa_note_for_pr = match &final_qa {
                Some((false, explanation)) => Some(format!(
                    "\n\n> **Visual QA flagged after {} attempt{}:** {}",
                    qa_attempts,
                    if qa_attempts == 1 { "" } else { "s" },
                    explanation
                )),
                _ => None,
            };

            // Last cancellation check before the PR is opened. If Matt deleted
            // the task during QA, don't create a PR for it.
            if !task_is_live(config, &task_id).await {
                log::info!("[worker] Task {} cancelled before PR creation; skipping", task_id);
                if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
                return Ok("Task was cancelled".to_string());
            }

            // 8. Create PR targeting the branch we stacked on (not always main/master).
            let pr_result = create_pr(
                config, &repo_path, &title, &description, &task_id,
                &branch, resolved_base_branch.as_deref(),
                &screenshot_dir, preview_url.is_some(),
                qa_note_for_pr.as_deref(),
            ).await;

            match pr_result {
                Ok(pr_url) => {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "status": "review",
                        "pr_url": pr_url,
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
                    notify_callback(config, &task_id, "review", Some(&pr_url), None);
                    agent_comment(config, &task_id, &format!("PR's up: {}. Let me know if you want any changes.", pr_url)).await;

                    // Only fire the "PR's up" Telegram now if there's no
                    // downstream automation coming (no auto-merge gate, no
                    // auto-pr-review pass). Otherwise the terminal path in
                    // try_auto_merge / spawn_pr_review_task owns the
                    // telegram so Matt only gets pinged once the whole
                    // pipeline is done.
                    let auto_merge_on_for_telegram = cached_settings.as_ref()
                        .and_then(|s| s.get("autoMergeEnabled"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let pr_review_on_for_telegram = cached_settings.as_ref()
                        .and_then(|s| s.get("autoPrReviewEnabled"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let has_downstream = auto_merge_on_for_telegram || pr_review_on_for_telegram;
                    if notify_task_completed_code && !has_downstream {
                        send_telegram(config, &format!(
                            "PR's up for *{}*: {}",
                            escape_markdown_v2(&title),
                            escape_markdown_v2(&pr_url)
                        )).await;
                    }

                    // Auto-merge gate. Never throws; worst case it leaves the PR in review.
                    let outcome = review::try_auto_merge(
                        config, &repo_path, &pr_url, &task_id, &title, &description, &cached_settings,
                    ).await;
                    match outcome {
                        review::AutoMergeOutcome::Merged => {
                            notify_callback(config, &task_id, "done", Some(&pr_url), None);
                            agent_comment(config, &task_id, "Auto-merged. All gates green.").await;
                            if notify_task_completed_code {
                                send_telegram(config, &format!(
                                    "Auto-merged *{}*",
                                    escape_markdown_v2(&title)
                                )).await;
                            }
                        }
                        review::AutoMergeOutcome::Blocked { reason, scores } => {
                            let mut updates = serde_json::json!({
                                "auto_merge_blocked_reason": reason,
                                "updated_at": chrono::Utc::now().to_rfc3339(),
                            });
                            if let Some(s) = scores {
                                updates["review_scores"] = s;
                            }
                            let _ = supabase::update_task(config, &task_id, &updates).await;
                            agent_comment(config, &task_id, &format!("Auto-merge blocked: {}", reason)).await;
                            if notify_task_completed_code {
                                send_telegram(config, &format!(
                                    "Auto-merge blocked for *{}* — your call: {}\\.\nReason: {}",
                                    escape_markdown_v2(&title),
                                    escape_markdown_v2(&pr_url),
                                    escape_markdown_v2(reason.as_str()),
                                )).await;
                            }
                        }
                        review::AutoMergeOutcome::Skipped => {}
                    }

                    // Codex $samwise-pr-review pass. Only runs when auto-merge is OFF —
                    // when auto-merge is on, try_auto_merge already did its own Codex pass
                    // and either merged or left a blocked reason on the card.
                    let auto_merge_on = cached_settings.as_ref()
                        .and_then(|s| s.get("autoMergeEnabled"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let auto_pr_review_on = cached_settings.as_ref()
                        .and_then(|s| s.get("autoPrReviewEnabled"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    if !auto_merge_on && auto_pr_review_on {
                        spawn_pr_review_task(config.clone(), task_id.clone(), pr_url.clone(), repo_path.clone());
                    }

                    Ok(format!("PR created: {}", pr_url))
                }
                Err(e) => {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "status": "review",
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
                    notify_callback(config, &task_id, "review", None, None);
                    agent_comment(config, &task_id, &format!("Code changes are done but PR creation failed: {}. You can push manually.", e)).await;
                    if notify_task_completed_code {
                        send_telegram(config, &format!(
                            "Code done for *{}* but PR failed: {}",
                            escape_markdown_v2(&title),
                            escape_markdown_v2(&e)
                        )).await;
                    }
                    Ok("Code changes complete (no PR)".to_string())
                }
            }
        }
        Err(e) => {
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "failed",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;
            notify_callback(config, &task_id, "failed", None, Some(&e));
            agent_comment(config, &task_id, &format!("Ran into an issue: {}. You might want to re-queue this or take a look.", e)).await;
            if notify_task_failed {
                send_telegram(config, &format!(
                    "Hit a snag on *{}*: {}",
                    escape_markdown_v2(&title),
                    escape_markdown_v2(&e)
                )).await;
            }
            Err(format!("Task failed: {}", e))
        }
    };

    // Cleanup: kill dev server if we started one
    if let Some(h) = dev_server_handle.take() {
        let _ = dev_server::kill_dev_server(h).await;
    }

    // Worktree persists on disk. If the PR opened successfully, it lives until merged
    // or closed (daily sweep reaps it then). If no PR materialized, the sweep treats it
    // as orphaned after 48h and removes it.
    let _ = &main_repo_path; // kept for sweep-side use; silence unused-warning if any
    task_result
}

/// Resolve the main repo path for a linked worktree by asking git.
/// `git -C <wt> rev-parse --git-common-dir` points at <main>/.git; the parent is the main repo.
async fn main_repo_for_worktree(wt_str: &str) -> Option<String> {
    let common_dir = run_git(&["rev-parse", "--git-common-dir"], wt_str).await.ok()?;
    let common_dir = common_dir.trim();
    let abs = if std::path::Path::new(common_dir).is_absolute() {
        std::path::PathBuf::from(common_dir)
    } else {
        std::path::PathBuf::from(wt_str).join(common_dir)
    };
    abs.parent().map(|p| p.to_string_lossy().into_owned())
}

/// Walk ~/samwise/worktrees/<repo>/<short_id>, query the matching PR via `gh pr list
/// --head sam/<short_id>`, and remove worktrees whose PRs are merged or closed. Worktrees
/// without a PR that are >48h old are treated as orphans (crashed task) and removed too.
/// Returns (removed, kept).
/// Detect the project type and run its build. Returns:
/// - Ok(Some(cmd)) if a build ran and passed
/// - Ok(None) if we didn't find anything to run (no gate)
/// - Err((cmd, log_tail)) if the build ran and failed; log_tail is the last
///   ~2000 chars of combined stdout+stderr so codex-fix has useful context.
///
/// Detection order (first match wins):
///   package.json with a `build` script → npm run build
///   Cargo.toml                         → cargo build
///
/// Kept deliberately narrow. Projects with non-standard builds can be
/// opted out by removing their build script; a future `ae_projects.build_command`
/// column could let Matt override per-project.
async fn run_build_check(repo_path: &str) -> Result<Option<String>, (String, String)> {
    let pkg = join_path(repo_path, "package.json");
    let cargo = join_path(repo_path, "Cargo.toml");

    let (cmd_label, prog, args): (String, &str, Vec<&str>) = if tokio::fs::metadata(&pkg).await.is_ok() {
        let pkg_txt = tokio::fs::read_to_string(&pkg).await.unwrap_or_default();
        let has_build = serde_json::from_str::<serde_json::Value>(&pkg_txt)
            .ok()
            .and_then(|v| v.get("scripts").and_then(|s| s.get("build")).cloned())
            .is_some();
        if !has_build {
            return Ok(None);
        }
        ("npm run build".to_string(), "npm", vec!["run", "build"])
    } else if tokio::fs::metadata(&cargo).await.is_ok() {
        ("cargo build".to_string(), "cargo", vec!["build"])
    } else {
        return Ok(None);
    };

    // 15-minute cap so a wedged build can't freeze the whole worker.
    let child = async_cmd(prog)
        .args(&args)
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    let out = match tokio::time::timeout(std::time::Duration::from_secs(900), child).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err((cmd_label, format!("spawn failed: {}", e))),
        Err(_) => return Err((cmd_label, "build timed out after 15m".to_string())),
    };

    if out.status.success() {
        return Ok(Some(cmd_label));
    }

    // Assemble a readable tail for codex-fix to chew on. Cargo and npm both
    // dump errors near the end of stderr; grab the last stderr chunk and, if
    // short, pad with stdout.
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let combined = if stderr.trim().is_empty() { stdout } else { stderr };
    let tail = if combined.len() > 2000 {
        combined[combined.len() - 2000..].to_string()
    } else {
        combined
    };
    Err((cmd_label, tail.trim().to_string()))
}

/// Reset any tasks stuck in `in_progress` back to `queued`. Runs at worker
/// startup to recover from crashes (the sole worker exited mid-task, leaving
/// the row claimed forever). Also clears claimed_by/claimed_at so the next
/// poll cycle picks them up normally.
async fn recover_stuck_tasks(config: &SupabaseConfig) -> usize {
    let Ok(tasks) = supabase::fetch_tasks(config, Some("in_progress")).await else {
        return 0;
    };
    let Some(arr) = tasks.as_array() else { return 0; };
    let mut recovered = 0usize;
    for task in arr {
        let Some(id) = task.get("id").and_then(|v| v.as_str()) else { continue; };
        let updates = serde_json::json!({
            "status": "queued",
            "worker_id": serde_json::Value::Null,
            "claimed_at": serde_json::Value::Null,
        });
        if supabase::update_task(config, id, &updates).await.is_ok() {
            recovered += 1;
        }
    }
    recovered
}

/// Collect short-task-id → status map so the sweep can identify failed /
/// vanished tasks whose worktrees should be cleaned up immediately (not
/// waiting for the 48h orphan rule).
///
/// Keyed by `short_task_id` because worktree directories and branch names
/// use the short form, not the full UUID. Previously this was keyed by the
/// full UUID, which silently made every lookup miss and sent every worktree
/// down the "task row gone" branch — the sweep then deleted remote branches
/// and GitHub auto-closed the still-open PRs attached to them.
async fn failed_task_ids(config: &SupabaseConfig) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let Ok(all) = supabase::fetch_tasks(config, None).await else { return out; };
    if let Some(arr) = all.as_array() {
        for t in arr {
            let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !id.is_empty() { out.insert(short_task_id(&id), status); }
        }
    }
    out
}

async fn sweep_merged_worktrees() -> (usize, usize) {
    sweep_worktrees_inner(None).await
}

async fn sweep_worktrees_with_config(config: &SupabaseConfig) -> (usize, usize) {
    sweep_worktrees_inner(Some(config)).await
}

async fn sweep_worktrees_inner(config: Option<&SupabaseConfig>) -> (usize, usize) {
    let root = worktrees_root();
    if tokio::fs::metadata(&root).await.is_err() {
        return (0, 0);
    }

    // Fetch task status map once. Lets us identify failed or vanished tasks
    // whose worktrees should be cleaned up immediately, and delete them
    // without waiting for the 48h orphan rule to fire.
    let task_statuses = match config {
        Some(c) => failed_task_ids(c).await,
        None => std::collections::HashMap::new(),
    };

    let mut removed = 0usize;
    let mut kept = 0usize;
    let mut touched_main_repos: std::collections::HashSet<String> = Default::default();

    let Ok(mut repos) = tokio::fs::read_dir(&root).await else { return (0, 0); };
    while let Ok(Some(repo_entry)) = repos.next_entry().await {
        let repo_dir = repo_entry.path();
        if !repo_dir.is_dir() { continue; }
        let Ok(mut wts) = tokio::fs::read_dir(&repo_dir).await else { continue; };
        while let Ok(Some(wt_entry)) = wts.next_entry().await {
            let wt_path = wt_entry.path();
            if !wt_path.is_dir() { continue; }
            let wt_str = wt_path.to_string_lossy().into_owned();
            let short_id = wt_path.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let branch = task_branch_name(&short_id);

            let Some(main_repo) = main_repo_for_worktree(&wt_str).await else {
                kept += 1;
                continue;
            };
            touched_main_repos.insert(main_repo.clone());

            // Task-status-driven removal. If the task row is failed OR the task
            // is gone entirely (but we have a task map to check against), nuke
            // the worktree immediately — no reason to keep a worktree for a
            // task Matt will never revisit.
            let task_match = task_statuses.get(&short_id);
            let task_based_removal = match (config.is_some(), task_match) {
                (true, Some(status)) if status == "failed" || status == "cancelled" => {
                    Some(format!("task {}", status))
                }
                (true, None) if !task_statuses.is_empty() => {
                    Some("task row gone".to_string())
                }
                _ => None,
            };

            // Always check PR state first. Even a task flagged failed or
            // missing can have a PR Matt is reviewing — never kill the remote
            // branch under an OPEN PR, because GitHub auto-closes it.
            let pr_state_raw = async_cmd("gh")
                .args(["pr", "list", "--head", &branch, "--state", "all", "--json", "state", "--limit", "1"])
                .current_dir(&main_repo)
                .output().await;

            let pr_state: Option<String> = match pr_state_raw {
                Ok(out) if out.status.success() => {
                    let body = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if body.contains("\"state\":\"MERGED\"") { Some("MERGED".into()) }
                    else if body.contains("\"state\":\"CLOSED\"") { Some("CLOSED".into()) }
                    else if body.contains("\"state\":\"OPEN\"") { Some("OPEN".into()) }
                    else if body == "[]" { Some("NONE".into()) }
                    else { None }
                }
                Ok(out) => {
                    log::warn!("[sweep] gh pr list failed for {}: {}", branch, String::from_utf8_lossy(&out.stderr).trim());
                    None
                }
                Err(e) => {
                    log::warn!("[sweep] gh invocation failed: {}", e);
                    None
                }
            };

            // Hard gate: an open PR always keeps the worktree + branch alive.
            if pr_state.as_deref() == Some("OPEN") {
                kept += 1;
                continue;
            }

            let (should_remove, reason) = match (task_based_removal, pr_state.as_deref()) {
                (_, Some("MERGED")) => (true, "PR merged".to_string()),
                (_, Some("CLOSED")) => (true, "PR closed".to_string()),
                (Some(r), _) => (true, r),
                (None, Some("NONE")) => {
                    let age_secs = wt_entry.metadata().await.ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.elapsed().ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    if age_secs > 48 * 3600 {
                        (true, "orphan (no PR, >48h)".to_string())
                    } else {
                        (false, "no PR yet".to_string())
                    }
                }
                _ => (false, "gh failed".to_string()),
            };

            if should_remove {
                log::info!("[sweep] removing worktree {} ({})", wt_str, reason);
                let _ = run_git(&["worktree", "remove", "--force", &wt_str], &main_repo).await;
                let _ = run_git(&["branch", "-D", &branch], &main_repo).await;
                // Only delete the remote branch when we're sure no open PR is
                // attached. The open-PR guard above already returned early,
                // and PR-merged / PR-closed states mean the branch is already
                // detached from a live review.
                let _ = run_git(&["push", "origin", "--delete", &branch], &main_repo).await;
                let _ = tokio::fs::remove_dir_all(&wt_path).await;
                removed += 1;
            } else {
                kept += 1;
            }
        }
    }

    // Drop any dangling worktree entries whose directories went away outside our flow.
    for main_repo in touched_main_repos {
        let _ = run_git(&["worktree", "prune"], &main_repo).await;
    }

    (removed, kept)
}

// Chat message processing has been moved to commands/chat.rs (direct API, no worker dependency)

// ── Helpers ─────────────────────────────────────────────────────────

fn emit_worker_event(app: &tauri::AppHandle, event_type: &str, message: &str, task_id: Option<&str>) {
    let _ = app.emit("worker-event", WorkerEvent {
        event_type: event_type.to_string(),
        message: message.to_string(),
        task_id: task_id.map(|s| s.to_string()),
    });
}

fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// Escape special characters for Telegram MarkdownV2 parse mode.
fn escape_markdown_v2(text: &str) -> String {
    let special = ['_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!'];
    let mut escaped = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if special.contains(&ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// Send a Telegram notification. Silently skips if token/chat_id are missing.
/// Never blocks or fails task execution.
async fn send_telegram(config: &SupabaseConfig, message: &str) {
    let (token, chat_id) = match (&config.telegram_bot_token, &config.telegram_chat_id) {
        (Some(t), Some(c)) if !t.is_empty() && !c.is_empty() => (t.clone(), c.clone()),
        _ => return,
    };

    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "MarkdownV2",
    });

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(_) => return,
    };

    if let Err(e) = client.post(&url).json(&body).send().await {
        log::warn!("[worker] Telegram send failed: {}", e);
    }
}

// ── Attachments ─────────────────────────────────────────────────────

/// Upload raw bytes to the `task-attachments` Supabase Storage bucket and
/// return the public URL. Used by the Telegram ingress path after it
/// downloads a photo/document via Telegram's getFile API.
async fn upload_bytes_to_task_attachments(
    config: &SupabaseConfig,
    bytes: Vec<u8>,
    mime: &str,
    suggested_name: Option<&str>,
) -> Result<String, String> {
    let key_prefix = uuid::Uuid::new_v4().to_string();
    let ext = suggested_name
        .and_then(|n| n.rfind('.').map(|i| n[i..].to_string()))
        .unwrap_or_else(|| match mime {
            "image/png" => ".png".into(),
            "image/jpeg" | "image/jpg" => ".jpg".into(),
            "image/gif" => ".gif".into(),
            "image/webp" => ".webp".into(),
            "image/svg+xml" => ".svg".into(),
            "application/pdf" => ".pdf".into(),
            _ => ".bin".into(),
        });
    let key = format!("{}{}", key_prefix, ext);
    let url = format!(
        "{}/storage/v1/object/task-attachments/{}",
        config.url.trim_end_matches('/'),
        key
    );
    let token = config
        .service_role_key
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&config.anon_key);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("client: {}", e))?;
    let resp = client
        .post(&url)
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", mime)
        .header("x-upsert", "false")
        .body(bytes)
        .send()
        .await
        .map_err(|e| format!("storage upload: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("storage {}: {}", status, body));
    }
    Ok(format!(
        "{}/storage/v1/object/public/task-attachments/{}",
        config.url.trim_end_matches('/'),
        key
    ))
}

/// Download an attachment URL to a local file under /tmp/samwise-attachments/<task_id>/
/// so Claude Code can read it off disk. Returns (local_path, original_name).
async fn download_attachment_for_task(
    task_id: &str,
    url: &str,
    name_hint: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    let dir = std::path::PathBuf::from(std::env::temp_dir())
        .join("samwise-attachments")
        .join(task_id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(format!("mkdir {}: {}", dir.display(), e));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("client: {}", e))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {}: {}", url, e))?;
    if !resp.status().is_success() {
        return Err(format!("GET {} returned {}", url, resp.status()));
    }
    let name = name_hint
        .map(|s| s.to_string())
        .or_else(|| {
            url.rsplit('/')
                .next()
                .and_then(|s| s.split('?').next())
                .map(|s| s.to_string())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}.bin", uuid::Uuid::new_v4()));
    let safe = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    let path = dir.join(&safe);
    let bytes = resp.bytes().await.map_err(|e| format!("body: {}", e))?;
    std::fs::write(&path, &bytes).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(path)
}

/// Pull task.attachments out of the raw JSON row and download each one to a
/// per-task scratch dir. Returns the list of successfully downloaded local
/// paths. Errors are logged but never fatal — a broken URL shouldn't kill
/// the whole task.
async fn materialize_task_attachments(
    task_id: &str,
    task: &serde_json::Value,
) -> Vec<std::path::PathBuf> {
    let Some(arr) = task.get("attachments").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in arr {
        let url = entry.get("url").and_then(|v| v.as_str());
        let name = entry.get("name").and_then(|v| v.as_str());
        let Some(u) = url else { continue; };
        match download_attachment_for_task(task_id, u, name).await {
            Ok(p) => out.push(p),
            Err(e) => log::warn!("[worker] attachment download failed ({}): {}", u, e),
        }
    }
    out
}

/// Fetch a Telegram-hosted file by its `file_id` and return the raw bytes
/// plus a best-guess mime/name. Telegram getFile returns a `file_path` which
/// is then fetched from `https://api.telegram.org/file/bot<token>/<file_path>`.
async fn download_telegram_file(
    token: &str,
    file_id: &str,
) -> Result<(Vec<u8>, String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("client: {}", e))?;

    // Step 1: getFile → file_path
    let info_url = format!(
        "https://api.telegram.org/bot{}/getFile?file_id={}",
        token, file_id
    );
    let info: serde_json::Value = client
        .get(&info_url)
        .send()
        .await
        .map_err(|e| format!("getFile: {}", e))?
        .json()
        .await
        .map_err(|e| format!("getFile body: {}", e))?;

    let file_path = info
        .get("result")
        .and_then(|r| r.get("file_path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("getFile missing file_path: {}", info))?;

    // Step 2: download file
    let dl_url = format!("https://api.telegram.org/file/bot{}/{}", token, file_path);
    let resp = client
        .get(&dl_url)
        .send()
        .await
        .map_err(|e| format!("download: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("download {}: {}", dl_url, resp.status()));
    }

    let name = file_path.rsplit('/').next().unwrap_or("telegram-file").to_string();
    let ext = name.rfind('.').map(|i| &name[i + 1..]).unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string();

    let bytes = resp.bytes().await.map_err(|e| format!("body: {}", e))?;
    Ok((bytes.to_vec(), mime, name))
}

// ── Telegram Inbound ────────────────────────────────────────────────

/// Check for new Telegram messages and process them through Sam's chat logic.
async fn check_telegram_messages(
    config: &SupabaseConfig,
    last_update_id: &Arc<tokio::sync::Mutex<Option<i64>>>,
    machine_name: &str,
) {
    let (token, expected_chat_id) = match (&config.telegram_bot_token, &config.telegram_chat_id) {
        (Some(t), Some(c)) if !t.is_empty() && !c.is_empty() => (t.clone(), c.clone()),
        _ => return,
    };

    // Get the offset (last_update_id + 1)
    let offset = {
        let guard = last_update_id.lock().await;
        guard.map(|id| id + 1)
    };

    // Poll Telegram getUpdates
    let mut url = format!("https://api.telegram.org/bot{}/getUpdates?timeout=0", token);
    if let Some(off) = offset {
        url.push_str(&format!("&offset={}", off));
    }

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(_) => return,
    };

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[worker] Telegram getUpdates failed: {}", e);
            return;
        }
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(b) => b,
        Err(_) => return,
    };

    let Some(results) = body.get("result").and_then(|r| r.as_array()) else { return; };
    if results.is_empty() { return; }

    // Telegram splits long messages (>4096 chars) into multiple updates that all
    // arrive within a second. Treating each as its own conversation turn breaks
    // pastes of bug reports into 3-4 separate tasks. Instead, collect all text
    // messages from the configured chat in this poll batch and process as one.
    // Photos/documents sent in the same batch are uploaded to storage and
    // attached to a directly-created task (bypassing the chat flow since a
    // screenshot almost always means "here's a bug, fix it").
    let mut combined_parts: Vec<String> = Vec::new();
    let mut pending_file_ids: Vec<(String, Option<String>)> = Vec::new(); // (file_id, caption)
    let mut highest_update_id: i64 = 0;

    for update in results {
        let update_id = update.get("update_id").and_then(|v| v.as_i64()).unwrap_or(0);
        if update_id > highest_update_id { highest_update_id = update_id; }

        let message = match update.get("message") {
            Some(m) => m,
            None => continue,
        };

        let chat_id = message.get("chat").and_then(|c| c.get("id")).and_then(|v| v.as_i64());
        let chat_id_str = chat_id.map(|id| id.to_string()).unwrap_or_default();
        if chat_id_str != expected_chat_id { continue; }

        let caption = message
            .get("caption")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // Photos: Telegram sends an array of sizes; take the largest (last).
        if let Some(sizes) = message.get("photo").and_then(|v| v.as_array()) {
            if let Some(largest) = sizes.last() {
                if let Some(fid) = largest.get("file_id").and_then(|v| v.as_str()) {
                    pending_file_ids.push((fid.to_string(), caption.clone()));
                    if let Some(c) = caption.clone() { combined_parts.push(c); }
                    continue;
                }
            }
        }

        // Documents (e.g. PDFs, images sent as files): also route to attachments
        // when the mime looks like media. Non-media docs are ignored for now.
        if let Some(doc) = message.get("document") {
            let mime = doc.get("mime_type").and_then(|v| v.as_str()).unwrap_or("");
            let is_media = mime.starts_with("image/") || mime == "application/pdf";
            if is_media {
                if let Some(fid) = doc.get("file_id").and_then(|v| v.as_str()) {
                    pending_file_ids.push((fid.to_string(), caption.clone()));
                    if let Some(c) = caption.clone() { combined_parts.push(c); }
                    continue;
                }
            }
        }

        if let Some(text) = message.get("text").and_then(|v| v.as_str()) {
            if !text.is_empty() {
                combined_parts.push(text.to_string());
            }
        }
    }

    // Advance offset past everything we saw, whether we processed text or not,
    // so we don't re-fetch the same updates next tick.
    if highest_update_id > 0 {
        let mut guard = last_update_id.lock().await;
        *guard = Some(highest_update_id);
    }

    // Media branch: any photos/docs in this batch → create a task directly
    // with attachments. The combined text (including caption) becomes the
    // task body. This bypasses the chat flow because a screenshot almost
    // always means "here's a bug, fix it".
    if !pending_file_ids.is_empty() {
        let mut stored: Vec<serde_json::Value> = Vec::new();
        for (fid, _caption) in &pending_file_ids {
            match download_telegram_file(&token, fid).await {
                Ok((bytes, mime, name)) => {
                    match upload_bytes_to_task_attachments(config, bytes, &mime, Some(&name)).await {
                        Ok(url) => stored.push(serde_json::json!({
                            "url": url, "name": name, "mime": mime,
                        })),
                        Err(e) => log::warn!("[worker] Telegram attachment upload failed: {}", e),
                    }
                }
                Err(e) => log::warn!("[worker] Telegram getFile/download failed for {}: {}", fid, e),
            }
        }

        let body_text = combined_parts.join("\n\n");
        let (title, description) = if body_text.is_empty() {
            (
                format!("Image from Telegram ({} attached)", stored.len()),
                "Matt sent images from Telegram with no caption. Open the attachments to see the bug/screenshot, then ask for clarification or proceed if the intent is obvious.".to_string(),
            )
        } else {
            let first_line = body_text.lines().next().unwrap_or("Image from Telegram").chars().take(120).collect::<String>();
            (first_line, body_text)
        };

        let mut task_row = serde_json::json!({
            "title": title,
            "description": description,
            "status": "queued",
            "priority": "medium",
            "task_type": "code",
            "source": "telegram",
            "attachments": stored,
        });
        // Best-effort project inference via substring match on ae_projects.
        if let Ok(projects) = supabase::fetch_projects(config).await {
            if let Some(arr) = projects.as_array() {
                let hay = format!("{}\n{}", task_row["title"].as_str().unwrap_or(""), task_row["description"].as_str().unwrap_or("")).to_lowercase();
                let mut names: Vec<&str> = arr.iter()
                    .filter_map(|p| p.get("name").and_then(|v| v.as_str()))
                    .collect();
                names.sort_by_key(|n| std::cmp::Reverse(n.len()));
                if let Some(n) = names.into_iter().find(|n| hay.contains(&n.to_lowercase())) {
                    task_row["project"] = serde_json::Value::String(n.to_string());
                    if let Some(row) = arr.iter().find(|p| p.get("name").and_then(|v| v.as_str()) == Some(n)) {
                        if let Some(v) = row.get("repo_url") { task_row["repo_url"] = v.clone(); }
                        if let Some(v) = row.get("repo_path") { task_row["repo_path"] = v.clone(); }
                        if let Some(v) = row.get("preview_url") { task_row["preview_url"] = v.clone(); }
                    }
                }
            }
        }

        match supabase::create_task(config, &task_row).await {
            Ok(_) => {
                log::info!("[worker] Telegram: created task with {} attachment(s)", stored.len());
                send_telegram(config, &format!(
                    "Got it. Queued a task with {} attachment{}.",
                    stored.len(), if stored.len() == 1 { "" } else { "s" }
                )).await;
            }
            Err(e) => log::warn!("[worker] Telegram attachment task insert failed: {}", e),
        }
        return;
    }

    if combined_parts.is_empty() { return; }

    let combined_text = combined_parts.join("\n\n");
    let part_count = combined_parts.len();
    if part_count > 1 {
        log::info!(
            "[worker] Telegram: merged {} message parts into one conversation turn ({} chars)",
            part_count, combined_text.len()
        );
    } else {
        log::info!("[worker] Telegram message received: {}", &combined_text[..combined_text.len().min(50)]);
    }

    process_telegram_message(config, &combined_text, machine_name).await;
}

// ── Remote Chat Message Processing ───────────────────────────────────

/// Check Supabase for user messages flagged needs_response=true (from viewer machines)
async fn check_remote_chat_messages(config: &SupabaseConfig, machine_name: &str) {
    let messages = match supabase::fetch_pending_chat_messages(config).await {
        Ok(m) => m,
        Err(e) => {
            log::debug!("[worker] Remote chat fetch failed (column may not exist yet): {}", e);
            return;
        }
    };

    if messages.is_empty() {
        return;
    }

    for msg in &messages {
        let msg_id = msg.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or_default();

        if msg_id.is_empty() || content.is_empty() {
            if !msg_id.is_empty() {
                let _ = supabase::mark_message_responded(config, msg_id).await;
            }
            continue;
        }

        // Mark as responded BEFORE processing to prevent duplicate pickup on next poll
        if let Err(e) = supabase::mark_message_responded(config, msg_id).await {
            log::warn!("[worker] Failed to claim message {}: {}", msg_id, e);
            continue; // Skip if we can't claim it
        }

        let preview: String = content.chars().take(50).collect();
        log::info!("[worker] Remote chat message received: {}", preview);

        process_remote_chat_message(config, msg_id, content, machine_name).await;
    }
}

/// Process a single remote chat message: generate Sam's response, save to ae_messages, mark responded.
async fn process_remote_chat_message(config: &SupabaseConfig, message_id: &str, user_message: &str, machine_name: &str) {
    use super::chat;

    // 0. Fast-path: confirmation of pending tasks (checked BEFORE status query)
    if let Some(response_text) = chat::handle_pending_confirmation(config, user_message).await {
        let _ = supabase::send_message(config, &serde_json::json!({
            "role": "agent",
            "content": &response_text,
            "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
        })).await;
        return;
    }

    // 0b. Fast-path: status queries skip Claude Code entirely
    if chat::is_status_query(user_message) {
        let board_ctx = build_simple_board_context(config, machine_name).await;
        let response_text = chat::build_status_response(&board_ctx);
        let _ = supabase::send_message(config, &serde_json::json!({
            "role": "agent",
            "content": &response_text,
            "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
        })).await;
        log::info!("[worker] Remote chat status fast-path for message {}", message_id);
        return;
    }

    // 1. Build context
    let recent_chat = chat::fetch_recent_chat(config).await;
    let project_registry = chat::build_project_registry(config).await;
    let board_ctx = build_simple_board_context(config, machine_name).await;

    // 1b. Extract @ mentions
    let projects_all = supabase::fetch_projects(config).await.ok().unwrap_or(serde_json::json!([]));
    let mentioned_projects = chat::extract_project_mentions(user_message, &projects_all);

    // 2. Build prompt
    let effective_message = if !mentioned_projects.is_empty() {
        format!(
            "{}\n\n[System: Matt explicitly tagged @{}. Use this project for any tasks you create.]",
            user_message,
            mentioned_projects.join(", @")
        )
    } else {
        user_message.to_string()
    };
    let prompt = chat::build_system_prompt(&board_ctx, &project_registry, &recent_chat, &effective_message);

    // 3. Call Claude Code CLI one-shot
    let raw_response = match run_claude_code_opts(".", &prompt, 3, 90).await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[worker] Remote chat response failed: {}", e);
            let error_msg = format!("Sorry, hit a snag: {}. Try again?", e);
            let _ = supabase::send_message(config, &serde_json::json!({
                "role": "agent",
                "content": &error_msg,
                "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
            })).await;
            return;
        }
    };

    // 4. Parse for task creation
    let (clean_text, task_requests) = chat::parse_chat_response(&raw_response);

    // 5. Create tasks with @ mention handling
    for req in &task_requests {
        let mut enriched = req.clone();
        if let Some(mentioned) = mentioned_projects.first() {
            enriched["project"] = serde_json::Value::String(mentioned.clone());
            enriched["status"] = serde_json::Value::String("queued".to_string());
        } else {
            enriched["status"] = serde_json::Value::String("pending_confirmation".to_string());
        }
        // Backfill repo fields
        let project_name = enriched.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !project_name.is_empty() {
            if let Some(arr) = projects_all.as_array() {
                if let Some(proj) = arr.iter().find(|p| p.get("name").and_then(|v| v.as_str()) == Some(&project_name)) {
                    for field in &["repo_path", "repo_url", "preview_url"] {
                        if enriched.get(*field).and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                            if let Some(v) = proj.get(*field).filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                                enriched[*field] = v.clone();
                            }
                        }
                    }
                }
            }
        }
        if let Err(e) = supabase::create_task(config, &enriched).await {
            log::warn!("[worker] Failed to create task from remote chat: {}", e);
        }
    }

    // 6. Save Sam's response to ae_messages
    let response_text = if clean_text.trim().is_empty() {
        raw_response.trim().to_string()
    } else {
        clean_text.trim().to_string()
    };

    let _ = supabase::send_message(config, &serde_json::json!({
        "role": "agent",
        "content": &response_text,
        "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
    })).await;

    log::info!("[worker] Remote chat response sent for message {}", message_id);
}

/// Process a single Telegram message: save to chat, get Sam's response, create tasks, reply.
async fn process_telegram_message(config: &SupabaseConfig, user_message: &str, machine_name: &str) {
    use super::chat;

    // 1. Save user message to ae_messages (shows in desktop chat UI)
    let _ = supabase::send_message(config, &serde_json::json!({
        "role": "user",
        "content": user_message,
        "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
    })).await;

    // 1b. Fast-path: confirmation of pending tasks (checked BEFORE status query)
    if let Some(response_text) = chat::handle_pending_confirmation(config, user_message).await {
        let _ = supabase::send_message(config, &serde_json::json!({
            "role": "agent",
            "content": &response_text,
            "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
        })).await;
        send_telegram_plain(config, &response_text).await;
        return;
    }

    // 1c. Fast-path: status queries skip Claude Code entirely
    if chat::is_status_query(user_message) {
        let board_ctx = build_simple_board_context(config, machine_name).await;
        let response_text = chat::build_status_response(&board_ctx);
        let _ = supabase::send_message(config, &serde_json::json!({
            "role": "agent",
            "content": &response_text,
            "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
        })).await;
        send_telegram_plain(config, &response_text).await;
        return;
    }

    // 2. Build context (reuse chat.rs functions)
    let recent_chat = chat::fetch_recent_chat(config).await;
    let project_registry = chat::build_project_registry(config).await;
    let board_ctx = build_simple_board_context(config, machine_name).await;

    // 2b. Extract @ mentions
    let projects_all = supabase::fetch_projects(config).await.ok().unwrap_or(serde_json::json!([]));
    let mentioned_projects = chat::extract_project_mentions(user_message, &projects_all);

    // 3. Build prompt (inject @ mention if present)
    let effective_message = if !mentioned_projects.is_empty() {
        format!(
            "{}\n\n[System: Matt explicitly tagged @{}. Use this project for any tasks you create.]",
            user_message,
            mentioned_projects.join(", @")
        )
    } else {
        user_message.to_string()
    };
    let prompt = chat::build_system_prompt(&board_ctx, &project_registry, &recent_chat, &effective_message);

    // 4. Call Claude Code CLI one-shot
    // 600s Claude timeout — matches chat.rs. Long Sentry dumps and big error
    // pastes regularly push Opus past a minute; 90s was too tight.
    let raw_response = match run_claude_code_opts(".", &prompt, 3, 600).await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[worker] Telegram chat response failed: {}", e);
            let error_msg = format!("Sorry, hit a snag: {}. Try again?", e);
            let _ = supabase::send_message(config, &serde_json::json!({
                "role": "agent",
                "content": &error_msg,
                "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
            })).await;
            send_telegram(config, &escape_markdown_v2(&error_msg)).await;
            return;
        }
    };

    // 5. Parse for task creation
    let (clean_text, task_requests) = chat::parse_chat_response(&raw_response);

    // 6. Create tasks - enrich with project registry data + handle @ mentions
    // Mirrors chat.rs: if Sam picked a project (via @mention OR his own
    // inference), task goes straight to queued. Only gate when we truly
    // can't resolve a project.
    for req in &task_requests {
        let mut enriched = req.clone();

        // Override project with @ mention if present
        if let Some(mentioned) = mentioned_projects.first() {
            enriched["project"] = serde_json::Value::String(mentioned.clone());
        }

        // Rescue: Claude sometimes says "queuing up for operly" in the text
        // but omits the "project" field from the task JSON. Infer from the
        // user message + Sam's reply text.
        let has_project_now = enriched.get("project").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
        if !has_project_now {
            if let Some(arr) = projects_all.as_array() {
                let mut names: Vec<String> = arr.iter()
                    .filter_map(|p| p.get("name").and_then(|v| v.as_str()).map(str::to_string))
                    .collect();
                names.sort_by_key(|n| std::cmp::Reverse(n.len()));
                let haystack = format!(
                    "{}\n{}\n{}",
                    user_message.to_lowercase(),
                    clean_text.to_lowercase(),
                    raw_response.to_lowercase()
                );
                for name in &names {
                    if haystack.contains(&name.to_lowercase()) {
                        enriched["project"] = serde_json::Value::String(name.clone());
                        log::info!("[telegram] inferred project '{}' from conversation text", name);
                        break;
                    }
                }
            }
        }

        let has_project = enriched.get("project").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
        if !has_project {
            log::warn!("[telegram] Skipping task create: no project resolvable. Sam should ask via reply.");
            continue;
        }
        enriched["status"] = serde_json::Value::String("queued".to_string());

        // Backfill repo fields
        let project_name = enriched.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !project_name.is_empty() {
            if let Some(arr) = projects_all.as_array() {
                if let Some(proj) = arr.iter().find(|p| p.get("name").and_then(|v| v.as_str()) == Some(&project_name)) {
                    if enriched.get("repo_path").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                        if let Some(v) = proj.get("repo_path").filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                            enriched["repo_path"] = v.clone();
                        }
                    }
                    if enriched.get("repo_url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                        if let Some(v) = proj.get("repo_url").filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                            enriched["repo_url"] = v.clone();
                        }
                    }
                    if enriched.get("preview_url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                        if let Some(v) = proj.get("preview_url").filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                            enriched["preview_url"] = v.clone();
                        }
                    }
                }
            }
        }

        if let Err(e) = supabase::create_task(config, &enriched).await {
            log::warn!("[worker] Failed to create task from Telegram: {}", e);
        }
    }

    // 7. Save Sam's response to ae_messages
    let response_text = if clean_text.trim().is_empty() {
        raw_response.trim().to_string()
    } else {
        clean_text.trim().to_string()
    };

    let _ = supabase::send_message(config, &serde_json::json!({
        "role": "agent",
        "content": &response_text,
        "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
    })).await;

    // 7b. Send response back via Telegram first
    send_telegram_plain(config, &response_text).await;

    // 7c. Only prompt for project when Sam truly couldn't pick one. If any
    // task_request already has a project (from @mention or Sam's own
    // inference), trust it — Sam mentions the choice in his reply already.
    let any_task_unresolved = task_requests.iter().any(|r| {
        r.get("project").and_then(|v| v.as_str()).map(|s| s.is_empty()).unwrap_or(true)
    });
    if any_task_unresolved && mentioned_projects.is_empty() {
        if let Some(arr) = projects_all.as_array() {
            if !arr.is_empty() {
                let mut list = String::from("Which project?\n");
                for (i, proj) in arr.iter().enumerate() {
                    let name = proj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    list.push_str(&format!("{}. {}\n", i + 1, name));
                }
                list.push_str("\nReply with the number.");
                send_telegram_plain(config, &list).await;
            }
        }
    }
}

/// Build board context without WorkerState (for Telegram handler in worker loop)
async fn build_simple_board_context(config: &SupabaseConfig, machine_name: &str) -> String {
    let mut ctx = String::new();

    let tasks = match supabase::fetch_tasks(config, None).await {
        Ok(t) => t,
        Err(_) => return "Board: unable to fetch".to_string(),
    };

    let Some(arr) = tasks.as_array() else {
        return "Board: no tasks".to_string();
    };

    let mut counts = std::collections::HashMap::new();
    for task in arr {
        let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
        let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
        let priority = task.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");
        *counts.entry(status.to_string()).or_insert(0u32) += 1;

        if status != "done" {
            ctx.push_str(&format!("- [{}] {} ({})\n", priority.to_uppercase(), title, status));
        }
    }

    let summary = format!(
        "Queued: {} | In Progress: {} | Testing: {} | Review: {}\n",
        counts.get("queued").unwrap_or(&0),
        counts.get("in_progress").unwrap_or(&0),
        counts.get("testing").unwrap_or(&0),
        counts.get("review").unwrap_or(&0),
    );

    format!("{}{}Worker: ONLINE on {}\n", summary, ctx, machine_name)
}

/// Send a Telegram message as plain text (no MarkdownV2 parsing issues).
async fn send_telegram_plain(config: &SupabaseConfig, message: &str) {
    let (token, chat_id) = match (&config.telegram_bot_token, &config.telegram_chat_id) {
        (Some(t), Some(c)) if !t.is_empty() && !c.is_empty() => (t.clone(), c.clone()),
        _ => return,
    };

    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
    });

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(_) => return,
    };

    if let Err(e) = client.post(&url).json(&body).send().await {
        log::warn!("[worker] Telegram reply failed: {}", e);
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

/// Post a proactive message to the chat sidebar (not tied to a specific task).
/// This is how the agent talks to Matt as a teammate.
async fn agent_chat(config: &SupabaseConfig, content: &str) {
    let _ = supabase::send_message(config, &serde_json::json!({
        "role": "agent",
        "content": content,
        "conversation_id": supabase::DEFAULT_CONVERSATION_ID,
    })).await;
}

/// Expire pending_confirmation tasks older than 30 minutes.
async fn expire_pending_confirmations(config: &SupabaseConfig) {
    let tasks = match supabase::fetch_tasks(config, Some("pending_confirmation")).await {
        Ok(t) => t,
        Err(_) => return,
    };

    let Some(arr) = tasks.as_array() else { return; };
    let now = chrono::Utc::now();
    let mut expired_titles: Vec<String> = Vec::new();

    for task in arr {
        let created_at = task.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let task_title = task.get("title").and_then(|v| v.as_str()).unwrap_or("untitled");

        if task_id.is_empty() || created_at.is_empty() { continue; }

        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_at) {
            let age = now - created.with_timezone(&chrono::Utc);
            if age.num_minutes() >= 30 {
                // Use conditional update to avoid racing with a user confirmation
                let _ = supabase::update_task_if_status(config, task_id, "pending_confirmation", &serde_json::json!({
                    "status": "failed",
                    "updated_at": now.to_rfc3339(),
                })).await;
                expired_titles.push(task_title.to_string());
            }
        }
    }

    // Send a single batched notification for all expired tasks
    if !expired_titles.is_empty() {
        let msg = if expired_titles.len() == 1 {
            format!("Task \"{}\" expired waiting for project confirmation. Create a new task with @project to retry.", expired_titles[0])
        } else {
            let names: Vec<String> = expired_titles.iter().map(|t| format!("\"{}\"", t)).collect();
            format!("{} tasks expired waiting for project confirmation: {}. Create new tasks with @project to retry.", expired_titles.len(), names.join(", "))
        };
        agent_chat(config, &msg).await;
    }
}

/// Run Claude Code CLI one-shot with explicit max_turns and timeout_secs.
/// Pass 0 for either to use defaults (no limit / no timeout).
/// Also used by commands/chat.rs for direct chat responses.
pub async fn run_claude_code_opts(cwd: &str, prompt: &str, max_turns: u32, timeout_secs: u64) -> Result<String, String> {
    let (exe, prefix_args) = super::claude_code::find_claude_command();

    let mut cmd = async_cmd(&exe);
    for arg in &prefix_args {
        cmd.arg(arg);
    }

    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("text")
        .arg("--dangerously-skip-permissions")
        .arg("--model")
        .arg(super::claude_code::CLAUDE_MODEL);

    if max_turns > 0 {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    // Resolve cwd. Callers pass "." for "no particular repo" chat calls, but
    // when Samwise.app is launched from Finder the inherited cwd is "/". Any
    // filesystem walk the CLI does from there hits /Volumes and triggers the
    // macOS "Allow network volumes" TCC prompt, which blocks the worker.
    // Pin those calls to a stable per-user Samwise scratch dir instead.
    let resolved_cwd = resolve_chat_cwd(cwd);
    cmd.current_dir(&resolved_cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to run Claude Code: {}", e))?;

    // Read stdout in background
    let stdout = child.stdout.take();
    let stdout_handle = tokio::spawn(async move {
        let mut output = String::new();
        if let Some(mut reader) = stdout {
            use tokio::io::AsyncReadExt;
            let _ = reader.read_to_string(&mut output).await;
        }
        output
    });

    // Read stderr in background (contains progress info)
    let stderr = child.stderr.take();
    let stderr_handle = tokio::spawn(async move {
        let mut output = String::new();
        if let Some(mut reader) = stderr {
            use tokio::io::AsyncReadExt;
            let _ = reader.read_to_string(&mut output).await;
        }
        output
    });

    // Wait for process with optional timeout
    let status = if timeout_secs > 0 {
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait()
        ).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(format!("Claude Code process error: {}", e)),
            Err(_) => {
                let _ = child.kill().await;
                return Err(format!("Claude Code timed out after {}s", timeout_secs));
            }
        }
    } else {
        child.wait().await.map_err(|e| format!("Claude Code process error: {}", e))?
    };

    let stdout_text = stdout_handle.await.unwrap_or_default();
    let stderr_text = stderr_handle.await.unwrap_or_default();

    if !status.success() {
        let stderr = stderr_text.trim();
        if let Some(msg) = detect_login_required(stderr).or_else(|| detect_login_required(stdout_text.trim())) {
            return Err(msg);
        }
        if let Some(msg) = detect_rate_limit(stderr).or_else(|| detect_rate_limit(stdout_text.trim())) {
            return Err(msg);
        }
        // Fall back to stdout tail when stderr is empty so the failure
        // message is never a naked "Claude Code failed (exit X): ".
        let detail = if !stderr.is_empty() {
            stderr.to_string()
        } else {
            let tail = stdout_text.trim();
            let snippet: String = if tail.len() > 1200 {
                format!("…{}", &tail[tail.len() - 1200..])
            } else {
                tail.to_string()
            };
            if snippet.is_empty() {
                "no stderr, no stdout captured".to_string()
            } else {
                format!("no stderr — stdout tail: {}", snippet)
            }
        };
        return Err(format!("Claude Code failed (exit {}): {}", status, detail));
    }

    // Some rate-limit errors show on stdout with exit=0. Catch those too.
    if let Some(msg) = detect_rate_limit(stdout_text.trim()) {
        return Err(msg);
    }

    Ok(stdout_text.trim().to_string())
}

/// Resolve the cwd for chat Claude Code invocations.
///
/// Callers pass "." to mean "no specific repo — just chat". When Samwise.app
/// is launched from Finder the process cwd is "/", so "." makes the CLI walk
/// the filesystem from root. That hits /Volumes/* and triggers the macOS
/// "Allow network volumes" TCC prompt, which blocks the worker behind a modal
/// the user has to dismiss on the Mac mini.
///
/// Resolution:
/// - Absolute paths and non-"." relative paths pass through unchanged (task
///   workers always supply a concrete repo path).
/// - "." and "" are rewritten to $HOME/samwise, creating the dir if needed.
///   Fall back to $HOME, then "/tmp" as a last resort. Never "/".
fn resolve_chat_cwd(cwd: &str) -> String {
    let trimmed = cwd.trim();
    if !trimmed.is_empty() && trimmed != "." {
        return trimmed.to_string();
    }
    if let Ok(home) = std::env::var("HOME") {
        let scratch = std::path::PathBuf::from(&home).join("samwise");
        let _ = std::fs::create_dir_all(&scratch);
        if scratch.exists() {
            return scratch.to_string_lossy().into_owned();
        }
        return home;
    }
    "/tmp".to_string()
}

/// Detect the Claude Code CLI "not logged in" state so Matt gets a clear
/// instruction instead of a generic "Claude Code failed" message.
fn detect_login_required(text: &str) -> Option<String> {
    if text.is_empty() { return None; }
    let lower = text.to_lowercase();
    let needles = [
        "not logged in",
        "please run /login",
        "run `claude /login`",
        "run /login",
    ];
    if needles.iter().any(|n| lower.contains(n)) {
        return Some("Claude Code CLI isn't logged in on this machine. Run `claude /login` in a terminal, then retry.".to_string());
    }
    None
}

/// Detect the common shapes of Claude/Anthropic rate-limit and quota errors.
/// When matched, returns a surface-able message for Matt instead of letting
/// the generic "Claude Code failed" swallow useful context. Returns None if
/// the output doesn't look like a rate-limit failure.
///
/// Needles must be specific enough to not match natural-language task
/// descriptions. "rate limit" / "quota" / "429" appear in bug reports Sam
/// relays verbatim (e.g. "users hit 429 when..."), so we only match
/// machine-generated error shapes: API error type strings, CLI banner text,
/// or explicit HTTP status envelopes.
fn detect_rate_limit(text: &str) -> Option<String> {
    if text.is_empty() { return None; }
    let lower = text.to_lowercase();
    let needles = [
        "rate_limit_error",
        "rate_limit_exceeded",
        "overloaded_error",
        "insufficient_quota",
        "claude ai usage limit reached",
        "\"status\":429",
        "\"status\": 429",
        "status code 429",
        "http 429",
    ];
    if needles.iter().any(|n| lower.contains(n)) {
        let short = if text.len() > 400 { &text[..400] } else { text };
        return Some(format!("Hit a Claude rate / usage limit. Wait a few minutes and retry. Raw: {}", short.trim()));
    }
    None
}

/// Run Claude Code CLI with stream-json output, posting progress comments as it works.
/// Returns the final text output. Used for worker tasks where transparency is important.
pub async fn run_claude_code_streaming(
    cwd: &str,
    prompt: &str,
    max_turns: u32,
    timeout_secs: u64,
    config: &supabase::SupabaseConfig,
    task_id: &str,
    process_id_slot: Arc<tokio::sync::Mutex<Option<u32>>>,
) -> Result<String, String> {
    let (exe, prefix_args) = super::claude_code::find_claude_command();

    let mut cmd = async_cmd(&exe);
    for arg in &prefix_args {
        cmd.arg(arg);
    }

    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions")
        .arg("--model")
        .arg(super::claude_code::CLAUDE_MODEL);

    if max_turns > 0 {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    let resolved_cwd = resolve_chat_cwd(cwd);
    cmd.current_dir(&resolved_cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to run Claude Code: {}", e))?;

    // Store PID so the process can be killed externally via stop_current_task
    {
        let mut pid = process_id_slot.lock().await;
        *pid = Some(child.id().unwrap_or(0));
    }

    let stdout = child.stdout.take();
    let config_clone = config.clone();
    let task_id_owned = task_id.to_string();

    // Shared "last activity" timestamp. The stream parser updates it whenever
    // it posts a progress comment; the heartbeat task reads it and posts a
    // "still working" nudge when the stream goes quiet for too long (codex-fix,
    // deep thinking, long tool calls). Prevents the UI from going silent.
    let last_activity = std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    let heartbeat_alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    // Signals the outer caller that Matt cancelled the task (deleted it or set
    // status=cancelled). The heartbeat polls the row; if it disappears or flips
    // to cancelled, it kills the Claude child process and sets this flag.
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let task_started_at = std::time::Instant::now();

    // Heartbeat + cancellation-watcher task. Polls every 15s. Two jobs:
    //  - If the task has been quiet in the stream parser for 2+ min, post a
    //    "still working" nudge so the UI never looks dead.
    //  - If the task row was deleted or its status flipped to cancelled,
    //    kill the Claude child process and flip the `cancelled` flag so the
    //    outer caller bails out of the rest of the task pipeline.
    {
        let last_activity_hb = last_activity.clone();
        let alive_hb = heartbeat_alive.clone();
        let cancelled_hb = cancelled.clone();
        let pid_slot_hb = process_id_slot.clone();
        let config_hb = config.clone();
        let task_id_hb = task_id.to_string();
        tokio::spawn(async move {
            use std::sync::atomic::Ordering;
            let mut last_hb_post = std::time::Instant::now();
            while alive_hb.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                if !alive_hb.load(Ordering::Relaxed) { break; }

                // Cancellation check: row missing or cancelled = stop
                let still_live = task_is_live(&config_hb, &task_id_hb).await;
                if !still_live {
                    log::info!("[worker] Task {} was deleted/cancelled mid-run; killing Claude Code subprocess", task_id_hb);
                    cancelled_hb.store(true, Ordering::Relaxed);
                    // Kill the Claude child so stdout_handle wakes up and the
                    // main wait() returns.
                    let pid_opt = { *pid_slot_hb.lock().await };
                    if let Some(pid) = pid_opt {
                        if pid > 0 {
                            #[cfg(unix)]
                            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                        }
                    }
                    alive_hb.store(false, Ordering::Relaxed);
                    break;
                }

                // Quiet-nudge check
                let quiet_for = {
                    let guard = last_activity_hb.lock().unwrap_or_else(|e| e.into_inner());
                    guard.elapsed()
                };
                if quiet_for >= std::time::Duration::from_secs(120)
                    && last_hb_post.elapsed() >= std::time::Duration::from_secs(120)
                {
                    let mins = task_started_at.elapsed().as_secs() / 60;
                    let msg = format!("Still working. {} min in, nothing's hung.", mins);
                    agent_comment(&config_hb, &task_id_hb, &msg).await;
                    last_hb_post = std::time::Instant::now();
                }
            }
        });
    }

    let last_activity_stream = last_activity.clone();

    // Read stdout line-by-line, parse stream-json, post progress comments.
    // Also captures raw-stdout tail + any `result` event carrying an error so
    // non-zero exits can surface a useful diagnostic instead of an empty string
    // (Claude Code CLI emits errors on stdout as JSON, not stderr).
    let stdout_handle = tokio::spawn(async move {
        let mut result_text = String::new();
        let mut raw_tail = String::new();
        let mut error_summary: Option<String> = None;
        const RAW_TAIL_CAP: usize = 4096;
        let mut last_comment_time = std::time::Instant::now();
        // Throttle: don't post more than one progress comment per 10 seconds
        const MIN_COMMENT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

        if let Some(reader) = stdout {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut lines = BufReader::new(reader).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() { continue; }

                // Keep a rolling tail of raw stdout so exit-1 diagnostics are
                // never empty when stderr is silent (common for stream-json).
                if raw_tail.len() + line.len() + 1 > RAW_TAIL_CAP {
                    let drop = (raw_tail.len() + line.len() + 1).saturating_sub(RAW_TAIL_CAP);
                    if drop >= raw_tail.len() { raw_tail.clear(); }
                    else { raw_tail.drain(..drop); }
                }
                raw_tail.push_str(&line);
                raw_tail.push('\n');

                // Try to parse as JSON
                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let event_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");

                // Capture result text + any error carried in the final event.
                // Claude Code stream-json emits errors on stdout, e.g.
                //   {"type":"result","subtype":"error_max_turns","is_error":true,...}
                //   {"type":"result","subtype":"error_during_execution","is_error":true,"error":"..."}
                if event_type == "result" {
                    if let Some(text) = parsed.get("result")
                        .and_then(|v| v.as_str())
                        .or_else(|| parsed.get("result_text").and_then(|v| v.as_str()))
                    {
                        result_text = text.to_string();
                    }
                    let is_error = parsed.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                    let subtype = parsed.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
                    if is_error || subtype.starts_with("error") {
                        let detail = parsed.get("error").and_then(|v| v.as_str())
                            .or_else(|| parsed.get("message").and_then(|v| v.as_str()))
                            .unwrap_or("");
                        let summary = if detail.is_empty() {
                            if subtype.is_empty() { "Claude Code reported an error".to_string() }
                            else { format!("Claude Code error: {}", subtype) }
                        } else if subtype.is_empty() {
                            format!("Claude Code error: {}", detail)
                        } else {
                            format!("Claude Code error ({}): {}", subtype, detail)
                        };
                        error_summary = Some(summary);
                    }
                    continue;
                }

                // Extract progress info from assistant messages
                if event_type == "assistant" {
                    if let Some(content) = parsed.get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                    {
                        for block in content {
                            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

                            if block_type == "tool_use" {
                                let tool_name = block.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                                let input = block.get("input");

                                // Build a human-readable progress message
                                let progress = match tool_name {
                                    "Read" | "read_file" => {
                                        let path = input.and_then(|i| i.get("file_path").or(i.get("path")))
                                            .and_then(|v| v.as_str()).unwrap_or("...");
                                        // Show just the filename, not full path
                                        let short = path.rsplit(['/', '\\']).next().unwrap_or(path);
                                        format!("Reading {}", short)
                                    }
                                    "Edit" | "edit_file" => {
                                        let path = input.and_then(|i| i.get("file_path").or(i.get("path")))
                                            .and_then(|v| v.as_str()).unwrap_or("...");
                                        let short = path.rsplit(['/', '\\']).next().unwrap_or(path);
                                        format!("Editing {}", short)
                                    }
                                    "Write" | "write_file" => {
                                        let path = input.and_then(|i| i.get("file_path").or(i.get("path")))
                                            .and_then(|v| v.as_str()).unwrap_or("...");
                                        let short = path.rsplit(['/', '\\']).next().unwrap_or(path);
                                        format!("Writing {}", short)
                                    }
                                    "Bash" | "bash" => {
                                        let command = input.and_then(|i| i.get("command"))
                                            .and_then(|v| v.as_str()).unwrap_or("...");
                                        let short: String = command.chars().take(80).collect();
                                        format!("Running: {}", short)
                                    }
                                    "Grep" | "grep" => {
                                        let pattern = input.and_then(|i| i.get("pattern"))
                                            .and_then(|v| v.as_str()).unwrap_or("...");
                                        format!("Searching for \"{}\"", pattern)
                                    }
                                    "Glob" | "glob" => {
                                        let pattern = input.and_then(|i| i.get("pattern"))
                                            .and_then(|v| v.as_str()).unwrap_or("...");
                                        format!("Finding files: {}", pattern)
                                    }
                                    "Agent" | "agent" => {
                                        "Spawning a sub-agent...".to_string()
                                    }
                                    _ => {
                                        format!("Using {}", tool_name)
                                    }
                                };

                                // Post comment if enough time has passed since last one
                                if last_comment_time.elapsed() >= MIN_COMMENT_INTERVAL {
                                    agent_comment(&config_clone, &task_id_owned, &progress).await;
                                    last_comment_time = std::time::Instant::now();
                                    // Reset the heartbeat timer; real activity doesn't need a nudge.
                                    if let Ok(mut g) = last_activity_stream.lock() {
                                        *g = std::time::Instant::now();
                                    }
                                } else {
                                    log::debug!("[worker] Throttled progress: {}", progress);
                                }
                            }
                        }
                    }
                }
            }
        }

        (result_text, raw_tail, error_summary)
    });

    // Read stderr in background
    let stderr = child.stderr.take();
    let stderr_handle = tokio::spawn(async move {
        let mut output = String::new();
        if let Some(mut reader) = stderr {
            use tokio::io::AsyncReadExt;
            let _ = reader.read_to_string(&mut output).await;
        }
        output
    });

    // Wait for process with optional timeout
    let status = if timeout_secs > 0 {
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait()
        ).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                heartbeat_alive.store(false, std::sync::atomic::Ordering::Relaxed);
                return Err(format!("Claude Code process error: {}", e));
            }
            Err(_) => {
                heartbeat_alive.store(false, std::sync::atomic::Ordering::Relaxed);
                let _ = child.kill().await;
                return Err(format!("Claude Code timed out after {}s", timeout_secs));
            }
        }
    } else {
        match child.wait().await {
            Ok(s) => s,
            Err(e) => {
                heartbeat_alive.store(false, std::sync::atomic::Ordering::Relaxed);
                return Err(format!("Claude Code process error: {}", e));
            }
        }
    };

    // Process exited cleanly. Shut the heartbeat down so it doesn't fire a
    // stale "still working" message just as we're reporting success.
    heartbeat_alive.store(false, std::sync::atomic::Ordering::Relaxed);

    // If the cancellation watcher killed the subprocess, report that
    // specifically so callers can skip the post-work pipeline (no PR, no
    // comments on a task that no longer exists).
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("TASK_CANCELLED".to_string());
    }

    let (result_text, raw_tail, error_summary) =
        stdout_handle.await.unwrap_or_default();
    let stderr_text = stderr_handle.await.unwrap_or_default();

    if !status.success() {
        let stderr = stderr_text.trim();
        if let Some(msg) = detect_rate_limit(stderr)
            .or_else(|| detect_rate_limit(&result_text))
            .or_else(|| detect_rate_limit(&raw_tail))
        {
            return Err(msg);
        }
        // Build the best possible diagnostic. Prefer: parsed error from the
        // stream-json `result` event → stderr → tail of raw stdout. Never
        // return an empty "Claude Code failed (exit ...): " again.
        let detail = if let Some(s) = error_summary {
            s
        } else if !stderr.is_empty() {
            stderr.to_string()
        } else {
            let tail = raw_tail.trim();
            let snippet: String = if tail.len() > 1200 {
                format!("…{}", &tail[tail.len() - 1200..])
            } else {
                tail.to_string()
            };
            if snippet.is_empty() {
                "no stderr, no stdout captured".to_string()
            } else {
                format!("no stderr — stdout tail: {}", snippet)
            }
        };
        return Err(format!("Claude Code failed (exit {}): {}", status, detail));
    }
    if let Some(msg) = detect_rate_limit(&result_text) {
        return Err(msg);
    }

    Ok(result_text.trim().to_string())
}

/// Is the task still active? Returns false if the row is gone or its status
/// flipped to cancelled. Used by the streaming heartbeat to stop work mid-run
/// when Matt deletes/cancels a task from the UI.
async fn task_is_live(config: &SupabaseConfig, task_id: &str) -> bool {
    let url = format!(
        "{}/rest/v1/ae_tasks?id=eq.{}&select=status",
        config.url, task_id
    );
    let key = config.service_role_key.as_deref().unwrap_or(&config.anon_key);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(_) => return true, // don't cancel work on a transient client error
    };
    let resp = match client
        .get(&url)
        .header("apikey", key)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return true,
    };
    let body: serde_json::Value = match resp.json().await {
        Ok(b) => b,
        Err(_) => return true,
    };
    let Some(arr) = body.as_array() else { return true; };
    if arr.is_empty() { return false; } // row deleted
    let status = arr[0].get("status").and_then(|v| v.as_str()).unwrap_or("");
    !matches!(status, "cancelled")
}

/// Take a screenshot using Playwright (headless chromium).
/// Retries once if the first capture fails, since the app may still be hydrating.
async fn take_screenshot(url: &str, output_path: &str, viewport: &str) -> Result<(), String> {
    // Playwright is installed as a dev dep of the Samwise repo (not globally),
    // so `npx playwright` has to run with cwd inside that repo or npx tries to
    // download a fresh copy that doesn't have the chromium browser installed.
    // Under launchd SamWise's cwd is `/` so the bare call silently fails.
    let playwright_cwd = std::env::var("HOME")
        .map(|h| format!("{}/samwise/Personal-Apps/Samwise", h))
        .unwrap_or_else(|_| "/Users/mjohnst/samwise/Personal-Apps/Samwise".to_string());

    // Give Chromium an isolated user-data-dir inside Samwise's own container
    // area so it never touches ~/Library/Application Support/Google/Chrome or
    // ~/Library/Application Support/Chromium. Under macOS Sequoia's stricter
    // TCC, reading another app's Application Support triggers the "SamWise
    // would like to access data from other apps" prompt and blocks the
    // worker until Matt dismisses it. A fresh scratch dir per run avoids
    // state leakage too.
    let samwise_scratch = std::env::var("HOME")
        .map(|h| format!("{}/Library/Application Support/com.mattjohnston.agent-one/playwright-profile", h))
        .unwrap_or_else(|_| "/tmp/samwise-playwright-profile".to_string());
    let _ = tokio::fs::create_dir_all(&samwise_scratch).await;

    for attempt in 1..=2 {
        let output = async_cmd("npx")
            .args([
                "playwright", "screenshot",
                "--browser", "chromium",
                "--viewport-size", viewport,
                // Give the SPA time to finish auth/Supabase hydration before
                // the snapshot. Two seconds was routinely catching Operly's
                // splash screen even when the dev server had the right env.
                "--wait-for-timeout", "6000",
                "--timeout", "30000",
                // Chromium probes macOS for media devices on launch, which
                // triggers the "Apple Music" / microphone / camera TCC
                // prompts. We never need any of that for a screenshot.
                // These flags skip those subsystems entirely.
                "--user-agent", "Mozilla/5.0 SamWise-QA",
                url, output_path,
            ])
            .env("PLAYWRIGHT_CHROMIUM_ARGS", format!(
                "--user-data-dir={} --disable-features=MediaSessionService,MediaRouter,MediaFoundationVideoCapture,HardwareMediaKeyHandling --mute-audio --disable-audio-output --use-fake-ui-for-media-stream --deny-permission-prompts --no-default-browser-check --disable-sync --disable-background-networking --no-first-run --password-store=basic --use-mock-keychain",
                samwise_scratch
            ))
            .current_dir(&playwright_cwd)
            .output()
            .await
            .map_err(|e| format!("Playwright failed: {}", e))?;
        if output.status.success() {
            return Ok(());
        }
        if attempt == 2 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("playwright screenshot: {}", {
                let s = if stderr.trim().is_empty() { stdout.to_string() } else { stderr.to_string() };
                let s = s.trim();
                if s.len() > 400 { s[s.len() - 400..].to_string() } else { s.to_string() }
            }));
        }
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    }
    Ok(())
}

/// Run visual QA: compare before/after screenshots using Claude Code vision.
/// Uses absolute paths so Claude Code can reliably read the images.
/// Retries once with a stricter prompt if JSON parsing fails.
async fn run_visual_qa(cwd: &str, title: &str, description: &str, screenshot_dir: &str) -> Result<(bool, String), String> {
    // Copy screenshots into the working directory so Claude Code can access them
    let agent_dir = join_path(cwd, ".agent-one");
    let _ = tokio::fs::create_dir_all(&agent_dir).await;

    let screenshot_names = ["before-desktop.png", "after-desktop.png", "before-mobile.png", "after-mobile.png"];
    for name in &screenshot_names {
        let src = join_path(screenshot_dir, name);
        let dst = join_path(&agent_dir, name);
        let _ = tokio::fs::copy(&src, &dst).await;
    }

    // Build absolute paths for the prompt (forward slashes for cross-platform clarity)
    let abs_agent_dir = std::path::Path::new(cwd).join(".agent-one");
    let abs_path = abs_agent_dir.to_string_lossy().replace('\\', "/");

    let prompt = format!(
        r#"You are a visual QA reviewer. Read these screenshot files using their absolute paths:

BEFORE (desktop): {abs}/before-desktop.png
AFTER  (desktop): {abs}/after-desktop.png
BEFORE (mobile):  {abs}/before-mobile.png
AFTER  (mobile):  {abs}/after-mobile.png

Task: {title}
Description: {desc}

Compare the before and after screenshots. Check:
1. Does the change match the task description?
2. Are there any visual regressions (broken layout, missing elements, overlapping text)?
3. Does the mobile view look correct?

Reply with ONLY this JSON (no markdown, no code fences):
{{"pass": true, "explanation": "brief explanation"}}
or
{{"pass": false, "explanation": "what's wrong"}}"#,
        abs = abs_path, title = title, desc = description
    );

    // First attempt: 5 max turns, 120s timeout
    let result = run_claude_code_opts(cwd, &prompt, 5, 120).await?;

    if let Some(parsed) = try_parse_qa_json(&result) {
        cleanup_agent_dir(&agent_dir).await;
        return Ok(parsed);
    }

    // Retry with a stricter prompt if JSON parsing failed
    log::warn!("[worker] Visual QA first attempt returned unparseable output, retrying with stricter prompt");

    let retry_prompt = format!(
        r#"Read these image files and compare them:
- {abs}/before-desktop.png vs {abs}/after-desktop.png
- {abs}/before-mobile.png vs {abs}/after-mobile.png

Does the visual change look correct for this task: "{title}"?

IMPORTANT: Your entire response must be valid JSON with no other text. Example:
{{"pass": true, "explanation": "Changes look correct"}}"#,
        abs = abs_path, title = title
    );

    let retry_result = run_claude_code_opts(cwd, &retry_prompt, 3, 90).await?;

    let parsed = try_parse_qa_json(&retry_result)
        .unwrap_or((true, format!("QA output (unparseable after retry): {}", truncate(&retry_result, 200))));

    cleanup_agent_dir(&agent_dir).await;
    Ok(parsed)
}

/// Try to extract {"pass": bool, "explanation": string} from Claude's response.
fn try_parse_qa_json(text: &str) -> Option<(bool, String)> {
    // Try direct parse first
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(pass) = parsed.get("pass").and_then(|v| v.as_bool()) {
            let explanation = parsed.get("explanation").and_then(|v| v.as_str()).unwrap_or("No explanation").to_string();
            return Some((pass, explanation));
        }
    }

    // Try to find JSON in the response text (Claude sometimes wraps in markdown)
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            let json_str = &text[start..=end];
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(pass) = parsed.get("pass").and_then(|v| v.as_bool()) {
                    let explanation = parsed.get("explanation").and_then(|v| v.as_str()).unwrap_or("No explanation").to_string();
                    return Some((pass, explanation));
                }
            }
        }
    }

    None
}

/// Remove the .agent-one/ temp directory after QA
async fn cleanup_agent_dir(agent_dir: &str) {
    if let Err(e) = tokio::fs::remove_dir_all(agent_dir).await {
        log::warn!("[worker] Failed to clean up {}: {}", agent_dir, e);
    }
}

/// Detect the default branch for a repo (main, master, etc.)
async fn detect_base_branch(repo_path: &str) -> String {
    // Try git symbolic-ref for the remote HEAD
    if let Ok(output) = async_cmd("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .current_dir(repo_path)
        .output()
        .await
    {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Returns "origin/main" or "origin/master", strip the prefix
            if let Some(name) = branch.strip_prefix("origin/") {
                return name.to_string();
            }
            return branch;
        }
    }
    // Fallback: check if "main" exists, otherwise "master"
    if let Ok(output) = async_cmd("git")
        .args(["rev-parse", "--verify", "refs/heads/main"])
        .current_dir(repo_path)
        .output()
        .await
    {
        if output.status.success() { return "main".to_string(); }
    }
    "master".to_string()
}

/// Upload before/after screenshots to Supabase Storage, return markdown image links.
async fn upload_screenshots_to_storage(
    config: &super::supabase::SupabaseConfig,
    task_id: &str,
    screenshot_dir: &str,
) -> Result<(Vec<String>, String), String> {
    let mut urls = Vec::new();
    let screenshot_names = ["before-desktop.png", "after-desktop.png", "before-mobile.png", "after-mobile.png"];

    for name in &screenshot_names {
        let local_path = join_path(screenshot_dir, name);
        if tokio::fs::metadata(&local_path).await.is_ok() {
            let storage_path = format!("{}/{}", task_id, name);
            match supabase::upload_to_storage(config, "agent-one-screenshots", &storage_path, &local_path).await {
                Ok(url) => urls.push(url),
                Err(e) => log::warn!("[worker] Failed to upload {}: {}", name, e),
            }
        }
    }

    // Build markdown for PR body
    let mut md = String::new();
    if urls.len() >= 4 {
        md.push_str("### Visual QA\n\n");
        md.push_str("| Desktop Before | Desktop After |\n|--------|-------|\n");
        md.push_str(&format!("| ![before]({}) | ![after]({}) |\n\n", urls[0], urls[1]));
        md.push_str("| Mobile Before | Mobile After |\n|--------|-------|\n");
        md.push_str(&format!("| ![before]({}) | ![after]({}) |\n\n", urls[2], urls[3]));
    } else if urls.len() >= 2 {
        md.push_str("### Visual QA\n\n");
        md.push_str(&format!("| Before | After |\n|--------|-------|\n| ![before]({}) | ![after]({}) |\n\n", urls[0], urls[1]));
    }

    Ok((urls, md))
}

/// Spawn a detached Codex `$samwise-pr-review` run for a task. On verdict,
/// update the task status (approved / fixes_needed) or leave it in review
/// (inconclusive) and post the markdown body as a Sam comment. Always
/// stamps `last_pr_review_at` so the poll-loop watcher doesn't re-fire on
/// the same card.
pub fn spawn_pr_review_task(
    config: SupabaseConfig,
    task_id: String,
    pr_url: String,
    repo_path: String,
) {
    tokio::spawn(async move {
        // Stamp first so the poll-loop watcher (which also triggers on fixes_needed
        // -> review) doesn't double-fire before the codex run finishes.
        let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
            "last_pr_review_at": chrono::Utc::now().to_rfc3339(),
        })).await;

        agent_comment(&config, &task_id, "Running $samwise-pr-review on this PR — hang tight, Codex takes a minute.").await;

        let result = match review::run_samwise_pr_review(&pr_url, &repo_path).await {
            Ok(r) => r,
            Err(e) => {
                log::warn!("[pr-review] run failed for task {}: {}", task_id, e);
                agent_comment(&config, &task_id, &format!("Codex review errored: {}. Leaving the card in Review.", e)).await;
                return;
            }
        };

        // One-line headline so Matt can see the verdict at a glance without
        // scrolling or reading the whole markdown body. Post BEFORE the body
        // so it appears at the top of the review section in the activity log.
        let headline = match result.verdict {
            review::PrReviewVerdict::MergeNow =>
                "Codex says: **MERGE**. Moving to Ready to Merge.".to_string(),
            review::PrReviewVerdict::FixIssues =>
                "Codex says: **FIX**. Moving to Fixes Needed. Blockers in the review below.".to_string(),
            review::PrReviewVerdict::Inconclusive =>
                "Codex says: **INCONCLUSIVE**. Leaving in Review — no clean verdict.".to_string(),
        };
        agent_comment(&config, &task_id, &headline).await;

        // Post the markdown body verbatim so Matt (and CS) can read the findings.
        if !result.markdown.trim().is_empty() {
            agent_comment(&config, &task_id, &result.markdown).await;
        }

        match result.verdict {
            review::PrReviewVerdict::MergeNow => {
                let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
                    "status": "approved",
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })).await;
                notify_callback(&config, &task_id, "approved", Some(&pr_url), None);
                send_terminal_telegram(
                    &config, &task_id,
                    &format!("Ready to merge: {}", pr_url),
                    "Codex gave the green light. Your turn to hit merge.",
                ).await;
            }
            review::PrReviewVerdict::FixIssues => {
                let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
                    "status": "fixes_needed",
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })).await;
                notify_callback(&config, &task_id, "fixes_needed", Some(&pr_url), None);

                // Kick the auto-fix loop if enabled, not flagged REQUIRES_HUMAN,
                // and we're under the cycle cap. Runs detached so this callback
                // returns fast. That path fires its own telegram on the
                // terminal state (caps, REQUIRES_HUMAN, or disabled).
                maybe_spawn_auto_fix(
                    config.clone(),
                    task_id.clone(),
                    pr_url.clone(),
                    repo_path.clone(),
                    result.markdown.clone(),
                    result.requires_human,
                ).await;
            }
            review::PrReviewVerdict::Inconclusive => {
                // Leave in review. Markdown body was already posted as a comment above.
                send_terminal_telegram(
                    &config, &task_id,
                    &format!("Codex review inconclusive: {}", pr_url),
                    "PR is sitting in Review. Details in the card comments.",
                ).await;
            }
        }
    });
}

/// Fire a Telegram notification at the end of Sam's automation for a task.
/// Reads the notify-task-completed-code setting so it respects Matt's
/// notification preferences. Includes the task title for context.
async fn send_terminal_telegram(
    config: &SupabaseConfig,
    task_id: &str,
    headline: &str,
    detail: &str,
) {
    // Gated by the same flag that used to fire the PR-open telegram.
    let home = std::env::var("HOME").unwrap_or_default();
    let settings_path = std::path::PathBuf::from(&home)
        .join("Library/Application Support/com.mattjohnston.agent-one/settings.json");
    let settings_val: Option<serde_json::Value> = tokio::fs::read_to_string(&settings_path).await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let master = settings_val.as_ref()
        .and_then(|s| s.get("telegramNotificationsEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !master { return; }
    let notify_completed = settings_val.as_ref()
        .and_then(|s| s.get("telegramNotifyTaskCompletedCode"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !notify_completed { return; }

    let title = match supabase::fetch_task(config, task_id).await {
        Ok(Some(t)) => t.get("title").and_then(|v| v.as_str()).unwrap_or("untitled").to_string(),
        _ => "untitled".to_string(),
    };

    let msg = format!(
        "*{}*\n{}\n_{}_",
        escape_markdown_v2(&title),
        escape_markdown_v2(headline),
        escape_markdown_v2(detail),
    );
    send_telegram(config, &msg).await;
}

#[derive(Debug, Clone)]
struct DeployCommand {
    category: &'static str,
    label: String,
    command: String,
    cwd: String,
}

#[derive(Debug, Clone, Default)]
struct DeployPlan {
    commands: Vec<DeployCommand>,
    railway_reasons: Vec<String>,
    supabase_migrations: Vec<String>,
    supabase_functions: Vec<String>,
}

#[derive(Debug, Clone)]
struct MergeDeployError {
    message: String,
    pr_merged: bool,
}

impl MergeDeployError {
    fn new(message: impl Into<String>, pr_merged: bool) -> Self {
        Self { message: message.into(), pr_merged }
    }
}

/// Pick up merge/deploy requests written by the desktop or web UI. The UI only
/// mutates task.context; this worker owns the privileged local CLIs.
pub async fn sweep_merge_deploy_requests(config: &SupabaseConfig) {
    let Ok(tasks) = supabase::fetch_tasks(config, None).await else { return };
    let Some(arr) = tasks.as_array() else { return };

    for task in arr {
        let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(status, "approved" | "fixes_needed" | "review") { continue; }
        if !merge_deploy_request_is_pending(task) { continue; }
        start_merge_deploy_task(config, task.clone(), status == "approved", "Merge + Deploy requested from the UI.").await;
    }
}

async fn start_merge_deploy_task(
    config: &SupabaseConfig,
    task: Value,
    should_merge_if_open: bool,
    start_comment: &str,
) {
    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let pr_url = task.get("pr_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if task_id.is_empty() || pr_url.is_empty() { return; }

    let mut context = task_context_object(&task);
    context.insert(MERGE_DEPLOY_STATUS_KEY.to_string(), Value::String("running".to_string()));
    context.insert(MERGE_DEPLOY_STARTED_AT_KEY.to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
    context.insert(MERGE_DEPLOY_ERROR_KEY.to_string(), Value::Null);
    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
        "context": Value::Object(context),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;

    agent_comment(config, &task_id, start_comment).await;

    let config_clone = config.clone();
    tokio::spawn(async move {
        match run_merge_deploy_workflow(&config_clone, &task, should_merge_if_open).await {
            Ok(summary) => {
                let latest_task = supabase::fetch_task(&config_clone, &task_id).await.ok().flatten().unwrap_or_else(|| task.clone());
                let mut context = task_context_object(&latest_task);
                context.insert(MERGE_DEPLOY_STATUS_KEY.to_string(), Value::String("succeeded".to_string()));
                context.insert(MERGE_DEPLOY_COMPLETED_AT_KEY.to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
                context.insert(MERGE_DEPLOY_ERROR_KEY.to_string(), Value::Null);
                let _ = supabase::update_task(&config_clone, &task_id, &serde_json::json!({
                    "status": "done",
                    "completed_at": chrono::Utc::now().to_rfc3339(),
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                    "review_cycle_count": 0,
                    "context": Value::Object(context),
                    "failure_reason": Value::Null,
                })).await;
                notify_callback(&config_clone, &task_id, "done", Some(&pr_url), None);
                let origin_system = latest_task.get("origin_system").and_then(|v| v.as_str()).unwrap_or("");
                let origin_id = latest_task.get("origin_id").and_then(|v| v.as_str()).unwrap_or("");
                close_origin_ticket(&config_clone, &task_id, origin_system, origin_id, &pr_url);
                agent_comment(&config_clone, &task_id, &format!("Merge + Deploy complete. Moving the card to Done.\n\n{}", summary)).await;
            }
            Err(err) => {
                let latest_task = supabase::fetch_task(&config_clone, &task_id).await.ok().flatten().unwrap_or_else(|| task.clone());
                let mut context = task_context_object(&latest_task);
                context.insert(MERGE_DEPLOY_STATUS_KEY.to_string(), Value::String("failed".to_string()));
                context.insert(MERGE_DEPLOY_COMPLETED_AT_KEY.to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
                context.insert(MERGE_DEPLOY_ERROR_KEY.to_string(), Value::String(truncate(&err.message, 900)));
                let next_status = if err.pr_merged { "fixes_needed" } else { "approved" };
                let _ = supabase::update_task(&config_clone, &task_id, &serde_json::json!({
                    "status": next_status,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                    "context": Value::Object(context),
                    "failure_reason": format!("Merge + Deploy failed: {}", truncate(&err.message, 1000)),
                })).await;
                notify_callback(&config_clone, &task_id, next_status, Some(&pr_url), Some(&err.message));
                agent_comment(
                    &config_clone,
                    &task_id,
                    &format!(
                        "Merge + Deploy failed{}. Leaving this card in {}.\n\nReason: {}",
                        if err.pr_merged { " after the PR was merged" } else { "" },
                        if err.pr_merged { "Fixes Needed" } else { "Ready to Merge" },
                        truncate(&err.message, 1800)
                    ),
                ).await;
            }
        }
    });
}

async fn run_merge_deploy_workflow(
    config: &SupabaseConfig,
    task: &Value,
    should_merge_if_open: bool,
) -> Result<String, MergeDeployError> {
    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let pr_url = task.get("pr_url").and_then(|v| v.as_str()).unwrap_or("");
    let repo_path = task.get("repo_path").and_then(|v| v.as_str()).unwrap_or("");
    if task_id.is_empty() {
        return Err(MergeDeployError::new("task id missing", false));
    }
    if !review::is_safe_pr_url(pr_url) {
        return Err(MergeDeployError::new(format!("unsafe or missing PR URL: {}", pr_url), false));
    }
    if repo_path.is_empty() || !Path::new(repo_path).is_dir() {
        return Err(MergeDeployError::new(format!("repo_path is missing or not a directory: {}", repo_path), false));
    }

    let files = review::fetch_pr_files(pr_url, repo_path)
        .await
        .map_err(|e| MergeDeployError::new(format!("failed to list PR files: {}", e), false))?;

    let mut pr_merged = gh_pr_is_merged(pr_url, repo_path)
        .await
        .map_err(|e| MergeDeployError::new(format!("failed to read PR state: {}", e), false))?;

    if !pr_merged {
        if !should_merge_if_open {
            return Err(MergeDeployError::new("PR is not merged yet, and this request is only allowed to deploy an already-merged PR.", false));
        }
        let head_sha = review::fetch_pr_head_sha(pr_url, repo_path)
            .await
            .map_err(|e| MergeDeployError::new(format!("failed to read PR head SHA: {}", e), false))?;
        match review::wait_for_ci(pr_url, repo_path).await {
            Ok(true) => {}
            Ok(false) => return Err(MergeDeployError::new("non-Vercel CI checks are not green; merge blocked", false)),
            Err(e) => return Err(MergeDeployError::new(format!("CI check failed before merge: {}", e), false)),
        }
        review::gh_merge(pr_url, repo_path, &head_sha)
            .await
            .map_err(|e| MergeDeployError::new(format!("GitHub merge failed: {}", e), false))?;
        pr_merged = true;
        agent_comment(config, task_id, "PR merged on GitHub. Preparing post-merge deploy plan.").await;
    }

    let default_branch = detect_default_branch(repo_path).await;
    let deploy_path = prepare_deploy_checkout(repo_path, task_id, &default_branch)
        .await
        .map_err(|e| MergeDeployError::new(e, pr_merged))?;
    let plan = build_deploy_plan(&deploy_path, &files)
        .await
        .map_err(|e| MergeDeployError::new(e, pr_merged))?;

    persist_deploy_plan(config, task_id, &plan).await;
    agent_comment(config, task_id, &format!("Post-merge deploy plan:\n\n{}", deploy_plan_markdown(&plan))).await;

    if plan.commands.is_empty() {
        return Ok("No Railway server, Supabase migration, or Supabase Edge Function deploy steps were detected for this PR.".to_string());
    }

    for command in &plan.commands {
        run_deploy_command(command)
            .await
            .map_err(|e| MergeDeployError::new(e, pr_merged))?;
    }

    Ok(deploy_plan_markdown(&plan))
}

async fn gh_pr_is_merged(pr_url: &str, repo_path: &str) -> Result<bool, String> {
    let output = async_cmd("gh")
        .args(["pr", "view", pr_url, "--json", "state,mergedAt"])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| format!("spawn gh: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let parsed: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("parse gh pr view: {}", e))?;
    let state = parsed.get("state").and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
    let merged_at = parsed.get("mergedAt").and_then(|v| v.as_str()).unwrap_or("");
    Ok(state == "MERGED" || !merged_at.is_empty())
}

async fn prepare_deploy_checkout(repo_path: &str, task_id: &str, default_branch: &str) -> Result<String, String> {
    let deploy_path = task_worktree_path(repo_path, task_id)
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| repo_path.to_string());

    let dirty = run_git(&["status", "--porcelain"], &deploy_path).await?;
    if !dirty.trim().is_empty() {
        return Err(format!("deployment checkout is dirty at {}; refusing to deploy over local changes", deploy_path));
    }

    run_git(&["fetch", "origin", "--prune"], &deploy_path).await?;
    if deploy_path == repo_path {
        run_git(&["checkout", default_branch], &deploy_path).await?;
        run_git(&["pull", "--ff-only", "origin", default_branch], &deploy_path).await?;
    } else {
        run_git(&["checkout", "--detach", &format!("origin/{}", default_branch)], &deploy_path).await?;
    }
    Ok(deploy_path)
}

async fn build_deploy_plan(repo_path: &str, files: &[String]) -> Result<DeployPlan, String> {
    let mut plan = DeployPlan::default();

    for file in files {
        if file.starts_with("supabase/migrations/") && file.ends_with(".sql") {
            push_unique_string(&mut plan.supabase_migrations, file.clone());
        }
        if let Some(name) = edge_function_name(file) {
            push_unique_string(&mut plan.supabase_functions, name);
        }
    }

    if !plan.supabase_migrations.is_empty() {
        plan.commands.push(DeployCommand {
            category: "supabase_migrations",
            label: format!("Supabase migrations ({})", plan.supabase_migrations.join(", ")),
            command: "npx --yes supabase db push".to_string(),
            cwd: repo_path.to_string(),
        });
    }

    for function_name in &plan.supabase_functions {
        plan.commands.push(DeployCommand {
            category: "supabase_edge_functions",
            label: format!("Supabase Edge Function {}", function_name),
            command: format!("npx --yes supabase functions deploy {}", shell_quote_simple(function_name)),
            cwd: repo_path.to_string(),
        });
    }

    let package_scripts = read_package_scripts(repo_path).await;
    let touches_tools = files.iter().any(|f| f.starts_with("tools-server/"));
    let touches_server = files.iter().any(|f| is_server_deploy_path(f));

    if touches_tools && package_scripts.iter().any(|s| s == "tools:deploy") {
        plan.railway_reasons.push("tools-server/ changed; using npm run tools:deploy".to_string());
        plan.commands.push(DeployCommand {
            category: "railway",
            label: "Railway tools-server".to_string(),
            command: "npm run tools:deploy".to_string(),
            cwd: repo_path.to_string(),
        });
    }

    if touches_server && package_scripts.iter().any(|s| s == "server:deploy") {
        plan.railway_reasons.push("server/root deploy files changed; using npm run server:deploy".to_string());
        plan.commands.push(DeployCommand {
            category: "railway",
            label: "Railway server".to_string(),
            command: "npm run server:deploy".to_string(),
            cwd: repo_path.to_string(),
        });
    }

    if !plan.commands.iter().any(|c| c.category == "railway") {
        for root in discover_railway_roots(repo_path) {
            let rel = path_relative_to(&root, repo_path);
            let matches_root = files.iter().any(|f| railway_root_matches_file(&rel, f));
            if !matches_root { continue; }
            let label = if rel.is_empty() { "Railway server".to_string() } else { format!("Railway server ({})", rel) };
            let command = if rel.is_empty() {
                "railway up --detach".to_string()
            } else {
                format!("railway up --detach --path-as-root {}", shell_quote_simple(&rel))
            };
            plan.railway_reasons.push(format!("{} changed; using {}", if rel.is_empty() { "Railway root".to_string() } else { rel.clone() }, command));
            plan.commands.push(DeployCommand {
                category: "railway",
                label,
                command,
                cwd: repo_path.to_string(),
            });
        }
    }

    Ok(plan)
}

async fn run_deploy_command(command: &DeployCommand) -> Result<(), String> {
    log::info!("[merge-deploy] running {} in {}: {}", command.label, command.cwd, command.command);
    let doppler_scope = dev_server::doppler_scope_for_checkout(&command.cwd).await;
    let mut cmd = if let Some(scope) = doppler_scope.as_deref() {
        let mut c = async_cmd("doppler");
        c.args(["run", "--scope", scope, "--", "sh", "-lc", &command.command]);
        c
    } else {
        let mut c = async_cmd("sh");
        c.args(["-lc", &command.command]);
        c
    };
    cmd.current_dir(&command.cwd)
        .env("CI", "true")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = tokio::time::timeout(std::time::Duration::from_secs(20 * 60), cmd.output())
        .await
        .map_err(|_| format!("{} timed out after 20 minutes", command.label))?
        .map_err(|e| format!("spawn {}: {}", command.label, e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr));
    let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout));
    Err(format!(
        "{} failed with {}. stderr: {} stdout: {}",
        command.label,
        output.status,
        truncate(stderr.trim(), 900),
        truncate(stdout.trim(), 500)
    ))
}

async fn persist_deploy_plan(config: &SupabaseConfig, task_id: &str, plan: &DeployPlan) {
    let commands: Vec<Value> = plan.commands.iter().map(|c| serde_json::json!({
        "category": c.category,
        "label": c.label,
        "command": c.command,
        "cwd": c.cwd,
    })).collect();
    if let Ok(Some(task)) = supabase::fetch_task(config, task_id).await {
        let mut context = task_context_object(&task);
        context.insert(MERGE_DEPLOY_PLAN_KEY.to_string(), serde_json::json!({
            "railway_reasons": &plan.railway_reasons,
            "supabase_migrations": &plan.supabase_migrations,
            "supabase_functions": &plan.supabase_functions,
            "commands": commands,
        }));
        let _ = supabase::update_task(config, task_id, &serde_json::json!({
            "context": Value::Object(context),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })).await;
    }
}

fn deploy_plan_markdown(plan: &DeployPlan) -> String {
    let railway = if plan.railway_reasons.is_empty() {
        "no - no Railway server deploy path matched the PR files".to_string()
    } else {
        format!("yes - {}", plan.railway_reasons.join("; "))
    };
    let migrations = if plan.supabase_migrations.is_empty() {
        "no - no supabase/migrations/*.sql files changed".to_string()
    } else {
        format!("yes - {}", plan.supabase_migrations.join(", "))
    };
    let functions = if plan.supabase_functions.is_empty() {
        "no - no supabase/functions/<name>/ files changed".to_string()
    } else {
        format!("yes - {}", plan.supabase_functions.join(", "))
    };
    let commands = if plan.commands.is_empty() {
        "Commands: none".to_string()
    } else {
        format!(
            "Commands:\n{}",
            plan.commands.iter()
                .map(|c| format!("- {}: `{}` in `{}`", c.label, c.command, c.cwd))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    format!(
        "- Railway server: {}\n- Supabase migrations: {}\n- Supabase Edge Functions: {}\n\n{}",
        railway, migrations, functions, commands
    )
}

fn merge_deploy_request_is_pending(task: &Value) -> bool {
    let context = task.get("context").and_then(|v| v.as_object());
    let Some(context) = context else { return false; };
    let status = context.get(MERGE_DEPLOY_STATUS_KEY).and_then(|v| v.as_str()).unwrap_or("");
    if status == "running" || status == "succeeded" {
        return false;
    }
    status == "requested" && context.get(MERGE_DEPLOY_REQUESTED_AT_KEY).and_then(|v| v.as_str()).is_some()
}

fn task_context_object(task: &Value) -> serde_json::Map<String, Value> {
    task.get("context")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default()
}

fn task_worktree_path(main_repo_path: &str, task_id: &str) -> Option<PathBuf> {
    let repo_name = Path::new(main_repo_path).file_name()?.to_string_lossy().into_owned();
    Some(worktrees_root().join(repo_name).join(short_task_id(task_id)))
}

fn edge_function_name(file: &str) -> Option<String> {
    let rest = file.strip_prefix("supabase/functions/")?;
    let name = rest.split('/').next()?.trim();
    if name.is_empty() { None } else { Some(name.to_string()) }
}

fn is_server_deploy_path(file: &str) -> bool {
    file.starts_with("server/")
        || matches!(
            file,
            "Dockerfile" | "railway.json" | "railway.toml" | "nixpacks.toml" | ".railway-redeploy" |
            "package.json" | "package-lock.json" | "tsconfig.server.json"
        )
}

async fn read_package_scripts(repo_path: &str) -> Vec<String> {
    let path = Path::new(repo_path).join("package.json");
    let Ok(raw) = tokio::fs::read_to_string(path).await else { return Vec::new(); };
    let Ok(parsed) = serde_json::from_str::<Value>(&raw) else { return Vec::new(); };
    parsed.get("scripts")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

fn discover_railway_roots(repo_path: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    discover_railway_roots_inner(Path::new(repo_path), Path::new(repo_path), 0, &mut roots);
    roots
}

fn discover_railway_roots_inner(root: &Path, dir: &Path, depth: usize, roots: &mut Vec<PathBuf>) {
    if depth > 3 { return; }
    let name = dir.file_name().and_then(|v| v.to_str()).unwrap_or("");
    if matches!(name, ".git" | "node_modules" | "dist" | "build" | ".next" | "target") {
        return;
    }
    if dir.join("railway.json").is_file() || dir.join("railway.toml").is_file() {
        if !roots.iter().any(|p| p == dir) {
            roots.push(dir.to_path_buf());
        }
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            discover_railway_roots_inner(root, &path, depth + 1, roots);
        }
    }
    if depth == 0 && root.join("nixpacks.toml").is_file() && !roots.iter().any(|p| p == root) {
        roots.push(root.to_path_buf());
    }
}

fn path_relative_to(path: &Path, base: &str) -> String {
    path.strip_prefix(base)
        .ok()
        .map(|p| p.to_string_lossy().trim_matches('/').to_string())
        .unwrap_or_default()
}

fn railway_root_matches_file(root_rel: &str, file: &str) -> bool {
    if root_rel.is_empty() {
        return is_server_deploy_path(file);
    }
    file == root_rel || file.starts_with(&format!("{}/", root_rel))
}

fn push_unique_string(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}

fn shell_quote_simple(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn redact_secrets(value: &str) -> String {
    let mut out = value.to_string();
    let patterns = [
        r"(?i)(token|key|secret|password)=([^\s]+)",
        r"(?i)(bearer\s+)[A-Za-z0-9._\-]+",
        r"dp\.st\.[A-Za-z0-9._\-]+",
        r"eyJ[A-Za-z0-9._\-]+",
    ];
    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            out = re.replace_all(&out, "$1<redacted>").to_string();
        }
    }
    out
}

/// Scan cards whose PR status on GitHub has advanced without Sam knowing
/// — PR merged or closed while the card was still sitting in review /
/// fixes_needed / approved. Merged PRs run the post-merge deploy plan before
/// moving to done. Runs on the poll-loop cadence.
pub async fn sweep_pr_merged_cards(config: &SupabaseConfig) {
    // Candidate statuses: any state where the card is "waiting on Matt" but
    // GitHub could have moved on. `review` is in scope for the case where
    // Matt merges a PR before (or instead of) a Codex verdict landing.
    let Ok(tasks) = supabase::fetch_tasks(config, None).await else { return };
    let Some(arr) = tasks.as_array() else { return };

    for task in arr {
        let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(status, "approved" | "review" | "fixes_needed") { continue; }

        let pr_url = task.get("pr_url").and_then(|v| v.as_str()).unwrap_or("");
        let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if pr_url.is_empty() || task_id.is_empty() { continue; }

        // Ask GitHub. Any failure = skip this tick; we'll retry next poll.
        let out = async_cmd("gh")
            .args(["pr", "view", pr_url, "--json", "state,mergedAt"])
            .output().await;
        let Ok(o) = out else { continue };
        if !o.status.success() { continue; }
        let body = String::from_utf8_lossy(&o.stdout);
        let parsed: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let gh_state = parsed.get("state").and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
        let merged_at = parsed.get("mergedAt").and_then(|v| v.as_str()).unwrap_or("");

        if gh_state == "MERGED" || !merged_at.is_empty() {
            let md_status = task
                .get("context")
                .and_then(|v| v.as_object())
                .and_then(|c| c.get(MERGE_DEPLOY_STATUS_KEY))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if md_status == "running" || md_status == "failed" {
                continue;
            }
            if md_status == "succeeded" {
                let _ = supabase::update_task(config, task_id, &serde_json::json!({
                    "status": "done",
                    "completed_at": chrono::Utc::now().to_rfc3339(),
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                    "review_cycle_count": 0,
                })).await;
                notify_callback(config, task_id, "done", Some(pr_url), None);
                continue;
            }
            start_merge_deploy_task(
                config,
                task.clone(),
                false,
                "PR merged on GitHub. Running post-merge deploy plan before moving the card to Done.",
            ).await;
            continue;
        }

        if gh_state == "CLOSED" {
            let _ = supabase::update_task(config, task_id, &serde_json::json!({
                "status": "done",
                "completed_at": chrono::Utc::now().to_rfc3339(),
                "updated_at": chrono::Utc::now().to_rfc3339(),
                "review_cycle_count": 0,
            })).await;
            notify_callback(config, task_id, "done", Some(pr_url), Some("PR closed without merging"));
            agent_comment(config, task_id, "PR was closed without merging. Moving the card to Done.").await;
            continue;
        }
        // OPEN / anything else: leave alone.
    }
}

/// Decide whether to fire the auto-fix loop after a `fix_issues` verdict.
/// Gated by the `autoFixFromFixesNeededEnabled` setting, Codex's
/// `REQUIRES_HUMAN` flag, and a 3-cycle cap per card.
async fn maybe_spawn_auto_fix(
    config: SupabaseConfig,
    task_id: String,
    pr_url: String,
    repo_path: String,
    review_markdown: String,
    requires_human: bool,
) {
    if requires_human {
        agent_comment(&config, &task_id, "Codex flagged this as needing your judgment (REQUIRES_HUMAN: yes). Leaving in Fixes Needed for you.").await;
        send_terminal_telegram(
            &config, &task_id,
            &format!("Needs your judgment: {}", pr_url),
            "Codex flagged blockers that need a product/architecture call.",
        ).await;
        return;
    }

    // Read the setting from disk so the detached path doesn't need cached_settings plumbed in.
    let home = std::env::var("HOME").unwrap_or_default();
    let settings_path = std::path::PathBuf::from(&home)
        .join("Library/Application Support/com.mattjohnston.agent-one/settings.json");
    let settings_val: Option<serde_json::Value> = tokio::fs::read_to_string(&settings_path).await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let auto_fix_on = settings_val.as_ref()
        .and_then(|s| s.get("autoFixFromFixesNeededEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !auto_fix_on {
        send_terminal_telegram(
            &config, &task_id,
            &format!("Fixes needed: {}", pr_url),
            "Auto-fix is off. Details in the card comments.",
        ).await;
        return;
    }

    // Cycle cap: read current count from the task row.
    let cycle_count = match supabase::fetch_task(&config, &task_id).await {
        Ok(Some(t)) => t.get("review_cycle_count").and_then(|v| v.as_i64()).unwrap_or(0),
        _ => 0,
    };
    if cycle_count >= 3 {
        agent_comment(&config, &task_id, "Hit the 3-cycle auto-fix cap on this PR. Stopping so I don't thrash — take a look when you get a sec.").await;
        send_terminal_telegram(
            &config, &task_id,
            &format!("Auto-fix capped: {}", pr_url),
            "3 auto-fix cycles and Codex still sees blockers. Your call.",
        ).await;
        return;
    }

    spawn_auto_fix_task(config, task_id, pr_url, repo_path, review_markdown, cycle_count as u32);
}

/// Run Claude Code against the existing worktree with a prompt derived from
/// the Codex review blockers, commit, and push. Updating the PR triggers
/// the re-review watcher on the next poll tick.
pub fn spawn_auto_fix_task(
    config: SupabaseConfig,
    task_id: String,
    pr_url: String,
    repo_path: String,
    review_markdown: String,
    prev_cycle_count: u32,
) {
    tokio::spawn(async move {
        let new_cycle = prev_cycle_count + 1;

        let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
            "status": "in_progress",
            "review_cycle_count": new_cycle,
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })).await;
        notify_callback(&config, &task_id, "in_progress", Some(&pr_url), None);

        agent_comment(&config, &task_id, &format!(
            "Running auto-fix cycle {}/3 on this PR. Feeding Codex's blocker list back to Claude Code.",
            new_cycle
        )).await;

        // Pre-flight: make sure the worktree is on `sam/<short_id>` before we
        // hand it to Claude Code. The sweep path used to pass Matt's main
        // checkout here, which sat on main — Claude would then commit to main
        // and the post-run branch-guard would kill the push. That's fixed at
        // the sweep, but this guard means any other path that gets the wrong
        // directory still fails loud instead of quietly producing a main-
        // branch commit.
        let expected_branch = task_branch_name(&short_task_id(&task_id));
        let checkout = async_cmd("git")
            .args(["checkout", &expected_branch])
            .current_dir(&repo_path)
            .output().await;
        match checkout {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                fail_auto_fix(
                    &config, &task_id, &pr_url,
                    &format!("pre-flight checkout '{}' failed: {}", expected_branch, stderr.trim()),
                ).await;
                return;
            }
            Err(e) => {
                fail_auto_fix(
                    &config, &task_id, &pr_url,
                    &format!("pre-flight checkout '{}' spawn failed: {}", expected_branch, e),
                ).await;
                return;
            }
        }

        // Build fix prompt. Focus Claude Code on the blockers only — not risks,
        // not "not verified" items. The review markdown is already structured
        // with ## Blockers so we can hand it over wholesale.
        let prompt = format!(
            "You are Sam fixing the blockers a Codex review flagged on this PR. \
The repo is already checked out at this worktree, and you are on the PR's head branch.\n\n\
## Codex review\n\n{}\n\n\
## Instructions\n\
Address every item under the `## Blockers` section above. Do NOT touch items \
under `## Risks` or `## Not verified` unless they are also blockers. Do not \
add new features, refactor unrelated code, or bump dependencies.\n\n\
When you are done, stage and commit everything with a short structured body:\n\
```\n\
git add -A && git commit -m \"$(cat <<'EOF'\n\
samwise: address review blockers (cycle {}/3)\n\n\
Blockers addressed:\n\
- <bullet per blocker>\n\n\
How it was fixed:\n\
- <bullet per change>\n\n\
Deployment required:\n\
- Railway server: <yes/no/unknown> - <plain reason, including service name if yes>\n\
- Supabase migrations: <yes/no/unknown> - <plain reason, including migration filenames if yes>\n\
- Supabase Edge Functions: <yes/no/unknown> - <plain reason, including function names if yes>\n\n\
For Customer Success:\n\
- <one plain sentence, or \"internal only, no customer message needed\">\n\
EOF\n\
)\"\n\
```\n\
Do not leave placeholders in the deployment section; say \"no\" explicitly when \
Railway, Supabase migrations, or Edge Functions do not need deployment.\n\n\
Do not push — that is handled after this step. Do not open a second PR.\n\
If a blocker is genuinely unfixable without Matt's input (e.g. needs a product \
decision or schema change), stop and explain which blocker and why, without \
making any other changes.",
            review_markdown, new_cycle
        );

        let process_id_slot: Arc<tokio::sync::Mutex<Option<u32>>> = Arc::new(tokio::sync::Mutex::new(None));
        let claude_result = run_claude_code_streaming(&repo_path, &prompt, 0, 900, &config, &task_id, process_id_slot).await;

        match claude_result {
            Ok(_) => {
                // Push whatever Claude committed. Find the branch first.
                let branch_out = async_cmd("git")
                    .args(["rev-parse", "--abbrev-ref", "HEAD"])
                    .current_dir(&repo_path)
                    .output().await;
                let branch = branch_out.ok()
                    .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                if branch.is_empty() || branch == "HEAD" {
                    fail_auto_fix(&config, &task_id, &pr_url, "couldn't resolve PR branch for push").await;
                    return;
                }

                // HARD SAFETY GUARD: auto-fix is only ever allowed to push to
                // Sam's own task branch (`sam/<short_id>`). If the worktree
                // somehow ended up on main/master or any other branch, abort
                // instead of pushing. GitHub rejected the first accidental
                // `main -> main` push we saw (non-fast-forward), but we
                // cannot trust GitHub to be the last line of defense.
                let short_id = short_task_id(&task_id);
                let expected_branch = task_branch_name(&short_id);
                if branch != expected_branch {
                    fail_auto_fix(
                        &config, &task_id, &pr_url,
                        &format!(
                            "refusing to push: worktree on '{}' but auto-fix may only push to '{}'",
                            branch, expected_branch
                        ),
                    ).await;
                    return;
                }

                // Capture HEAD before push so we can detect whether Claude
                // actually created a new commit. `git push` succeeds with
                // "Everything up-to-date" on a no-op; without this check we'd
                // flip the card back to review and burn a cycle on an
                // unchanged PR.
                let head_before_out = async_cmd("git")
                    .args(["rev-parse", "HEAD"])
                    .current_dir(&repo_path)
                    .output().await;
                let head_before = head_before_out.ok()
                    .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                let upstream_sha_out = async_cmd("git")
                    .args(["rev-parse", &format!("origin/{}", branch)])
                    .current_dir(&repo_path)
                    .output().await;
                let upstream_sha = upstream_sha_out.ok()
                    .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                if !head_before.is_empty() && !upstream_sha.is_empty() && head_before == upstream_sha {
                    // No new commit versus the already-pushed PR head. Claude
                    // either decided the blockers weren't fixable or did
                    // nothing. Don't bounce the card back to review; leave in
                    // fixes_needed with a clear comment so Matt knows. This is
                    // a terminal state for the auto-fix cycle, so fire a
                    // telegram like every other terminal branch — without it
                    // the card sits silently in Fixes Needed and Matt has no
                    // way to know auto-fix gave up.
                    let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
                        "status": "fixes_needed",
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
                    notify_callback(&config, &task_id, "fixes_needed", Some(&pr_url), Some("no commit produced"));
                    agent_comment(&config, &task_id, "Auto-fix run finished without producing a new commit — either Claude decided the blockers needed your call, or nothing was actionable from the review. Leaving in Fixes Needed.").await;
                    send_terminal_telegram(
                        &config, &task_id,
                        &format!("Fixes needed: {}", pr_url),
                        "Auto-fix ran but Claude didn't produce a commit. Your call on the blockers.",
                    ).await;
                    return;
                }

                let push = async_cmd("git")
                    .args(["push", "origin", &branch])
                    .current_dir(&repo_path)
                    .output().await;

                match push {
                    Ok(o) if o.status.success() => {
                        // Clear last_pr_review_at so the watcher re-runs $samwise-pr-review on the updated PR.
                        let _ = supabase::update_task(&config, &task_id, &serde_json::json!({
                            "status": "review",
                            "last_pr_review_at": serde_json::Value::Null,
                            "updated_at": chrono::Utc::now().to_rfc3339(),
                        })).await;
                        notify_callback(&config, &task_id, "review", Some(&pr_url), None);
                        agent_comment(&config, &task_id, "Pushed fixes. Codex will re-review on the next poll.").await;
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        fail_auto_fix(&config, &task_id, &pr_url, &format!("git push failed: {}", stderr.trim())).await;
                    }
                    Err(e) => {
                        fail_auto_fix(&config, &task_id, &pr_url, &format!("git push spawn failed: {}", e)).await;
                    }
                }
            }
            Err(e) => {
                fail_auto_fix(&config, &task_id, &pr_url, &format!("Claude Code errored: {}", e)).await;
            }
        }
    });
}

async fn fail_auto_fix(config: &SupabaseConfig, task_id: &str, pr_url: &str, reason: &str) {
    let _ = supabase::update_task(config, task_id, &serde_json::json!({
        "status": "fixes_needed",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;
    notify_callback(config, task_id, "fixes_needed", Some(pr_url), Some(reason));
    agent_comment(config, task_id, &format!("Auto-fix attempt failed: {}. Leaving in Fixes Needed.", reason)).await;
    send_terminal_telegram(
        config, task_id,
        &format!("Auto-fix failed: {}", pr_url),
        &format!("Reason: {}", reason),
    ).await;
}

/// Scan tasks in `review` status and fire `spawn_pr_review_task` for any
/// that have a PR URL and whose `updated_at` is newer than
/// `last_pr_review_at`. Called once per worker poll tick. Only fires when
/// auto-merge is off and autoPrReviewEnabled is on.
pub async fn sweep_pr_review_queue(
    config: &SupabaseConfig,
    cached_settings: &Option<serde_json::Value>,
) {
    let auto_merge_on = cached_settings.as_ref()
        .and_then(|s| s.get("autoMergeEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let auto_pr_review_on = cached_settings.as_ref()
        .and_then(|s| s.get("autoPrReviewEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if auto_merge_on || !auto_pr_review_on {
        return;
    }

    let Ok(tasks) = supabase::fetch_tasks(config, Some("review")).await else { return };
    let Some(arr) = tasks.as_array() else { return };

    for task in arr {
        let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let pr_url = task.get("pr_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let main_repo_path = task.get("repo_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if task_id.is_empty() || pr_url.is_empty() || main_repo_path.is_empty() { continue; }

        let updated_at = task.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
        let last_review = task.get("last_pr_review_at").and_then(|v| v.as_str());

        // Fire if never reviewed, OR updated_at > last_pr_review_at (card moved back in),
        // OR the last review was long enough ago to retry an inconclusive Codex
        // decision. This keeps cards from sitting in Review forever when host
        // checks were merely pending during the first pass.
        let should_run = match last_review {
            None => true,
            Some(last) if !last.is_empty() => {
                match (
                    chrono::DateTime::parse_from_rfc3339(updated_at),
                    chrono::DateTime::parse_from_rfc3339(last),
                ) {
                    (Ok(u), Ok(l)) => {
                        u > l || chrono::Utc::now().signed_duration_since(l.with_timezone(&chrono::Utc)) > chrono::Duration::minutes(30)
                    }
                    _ => true,
                }
            }
            _ => true,
        };
        if !should_run { continue; }

        // Use the task's worktree, not Matt's main checkout. Matt's checkout sits
        // on main and passing it straight through had auto-fix running Claude Code
        // against main — so any fix commits went to main, and the branch-guard
        // correctly refused the push, killing the auto-fix cycle.
        let repo_name = std::path::Path::new(&main_repo_path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "repo".to_string());
        let short_id = short_task_id(&task_id);
        let worktree_path = worktrees_root().join(&repo_name).join(&short_id);
        let repo_path = if worktree_path.is_dir() {
            worktree_path.to_string_lossy().into_owned()
        } else {
            log::warn!(
                "[pr-review-sweep] worktree missing for task {} at {}; skipping",
                task_id, worktree_path.display()
            );
            continue;
        };

        spawn_pr_review_task(config.clone(), task_id, pr_url, repo_path);
    }
}

/// Fire a signed HTTP callback for a task status transition, if the task
/// carries a `callback_url`. No-op otherwise. Runs in the background so a
/// slow or flaky callback endpoint never blocks the worker loop.
///
/// Payload:
/// ```json
/// {
///   "task_id": "...",
///   "status": "in_progress" | "review" | "done" | "failed",
///   "title": "...",
///   "project": "...",
///   "pr_url": "..." | null,
///   "failure_reason": "..." | null,
///   "timestamp": "2026-04-23T..."
/// }
/// ```
///
/// Signature: if `callback_secret` is set on the task, an HMAC-SHA256 of the
/// raw JSON body is sent as `X-Samwise-Signature: sha256=<hex>`. The caller
/// can verify by recomputing against the shared secret.
pub fn notify_callback(
    config: &SupabaseConfig,
    task_id: &str,
    status: &str,
    pr_url: Option<&str>,
    failure_reason: Option<&str>,
) {
    let cfg = config.clone();
    let task_id = task_id.to_string();
    let status = status.to_string();
    let pr_url = pr_url.map(|s| s.to_string());
    let failure_reason = failure_reason.map(|s| s.to_string());

    tokio::spawn(async move {
        let task = match supabase::fetch_task(&cfg, &task_id).await {
            Ok(Some(t)) => t,
            Ok(None) => return,
            Err(e) => {
                log::warn!("[callback] fetch_task({}) failed: {}", task_id, e);
                return;
            }
        };
        let callback_url = match task.get("callback_url").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return,
        };
        let callback_secret = task.get("callback_secret").and_then(|v| v.as_str()).map(|s| s.to_string());
        let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let project = task.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let payload = serde_json::json!({
            "task_id": task_id,
            "status": status,
            "title": title,
            "project": project,
            "pr_url": pr_url,
            "failure_reason": failure_reason,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let body = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("[callback] serialize failed: {}", e);
                return;
            }
        };

        let mut req = reqwest::Client::new()
            .post(&callback_url)
            .header("content-type", "application/json")
            .header("user-agent", "samwise-worker/1");
        if let Some(secret) = callback_secret.as_deref() {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
                mac.update(body.as_bytes());
                let sig = hex::encode(mac.finalize().into_bytes());
                req = req.header("x-samwise-signature", format!("sha256={}", sig));
            }
        }
        req = req.body(body);

        match tokio::time::timeout(std::time::Duration::from_secs(10), req.send()).await {
            Ok(Ok(resp)) => {
                let status_code = resp.status();
                if !status_code.is_success() {
                    log::warn!("[callback] {} -> {} for task {}", callback_url, status_code, task_id);
                }
            }
            Ok(Err(e)) => log::warn!("[callback] {} failed: {}", callback_url, e),
            Err(_) => log::warn!("[callback] {} timed out", callback_url),
        }
    });
}

/// After merge+deploy is green, close the upstream origin ticket (Operly
/// triage, Banana triage, Sentry issue). The closeout edge function does the
/// real work — we just hand it the origin pointer plus the merged PR. Skips
/// `manual` and tasks without origin metadata. Failures do not retry inline:
/// a comment is posted on the Sam task so Matt can re-fire from the queue UI.
fn close_origin_ticket(
    config: &SupabaseConfig,
    task_id: &str,
    origin_system: &str,
    origin_id: &str,
    pr_url: &str,
) {
    if origin_system.is_empty() || origin_system == "manual" || origin_id.is_empty() {
        return;
    }

    let cfg = config.clone();
    let task_id = task_id.to_string();
    let origin_system = origin_system.to_string();
    let origin_id = origin_id.to_string();
    let pr_url = pr_url.to_string();

    tokio::spawn(async move {
        let payload = serde_json::json!({
            "system": origin_system,
            "origin_id": origin_id,
            "pr_url": pr_url,
            "task_id": task_id,
        });

        let req = reqwest::Client::new()
            .post(CLOSE_ORIGIN_TICKET_URL)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", cfg.anon_key))
            .header("apikey", cfg.anon_key.clone())
            .header("user-agent", "samwise-worker/1")
            .json(&payload);

        let send = tokio::time::timeout(std::time::Duration::from_secs(15), req.send()).await;
        let (status_label, error_detail): (String, Option<String>) = match send {
            Ok(Ok(resp)) => {
                let status = resp.status();
                if status.is_success() {
                    log::info!(
                        "[close-origin] {} ticket {} closed for task {}",
                        origin_system, origin_id, task_id
                    );
                    return;
                }
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("(read body failed: {})", e));
                (status.to_string(), Some(truncate(body.trim(), 400).to_string()))
            }
            Ok(Err(e)) => ("network error".to_string(), Some(e.to_string())),
            Err(_) => ("timeout".to_string(), Some("no response within 15s".to_string())),
        };

        log::warn!(
            "[close-origin] failed for task {} ({} / {}): {} — {}",
            task_id,
            origin_system,
            origin_id,
            status_label,
            error_detail.as_deref().unwrap_or("")
        );

        let comment = format!(
            "Closeout failed: {}{}. Run manually.",
            status_label,
            error_detail
                .as_deref()
                .map(|d| format!(" — {}", d))
                .unwrap_or_default(),
        );
        agent_comment(&cfg, &task_id, &comment).await;
    });
}

/// Summarize the branch diff into three short sections for the PR body:
/// what was fixed (user-visible), how it was fixed (technical), and a
/// paste-ready Customer Success blurb. Best-effort — returns None on any
/// failure so PR creation still proceeds.
async fn summarize_pr_changes(
    repo_path: &str,
    base_branch: &str,
    title: &str,
    description: &str,
) -> Option<String> {
    // Grab the branch diff vs base. Cap the size so we don't blow past the
    // CLI prompt limit on large refactors — the summary just gets a
    // representative slice in that case.
    let diff_out = async_cmd("git")
        .args(["diff", &format!("origin/{}..HEAD", base_branch)])
        .current_dir(repo_path)
        .output()
        .await
        .ok()?;
    if !diff_out.status.success() {
        return None;
    }
    let mut diff = String::from_utf8_lossy(&diff_out.stdout).to_string();
    const MAX_DIFF_BYTES: usize = 60_000;
    let truncated = diff.len() > MAX_DIFF_BYTES;
    if truncated {
        diff.truncate(MAX_DIFF_BYTES);
        diff.push_str("\n\n...[diff truncated]...\n");
    }
    if diff.trim().is_empty() {
        return None;
    }

    let prompt = format!(
        "You are summarizing a code change for a pull request Matt will review.\n\
Return ONLY a single JSON object with exactly these keys, no prose, no markdown fence:\n\
{{\n  \"what\": \"...\",\n  \"how\": \"...\",\n  \"customer_message\": \"...\"\n}}\n\n\
Field rules:\n\
- what: 1-3 short bullets (plain English, user/customer POV) describing the bug or feature. Lead with the observable symptom.\n\
- how: 1-4 short bullets describing the technical change. Mention files or functions touched and the approach.\n\
- customer_message: one or two plain-text sentences Customer Success can paste to the customer. No code terms, no markdown, no filenames, no apologies longer than needed. If the change is internal-only, set this to exactly \"internal only, no customer message needed\".\n\n\
## Task title\n{title}\n\n## Task description\n{description}\n\n## Diff (base: {base_branch})\n```diff\n{diff}\n```\n",
        title = title, description = description, base_branch = base_branch, diff = diff
    );

    let raw = run_claude_code_opts(repo_path, &prompt, 1, 180).await.ok()?;
    let trimmed = raw.trim();
    let json_start = trimmed.find('{')?;
    let json_end = trimmed.rfind('}')?;
    if json_end <= json_start { return None; }
    let json_slice = &trimmed[json_start..=json_end];
    let parsed: serde_json::Value = serde_json::from_str(json_slice).ok()?;
    let what = parsed.get("what").and_then(|v| v.as_str()).unwrap_or("").trim();
    let how = parsed.get("how").and_then(|v| v.as_str()).unwrap_or("").trim();
    let cs = parsed.get("customer_message").and_then(|v| v.as_str()).unwrap_or("").trim();
    if what.is_empty() && how.is_empty() && cs.is_empty() { return None; }

    let mut md = String::new();
    md.push_str("### What was fixed\n");
    md.push_str(if what.is_empty() { "_not provided_" } else { what });
    md.push_str("\n\n### How it was fixed\n");
    md.push_str(if how.is_empty() { "_not provided_" } else { how });
    md.push_str("\n\n### For Customer Success\n");
    md.push_str(if cs.is_empty() { "_not provided_" } else { cs });
    md.push('\n');
    Some(md)
}

/// Create a PR with before/after screenshots uploaded to Supabase Storage.
async fn create_pr(
    config: &super::supabase::SupabaseConfig,
    repo_path: &str,
    title: &str,
    description: &str,
    task_id: &str,
    branch: &Option<String>,
    base_branch_override: Option<&str>,
    screenshot_dir: &str,
    has_screenshots: bool,
    qa_note: Option<&str>,
) -> Result<String, String> {
    // Branch should already be resolved by execute_task, but fallback just in case
    let branch_name = branch.clone().unwrap_or_else(|| "agent-one/patch".to_string());
    let base_branch = match base_branch_override {
        Some(b) if !b.is_empty() => b.to_string(),
        _ => detect_base_branch(repo_path).await,
    };

    // We should already be on branch_name from prepare_workspace; checkout (not -B)
    // so we fail loudly if somehow we're not, instead of blowing away the branch ref.
    let _ = async_cmd("git")
        .args(["checkout", &branch_name])
        .current_dir(repo_path)
        .output()
        .await;

    // Add .agent-one to .gitignore if not already there
    let gitignore_path = join_path(repo_path, ".gitignore");
    if let Ok(contents) = tokio::fs::read_to_string(&gitignore_path).await {
        if !contents.contains(".agent-one") {
            let _ = tokio::fs::write(&gitignore_path, format!("{}\n.agent-one/\n", contents.trim_end())).await;
        }
    }

    // Stage anything Claude left uncommitted (usually nothing; the prompt asks him to commit).
    let stage = async_cmd("git").args(["add", "-A"]).current_dir(repo_path).output().await.map_err(|e| format!("git add failed: {}", e))?;
    if !stage.status.success() { return Err("git add failed".to_string()); }

    let has_staged = !async_cmd("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(repo_path)
        .output().await
        .map_err(|e| format!("git diff check failed: {}", e))?
        .status.success();

    let commits_ahead: u32 = async_cmd("git")
        .args(["rev-list", "--count", &format!("origin/{}..HEAD", base_branch)])
        .current_dir(repo_path)
        .output().await
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    if has_staged {
        // Claude didn't commit (or left leftovers); commit them for him.
        let commit_msg = format!("samwise: {}", title);
        let commit = async_cmd("git").args(["commit", "-m", &commit_msg]).current_dir(repo_path).output().await.map_err(|e| format!("git commit failed: {}", e))?;
        if !commit.status.success() {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            return Err(format!("git commit failed: {}", stderr));
        }
    } else if commits_ahead == 0 {
        // Nothing staged AND no new commits vs base -> genuinely no work done.
        return Err("No changes on branch vs base".to_string());
    }
    // else: Claude already committed, nothing staged -> just push what's on the branch.

    // Push
    let push = async_cmd("git").args(["push", "-u", "origin", &branch_name]).current_dir(repo_path).output().await.map_err(|e| format!("git push failed: {}", e))?;
    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        return Err(format!("git push failed: {}", stderr));
    }

    // Build PR body with Supabase Storage URLs instead of repo-relative paths
    let mut pr_body = format!("## {}\n\n{}\n\n", title, description);

    // Ask Claude to summarize the diff into three sections Matt actually reads:
    // what was fixed (customer-visible), how it was fixed (technical),
    // and a paste-ready blurb for Customer Success. Best-effort: if the
    // summarizer fails or returns junk, we still ship the PR without it.
    if let Some(summary_md) = summarize_pr_changes(repo_path, &base_branch, title, description).await {
        pr_body.push_str(&summary_md);
        pr_body.push('\n');
    }

    if has_screenshots {
        let (_, screenshot_md) = upload_screenshots_to_storage(config, task_id, screenshot_dir).await
            .unwrap_or_default();
        pr_body.push_str(&screenshot_md);
    }
    if let Some(note) = qa_note {
        pr_body.push_str(note);
    }
    pr_body.push_str("\n\n---\nAutomated by SamWise");

    // Create PR with explicit base branch
    let pr = async_cmd("gh")
        .args(["pr", "create", "--title", title, "--body", &pr_body, "--head", &branch_name, "--base", &base_branch])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| format!("gh pr create failed: {}", e))?;

    let pr_url = if pr.status.success() {
        String::from_utf8_lossy(&pr.stdout).trim().to_string()
    } else {
        let stderr = String::from_utf8_lossy(&pr.stderr).to_string();
        // Re-queued task: PR already exists for this branch. The push above just
        // updated it with the new commits, so look up the URL and refresh the body
        // (screenshots, summary) instead of erroring out.
        if stderr.contains("already exists") {
            let existing = async_cmd("gh")
                .args(["pr", "view", &branch_name, "--json", "url", "-q", ".url"])
                .current_dir(repo_path)
                .output()
                .await
                .map_err(|e| format!("gh pr view failed: {}", e))?;
            if !existing.status.success() {
                let view_err = String::from_utf8_lossy(&existing.stderr);
                return Err(format!(
                    "gh pr create failed (already exists) and gh pr view failed: {}",
                    view_err
                ));
            }
            let url = String::from_utf8_lossy(&existing.stdout).trim().to_string();
            if url.is_empty() {
                return Err(format!("gh pr create failed: {}", stderr));
            }
            // Refresh the body so re-runs get the latest screenshots and summary.
            let _ = async_cmd("gh")
                .args(["pr", "edit", &branch_name, "--body", &pr_body])
                .current_dir(repo_path)
                .output()
                .await;
            url
        } else {
            return Err(format!("gh pr create failed: {}", stderr));
        }
    };

    // Clean up local screenshot directory
    let _ = tokio::fs::remove_dir_all(screenshot_dir).await;

    Ok(pr_url)
}
