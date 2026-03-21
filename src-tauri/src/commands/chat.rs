use serde::Serialize;
use serde_json::Value;
use std::io::Write;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

use super::supabase::{self, SupabaseState};
use super::worker::{find_claude_exe, WorkerState};

// ── Persistent chat session ─────────────────────────────────────────

struct ChatSession {
    stdin: std::process::ChildStdin,
    response_rx: std::sync::mpsc::Receiver<String>,
    #[allow(dead_code)]
    child: std::process::Child,
}

fn chat_session() -> &'static Arc<Mutex<Option<ChatSession>>> {
    static SESSION: OnceLock<Arc<Mutex<Option<ChatSession>>>> = OnceLock::new();
    SESSION.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Spawn (or re-spawn) the persistent Claude Code session for chat.
/// Uses pipe mode with stream-json so the process stays alive between messages.
fn spawn_chat_session() -> Result<ChatSession, String> {
    let claude_exe = find_claude_exe();

    let mut cmd = if claude_exe.ends_with(".cmd") {
        let mut c = std::process::Command::new("cmd.exe");
        c.arg("/C").arg(&claude_exe);
        c
    } else {
        std::process::Command::new(&claude_exe)
    };

    cmd.arg("-p")
        .arg("--output-format").arg("stream-json")
        .arg("--input-format").arg("stream-json")
        .arg("--verbose")
        .arg("--dangerously-skip-permissions");

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn Claude Code: {}", e))?;

    let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to get stdout")?;

    // Channel for collecting response lines from stdout reader thread
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    // Stdout reader thread - reads stream-json lines and sends to channel
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(data) if !data.is_empty() => {
                    if tx.send(data).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    });

    // Stderr reader thread - just drain it so the process doesn't block
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(data) => {
                        if !data.is_empty() {
                            log::debug!("[sam-chat stderr] {}", data);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    log::info!("[chat] Spawned persistent Claude Code session for Sam");

    Ok(ChatSession {
        stdin,
        response_rx: rx,
        child,
    })
}

/// Send a message to the persistent session and collect the full response.
fn send_and_collect(session: &mut ChatSession, prompt: &str) -> Result<String, String> {
    // Build stream-json input message
    let input = serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": prompt,
        }
    });

    let input_str = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    // Write to stdin
    session.stdin.write_all(input_str.as_bytes()).map_err(|e| format!("stdin write failed: {}", e))?;
    session.stdin.write_all(b"\n").map_err(|e| format!("stdin newline failed: {}", e))?;
    session.stdin.flush().map_err(|e| format!("stdin flush failed: {}", e))?;

    // Collect response lines until we see a "result" type (response complete)
    let mut assistant_text = String::new();
    let timeout = std::time::Duration::from_secs(90);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err("Chat response timed out after 90s".to_string());
        }

        match session.response_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(line) => {
                // Parse the stream-json line
                if let Ok(parsed) = serde_json::from_str::<Value>(&line) {
                    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match msg_type {
                        "assistant" => {
                            // Full assistant message - extract text content
                            if let Some(message) = parsed.get("message") {
                                if let Some(content) = message.get("content") {
                                    if let Some(arr) = content.as_array() {
                                        for block in arr {
                                            if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                                                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                                    assistant_text.push_str(text);
                                                }
                                            }
                                        }
                                    } else if let Some(text) = content.as_str() {
                                        assistant_text.push_str(text);
                                    }
                                }
                            }
                        }
                        "result" => {
                            // Response complete
                            break;
                        }
                        _ => {
                            // stream_event, system, etc - ignore for now
                        }
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Claude Code session closed unexpectedly".to_string());
            }
        }
    }

    if assistant_text.is_empty() {
        return Err("Claude Code returned empty response".to_string());
    }

    Ok(assistant_text.trim().to_string())
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct CreatedTaskInfo {
    pub id: String,
    pub title: String,
    pub task_type: String,
}

#[derive(Serialize, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub message_id: Option<String>,
    pub created_tasks: Vec<CreatedTaskInfo>,
}

// ── Main chat command ───────────────────────────────────────────────

#[tauri::command]
pub async fn chat_respond(
    user_message: String,
    sb_state: tauri::State<'_, SupabaseState>,
    worker_state: tauri::State<'_, WorkerState>,
) -> Result<ChatResponse, String> {
    let config = sb_state.get_config().await;
    if config.url.is_empty() {
        return Err("Supabase not configured".to_string());
    }

    // 1. Fetch conversation context BEFORE saving the new message
    let recent_chat = fetch_recent_chat(&config).await;

    // 2. Save user message to Supabase
    if let Err(e) = supabase::send_message(&config, &serde_json::json!({
        "role": "user",
        "content": &user_message,
    })).await {
        log::warn!("[chat] Failed to save user message: {}", e);
    }

    // 3. Fetch board state
    let board_context = build_board_context(&config, &worker_state).await;

    // 4. Build the full prompt (board context + conversation + new message)
    let prompt = build_system_prompt(&board_context, &recent_chat, &user_message);

    // 5. Send to persistent Claude Code session
    let raw_response = {
        let mut session_guard = chat_session().lock().await;

        // Spawn session if not alive
        let needs_spawn = session_guard.is_none();
        if needs_spawn {
            match spawn_chat_session() {
                Ok(s) => { *session_guard = Some(s); }
                Err(e) => return Err(format!("Failed to start Sam's chat session: {}", e)),
            }
        }

        let session = session_guard.as_mut().unwrap();

        // Try to send; if session is dead, respawn once
        match send_and_collect(session, &prompt) {
            Ok(response) => response,
            Err(e) => {
                log::warn!("[chat] Session failed ({}), respawning...", e);
                match spawn_chat_session() {
                    Ok(new_session) => {
                        *session_guard = Some(new_session);
                        let session = session_guard.as_mut().unwrap();
                        send_and_collect(session, &prompt)?
                    }
                    Err(e2) => return Err(format!("Failed to restart Sam: {}", e2)),
                }
            }
        }
    };

    // 6. Parse response for task creation
    let (clean_text, task_requests) = parse_chat_response(&raw_response);

    // 7. Create any tasks
    let mut created_tasks = Vec::new();
    for req in &task_requests {
        match supabase::create_task(&config, req).await {
            Ok(result) => {
                if let Some(arr) = result.as_array() {
                    if let Some(task) = arr.first() {
                        let id = task.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let task_type = task.get("task_type").and_then(|v| v.as_str()).unwrap_or("code").to_string();
                        if !id.is_empty() {
                            created_tasks.push(CreatedTaskInfo { id, title, task_type });
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("[chat] Failed to create task: {}", e);
            }
        }
    }

    // 8. Save agent response to Supabase
    let response_text = if clean_text.trim().is_empty() {
        raw_response.trim().to_string()
    } else {
        clean_text.trim().to_string()
    };

    let message_id = match supabase::send_message(&config, &serde_json::json!({
        "role": "agent",
        "content": &response_text,
    })).await {
        Ok(result) => {
            result.as_array()
                .and_then(|arr| arr.first())
                .and_then(|msg| msg.get("id"))
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
        }
        Err(e) => {
            log::warn!("[chat] Failed to save agent response: {}", e);
            None
        }
    };

    Ok(ChatResponse {
        content: response_text,
        message_id,
        created_tasks,
    })
}

// ── Fetch recent chat messages ──────────────────────────────────────

async fn fetch_recent_chat(config: &supabase::SupabaseConfig) -> String {
    let messages = match supabase::fetch_messages(config).await {
        Ok(m) => m,
        Err(_) => return String::new(),
    };

    let Some(arr) = messages.as_array() else {
        return String::new();
    };

    let recent: Vec<String> = arr.iter().rev().take(20).rev().map(|m| {
        let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let display_role = match role {
            "user" => "Matt",
            "agent" => "Sam",
            "system" => "System",
            _ => role,
        };
        format!("{}: {}", display_role, content)
    }).collect();

    recent.join("\n")
}

// ── Build board context ─────────────────────────────────────────────

async fn build_board_context(
    config: &supabase::SupabaseConfig,
    worker_state: &WorkerState,
) -> String {
    let mut ctx = String::new();

    let tasks = match supabase::fetch_tasks(config, None).await {
        Ok(t) => t,
        Err(_) => return "Board: unable to fetch".to_string(),
    };

    let Some(arr) = tasks.as_array() else {
        return "Board: no tasks".to_string();
    };

    let mut queued = Vec::new();
    let mut in_progress = Vec::new();
    let mut testing = Vec::new();
    let mut review = Vec::new();
    let mut approved = Vec::new();

    for task in arr {
        let status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
        let priority = task.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");
        let task_type = task.get("task_type").and_then(|v| v.as_str()).unwrap_or("code");
        let project = task.get("project").and_then(|v| v.as_str()).unwrap_or("");

        let desc = format!("[{}] {} ({}{})",
            priority.to_uppercase(),
            title,
            task_type,
            if project.is_empty() { String::new() } else { format!(", {}", project) }
        );

        match status {
            "queued" => queued.push(desc),
            "in_progress" => in_progress.push(desc),
            "testing" => testing.push(desc),
            "review" => review.push(desc),
            "approved" => approved.push(desc),
            _ => {}
        }
    }

    ctx.push_str(&format!(
        "Queued: {} | In Progress: {} | Testing: {} | Review: {} | Approved: {}\n",
        queued.len(), in_progress.len(), testing.len(), review.len(), approved.len()
    ));

    let all_active: Vec<(&str, &Vec<String>)> = vec![
        ("In Progress", &in_progress),
        ("Testing", &testing),
        ("Review", &review),
        ("Queued", &queued),
        ("Approved", &approved),
    ];

    for (label, tasks) in all_active {
        for t in tasks {
            ctx.push_str(&format!("- {} ({})\n", t, label));
        }
    }

    let running = worker_state.running.load(std::sync::atomic::Ordering::Relaxed);
    let machine = worker_state.machine_name.lock().await.clone();
    let current_task = worker_state.current_task_id.lock().await.clone();

    if running {
        let machine_str = machine.as_deref().unwrap_or("unknown");
        if let Some(task_id) = &current_task {
            ctx.push_str(&format!("Worker: ONLINE on {}, currently working on task {}\n", machine_str, task_id));
        } else {
            ctx.push_str(&format!("Worker: ONLINE on {}, idle\n", machine_str));
        }
    } else {
        ctx.push_str("Worker: OFFLINE (not running on this machine)\n");
    }

    ctx
}

// ── Build system prompt ─────────────────────────────────────────────

fn build_system_prompt(board_context: &str, recent_chat: &str, user_message: &str) -> String {
    format!(
        r#"You are Sam (SamWise), Matt's AI developer and teammate. You work 24/7. You're proactive, competent, and talk like a senior dev on Slack - casual but professional. Dry humor, genuine loyalty. You never say "I'm an AI" or "As an AI." You are a teammate.

## Current Board State
{board_context}

## What You Can Do
- Answer questions directly (quick answers, advice, perspective, explanations)
- Create tasks for real work that requires coding, research, or investigation

## When to Create a Task
If Matt asks you to BUILD, FIX, IMPLEMENT, REFACTOR, RESEARCH, INVESTIGATE, or DO something that requires actual work beyond a quick answer, create a task. Include a JSON block in your response like this:

```json
{{"create_task": {{"title": "Short descriptive title", "description": "Detailed description of what needs to be done", "priority": "medium", "task_type": "code", "project": "project-name", "source": "chat"}}}}
```

- task_type "code" = write code, make changes, open a PR
- task_type "research" = investigate, analyze, read code, report findings (no PR)
- priority: "critical", "high", "medium", "low"
- project: the project/repo name if mentioned, otherwise omit

Do NOT create tasks for simple questions, opinions, quick lookups, or general chat.
When you create a task, mention it naturally in your response (e.g. "On it, I've queued that up.").

## Recent Conversation
{recent_chat}

Matt's latest message: {user_message}

Respond naturally. Keep it brief and conversational."#,
        board_context = board_context,
        recent_chat = recent_chat,
        user_message = user_message,
    )
}

// ── Parse response for task creation blocks ─────────────────────────

fn parse_chat_response(raw: &str) -> (String, Vec<Value>) {
    let mut clean_text = raw.to_string();
    let mut task_requests = Vec::new();

    let mut search_from = 0;
    loop {
        let Some(start) = clean_text[search_from..].find("```json") else { break; };
        let start = search_from + start;
        let json_start = start + 7;

        let Some(end) = clean_text[json_start..].find("```") else { break; };
        let end = json_start + end;

        let json_str = clean_text[json_start..end].trim();

        if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
            if let Some(task_data) = parsed.get("create_task") {
                let mut task = serde_json::json!({
                    "status": "queued",
                    "assignee": "agent",
                    "source": "chat",
                });

                if let Some(obj) = task_data.as_object() {
                    for (key, value) in obj {
                        task[key] = value.clone();
                    }
                }

                if task.get("title").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
                    task_requests.push(task);
                }

                let block_end = end + 3;
                clean_text = format!("{}{}", &clean_text[..start], &clean_text[block_end..]);
                continue;
            }
        }

        search_from = end + 3;
    }

    (clean_text, task_requests)
}
