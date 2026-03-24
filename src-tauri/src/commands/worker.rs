use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};

use super::dev_server;
use super::supabase::{self, SupabaseConfig, SupabaseState};
use crate::process::async_cmd;

// ── State ────────────────────────────────────────────────────────────

pub struct WorkerState {
    pub running: Arc<AtomicBool>,
    pub machine_name: Arc<tokio::sync::Mutex<Option<String>>>,
    pub current_task_id: Arc<tokio::sync::Mutex<Option<String>>>,
    pub last_telegram_update_id: Arc<tokio::sync::Mutex<Option<i64>>>,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            machine_name: Arc::new(tokio::sync::Mutex::new(None)),
            current_task_id: Arc::new(tokio::sync::Mutex::new(None)),
            last_telegram_update_id: Arc::new(tokio::sync::Mutex::new(None)),
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
    let last_tg_update = Arc::clone(&state.last_telegram_update_id);
    let sb_config_arc = Arc::clone(&sb_state.config);
    let app_handle = app.clone();

    tokio::spawn(async move {
        worker_loop(running, current_task, last_tg_update, machine_name, sb_config_arc, app_handle).await;
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
    last_telegram_update_id: Arc<tokio::sync::Mutex<Option<i64>>>,
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

    while running.load(Ordering::Relaxed) {
        let config = sb_config_arc.read().await.clone();

        // Heartbeat every tick
        let _ = supabase::worker_heartbeat(&config, &machine_name).await;

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

                                        let result = execute_task(&app, &machine_name, &config, task.clone()).await;

                                        {
                                            let mut ct = current_task_id.lock().await;
                                            *ct = None;
                                        }

                                        match &result {
                                            Ok(msg) => {
                                                emit_worker_event(&app, "task_completed", msg, Some(&task_id));
                                                // Proactive chat: announce completion
                                                if msg.contains("PR created") {
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

    let repo_path = match repo_path {
        Some(p) => p,
        None => {
            // No repo_path found - cannot run Claude Code in an unknown directory
            let msg = if project_name.is_empty() {
                "No repo_path or project set on this task. I need to know which repo to work in. Add a project or set the repo_path.".to_string()
            } else {
                format!("Project \"{}\" doesn't have a repo_path configured. Add it in the Projects settings so I know where to find the code.", project_name)
            };
            agent_comment(config, &task_id, &msg).await;
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "failed",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;
            return Err(format!("No repo_path for task {}", task_id));
        }
    };

    // Resolve branch name ONCE - only needed for code tasks
    let branch: Option<String> = if is_research {
        None
    } else {
        task.get("branch")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(format!("agent-one/{}", uuid::Uuid::new_v4())))
    };

    // 1. Post initial comment
    agent_comment(config, &task_id, &format!("On it. Setting up for: {}", title)).await;

    // 2. Update status
    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
        "status": "in_progress",
        "updated_at": chrono::Utc::now().to_rfc3339(),
    })).await;

    emit_worker_event(app, "task_working", &format!("Working on: {}", title), Some(&task_id));
    if notify_task_started {
        send_telegram(config, &format!("Working on: *{}*", escape_markdown_v2(&title))).await;
    }

    // 3. Check out branch (code tasks only)
    if !is_research {
        if let Some(ref branch_name) = branch {
            let _ = async_cmd("git")
                .args(["checkout", "-B", branch_name])
                .current_dir(&repo_path)
                .output()
                .await;
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
                        match dev_server::wait_for_ready(handle.port, 60).await {
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
    let screenshot_dir = format!("C:\\agent-one-screenshots\\{}", task_id);
    if !is_research {
        if let Some(ref preview) = preview_url {
            let _ = tokio::fs::create_dir_all(&screenshot_dir).await;
            agent_comment(config, &task_id, "Taking before screenshots...").await;

            let _ = take_screenshot(preview, &format!("{}\\before-desktop.png", screenshot_dir), "1280,720").await;
            let _ = take_screenshot(preview, &format!("{}\\before-mobile.png", screenshot_dir), "393,852").await;
        }
    }

    // 5. Run Claude Code CLI
    let action_label = if is_research { "Running analysis with Claude Code..." } else { "Starting code changes with Claude Code..." };
    agent_comment(config, &task_id, action_label).await;

    // Build a context-aware prompt with repo info
    let mut prompt_parts: Vec<String> = Vec::new();

    // Read CLAUDE.md if it exists in the repo
    let claude_md_path = format!("{}\\CLAUDE.md", repo_path);
    if let Ok(claude_md) = tokio::fs::read_to_string(&claude_md_path).await {
        let claude_md_truncated = truncate(&claude_md, 2000);
        prompt_parts.push(format!("## Project Instructions (from CLAUDE.md)\n{}\n", claude_md_truncated));
    }

    // Read worker rules from cached settings
    if let Some(ref settings) = cached_settings {
        if let Some(rules) = settings.get("workerRules").and_then(|v| v.as_array()) {
            let rule_strings: Vec<String> = rules
                .iter()
                .filter_map(|r| r.as_str())
                .filter(|r| !r.trim().is_empty())
                .enumerate()
                .map(|(i, r)| format!("{}. {}", i + 1, r))
                .collect();
            if !rule_strings.is_empty() {
                prompt_parts.push(format!(
                    "## Worker Rules (MUST follow these)\n{}\n",
                    rule_strings.join("\n")
                ));
            }
        }
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

    // The actual task
    if is_research {
        prompt_parts.push(format!(
            "## Task\n**{}**\n\n{}\n\n## Instructions\nThis is a RESEARCH/ANALYSIS task. Do NOT make any code changes, do NOT commit, do NOT create files. Read, analyze, and provide a thorough written report. Be detailed and specific.",
            title, description
        ));
    } else {
        prompt_parts.push(format!(
            "## Task\n**{}**\n\n{}\n\n## Instructions\nComplete this task. Make all necessary code changes. Be thorough but focused. Commit your changes when done.",
            title, description
        ));
    }

    let prompt = prompt_parts.join("\n");

    // 30-minute timeout for task execution, 50 max turns to prevent runaway sessions
    let claude_result = run_claude_code_opts(&repo_path, &prompt, 50, 1800).await;

    let task_result = match claude_result {
        Ok(output) if output.is_empty() => {
            // Claude Code exited cleanly but produced no output - wrong directory or no-op
            agent_comment(config, &task_id, "Claude Code produced no output. The repo_path may be wrong or the task didn't result in any work. Re-check the project settings.").await;
            let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                "status": "failed",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await;
            if notify_task_failed {
                send_telegram(config, &format!(
                    "Hit a snag on *{}*: Claude Code produced no output",
                    escape_markdown_v2(&title)
                )).await;
            }
            if let Some(h) = dev_server_handle.take() { let _ = dev_server::kill_dev_server(h).await; }
            cleanup_branch(&repo_path, &branch).await;
            return Err("Claude Code produced no output".to_string());
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
                        agent_comment(config, &task_id, "Analysis complete. Full report saved -- click the Report tab above to read it.").await;
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

            // 6. Take AFTER screenshots
            if let Some(ref preview) = preview_url {
                agent_comment(config, &task_id, "Taking after screenshots...").await;

                let _ = take_screenshot(preview, &format!("{}\\after-desktop.png", screenshot_dir), "1280,720").await;
                let _ = take_screenshot(preview, &format!("{}\\after-mobile.png", screenshot_dir), "393,852").await;
            }

            // 6b. Upload screenshots to Supabase Storage and update task with public URLs
            if preview_url.is_some() {
                let (urls, _) = upload_screenshots_to_storage(config, &task_id, &screenshot_dir).await
                    .unwrap_or_default();
                // Split into before (first 2) and after (last 2)
                let before_urls: Vec<&String> = urls.iter().filter(|u| u.contains("before-")).collect();
                let after_urls: Vec<&String> = urls.iter().filter(|u| u.contains("after-")).collect();
                let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                    "screenshots_before": before_urls,
                    "screenshots_after": after_urls,
                })).await;
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

            // 8. Create PR (branch is already resolved, no UUID regeneration)
            let pr_result = create_pr(config, &repo_path, &title, &description, &task_id, &branch, &screenshot_dir, preview_url.is_some()).await;

            match pr_result {
                Ok(pr_url) => {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "status": "review",
                        "pr_url": pr_url,
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
                    agent_comment(config, &task_id, &format!("PR's up: {}. Let me know if you want any changes.", pr_url)).await;
                    if notify_task_completed_code {
                        send_telegram(config, &format!(
                            "PR's up for *{}*: {}",
                            escape_markdown_v2(&title),
                            escape_markdown_v2(&pr_url)
                        )).await;
                    }
                    Ok(format!("PR created: {}", pr_url))
                }
                Err(e) => {
                    let _ = supabase::update_task(config, &task_id, &serde_json::json!({
                        "status": "review",
                        "updated_at": chrono::Utc::now().to_rfc3339(),
                    })).await;
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

    // Cleanup: switch back to default branch and delete the local feature branch.
    // The remote branch persists for the PR; this just prevents local accumulation.
    cleanup_branch(&repo_path, &branch).await;

    task_result
}

/// Clean up local feature branch after task execution.
/// Discards uncommitted changes, switches to base branch, and safely deletes the feature branch.
/// Uses `-d` (not `-D`) to refuse deleting branches with unpushed commits.
async fn cleanup_branch(repo_path: &str, branch: &Option<String>) {
    let Some(ref branch_name) = branch else { return; };

    // Discard any uncommitted changes (failed tasks may leave dirty worktree)
    if let Err(e) = async_cmd("git").args(["reset", "--hard"]).current_dir(repo_path).output().await {
        log::warn!("[worker] Branch cleanup: git reset --hard failed: {}", e);
    }
    if let Err(e) = async_cmd("git").args(["clean", "-fd"]).current_dir(repo_path).output().await {
        log::warn!("[worker] Branch cleanup: git clean -fd failed: {}", e);
    }

    // Switch back to default branch
    let base = detect_base_branch(repo_path).await;
    let checkout = async_cmd("git").args(["checkout", &base]).current_dir(repo_path).output().await;
    match checkout {
        Ok(out) if out.status.success() => {
            // Safe delete: -d refuses if commits aren't pushed (prevents data loss)
            if let Err(e) = async_cmd("git").args(["branch", "-d", branch_name]).current_dir(repo_path).output().await {
                log::warn!("[worker] Branch cleanup: git branch -d failed: {}", e);
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            log::warn!("[worker] Branch cleanup: checkout {} failed: {}", base, stderr.trim());
        }
        Err(e) => {
            log::warn!("[worker] Branch cleanup: checkout {} failed: {}", base, e);
        }
    }
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

    for update in results {
        let update_id = update.get("update_id").and_then(|v| v.as_i64()).unwrap_or(0);

        // Update the offset
        {
            let mut guard = last_update_id.lock().await;
            *guard = Some(update_id);
        }

        // Extract message text
        let message = match update.get("message") {
            Some(m) => m,
            None => continue,
        };

        let chat_id = message.get("chat").and_then(|c| c.get("id")).and_then(|v| v.as_i64());
        let chat_id_str = chat_id.map(|id| id.to_string()).unwrap_or_default();

        // Only process messages from the configured chat
        if chat_id_str != expected_chat_id {
            continue;
        }

        let text = match message.get("text").and_then(|v| v.as_str()) {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => continue,
        };

        log::info!("[worker] Telegram message received: {}", &text[..text.len().min(50)]);

        // Process through Sam's chat logic
        process_telegram_message(config, &text, machine_name).await;
    }
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
    let raw_response = match run_claude_code_opts(".", &prompt, 3, 90).await {
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
    for req in &task_requests {
        let mut enriched = req.clone();

        // Override project with @ mention if present
        if let Some(mentioned) = mentioned_projects.first() {
            enriched["project"] = serde_json::Value::String(mentioned.clone());
            enriched["status"] = serde_json::Value::String("queued".to_string());
        } else {
            // No @ mention: pending_confirmation
            enriched["status"] = serde_json::Value::String("pending_confirmation".to_string());
        }

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

    // 7c. If tasks created with pending_confirmation, send numbered project list via Telegram
    if !task_requests.is_empty() && mentioned_projects.is_empty() {
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
    let (exe, prefix_args) = find_claude_command();

    let mut cmd = async_cmd(&exe);
    for arg in &prefix_args {
        cmd.arg(arg);
    }

    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("text")
        .arg("--dangerously-skip-permissions");

    if max_turns > 0 {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    cmd.current_dir(cwd)
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
        return Err(format!("Claude Code failed (exit {}): {}", status, stderr_text.trim()));
    }

    Ok(stdout_text.trim().to_string())
}

/// Returns (executable, prefix_args) for spawning the Claude CLI.
/// On Windows with npm install, we bypass claude.cmd to avoid cmd.exe pipe
/// inheritance issues - directly invoking `node cli.js` keeps stdin reliable.
/// (Pattern borrowed from assistantos-v2's proven implementation.)
pub fn find_claude_command() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        // Official installer: direct .exe, no intermediary needed
        let local_bin = format!("{}\\.local\\bin\\claude.exe", home);
        if std::path::Path::new(&local_bin).exists() { return (local_bin, vec![]); }
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        // npm global install: bypass claude.cmd to avoid cmd.exe stdin pipe issues.
        // claude.cmd -> cmd.exe /c -> node cli.js breaks stdin with CREATE_NO_WINDOW.
        // Directly invoking node + cli.js avoids the cmd.exe layer entirely.
        let cli_js = format!("{}\\npm\\node_modules\\@anthropic-ai\\claude-code\\cli.js", appdata);
        if std::path::Path::new(&cli_js).exists() { return ("node".to_string(), vec![cli_js]); }
        // Fall back to claude on PATH
        ("claude".to_string(), vec![])
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        let local_bin = format!("{}/.local/bin/claude", home);
        if std::path::Path::new(&local_bin).exists() { return (local_bin, vec![]); }
        ("claude".to_string(), vec![])
    }
}

// Backwards-compat shim for imports that expect the old signature
pub fn find_claude_exe() -> String {
    find_claude_command().0
}

/// Take a screenshot using Playwright
async fn take_screenshot(url: &str, output_path: &str, viewport: &str) -> Result<(), String> {
    let output = async_cmd("npx")
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

/// Run visual QA: compare before/after screenshots using Claude Code vision.
/// Uses absolute paths so Claude Code can reliably read the images.
/// Retries once with a stricter prompt if JSON parsing fails.
async fn run_visual_qa(cwd: &str, title: &str, description: &str, screenshot_dir: &str) -> Result<(bool, String), String> {
    // Copy screenshots into the working directory so Claude Code can access them
    let agent_dir = format!("{}\\.agent-one", cwd);
    let _ = tokio::fs::create_dir_all(&agent_dir).await;

    let screenshot_names = ["before-desktop.png", "after-desktop.png", "before-mobile.png", "after-mobile.png"];
    for name in &screenshot_names {
        let src = format!("{}\\{}", screenshot_dir, name);
        let dst = format!("{}\\{}", agent_dir, name);
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
        let local_path = format!("{}\\{}", screenshot_dir, name);
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

/// Create a PR with before/after screenshots uploaded to Supabase Storage.
async fn create_pr(
    config: &super::supabase::SupabaseConfig,
    repo_path: &str,
    title: &str,
    description: &str,
    task_id: &str,
    branch: &Option<String>,
    screenshot_dir: &str,
    has_screenshots: bool,
) -> Result<String, String> {
    // Branch should already be resolved by execute_task, but fallback just in case
    let branch_name = branch.clone().unwrap_or_else(|| "agent-one/patch".to_string());
    let base_branch = detect_base_branch(repo_path).await;

    // Ensure we're on the right branch
    let _ = async_cmd("git")
        .args(["checkout", "-B", &branch_name])
        .current_dir(repo_path)
        .output()
        .await;

    // Add .agent-one to .gitignore if not already there
    let gitignore_path = format!("{}\\.gitignore", repo_path);
    if let Ok(contents) = tokio::fs::read_to_string(&gitignore_path).await {
        if !contents.contains(".agent-one") {
            let _ = tokio::fs::write(&gitignore_path, format!("{}\n.agent-one/\n", contents.trim_end())).await;
        }
    }

    // Stage all changes (but NOT .agent-one screenshots)
    let stage = async_cmd("git").args(["add", "-A"]).current_dir(repo_path).output().await.map_err(|e| format!("git add failed: {}", e))?;
    if !stage.status.success() { return Err("git add failed".to_string()); }

    // Check if there are changes
    let diff = async_cmd("git").args(["diff", "--cached", "--quiet"]).current_dir(repo_path).output().await.map_err(|e| format!("git diff check failed: {}", e))?;
    if diff.status.success() { return Err("No changes to commit".to_string()); }

    // Commit
    let commit_msg = format!("samwise: {}", title);
    let commit = async_cmd("git").args(["commit", "-m", &commit_msg]).current_dir(repo_path).output().await.map_err(|e| format!("git commit failed: {}", e))?;
    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        return Err(format!("git commit failed: {}", stderr));
    }

    // Push
    let push = async_cmd("git").args(["push", "-u", "origin", &branch_name]).current_dir(repo_path).output().await.map_err(|e| format!("git push failed: {}", e))?;
    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        return Err(format!("git push failed: {}", stderr));
    }

    // Build PR body with Supabase Storage URLs instead of repo-relative paths
    let mut pr_body = format!("## {}\n\n{}\n\n", title, description);
    if has_screenshots {
        let (_, screenshot_md) = upload_screenshots_to_storage(config, task_id, screenshot_dir).await
            .unwrap_or_default();
        pr_body.push_str(&screenshot_md);
    }
    pr_body.push_str("---\nAutomated by SamWise");

    // Create PR with explicit base branch
    let pr = async_cmd("gh")
        .args(["pr", "create", "--title", title, "--body", &pr_body, "--head", &branch_name, "--base", &base_branch])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| format!("gh pr create failed: {}", e))?;

    if !pr.status.success() {
        let stderr = String::from_utf8_lossy(&pr.stderr);
        return Err(format!("gh pr create failed: {}", stderr));
    }

    // Clean up local screenshot directory
    let _ = tokio::fs::remove_dir_all(screenshot_dir).await;

    Ok(String::from_utf8_lossy(&pr.stdout).trim().to_string())
}
