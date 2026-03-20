use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── Config State ────────────────────────────────────────────────────

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
    pub service_role_key: Option<String>,
}

#[derive(Default)]
pub struct SupabaseState {
    pub config: Arc<RwLock<SupabaseConfig>>,
}

impl SupabaseState {
    pub async fn get_config(&self) -> SupabaseConfig {
        self.config.read().await.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════
// PUBLIC INTERNAL API (callable from worker.rs and other Rust code)
// These take &SupabaseConfig directly, no Tauri state needed.
// ═══════════════════════════════════════════════════════════════════

fn build_client(config: &SupabaseConfig) -> Result<reqwest::Client, String> {
    let key = config.service_role_key.as_deref().unwrap_or(&config.anon_key);
    let mut headers = HeaderMap::new();
    headers.insert("apikey", HeaderValue::from_str(key).map_err(|e| e.to_string())?);
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", key)).map_err(|e| e.to_string())?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("Prefer", HeaderValue::from_static("return=representation"));
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

fn rest_url(config: &SupabaseConfig, table: &str) -> String {
    format!("{}/rest/v1/{}", config.url, table)
}

async fn handle_response(resp: reqwest::Response) -> Result<Value, String> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Supabase error ({}): {}", status, body));
    }
    resp.json::<Value>().await.map_err(|e| format!("Failed to parse response: {}", e))
}

// ── Tasks (internal) ────────────────────────────────────────────────

pub async fn fetch_tasks(config: &SupabaseConfig, status_filter: Option<&str>) -> Result<Value, String> {
    let client = build_client(config)?;
    let mut url = format!("{}?order=priority.asc,created_at.asc", rest_url(config, "ae_tasks"));
    if let Some(status) = status_filter {
        url.push_str(&format!("&status=eq.{}", status));
    }
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

pub async fn create_task(config: &SupabaseConfig, task: &Value) -> Result<Value, String> {
    let client = build_client(config)?;
    handle_response(client.post(&rest_url(config, "ae_tasks")).json(task).send().await.map_err(|e| e.to_string())?).await
}

pub async fn update_task(config: &SupabaseConfig, id: &str, updates: &Value) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?id=eq.{}", rest_url(config, "ae_tasks"), id);
    handle_response(client.patch(&url).json(updates).send().await.map_err(|e| e.to_string())?).await
}

pub async fn claim_task(config: &SupabaseConfig, task_id: &str, worker_id: &str) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?id=eq.{}&status=eq.queued", rest_url(config, "ae_tasks"), task_id);
    let now = chrono::Utc::now().to_rfc3339();
    let body = serde_json::json!({
        "status": "in_progress",
        "worker_id": worker_id,
        "claimed_at": now,
        "updated_at": now,
    });
    let result = handle_response(client.patch(&url).json(&body).send().await.map_err(|e| e.to_string())?).await?;
    if let Some(arr) = result.as_array() {
        if arr.is_empty() {
            return Err("Task is no longer queued (already claimed or changed)".to_string());
        }
    }
    Ok(result)
}

// ── Comments (internal) ─────────────────────────────────────────────

pub async fn fetch_comments(config: &SupabaseConfig, task_id: &str) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?task_id=eq.{}&order=created_at.asc", rest_url(config, "ae_comments"), task_id);
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

pub async fn post_comment(config: &SupabaseConfig, comment: &Value) -> Result<Value, String> {
    let client = build_client(config)?;
    handle_response(client.post(&rest_url(config, "ae_comments")).json(comment).send().await.map_err(|e| e.to_string())?).await
}

// ── Messages (internal) ─────────────────────────────────────────────

pub async fn fetch_messages(config: &SupabaseConfig) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?order=created_at.asc&limit=200", rest_url(config, "ae_messages"));
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

pub async fn send_message(config: &SupabaseConfig, message: &Value) -> Result<Value, String> {
    let client = build_client(config)?;
    handle_response(client.post(&rest_url(config, "ae_messages")).json(message).send().await.map_err(|e| e.to_string())?).await
}

// ── Workers (internal) ──────────────────────────────────────────────

pub async fn worker_heartbeat(config: &SupabaseConfig, machine_name: &str) -> Result<Value, String> {
    let client = build_client(config)?;
    let now = chrono::Utc::now().to_rfc3339();
    // id = machine_name (text PK has no default, machine_name is the stable identity)
    // on_conflict targets the unique machine_name column for upsert
    let body = serde_json::json!({
        "id": machine_name,
        "machine_name": machine_name,
        "status": "online",
        "last_heartbeat": now,
    });
    let resp = client
        .post(&format!("{}?on_conflict=machine_name", rest_url(config, "ae_workers")))
        .header("Prefer", "resolution=merge-duplicates,return=representation")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    handle_response(resp).await
}

pub async fn update_cron(config: &SupabaseConfig, id: &str, updates: &Value) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?id=eq.{}", rest_url(config, "ae_crons"), id);
    handle_response(client.patch(&url).json(updates).send().await.map_err(|e| e.to_string())?).await
}

pub async fn worker_offline(config: &SupabaseConfig, machine_name: &str) -> Result<(), String> {
    let client = build_client(config)?;
    let url = format!("{}?machine_name=eq.{}", rest_url(config, "ae_workers"), machine_name);
    let body = serde_json::json!({
        "status": "offline",
        "last_heartbeat": chrono::Utc::now().to_rfc3339(),
    });
    let resp = client.patch(&url).json(&body).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to set offline: {}", body));
    }
    Ok(())
}

// ── Triggers (internal) ─────────────────────────────────────────────

pub async fn fetch_triggers(config: &SupabaseConfig) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?order=created_at.asc", rest_url(config, "ae_triggers"));
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

pub async fn update_trigger(config: &SupabaseConfig, id: &str, updates: &Value) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?id=eq.{}", rest_url(config, "ae_triggers"), id);
    handle_response(client.patch(&url).json(updates).send().await.map_err(|e| e.to_string())?).await
}

// ── Trigger Events (internal) ───────────────────────────────────────

pub async fn fetch_trigger_events(config: &SupabaseConfig, trigger_id: &str) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?trigger_id=eq.{}&processed=eq.false&order=created_at.asc&limit=10",
        rest_url(config, "ae_trigger_events"), trigger_id);
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

pub async fn mark_trigger_event_processed(config: &SupabaseConfig, event_id: &str) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?id=eq.{}", rest_url(config, "ae_trigger_events"), event_id);
    let body = serde_json::json!({ "processed": true });
    handle_response(client.patch(&url).json(&body).send().await.map_err(|e| e.to_string())?).await
}

// ── Storage (internal) ──────────────────────────────────────────────

/// Upload a file to Supabase Storage and return the public URL.
pub async fn upload_to_storage(config: &SupabaseConfig, bucket: &str, path: &str, file_path: &str) -> Result<String, String> {
    let key = config.service_role_key.as_deref().unwrap_or(&config.anon_key);
    let file_bytes = tokio::fs::read(file_path).await.map_err(|e| format!("Failed to read file: {}", e))?;

    let client = reqwest::Client::new();
    let url = format!("{}/storage/v1/object/{}/{}", config.url, bucket, path);

    let resp = client.post(&url)
        .header("apikey", key)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "image/png")
        .header("x-upsert", "true")
        .body(file_bytes)
        .send()
        .await
        .map_err(|e| format!("Storage upload failed: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Storage upload error: {}", body));
    }

    // Return public URL
    Ok(format!("{}/storage/v1/object/public/{}/{}", config.url, bucket, path))
}

// ── Crons (internal) ────────────────────────────────────────────────

pub async fn fetch_crons(config: &SupabaseConfig) -> Result<Value, String> {
    let client = build_client(config)?;
    let url = format!("{}?order=created_at.asc", rest_url(config, "ae_crons"));
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

// ═══════════════════════════════════════════════════════════════════
// TAURI COMMAND WRAPPERS (thin wrappers that read config from state)
// ═══════════════════════════════════════════════════════════════════

// ── Config Commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_set_config(
    url: String,
    anon_key: String,
    service_role_key: Option<String>,
    state: tauri::State<'_, SupabaseState>,
) -> Result<(), String> {
    let mut config = state.config.write().await;
    config.url = url;
    config.anon_key = anon_key;
    config.service_role_key = service_role_key;
    Ok(())
}

#[tauri::command]
pub async fn supabase_get_config(
    state: tauri::State<'_, SupabaseState>,
) -> Result<SupabaseConfig, String> {
    Ok(state.get_config().await)
}

#[tauri::command]
pub async fn supabase_test_connection(
    state: tauri::State<'_, SupabaseState>,
) -> Result<String, String> {
    let config = state.get_config().await;
    if config.url.is_empty() || config.anon_key.is_empty() {
        return Err("Supabase URL and anon key are required".to_string());
    }
    let client = build_client(&config)?;
    let url = format!("{}/rest/v1/ae_tasks?select=id&limit=1", config.url);
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok("Connected successfully".to_string())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Connection failed: {}", body))
    }
}

#[tauri::command]
pub async fn supabase_load_doppler(
    state: tauri::State<'_, SupabaseState>,
) -> Result<SupabaseConfig, String> {
    let output = tokio::process::Command::new("doppler")
        .args(["secrets", "download", "--project", "agent-one", "--config", "prd", "--no-file", "--format", "json"])
        .output()
        .await
        .map_err(|e| format!("Failed to run doppler: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Doppler failed: {}", stderr.trim()));
    }

    let secrets: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse Doppler output: {}", e))?;

    let new_config = SupabaseConfig {
        url: secrets.get("SUPABASE_URL").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        anon_key: secrets.get("SUPABASE_ANON_KEY").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        service_role_key: secrets.get("SUPABASE_SERVICE_ROLE_KEY").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    let mut config = state.config.write().await;
    *config = new_config.clone();
    Ok(new_config)
}

// ── Task Commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_fetch_tasks(status_filter: Option<String>, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    fetch_tasks(&config, status_filter.as_deref()).await
}

#[tauri::command]
pub async fn supabase_create_task(task: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    create_task(&config, &task).await
}

#[tauri::command]
pub async fn supabase_update_task(id: String, updates: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    update_task(&config, &id, &updates).await
}

#[tauri::command]
pub async fn supabase_delete_task(id: String, state: tauri::State<'_, SupabaseState>) -> Result<(), String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    let url = format!("{}?id=eq.{}", rest_url(&config, "ae_tasks"), id);
    let resp = client.delete(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Delete failed: {}", body));
    }
    Ok(())
}

#[tauri::command]
pub async fn supabase_claim_task(task_id: String, worker_id: String, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    claim_task(&config, &task_id, &worker_id).await
}

// ── Comment Commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_fetch_comments(task_id: String, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    fetch_comments(&config, &task_id).await
}

#[tauri::command]
pub async fn supabase_post_comment(comment: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    post_comment(&config, &comment).await
}

#[tauri::command]
pub async fn supabase_delete_comment(comment_id: String, state: tauri::State<'_, SupabaseState>) -> Result<(), String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    let url = format!("{}?id=eq.{}", rest_url(&config, "ae_comments"), comment_id);
    let resp = client.delete(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Delete failed: {}", body));
    }
    Ok(())
}

// ── Message Commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_fetch_messages(state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    fetch_messages(&config).await
}

#[tauri::command]
pub async fn supabase_send_message(message: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    send_message(&config, &message).await
}

// ── Cron Commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_fetch_crons(state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    fetch_crons(&config).await
}

#[tauri::command]
pub async fn supabase_create_cron(cron: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    handle_response(client.post(&rest_url(&config, "ae_crons")).json(&cron).send().await.map_err(|e| e.to_string())?).await
}

#[tauri::command]
pub async fn supabase_update_cron(id: String, updates: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    let url = format!("{}?id=eq.{}", rest_url(&config, "ae_crons"), id);
    handle_response(client.patch(&url).json(&updates).send().await.map_err(|e| e.to_string())?).await
}

// ── Trigger Commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_fetch_triggers(state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    let url = format!("{}?order=created_at.asc", rest_url(&config, "ae_triggers"));
    handle_response(client.get(&url).send().await.map_err(|e| e.to_string())?).await
}

#[tauri::command]
pub async fn supabase_create_trigger(trigger: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    handle_response(client.post(&rest_url(&config, "ae_triggers")).json(&trigger).send().await.map_err(|e| e.to_string())?).await
}

#[tauri::command]
pub async fn supabase_update_trigger(id: String, updates: Value, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    let client = build_client(&config)?;
    let url = format!("{}?id=eq.{}", rest_url(&config, "ae_triggers"), id);
    handle_response(client.patch(&url).json(&updates).send().await.map_err(|e| e.to_string())?).await
}

// ── Worker Commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn supabase_worker_heartbeat(machine_name: String, state: tauri::State<'_, SupabaseState>) -> Result<Value, String> {
    let config = state.get_config().await;
    worker_heartbeat(&config, &machine_name).await
}

#[tauri::command]
pub async fn supabase_worker_offline(machine_name: String, state: tauri::State<'_, SupabaseState>) -> Result<(), String> {
    let config = state.get_config().await;
    worker_offline(&config, &machine_name).await
}
