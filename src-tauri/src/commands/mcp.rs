use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::state::{StdioMcpSession, StdioMcpState};

// ── MCP HTTP Transport Helpers ──────────────────────────────────────

fn parse_mcp_headers(headers_json: Option<String>) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    if let Some(raw) = headers_json {
        if raw.trim().is_empty() {
            return Ok(headers);
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("Invalid MCP headers JSON: {}", e))?;
        let obj = parsed
            .as_object()
            .ok_or_else(|| "MCP headers must be a JSON object".to_string())?;

        for (k, v) in obj {
            let value = v
                .as_str()
                .ok_or_else(|| format!("MCP header '{}' must be a string", k))?;
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| format!("Invalid header name '{}': {}", k, e))?;
            let header_value = HeaderValue::from_str(value)
                .map_err(|e| format!("Invalid header value for '{}': {}", k, e))?;
            headers.insert(name, header_value);
        }
    }
    Ok(headers)
}

async fn mcp_post_jsonrpc(
    client: &reqwest::Client,
    url: &str,
    base_headers: &HeaderMap,
    session_id: Option<&str>,
    body: serde_json::Value,
) -> Result<(serde_json::Value, Option<String>), String> {
    let mut headers = base_headers.clone();
    if let Some(sid) = session_id {
        let sid_header = HeaderValue::from_str(sid)
            .map_err(|e| format!("Invalid MCP session id header: {}", e))?;
        headers.insert(HeaderName::from_static("mcp-session-id"), sid_header);
    }

    let response = client
        .post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("MCP HTTP request failed: {}", e))?;

    let response_session_id = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read MCP response body: {}", e))?;

    if !status.is_success() {
        return Err(format!("MCP HTTP error {}: {}", status, text));
    }

    let parsed: serde_json::Value = parse_mcp_json_response(&text, body.get("id"))?;

    if let Some(err) = parsed.get("error") {
        return Err(format!("MCP JSON-RPC error: {}", err));
    }

    Ok((parsed, response_session_id))
}

fn parse_mcp_json_response(
    text: &str,
    expected_id: Option<&serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Empty MCP response body".to_string());
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(expected) = expected_id {
            if parsed.get("id") == Some(expected) {
                return Ok(parsed);
            }
        } else {
            return Ok(parsed);
        }

        if parsed.get("result").is_some() || parsed.get("error").is_some() {
            return Ok(parsed);
        }
    }

    let mut frames: Vec<serde_json::Value> = Vec::new();

    for line in trimmed.lines() {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = data.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(payload) {
            if let Some(expected) = expected_id {
                if parsed.get("id") == Some(expected) {
                    return Ok(parsed);
                }
            } else {
                return Ok(parsed);
            }
            frames.push(parsed);
        }
    }

    if let Some(frame) = frames
        .into_iter()
        .find(|v| v.get("result").is_some() || v.get("error").is_some())
    {
        return Ok(frame);
    }

    Err("Invalid MCP JSON response".to_string())
}

fn push_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidate.is_empty() && !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

fn mcp_endpoint_candidates(server_url: &str) -> Vec<String> {
    let raw = server_url.trim();
    if raw.is_empty() {
        return vec![];
    }

    let mut candidates = Vec::new();
    let normalized = raw.trim_end_matches('/').to_string();
    push_candidate(&mut candidates, normalized.clone());

    if let Ok(url) = reqwest::Url::parse(raw) {
        let path = url.path().trim_end_matches('/');

        if path.is_empty() || path == "/" {
            for suffix in ["/mcp", "/rpc", "/api/mcp"] {
                let mut candidate = url.clone();
                candidate.set_path(suffix);
                push_candidate(
                    &mut candidates,
                    candidate.to_string().trim_end_matches('/').to_string(),
                );
            }
        } else if !path.ends_with("/mcp") {
            let mut candidate = url.clone();
            candidate.set_path(&format!("{}/mcp", path));
            push_candidate(
                &mut candidates,
                candidate.to_string().trim_end_matches('/').to_string(),
            );
        }
    }

    candidates
}

async fn mcp_try_initialize_at_endpoint(
    client: &reqwest::Client,
    server_url: &str,
    headers: &HeaderMap,
    protocol_version: &str,
) -> Result<Option<String>, String> {
    let init_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "bananacode-init",
        "method": "initialize",
        "params": {
            "protocolVersion": protocol_version,
            "capabilities": {},
            "clientInfo": {
                "name": "Banana Code",
                "version": "0.1.0"
            }
        }
    });

    let (_init_response, mut session_id) =
        mcp_post_jsonrpc(client, server_url, headers, None, init_body).await?;

    let initialized_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });

    if let Ok((_, sid)) =
        mcp_post_jsonrpc(client, server_url, headers, session_id.as_deref(), initialized_body)
            .await
    {
        if sid.is_some() {
            session_id = sid;
        }
    }

    Ok(session_id)
}

async fn mcp_initialize_session(
    server_url: &str,
    auth_token: Option<String>,
    headers_json: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<(reqwest::Client, HeaderMap, Option<String>, String), String> {
    let mut headers = parse_mcp_headers(headers_json)?;
    if let Some(token) = auth_token {
        if !token.trim().is_empty() {
            let auth = HeaderValue::from_str(&format!("Bearer {}", token.trim()))
                .map_err(|e| format!("Invalid MCP auth token: {}", e))?;
            headers.insert(AUTHORIZATION, auth);
        }
    }

    headers.insert(
        HeaderName::from_static("accept"),
        HeaderValue::from_static("application/json, text/event-stream"),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(
            timeout_ms.unwrap_or(20000),
        ))
        .build()
        .map_err(|e| format!("Failed to create MCP HTTP client: {}", e))?;

    let endpoints = mcp_endpoint_candidates(server_url);
    if endpoints.is_empty() {
        return Err("MCP server URL is empty".to_string());
    }

    let protocol_versions = ["2024-11-05", "2024-10-07"];
    let mut errors = Vec::new();

    for endpoint in endpoints {
        for protocol_version in protocol_versions {
            match mcp_try_initialize_at_endpoint(&client, &endpoint, &headers, protocol_version)
                .await
            {
                Ok(session_id) => {
                    return Ok((client, headers, session_id, endpoint));
                }
                Err(err) => {
                    errors.push(format!(
                        "endpoint={} protocolVersion={} error={}",
                        endpoint, protocol_version, err
                    ));
                }
            }
        }
    }

    Err(format!(
        "Failed to initialize MCP session. Tried endpoints and protocol versions: {}",
        errors.join(" | ")
    ))
}

// ── MCP HTTP Commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn mcp_list_tools(
    server_url: String,
    auth_token: Option<String>,
    headers_json: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<String, String> {
    let (client, headers, session_id, resolved_url) =
        mcp_initialize_session(&server_url, auth_token, headers_json, timeout_ms).await?;

    let tools_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "bananacode-tools-list",
        "method": "tools/list",
        "params": {}
    });

    let (resp, _) = mcp_post_jsonrpc(
        &client,
        &resolved_url,
        &headers,
        session_id.as_deref(),
        tools_body,
    )
    .await?;

    let tools = resp
        .get("result")
        .and_then(|r| r.get("tools"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    Ok(serde_json::json!({ "tools": tools }).to_string())
}

#[tauri::command]
pub async fn mcp_call_tool(
    server_url: String,
    tool_name: String,
    arguments_json: String,
    auth_token: Option<String>,
    headers_json: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<String, String> {
    let (client, headers, session_id, resolved_url) =
        mcp_initialize_session(&server_url, auth_token, headers_json, timeout_ms).await?;

    let args_value: serde_json::Value = if arguments_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&arguments_json)
            .map_err(|e| format!("Invalid MCP tool arguments JSON: {}", e))?
    };

    let call_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "bananacode-tools-call",
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": args_value
        }
    });

    let (resp, _) = mcp_post_jsonrpc(
        &client,
        &resolved_url,
        &headers,
        session_id.as_deref(),
        call_body,
    )
    .await?;

    Ok(resp
        .get("result")
        .cloned()
        .unwrap_or(resp)
        .to_string())
}

// ── Stdio MCP Server Management ─────────────────────────────────────

async fn stdio_mcp_send_request_inner(
    stdin_tx: &tokio::sync::mpsc::Sender<Vec<u8>>,
    pending: &Arc<
        tokio::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>,
    >,
    alive: &Arc<AtomicBool>,
    next_id: &Arc<AtomicU64>,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !alive.load(Ordering::Relaxed) {
        return Err("Stdio MCP server is not running".to_string());
    }

    let id = next_id.fetch_add(1, Ordering::Relaxed);
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });

    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut pend = pending.lock().await;
        pend.insert(id, tx);
    }

    let mut data = serde_json::to_string(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;
    data.push('\n');

    stdin_tx
        .send(data.into_bytes())
        .await
        .map_err(|_| "Failed to write to MCP server stdin".to_string())?;

    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(response)) => {
            if let Some(err) = response.get("error") {
                Err(format!("MCP JSON-RPC error: {}", err))
            } else {
                Ok(response)
            }
        }
        Ok(Err(_)) => Err("MCP response channel closed".to_string()),
        Err(_) => {
            let mut pend = pending.lock().await;
            pend.remove(&id);
            Err("MCP request timed out".to_string())
        }
    }
}

#[tauri::command]
pub async fn stdio_mcp_spawn(
    server_id: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    state: tauri::State<'_, StdioMcpState>,
) -> Result<(), String> {
    // Remove any existing dead session, reject if still alive
    {
        let mut sessions = state.sessions.lock().await;
        if let Some(existing) = sessions.get(&server_id) {
            if existing.alive.load(Ordering::Relaxed) {
                return Err(format!(
                    "Stdio MCP session already running: {}",
                    server_id
                ));
            }
            sessions.remove(&server_id);
        }
    }

    let full_command = if args.is_empty() {
        command.clone()
    } else {
        format!("{} {}", command, args.join(" "))
    };

    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.arg("/C").arg(&full_command);
        c
    } else {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-lc").arg(&full_command);
        c
    };

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    for (k, v) in &env {
        cmd.env(k, v);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn stdio MCP server: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout from MCP server".to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to capture stdin of MCP server".to_string())?;

    let alive = Arc::new(AtomicBool::new(true));
    let next_id = Arc::new(AtomicU64::new(1));
    let pending: Arc<
        tokio::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>,
    > = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    // Stdin writer task
    let alive_w = Arc::clone(&alive);
    tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        while let Some(data) = stdin_rx.recv().await {
            if !alive_w.load(Ordering::Relaxed) {
                break;
            }
            if stdin.write_all(&data).await.is_err() {
                break;
            }
            if stdin.flush().await.is_err() {
                break;
            }
        }
    });

    // Stdout reader task
    let alive_r = Arc::clone(&alive);
    let pending_r = Arc::clone(&pending);
    let server_id_r = server_id.clone();
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while alive_r.load(Ordering::Relaxed) {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<serde_json::Value>(&trimmed) {
                        Ok(msg) => {
                            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                                let mut pend = pending_r.lock().await;
                                if let Some(sender) = pend.remove(&id) {
                                    let _ = sender.send(msg);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Stdio MCP {} received non-JSON line: {} (error: {})",
                                server_id_r, trimmed, e
                            );
                        }
                    }
                }
                Ok(None) => {
                    alive_r.store(false, Ordering::Relaxed);
                    break;
                }
                Err(e) => {
                    eprintln!("Stdio MCP {} reader error: {}", server_id_r, e);
                    alive_r.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }

        let mut pend = pending_r.lock().await;
        for (_, sender) in pend.drain() {
            let _ = sender.send(
                serde_json::json!({"error": {"code": -1, "message": "MCP server process exited"}}),
            );
        }
    });

    // Stderr reader task
    if let Some(stderr) = child.stderr.take() {
        let server_id_e = server_id.clone();
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("Stdio MCP {} stderr: {}", server_id_e, line);
            }
        });
    }

    // Send initialize request
    let init_result = stdio_mcp_send_request_inner(
        &stdin_tx,
        &pending,
        &alive,
        &next_id,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "Banana Code",
                "version": "0.1.0"
            }
        }),
    )
    .await;

    match init_result {
        Ok(_) => {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            });
            let mut data = serde_json::to_string(&notification)
                .map_err(|e| format!("Failed to serialize notification: {}", e))?;
            data.push('\n');
            if stdin_tx.send(data.into_bytes()).await.is_err() {
                alive.store(false, Ordering::Relaxed);
                return Err(
                    "Failed to send initialized notification to MCP server".to_string(),
                );
            }

            let session = StdioMcpSession {
                alive: Arc::clone(&alive),
                next_id,
                stdin_tx: stdin_tx.clone(),
                pending: Arc::clone(&pending),
            };
            {
                let mut sessions = state.sessions.lock().await;
                sessions.insert(server_id.clone(), session);
            }
            Ok(())
        }
        Err(e) => {
            alive.store(false, Ordering::Relaxed);
            Err(format!("Failed to initialize stdio MCP server: {}", e))
        }
    }
}

#[tauri::command]
pub async fn stdio_mcp_stop(
    server_id: String,
    state: tauri::State<'_, StdioMcpState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.remove(&server_id) {
        session.alive.store(false, Ordering::Relaxed);
        Ok(())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn stdio_mcp_list_tools(
    server_id: String,
    state: tauri::State<'_, StdioMcpState>,
) -> Result<String, String> {
    let sessions = state.sessions.lock().await;
    let session = sessions
        .get(&server_id)
        .ok_or_else(|| format!("No stdio MCP session: {}", server_id))?;

    let response = stdio_mcp_send_request_inner(
        &session.stdin_tx,
        &session.pending,
        &session.alive,
        &session.next_id,
        "tools/list",
        serde_json::json!({}),
    )
    .await?;

    let tools = response
        .get("result")
        .and_then(|r| r.get("tools"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    Ok(serde_json::json!({ "tools": tools }).to_string())
}

#[tauri::command]
pub async fn stdio_mcp_call_tool(
    server_id: String,
    tool_name: String,
    arguments_json: String,
    state: tauri::State<'_, StdioMcpState>,
) -> Result<String, String> {
    let sessions = state.sessions.lock().await;
    let session = sessions
        .get(&server_id)
        .ok_or_else(|| format!("No stdio MCP session: {}", server_id))?;

    let args_value: serde_json::Value = if arguments_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&arguments_json)
            .map_err(|e| format!("Invalid MCP tool arguments JSON: {}", e))?
    };

    let response = stdio_mcp_send_request_inner(
        &session.stdin_tx,
        &session.pending,
        &session.alive,
        &session.next_id,
        "tools/call",
        serde_json::json!({
            "name": tool_name,
            "arguments": args_value,
        }),
    )
    .await?;

    Ok(response
        .get("result")
        .cloned()
        .unwrap_or(response)
        .to_string())
}

#[tauri::command]
pub async fn stdio_mcp_status(
    server_id: String,
    state: tauri::State<'_, StdioMcpState>,
) -> Result<String, String> {
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&server_id) {
        if session.alive.load(Ordering::Relaxed) {
            Ok("running".to_string())
        } else {
            Ok("stopped".to_string())
        }
    } else {
        Ok("stopped".to_string())
    }
}
