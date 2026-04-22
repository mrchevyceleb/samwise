use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::sync::LazyLock;

static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|[\s,])@([\w-]+)").unwrap());

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

    // Wrap the entire chat flow in a 630s hard timeout so the UI never hangs forever.
    // The Claude Code call inside has its own 600s timeout; the extra 30s covers Supabase I/O.
    // Opus on long context (Sentry dumps, log pastes) regularly runs multi-minute.
    match tokio::time::timeout(
        std::time::Duration::from_secs(630),
        chat_respond_inner(config, user_message, &worker_state),
    ).await {
        Ok(result) => result,
        Err(_) => Err("Sam took too long to respond (timed out after 10.5 minutes). Try again.".to_string()),
    }
}

async fn chat_respond_inner(
    config: supabase::SupabaseConfig,
    user_message: String,
    worker_state: &WorkerState,
) -> Result<ChatResponse, String> {
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

    // 2b. Fast-path: confirmation of pending tasks (checked BEFORE status to avoid "yes how's it going" skipping confirmation)
    if let Some(response_text) = handle_pending_confirmation(&config, &user_message).await {
        let message_id = match supabase::send_message(&config, &serde_json::json!({
            "role": "agent",
            "content": &response_text,
            "conversation_id": DEFAULT_CONVERSATION_ID,
        })).await {
            Ok(result) => result.as_array()
                .and_then(|arr| arr.first())
                .and_then(|msg| msg.get("id"))
                .and_then(|id| id.as_str())
                .map(|s| s.to_string()),
            Err(_) => None,
        };

        return Ok(ChatResponse {
            content: response_text,
            message_id,
            created_tasks: Vec::new(),
        });
    }

    // 2c. Fast-path: status queries skip Claude Code entirely
    if is_status_query(&user_message) {
        let board_context = build_board_context(&config, worker_state).await;
        let response_text = build_status_response(&board_context);

        let message_id = match supabase::send_message(&config, &serde_json::json!({
            "role": "agent",
            "content": &response_text,
            "conversation_id": DEFAULT_CONVERSATION_ID,
        })).await {
            Ok(result) => result.as_array()
                .and_then(|arr| arr.first())
                .and_then(|msg| msg.get("id"))
                .and_then(|id| id.as_str())
                .map(|s| s.to_string()),
            Err(e) => {
                log::warn!("[chat] Failed to save status response: {}", e);
                None
            }
        };

        return Ok(ChatResponse {
            content: response_text,
            message_id,
            created_tasks: Vec::new(),
        });
    }

    // 3. Fetch board state and project registry
    let board_context = build_board_context(&config, worker_state).await;
    let project_registry = build_project_registry(&config).await;

    // 3b. Extract @ mentions for explicit project tagging
    let projects_for_mentions = supabase::fetch_projects(&config).await.ok().unwrap_or(serde_json::json!([]));
    let mentioned_projects = extract_project_mentions(&user_message, &projects_for_mentions);

    // 4. Build the full prompt (board context + projects + conversation + new message)
    let mut effective_message = user_message.clone();
    if !mentioned_projects.is_empty() {
        effective_message = format!(
            "{}\n\n[System: Matt explicitly tagged @{}. Use this project for any tasks you create.]",
            user_message,
            mentioned_projects.join(", @")
        );
    }
    let prompt = build_system_prompt(&board_context, &project_registry, &recent_chat, &effective_message);

    // 5. One-shot Claude Code call (same proven approach as the worker).
    // 3 turns, 600s timeout. Long error-log pastes and deep thinking can push
    // Opus past several minutes; shorter timeouts just produce false failures.
    let raw_response = run_claude_code_opts(".", &prompt, 3, 600).await?;

    // 6. Parse response for task creation
    let (clean_text, task_requests) = parse_chat_response(&raw_response);

    // 7. Create any tasks - enrich with project registry data and handle @ mentions
    let projects = if mentioned_projects.is_empty() {
        supabase::fetch_projects(&config).await.ok()
    } else {
        Some(projects_for_mentions.clone())
    };
    let mut created_tasks = Vec::new();
    for req in &task_requests {
        let mut enriched = req.clone();

        // If @ mention was used, override the project field
        if let Some(mentioned) = mentioned_projects.first() {
            enriched["project"] = serde_json::Value::String(mentioned.clone());
        }

        // Rescue: Claude sometimes says "queuing up for operly" in the text
        // but omits the "project" field from the task JSON. When the JSON is
        // missing a project, try to extract one by scanning Sam's reply text
        // AND the user's message for any registered project name. Saves the
        // task from landing in pending_confirmation when the intent was clear.
        let has_project_now = enriched.get("project").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
        if !has_project_now {
            if let Some(ref proj_arr) = projects {
                if let Some(arr) = proj_arr.as_array() {
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
                        let n_lower = name.to_lowercase();
                        if haystack.contains(&n_lower) {
                            enriched["project"] = serde_json::Value::String(name.clone());
                            log::info!("[chat] inferred project '{}' from conversation text", name);
                            break;
                        }
                    }
                }
            }
        }

        // Autonomous flow: if Sam picked a project (via @ mention, JSON emit,
        // or the text-inference rescue above), create the task as queued.
        // If NO project could be resolved, skip task creation entirely — Sam
        // should ask Matt to clarify in his chat reply rather than leaving
        // a dead confirm-UI stub in the DB. Matt explicitly killed that UI.
        let has_project = enriched.get("project").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
        if !has_project {
            log::warn!("[chat] Skipping task create: no project resolvable from JSON, @mention, or text inference. Sam should ask in reply.");
            continue;
        }
        enriched["status"] = serde_json::Value::String("queued".to_string());

        // Backfill repo fields from project registry
        let project_name = enriched.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !project_name.is_empty() {
            if let Some(ref proj_arr) = projects {
                if let Some(arr) = proj_arr.as_array() {
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
    let mut pending_confirm = Vec::new();

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
            "pending_confirmation" => pending_confirm.push(desc),
            _ => {}
        }
    }

    ctx.push_str(&format!(
        "Queued: {} | In Progress: {} | Testing: {} | Review: {} | Approved: {}{}\n",
        queued.len(), in_progress.len(), testing.len(), review.len(), approved.len(),
        if pending_confirm.is_empty() { String::new() } else { format!(" | Pending Confirmation: {}", pending_confirm.len()) }
    ));

    let all_active: Vec<(&str, &Vec<String>)> = vec![
        ("Pending Confirmation", &pending_confirm),
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
- base_branch (optional): if Matt wants the worktree stacked on a feature branch instead of main/master, include e.g. "base_branch": "feature/payments". Leave out to default to the repo's main branch.

## Project Selection (Autonomous)
Pick the project and get moving. Don't ask for confirmation — Matt can course-correct after if you're wrong.

- If Matt used an @tag, use that project.
- If he named the project in plain English ("fix this operly bug", "update banana-code"), use that project.
- If it's clear from context (he's quoting an error from an app, or referring to a feature of one), use that project.
- Mention which project you chose in your reply ("On it, queuing up for **operly**.") but don't ask permission.
- ONLY ask for clarification if it's genuinely ambiguous (e.g. "fix the login bug" when 3 apps have login).

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
                search_from = start; // Reset to where the block was, since text was mutated
                continue;
            }
        }

        search_from = end + 3;
    }

    (clean_text, task_requests)
}

// ── Status query fast-path ──────────────────────────────────────────

const STATUS_KEYWORDS: &[&str] = &[
    "status", "how's", "hows", "how is", "progress", "update on",
    "what are you working on", "what's happening", "whats happening",
    "any updates", "how's it going", "hows it going", "check in",
    "checkin", "what are you doing", "where are we", "sitrep",
];

const TASK_CREATION_KEYWORDS: &[&str] = &[
    "build", "fix", "implement", "create", "add", "make", "change",
    "refactor", "deploy", "migrate", "write", "set up", "setup",
    "integrate", "remove", "delete", "update the", "upgrade",
];

/// Returns true if the message is a simple status check (no work request).
pub fn is_status_query(msg: &str) -> bool {
    let lower = msg.to_lowercase();

    // Must contain at least one status keyword
    let has_status = STATUS_KEYWORDS.iter().any(|kw| lower.contains(kw));
    if !has_status {
        return false;
    }

    // Bail if it also contains task-creation intent
    let has_work = TASK_CREATION_KEYWORDS.iter().any(|kw| lower.contains(kw));
    if has_work {
        return false;
    }

    true
}

/// Build a casual Sam-style status response from board context (no AI call needed).
pub fn build_status_response(board_context: &str) -> String {
    // Detect fetch errors (build_board_context returns "Board: unable to fetch" on Supabase errors)
    if board_context.starts_with("Board: unable") {
        return "Couldn't reach Supabase right now. Try again in a moment.".to_string();
    }

    // Parse the board context to extract useful info
    let lines: Vec<&str> = board_context.lines().collect();

    let mut response = String::from("Here's where things stand:\n\n");

    // First line is usually the summary counts
    if let Some(first) = lines.first() {
        if first.contains("Queued:") {
            response.push_str(&format!("{}\n\n", first));
        }
    }

    // Find worker status
    let worker_line = lines.iter().find(|l| l.starts_with("Worker:"));

    // Find in-progress tasks (match both title-case from build_board_context and lowercase from build_simple_board_context)
    let in_progress: Vec<&&str> = lines.iter()
        .filter(|l| l.contains("(In Progress)") || l.contains("(in_progress)"))
        .collect();

    if !in_progress.is_empty() {
        response.push_str("Working on:\n");
        for task in &in_progress {
            response.push_str(&format!("{}\n", task));
        }
        response.push('\n');
    }

    // Find queued tasks
    let queued: Vec<&&str> = lines.iter()
        .filter(|l| l.contains("(Queued)") || l.contains("(queued)"))
        .collect();

    if !queued.is_empty() {
        response.push_str(&format!("{} in the queue", queued.len()));
        if queued.len() <= 3 {
            response.push_str(":\n");
            for task in &queued {
                response.push_str(&format!("{}\n", task));
            }
        } else {
            response.push_str(". ");
        }
        response.push('\n');
    }

    // Find review/testing tasks
    let review: Vec<&&str> = lines.iter()
        .filter(|l| l.contains("(Review)") || l.contains("(review)") || l.contains("(Testing)") || l.contains("(testing)"))
        .collect();

    if !review.is_empty() {
        response.push_str(&format!("{} waiting for review/testing.\n", review.len()));
    }

    if let Some(wl) = worker_line {
        response.push_str(&format!("\n{}", wl));
    }

    if in_progress.is_empty() && queued.is_empty() && review.is_empty() {
        return "Board's empty right now. Nothing queued, nothing in progress. Give me something to do.".to_string();
    }

    response.trim().to_string()
}

// ── @ mention extraction ────────────────────────────────────────────

/// Extract project names from @mentions in a message, matched against the project registry.
/// Requires @ to be preceded by whitespace, comma, or start of string (avoids triggering on emails).
pub fn extract_project_mentions(message: &str, projects: &Value) -> Vec<String> {
    let Some(arr) = projects.as_array() else {
        return Vec::new();
    };

    let project_names: Vec<String> = arr.iter()
        .filter_map(|p| p.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();

    let mut matched = Vec::new();
    for cap in MENTION_RE.captures_iter(message) {
        let mention = &cap[1];
        // Case-insensitive match against project names
        if let Some(name) = project_names.iter().find(|n| n.eq_ignore_ascii_case(mention)) {
            if !matched.contains(name) {
                matched.push(name.clone());
            }
        }
    }

    matched
}

// ── Confirmation detection ──────────────────────────────────────────

const AFFIRMATIVE: &[&str] = &[
    "yes", "yeah", "yep", "yup", "correct", "right", "confirm",
    "go ahead", "do it", "that's right", "thats right", "sure",
    "affirmative", "approved", "ok", "okay", "y",
];

const NEGATIVE: &[&str] = &[
    "no", "nope", "wrong", "cancel", "wrong project", "not that",
    "different project", "nah", "n",
];

/// Check if a message is a simple confirmation or rejection.
/// Returns Some(true) for affirmative, Some(false) for negative, None for neither.
pub fn is_confirmation(msg: &str) -> Option<bool> {
    let lower = msg.to_lowercase().trim().to_string();

    // Check if the entire message (or very short message) is a confirmation
    if lower.len() > 40 {
        return None; // Too long to be a simple yes/no
    }

    if AFFIRMATIVE.iter().any(|kw| lower == *kw || lower.starts_with(&format!("{} ", kw)) || lower.starts_with(&format!("{}!", kw))) {
        return Some(true);
    }

    if NEGATIVE.iter().any(|kw| lower == *kw || lower.starts_with(&format!("{} ", kw)) || lower.starts_with(&format!("{}!", kw))) {
        return Some(false);
    }

    // Check if it's a positive number (for numbered list selection, 1-99)
    if let Ok(num) = lower.parse::<u32>() {
        if num >= 1 {
            return Some(true); // Will be handled separately as a numbered selection
        }
    }

    None
}

// ── Shared confirmation handler ─────────────────────────────────────

/// Handle a pending confirmation message. Returns Some(response_text) if handled, None if not.
/// This is the single source of truth for confirmation logic across chat, Telegram, and remote chat.
/// Uses conditional PATCH (status=eq.pending_confirmation) to prevent race conditions.
pub async fn handle_pending_confirmation(
    config: &supabase::SupabaseConfig,
    user_message: &str,
) -> Option<String> {
    // Only check if the message looks like a confirmation
    let confirmation = is_confirmation(user_message)?;

    // Fetch pending tasks
    let pending_tasks = match supabase::fetch_tasks(config, Some("pending_confirmation")).await {
        Ok(tasks) => tasks,
        Err(_) => return None, // Can't reach Supabase, fall through to normal processing
    };

    let Some(arr) = pending_tasks.as_array() else { return None; };
    if arr.is_empty() { return None; }

    // Sort by created_at descending to get the most recent pending task
    let mut sorted = arr.clone();
    sorted.sort_by(|a, b| {
        let ta = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        let tb = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        tb.cmp(ta)
    });

    let Some(most_recent) = sorted.first() else { return None; };
    let task_id = most_recent.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let task_title = most_recent.get("title").and_then(|v| v.as_str()).unwrap_or("untitled");

    if task_id.is_empty() { return None; }

    let response = if confirmation {
        let lower = user_message.trim().to_lowercase();
        if let Ok(num) = lower.parse::<usize>() {
            // Number selection: fetch projects and match
            let projects = supabase::fetch_projects(config).await.ok();
            if let Some(proj_arr) = projects.as_ref().and_then(|v| v.as_array()) {
                if num >= 1 && num <= proj_arr.len() {
                    let selected = &proj_arr[num - 1];
                    let proj_name = selected.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let mut updates = serde_json::json!({"status": "queued", "project": proj_name});
                    for field in &["repo_path", "repo_url", "preview_url"] {
                        if let Some(v) = selected.get(*field).filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                            updates[*field] = v.clone();
                        }
                    }
                    // Conditional PATCH: only update if still pending_confirmation (prevents race)
                    let _ = supabase::update_task_if_status(config, task_id, "pending_confirmation", &updates).await;
                    format!("Got it. Queued \"{}\" on {}. I'll get to it shortly.", task_title, proj_name)
                } else {
                    format!("That number's out of range. Pick a number between 1 and {}.", proj_arr.len())
                }
            } else {
                let _ = supabase::update_task_if_status(config, task_id, "pending_confirmation", &serde_json::json!({"status": "queued"})).await;
                format!("Got it, queued up \"{}\".", task_title)
            }
        } else {
            // Simple "yes" confirmation
            let _ = supabase::update_task_if_status(config, task_id, "pending_confirmation", &serde_json::json!({"status": "queued"})).await;
            format!("Got it, queued up \"{}\". I'll get to it shortly.", task_title)
        }
    } else {
        // Rejected
        let _ = supabase::update_task_if_status(config, task_id, "pending_confirmation", &serde_json::json!({"status": "failed"})).await;
        format!("Cancelled \"{}\". Send it again with @project-name to pick the right one.", task_title)
    };

    Some(response)
}

// ── Persistent session helpers ──────────────────────────────────────
// These replace the one-shot `chat_respond` flow with a persistent
// Claude Code CLI session that stays alive between messages.

/// Build the system prompt for Sam's chat session (board context + personality).
/// Called once when spawning a new session, then the session retains history.
#[tauri::command]
pub async fn chat_build_system_prompt(
    sb_state: tauri::State<'_, SupabaseState>,
    worker_state: tauri::State<'_, WorkerState>,
) -> Result<String, String> {
    let config = sb_state.get_config().await;
    if config.url.is_empty() {
        return Err("Supabase not configured".to_string());
    }
    let recent_chat = fetch_recent_chat(&config).await;
    let board_context = build_board_context(&config, &worker_state).await;
    let project_registry = build_project_registry(&config).await;
    // Build prompt without a specific user message (session init)
    Ok(build_system_prompt(&board_context, &project_registry, &recent_chat, "[Session starting. Wait for Matt's first message.]"))
}

/// Fast-path check: handle confirmations and status queries without Claude.
/// Returns Some(response) if handled, None if the message needs Claude.
#[derive(Serialize, Clone)]
pub struct FastPathResult {
    pub handled: bool,
    pub response: Option<String>,
    pub message_id: Option<String>,
}

#[tauri::command]
pub async fn chat_check_fast_path(
    user_message: String,
    sb_state: tauri::State<'_, SupabaseState>,
    worker_state: tauri::State<'_, WorkerState>,
) -> Result<FastPathResult, String> {
    let config = sb_state.get_config().await;
    if config.url.is_empty() {
        return Err("Supabase not configured".to_string());
    }

    // Save user message
    if let Err(e) = supabase::send_message(&config, &serde_json::json!({
        "role": "user",
        "content": &user_message,
        "conversation_id": DEFAULT_CONVERSATION_ID,
    })).await {
        log::warn!("[chat] Failed to save user message: {}", e);
    }

    // Check confirmation fast-path
    if let Some(response_text) = handle_pending_confirmation(&config, &user_message).await {
        let message_id = save_agent_message(&config, &response_text).await;
        return Ok(FastPathResult { handled: true, response: Some(response_text), message_id });
    }

    // Check status query fast-path
    if is_status_query(&user_message) {
        let board_context = build_board_context(&config, &worker_state).await;
        let response_text = build_status_response(&board_context);
        let message_id = save_agent_message(&config, &response_text).await;
        return Ok(FastPathResult { handled: true, response: Some(response_text), message_id });
    }

    // Not a fast-path, needs Claude
    Ok(FastPathResult { handled: false, response: None, message_id: None })
}

/// Process a completed Claude response: extract task creation blocks, create tasks,
/// save agent message to Supabase.
#[tauri::command]
pub async fn chat_process_response(
    response_text: String,
    user_message: String,
    sb_state: tauri::State<'_, SupabaseState>,
) -> Result<ChatResponse, String> {
    let config = sb_state.get_config().await;

    let (clean_text, task_requests) = parse_chat_response(&response_text);

    // Extract @ mentions for project tagging (single fetch, reused for both)
    let projects_for_mentions = supabase::fetch_projects(&config).await.ok().unwrap_or(serde_json::json!([]));
    let mentioned_projects = extract_project_mentions(&user_message, &projects_for_mentions);
    let projects = Some(projects_for_mentions);

    let mut created_tasks = Vec::new();
    for req in &task_requests {
        let mut enriched = req.clone();

        if let Some(mentioned) = mentioned_projects.first() {
            enriched["project"] = serde_json::Value::String(mentioned.clone());
            enriched["status"] = serde_json::Value::String("queued".to_string());
        } else {
            enriched["status"] = serde_json::Value::String("pending_confirmation".to_string());
        }

        // Backfill repo fields from project registry
        let project_name = enriched.get("project").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !project_name.is_empty() {
            if let Some(ref proj_arr) = projects {
                if let Some(arr) = proj_arr.as_array() {
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
            Err(e) => log::warn!("[chat] Failed to create task: {}", e),
        }
    }

    // Save agent response to Supabase
    let final_text = if clean_text.trim().is_empty() {
        response_text.trim().to_string()
    } else {
        clean_text.trim().to_string()
    };
    let message_id = save_agent_message(&config, &final_text).await;

    Ok(ChatResponse {
        content: final_text,
        message_id,
        created_tasks,
    })
}

/// Expand a raw problem description into a structured task title + description.
/// Uses a quick one-shot Claude call (max 1 turn, 30s timeout).
#[derive(Serialize, Clone)]
pub struct ExpandedTask {
    pub title: String,
    pub description: String,
    pub priority: String,
    pub task_type: String,
}

#[tauri::command]
pub async fn ai_expand_task(
    raw_input: String,
    project: String,
) -> Result<ExpandedTask, String> {
    // Escape curly braces in user input to prevent format!() panics
    let safe_input = raw_input.replace('{', "{{").replace('}', "}}");
    let safe_project = if project.is_empty() { "unspecified".to_string() } else { project.replace('{', "{{").replace('}', "}}") };

    let prompt = format!(
        r#"You are a task creation assistant. The user described a problem in plain language. Your job is to create a structured task from it.

User's input: "{safe_input}"
Project: {safe_project}

Respond with ONLY a JSON object (no markdown fences, no explanation) in this exact format:
{{"title": "Fix: concise bug title OR Feat: concise feature title", "description": "Detailed step-by-step instructions for a developer to fix this. Include what files to investigate, what the expected behavior should be, and how to verify the fix works.", "priority": "medium", "task_type": "code"}}

Rules for the title:
- Start with "Fix:" for bugs or "Feat:" for new features
- Keep it under 80 characters
- Be specific (not "fix the thing", but "Fix: slideshow navigation not advancing on click")

Rules for the description:
- Write clear, actionable instructions as if briefing a developer
- Include what the current (broken) behavior is
- Include what the expected behavior should be
- Suggest files or areas to investigate if obvious from context
- Keep it 2-5 sentences

Rules for priority:
- "critical" = app is broken/unusable for users right now
- "high" = significant functionality broken but workarounds exist
- "medium" = minor bug or improvement
- "low" = cosmetic or nice-to-have

Rules for task_type:
- "code" = requires code changes and a PR
- "research" = investigation/analysis only, no code changes"#,
        safe_input = safe_input,
        safe_project = safe_project,
    );

    let result = super::worker::run_claude_code_opts(".", &prompt, 1, 30).await?;

    // Parse the JSON response
    let cleaned = result.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|e| format!("Failed to parse AI response: {}. Raw: {}", e, &result[..result.len().min(200)]))?;

    Ok(ExpandedTask {
        title: parsed.get("title").and_then(|v| v.as_str()).unwrap_or(&raw_input).to_string(),
        description: parsed.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        priority: parsed.get("priority").and_then(|v| v.as_str()).unwrap_or("medium").to_string(),
        task_type: parsed.get("task_type").and_then(|v| v.as_str()).unwrap_or("code").to_string(),
    })
}

/// Save an agent message to Supabase and return the message ID.
async fn save_agent_message(config: &supabase::SupabaseConfig, content: &str) -> Option<String> {
    match supabase::send_message(config, &serde_json::json!({
        "role": "agent",
        "content": content,
        "conversation_id": DEFAULT_CONVERSATION_ID,
    })).await {
        Ok(result) => result.as_array()
            .and_then(|arr| arr.first())
            .and_then(|msg| msg.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string()),
        Err(e) => {
            log::warn!("[chat] Failed to save agent message: {}", e);
            None
        }
    }
}
