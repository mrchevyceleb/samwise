use serde::Serialize;
use serde_json::Value;

use super::supabase::{self, SupabaseState};
use super::worker::{run_claude_code_opts, WorkerState};

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

// Fixed UUID for the default Sam<->Matt conversation.
// The ae_messages.conversation_id column is typed uuid, so "default" is
// silently rejected by PostgREST and replaced by gen_random_uuid(), which
// scatters every message into its own conversation.
const DEFAULT_CONVERSATION_ID: &str = "00000000-0000-0000-0000-000000000001";

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
        "conversation_id": DEFAULT_CONVERSATION_ID,
    })).await {
        log::warn!("[chat] Failed to save user message: {}", e);
    }

    // 3. Fetch board state and project registry
    let board_context = build_board_context(&config, &worker_state).await;
    let project_registry = build_project_registry(&config).await;

    // 4. Build the full prompt (board context + projects + conversation + new message)
    let prompt = build_system_prompt(&board_context, &project_registry, &recent_chat, &user_message);

    // 5. One-shot Claude Code call (same proven approach as the worker).
    // Chat is conversational so limit to 3 turns and 90s timeout.
    let raw_response = run_claude_code_opts(".", &prompt, 3, 90).await?;

    // 6. Parse response for task creation
    let (clean_text, task_requests) = parse_chat_response(&raw_response);

    // 7. Create any tasks - enrich with project registry data (repo_path, repo_url, preview_url)
    let projects = supabase::fetch_projects(&config).await.ok();
    let mut created_tasks = Vec::new();
    for req in &task_requests {
        let mut enriched = req.clone();
        if let Some(project_name) = req.get("project").and_then(|v| v.as_str()) {
            if let Some(ref proj_arr) = projects {
                if let Some(arr) = proj_arr.as_array() {
                    if let Some(proj) = arr.iter().find(|p| p.get("name").and_then(|v| v.as_str()) == Some(project_name)) {
                        // Backfill repo_path, repo_url, preview_url from project registry
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
        }
        match supabase::create_task(&config, &enriched).await {
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
        "conversation_id": DEFAULT_CONVERSATION_ID,
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

pub async fn fetch_recent_chat(config: &supabase::SupabaseConfig) -> String {
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

pub async fn build_board_context(
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

    // Fetch recent comments on active tasks so Sam knows what's been discussed
    if let Ok(comments) = supabase::fetch_recent_comments(config, 20).await {
        if let Some(comment_arr) = comments.as_array() {
            if !comment_arr.is_empty() {
                ctx.push_str("\n## Recent Task Comments\n");
                for c in comment_arr.iter().rev().take(15) {
                    let author = c.get("author").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let content = c.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let task_id = c.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                    // Find the task title for this comment
                    let task_title = arr.iter()
                        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(task_id))
                        .and_then(|t| t.get("title").and_then(|v| v.as_str()))
                        .unwrap_or("unknown task");
                    let display_author = if author == "agent" { "Sam" } else { "Matt" };
                    // Truncate long comments
                    let short: String = if content.chars().count() > 120 { content.chars().take(120).collect() } else { content.to_string() };
                    ctx.push_str(&format!("- [{}] {}: {}\n", task_title, display_author, &short));
                }
            }
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

// ── Build project registry context ──────────────────────────────────

pub async fn build_project_registry(config: &supabase::SupabaseConfig) -> String {
    let projects = match supabase::fetch_projects(config).await {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    let Some(arr) = projects.as_array() else {
        return String::new();
    };

    if arr.is_empty() {
        return String::new();
    }

    let mut ctx = String::from("## Known Projects\nONLY reference these projects. Do NOT invent repo URLs or project names.\n");

    for project in arr {
        let name = project.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let repo_url = project.get("repo_url").and_then(|v| v.as_str()).unwrap_or("");
        let repo_path = project.get("repo_path").and_then(|v| v.as_str()).unwrap_or("");
        let client = project.get("client").and_then(|v| v.as_str()).unwrap_or("");
        let preview_url = project.get("preview_url").and_then(|v| v.as_str()).unwrap_or("");
        let deploy = project.get("deploy_method").and_then(|v| v.as_str()).unwrap_or("");

        ctx.push_str(&format!("- **{}**", name));
        if !client.is_empty() { ctx.push_str(&format!(" ({})", client)); }
        ctx.push('\n');
        if !repo_url.is_empty() { ctx.push_str(&format!("  repo: {}\n", repo_url)); }
        if !repo_path.is_empty() { ctx.push_str(&format!("  path: {}\n", repo_path)); }
        if !preview_url.is_empty() { ctx.push_str(&format!("  preview: {}\n", preview_url)); }
        if !deploy.is_empty() { ctx.push_str(&format!("  deploy: {}\n", deploy)); }
    }

    ctx.push_str("\nWhen creating tasks, use the exact project name and repo_url from this list. If a project isn't listed here, ask Matt to add it first.\n");

    ctx
}

// ── Build system prompt ─────────────────────────────────────────────

pub fn build_system_prompt(board_context: &str, project_registry: &str, recent_chat: &str, user_message: &str) -> String {
    format!(
        r#"You are Sam (SamWise), Matt's AI developer and teammate. You work 24/7. You're proactive, competent, and talk like a senior dev on Slack - casual but professional. Dry humor, genuine loyalty. You never say "I'm an AI" or "As an AI." You are a teammate.

## Current Board State
{board_context}

{project_registry}

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
        project_registry = project_registry,
        recent_chat = recent_chat,
        user_message = user_message,
    )
}

// ── Parse response for task creation blocks ─────────────────────────

pub fn parse_chat_response(raw: &str) -> (String, Vec<Value>) {
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
