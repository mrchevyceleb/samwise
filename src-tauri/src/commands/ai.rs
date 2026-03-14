use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use tauri::Emitter;
use tauri::Manager;

// ── Event Payloads ──────────────────────────────────────────────────

#[derive(Clone, Serialize)]
struct AiStreamChunkPayload {
    request_id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct AiStreamDonePayload {
    request_id: String,
}

#[derive(Clone, Serialize)]
struct AiStreamErrorPayload {
    request_id: String,
    error: String,
}

// ── SSE Stream Helper ───────────────────────────────────────────────

/// Shared SSE line-parsing loop used by all streaming commands.
async fn stream_sse_response(
    app: &tauri::AppHandle,
    request_id: &str,
    response: reqwest::Response,
) -> Result<(), String> {
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(bytes) => {
                buffer.extend_from_slice(&bytes);

                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let mut line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();

                    if line_bytes.last() == Some(&b'\n') {
                        line_bytes.pop();
                    }
                    if line_bytes.last() == Some(&b'\r') {
                        line_bytes.pop();
                    }
                    if line_bytes.is_empty() {
                        continue;
                    }

                    let line = match String::from_utf8(line_bytes) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    if let Some(raw_data) = line.strip_prefix("data:") {
                        let data = raw_data.strip_prefix(' ').unwrap_or(raw_data);

                        if data.trim() == "[DONE]" {
                            let _ = app.emit(
                                "ai-stream-done",
                                AiStreamDonePayload {
                                    request_id: request_id.to_string(),
                                },
                            );
                            return Ok(());
                        }

                        let _ = app.emit(
                            "ai-stream-chunk",
                            AiStreamChunkPayload {
                                request_id: request_id.to_string(),
                                data: data.to_string(),
                            },
                        );
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Stream error: {}", e);
                let _ = app.emit(
                    "ai-stream-error",
                    AiStreamErrorPayload {
                        request_id: request_id.to_string(),
                        error: err_msg.clone(),
                    },
                );
                return Err(err_msg);
            }
        }
    }

    // Stream ended without [DONE] - still signal done
    let _ = app.emit(
        "ai-stream-done",
        AiStreamDonePayload {
            request_id: request_id.to_string(),
        },
    );

    Ok(())
}

// ── OpenRouter / OpenAI-compatible streaming ────────────────────────

#[tauri::command]
pub async fn ai_chat_stream(
    app: tauri::AppHandle,
    request_id: String,
    base_url: String,
    api_key: String,
    body_json: String,
) -> Result<(), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let api_key = api_key.trim().to_string();

    let body: serde_json::Value = serde_json::from_str(&body_json)
        .map_err(|e| format!("Invalid request body: {}", e))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "HTTP-Referer",
        HeaderValue::from_static("https://bananacode.app"),
    );
    headers.insert("X-Title", HeaderValue::from_static("Banana Code"));
    if !api_key.is_empty() && api_key != "lm-studio" {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| format!("Invalid API key format: {}", e))?,
        );
    }

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let err_msg = format!("Request failed: {}", e);
            let _ = app.emit(
                "ai-stream-error",
                AiStreamErrorPayload {
                    request_id: request_id.clone(),
                    error: err_msg.clone(),
                },
            );
            err_msg
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let err_msg = format!("API error {}: {}", status, body);
        let _ = app.emit(
            "ai-stream-error",
            AiStreamErrorPayload {
                request_id: request_id.clone(),
                error: err_msg.clone(),
            },
        );
        return Err(err_msg);
    }

    stream_sse_response(&app, &request_id, response).await
}

// ── Anthropic native SSE streaming ──────────────────────────────────

#[tauri::command]
pub async fn ai_chat_stream_anthropic(
    app: tauri::AppHandle,
    request_id: String,
    base_url: String,
    api_key: String,
    body_json: String,
) -> Result<(), String> {
    let url = format!("{}/messages", base_url.trim_end_matches('/'));
    let api_key = api_key.trim().to_string();

    let body: serde_json::Value = serde_json::from_str(&body_json)
        .map_err(|e| format!("Invalid request body: {}", e))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(&api_key)
            .map_err(|e| format!("Invalid API key format: {}", e))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let err_msg = format!("Request failed: {}", e);
            let _ = app.emit(
                "ai-stream-error",
                AiStreamErrorPayload {
                    request_id: request_id.clone(),
                    error: err_msg.clone(),
                },
            );
            err_msg
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let err_msg = format!("API error {}: {}", status, body);
        let _ = app.emit(
            "ai-stream-error",
            AiStreamErrorPayload {
                request_id: request_id.clone(),
                error: err_msg.clone(),
            },
        );
        return Err(err_msg);
    }

    stream_sse_response(&app, &request_id, response).await
}

// ── OpenAI Codex /responses endpoint ────────────────────────────────

#[tauri::command]
pub async fn ai_chat_stream_openai_codex(
    app: tauri::AppHandle,
    request_id: String,
    base_url: String,
    access_token: String,
    body_json: String,
    client_version: String,
) -> Result<(), String> {
    let base = base_url.trim_end_matches('/');
    let url = format!(
        "{}/responses?client_version={}",
        base,
        client_version.trim()
    );

    let body: serde_json::Value = serde_json::from_str(&body_json)
        .map_err(|e| format!("Invalid request body: {}", e))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", access_token.trim()))
            .map_err(|e| format!("Invalid access token format: {}", e))?,
    );
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("text/event-stream"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let err_msg = format!("Request failed: {}", e);
            let _ = app.emit(
                "ai-stream-error",
                AiStreamErrorPayload {
                    request_id: request_id.clone(),
                    error: err_msg.clone(),
                },
            );
            err_msg
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let err_msg = format!("API error {}: {}", status, body);
        let _ = app.emit(
            "ai-stream-error",
            AiStreamErrorPayload {
                request_id: request_id.clone(),
                error: err_msg.clone(),
            },
        );
        return Err(err_msg);
    }

    stream_sse_response(&app, &request_id, response).await
}

// ── Non-streaming AI completion (used by preview recovery) ──────────

#[tauri::command]
pub async fn ai_chat_complete(
    base_url: String,
    api_key: String,
    body_json: String,
    provider: String,
) -> Result<String, String> {
    let api_key = api_key.trim().to_string();

    let body: serde_json::Value = serde_json::from_str(&body_json)
        .map_err(|e| format!("Invalid request body: {}", e))?;

    let mut headers = HeaderMap::new();

    if provider == "anthropic" {
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&api_key)
                .map_err(|e| format!("Invalid API key format: {}", e))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static("2023-06-01"),
        );
    } else {
        // OpenRouter / OpenAI compatible
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_static("https://bananacode.app"),
        );
        headers.insert("X-Title", HeaderValue::from_static("Banana Code"));
        if !api_key.is_empty() && api_key != "lm-studio" {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| format!("Invalid API key format: {}", e))?,
            );
        }
    }

    let url = if provider == "anthropic" {
        format!("{}/messages", base_url.trim_end_matches('/'))
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    };

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI recovery request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("AI API error {}: {}", status, body));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read AI response: {}", e))
}

// ── Model Fetching ──────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_fetch_models(base_url: String, api_key: String) -> Result<String, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let api_key = api_key.trim().to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        "HTTP-Referer",
        HeaderValue::from_static("https://bananacode.app"),
    );
    headers.insert("X-Title", HeaderValue::from_static("Banana Code"));
    if !api_key.is_empty() && api_key != "lm-studio" {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| format!("Invalid API key format: {}", e))?,
        );
    }

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

// ── OAuth ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ai_exchange_openrouter_oauth_code(
    code: String,
    code_verifier: String,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let body = serde_json::json!({
        "code": code.trim(),
        "code_verifier": code_verifier.trim(),
        "code_challenge_method": "S256",
    });

    let response = client
        .post("https://openrouter.ai/api/v1/auth/keys")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenRouter OAuth exchange failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "OpenRouter OAuth exchange failed ({}): {}",
            status, text
        ));
    }

    Ok(text)
}

#[tauri::command]
pub async fn ai_openai_device_start(
    issuer: String,
    client_id: String,
) -> Result<String, String> {
    let issuer = issuer.trim().trim_end_matches('/').to_string();
    let client_id = client_id.trim().to_string();
    let url = format!("{}/api/accounts/deviceauth/usercode", issuer);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&serde_json::json!({ "client_id": client_id }))
        .send()
        .await
        .map_err(|e| format!("OpenAI device auth start failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "OpenAI device auth start failed ({}): {}",
            status, text
        ));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Invalid OpenAI device auth response: {}", e))?;
    let user_code = parsed
        .get("user_code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let device_auth_id = parsed
        .get("device_auth_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let interval = parsed
        .get("interval")
        .and_then(|v| v.as_i64())
        .unwrap_or(5);

    let out = serde_json::json!({
        "issuer": issuer,
        "client_id": client_id,
        "verification_url": format!("{}/codex/device", issuer),
        "user_code": user_code,
        "device_auth_id": device_auth_id,
        "interval": if interval > 0 { interval } else { 5 },
    });
    Ok(out.to_string())
}

#[tauri::command]
pub async fn ai_openai_device_poll(
    issuer: String,
    device_auth_id: String,
    user_code: String,
) -> Result<String, String> {
    let issuer = issuer.trim().trim_end_matches('/').to_string();
    let url = format!("{}/api/accounts/deviceauth/token", issuer);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "device_auth_id": device_auth_id.trim(),
            "user_code": user_code.trim(),
        }))
        .send()
        .await
        .map_err(|e| format!("OpenAI device auth poll failed: {}", e))?;

    if response.status().as_u16() == 403 || response.status().as_u16() == 404 {
        return Ok(String::new());
    }

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "OpenAI device auth poll failed ({}): {}",
            status, text
        ));
    }

    Ok(text)
}

#[tauri::command]
pub async fn ai_openai_exchange_authorization_code(
    issuer: String,
    client_id: String,
    authorization_code: String,
    code_verifier: String,
    redirect_uri: String,
) -> Result<String, String> {
    let issuer = issuer.trim().trim_end_matches('/').to_string();
    let url = format!("{}/oauth/token", issuer);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let form: HashMap<&str, String> = HashMap::from([
        ("grant_type", "authorization_code".to_string()),
        ("code", authorization_code.trim().to_string()),
        ("redirect_uri", redirect_uri.trim().to_string()),
        ("client_id", client_id.trim().to_string()),
        ("code_verifier", code_verifier.trim().to_string()),
    ]);

    let response = client
        .post(&url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("OpenAI OAuth exchange failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "OpenAI OAuth exchange failed ({}): {}",
            status, text
        ));
    }

    Ok(text)
}

#[tauri::command]
pub async fn ai_openai_refresh_oauth_token(
    issuer: String,
    client_id: String,
    refresh_token: String,
) -> Result<String, String> {
    let issuer = issuer.trim().trim_end_matches('/').to_string();
    let url = format!("{}/oauth/token", issuer);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let form: HashMap<&str, String> = HashMap::from([
        ("client_id", client_id.trim().to_string()),
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.trim().to_string()),
    ]);

    let response = client
        .post(&url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("OpenAI OAuth refresh failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "OpenAI OAuth refresh failed ({}): {}",
            status, text
        ));
    }

    Ok(text)
}

// ── Command Execution ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[tauri::command]
pub async fn run_command_sync(
    command: String,
    cwd: String,
    timeout_ms: Option<u64>,
) -> Result<CommandResult, String> {
    let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30000));

    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = tokio::process::Command::new("powershell.exe");
        c.arg("-Command").arg(&command);
        c
    } else {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-c").arg(&command);
        c
    };

    cmd.current_dir(&cwd);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.kill_on_drop(true);

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let wait_fut = child.wait_with_output();

    match tokio::time::timeout(timeout, wait_fut).await {
        Ok(result) => {
            let output = result.map_err(|e| format!("Failed to wait for command: {}", e))?;
            Ok(CommandResult {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
        Err(_) => Err("Command timed out".to_string()),
    }
}

// ── Chat Session Persistence ────────────────────────────────────────

fn sanitize_session_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err("Session ID cannot be empty".to_string());
    }
    if id.len() > 128 {
        return Err("Session ID too long".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid session ID: {}", id));
    }
    Ok(id.to_string())
}

#[tauri::command]
pub fn save_chat_session(
    app: tauri::AppHandle,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let session_id = sanitize_session_id(&session_id)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let chats_dir = app_data_dir.join("chats");
    fs::create_dir_all(&chats_dir)
        .map_err(|e| format!("Failed to create chats dir: {}", e))?;

    let file_path = chats_dir.join(format!("{}.json", session_id));
    fs::write(&file_path, data).map_err(|e| format!("Failed to write chat session: {}", e))
}

#[tauri::command]
pub fn load_chat_session(app: tauri::AppHandle, session_id: String) -> Result<String, String> {
    let session_id = sanitize_session_id(&session_id)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let file_path = app_data_dir
        .join("chats")
        .join(format!("{}.json", session_id));

    if !file_path.exists() {
        return Err(format!("Chat session not found: {}", session_id));
    }

    fs::read_to_string(&file_path).map_err(|e| format!("Failed to read chat session: {}", e))
}

#[tauri::command]
pub fn list_chat_sessions(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let chats_dir = app_data_dir.join("chats");

    if !chats_dir.exists() {
        return Ok(vec![]);
    }

    let entries =
        fs::read_dir(&chats_dir).map_err(|e| format!("Failed to read chats dir: {}", e))?;

    let mut session_ids: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    session_ids.sort();
    Ok(session_ids)
}

#[tauri::command]
pub fn delete_chat_session(app: tauri::AppHandle, session_id: String) -> Result<(), String> {
    let session_id = sanitize_session_id(&session_id)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let file_path = app_data_dir
        .join("chats")
        .join(format!("{}.json", session_id));

    if !file_path.exists() {
        return Err(format!("Chat session not found: {}", session_id));
    }

    fs::remove_file(&file_path).map_err(|e| format!("Failed to delete chat session: {}", e))
}
