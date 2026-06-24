use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tauri::{Emitter, Manager};
use crate::process::cmd;

// ── LLM Proxy ────────────────────────────────────────────────────────

/// When set, Claude Code routes through a local LiteLLM proxy instead of
/// hitting Anthropic directly. The proxy translates Anthropic Messages API
/// format to OpenAI/Fireworks format, so Claude Code's agent loop (tool use,
/// streaming, etc.) works unchanged with any OpenAI-compatible backend.
///
/// Configuration is read from settings.json (set via the Settings UI).
/// Keys: `llmProxyBaseUrl`, `llmProxyApiKey`.
pub struct LlmProxyConfig {
    pub base_url: String,
    pub api_key: String,
}

impl LlmProxyConfig {
    /// Load proxy config from settings.json. Returns None if proxy is not
    /// configured (empty base_url), which means "use Anthropic directly."
    pub fn load(app: &tauri::AppHandle) -> Option<Self> {
        let data_dir = app.path().app_data_dir().ok()?;
        let path = data_dir.join("settings.json");
        let raw = std::fs::read_to_string(&path).ok()?;
        let val: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let base_url = val.get("llmProxyBaseUrl")?.as_str()?.trim().to_string();
        if base_url.is_empty() {
            return None;
        }
        let api_key = val
            .get("llmProxyApiKey")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        Some(LlmProxyConfig { base_url, api_key })
    }

    /// Load proxy config from settings.json (async version for tokio callers).
    pub async fn load_async(app: &tauri::AppHandle) -> Option<Self> {
        let data_dir = app.path().app_data_dir().ok()?;
        let path = data_dir.join("settings.json");
        let raw = tokio::fs::read_to_string(&path).await.ok()?;
        let val: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let base_url = val.get("llmProxyBaseUrl")?.as_str()?.trim().to_string();
        if base_url.is_empty() {
            return None;
        }
        let api_key = val
            .get("llmProxyApiKey")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        Some(LlmProxyConfig { base_url, api_key })
    }

    /// Load proxy config from settings.json read as a raw JSON string.
    /// Used by worker.rs which already has the settings blob loaded.
    pub fn from_json(raw: &str) -> Option<Self> {
        let val: serde_json::Value = serde_json::from_str(raw).ok()?;
        let base_url = val.get("llmProxyBaseUrl")?.as_str()?.trim().to_string();
        if base_url.is_empty() {
            return None;
        }
        let api_key = val
            .get("llmProxyApiKey")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        Some(LlmProxyConfig { base_url, api_key })
    }
}

/// Inject proxy env vars into a Command so Claude Code routes through
/// the LiteLLM proxy instead of Anthropic directly. No-op if proxy is None.
pub fn inject_proxy_env(
    cmd: &mut std::process::Command,
    proxy: &Option<LlmProxyConfig>,
) {
    if let Some(p) = proxy {
        cmd.env("ANTHROPIC_BASE_URL", &p.base_url);
        cmd.env("ANTHROPIC_API_KEY", &p.api_key);
    } else {
        strip_direct_oauth_blockers(cmd);
    }
}

/// Same as inject_proxy_env but for tokio::process::Command.
pub fn inject_proxy_env_async(
    cmd: &mut tokio::process::Command,
    proxy: &Option<LlmProxyConfig>,
) {
    if let Some(p) = proxy {
        cmd.env("ANTHROPIC_BASE_URL", &p.base_url);
        cmd.env("ANTHROPIC_API_KEY", &p.api_key);
    } else {
        strip_direct_oauth_blockers_async(cmd);
    }
}

/// Env vars from the old proxy/API-key setup force Claude Code away from
/// OAuth subscription auth. When Sam is running direct Anthropic OAuth, scrub
/// them from every child process so stale systemd or shell env cannot recreate
/// the 401 "not logged in" outage.
const DIRECT_OAUTH_ENV_BLOCKERS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_SIMPLE",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "AUTOSAM_DEFAULT_MODEL",
];

fn strip_direct_oauth_blockers(cmd: &mut std::process::Command) {
    for key in DIRECT_OAUTH_ENV_BLOCKERS {
        cmd.env_remove(key);
    }
}

fn strip_direct_oauth_blockers_async(cmd: &mut tokio::process::Command) {
    for key in DIRECT_OAUTH_ENV_BLOCKERS {
        cmd.env_remove(key);
    }
}

// ── State ────────────────────────────────────────────────────────────

pub struct ClaudeCodeProcess {
    pub child: std::process::Child,
    pub stdin: Option<std::process::ChildStdin>,
    pub alive: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct ClaudeCodeState {
    pub processes: Arc<parking_lot::Mutex<HashMap<String, ClaudeCodeProcess>>>,
}

// ── Event Payloads ───────────────────────────────────────────────────

#[derive(Clone, Serialize)]
struct ClaudeCodeOutputPayload {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct ClaudeCodeClosedPayload {
    id: String,
    exit_code: Option<i32>,
}

// ── Helpers ──────────────────────────────────────────────────────────

/// The Claude model Sam uses for coding tasks. Opus 4.8 is the primary.
/// The worker auto-detects model unavailability and retries with the fallback.
pub const CLAUDE_MODEL: &str = "claude-opus-4-8";
pub const CLAUDE_MODEL_FALLBACK: &str = "claude-opus-4-20250514";

/// Reasoning effort Sam runs with on every Claude Code spawn. Opus 4.8+
/// use adaptive thinking and ignore the legacy MAX_THINKING_TOKENS budget; the live
/// lever is the CLI `--effort` flag (low|medium|high|xhigh|max). "xhigh" is the
/// recommended tier for coding/agentic work. Single source of truth alongside CLAUDE_MODEL.
pub const CLAUDE_EFFORT: &str = "xhigh";

/// Returns (executable, prefix_args) for spawning the Claude CLI.
///
/// On Windows with an npm install, we bypass `claude.cmd` and invoke `node cli.js`
/// directly. `claude.cmd -> cmd.exe /c -> node cli.js` breaks stdin inheritance when
/// combined with CREATE_NO_WINDOW, so the extra `cmd.exe` layer has to go.
pub fn find_claude_command() -> (String, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let local_bin = format!("{}\\.local\\bin\\claude.exe", home);
        if std::path::Path::new(&local_bin).exists() {
            return (local_bin, vec![]);
        }
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let cli_js = format!("{}\\npm\\node_modules\\@anthropic-ai\\claude-code\\cli.js", appdata);
        if std::path::Path::new(&cli_js).exists() {
            return ("node".to_string(), vec![cli_js]);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let local_bin = format!("{}/.local/bin/claude", home);
            if std::path::Path::new(&local_bin).exists() {
                return (local_bin, vec![]);
            }
        }
    }
    // Last resort: resolve via PATH.
    if let Ok(path) = which::which("claude") {
        return (path.to_string_lossy().into_owned(), vec![]);
    }
    ("claude".to_string(), vec![])
}

const CLAUDE_CREDENTIALS_FILE: &str = ".credentials.json";

fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn canonical_claude_credentials_path() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".claude").join(CLAUDE_CREDENTIALS_FILE))
}

fn configured_claude_credentials_path() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .map(|dir| dir.join(CLAUDE_CREDENTIALS_FILE))
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

fn credential_sync_lock() -> &'static StdMutex<()> {
    static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| StdMutex::new(()))
}

fn credentials_need_sync(source: &Path, target: &Path, force: bool) -> Result<bool, String> {
    if force || !target.exists() {
        return Ok(true);
    }
    let source_bytes = std::fs::read(source)
        .map_err(|e| format!("read source Claude credentials: {}", e))?;
    let target_bytes = std::fs::read(target)
        .map_err(|e| format!("read worker Claude credentials: {}", e))?;
    Ok(source_bytes != target_bytes)
}

fn atomic_copy_credentials(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("Claude credentials target has no parent: {}", target.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("create Claude config dir {}: {}", parent.display(), e))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp = parent.join(format!(
        ".{}.tmp-{}-{}",
        CLAUDE_CREDENTIALS_FILE,
        std::process::id(),
        nonce
    ));
    std::fs::copy(source, &tmp).map_err(|e| {
        format!(
            "copy Claude credentials {} -> {}: {}",
            source.display(),
            tmp.display(),
            e
        )
    })?;
    lock_down_credentials_permissions(&tmp);
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!(
            "replace Claude credentials {} -> {}: {}",
            tmp.display(),
            target.display(),
            e
        )
    })?;
    lock_down_credentials_permissions(target);
    Ok(())
}

#[cfg(unix)]
fn lock_down_credentials_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn lock_down_credentials_permissions(_path: &Path) {}

/// Keep AutoSam's isolated Claude config fresh from the canonical Claude Code
/// login without pointing the worker directly at Matt's live config dir. This
/// fixes the stale-copy OAuth 401 outage while preserving the separate
/// `CLAUDE_CONFIG_DIR` the service already uses.
pub fn sync_claude_oauth_credentials_if_needed(force: bool) -> Result<bool, String> {
    let _guard = credential_sync_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(target) = configured_claude_credentials_path() else {
        return Ok(false);
    };
    let Some(source) = canonical_claude_credentials_path() else {
        return Ok(false);
    };
    if same_path(&source, &target) || !source.exists() {
        return Ok(false);
    }
    if !credentials_need_sync(&source, &target, force)? {
        return Ok(false);
    }
    atomic_copy_credentials(&source, &target)?;
    Ok(true)
}

pub async fn sync_claude_oauth_credentials_if_needed_async(force: bool) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || sync_claude_oauth_credentials_if_needed(force))
        .await
        .map_err(|e| format!("join Claude credentials sync task: {}", e))?
}

// ── Commands ─────────────────────────────────────────────────────────

/// Spawn a persistent Claude Code process with stream-json on stdin/stdout.
#[tauri::command]
pub fn spawn_claude_code(
    id: String,
    cwd: String,
    args: Vec<String>,
    state: tauri::State<'_, ClaudeCodeState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Kill any existing process for this session
    {
        let mut processes = state.processes.lock();
        if let Some(mut old) = processes.remove(&id) {
            old.alive.store(false, Ordering::Relaxed);
            drop(old.stdin.take());
            let _ = old.child.kill();
            let _ = old.child.wait();
        }
    }

    let (claude_exe, prefix_args) = find_claude_command();
    let mut command = cmd(&claude_exe);
    for arg in &prefix_args {
        command.arg(arg);
    }

    // Base args for persistent stream-json mode
    command.arg("-p")
        .arg("--output-format").arg("stream-json")
        .arg("--input-format").arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--dangerously-skip-permissions")
        .arg("--model").arg(CLAUDE_MODEL)
        .arg("--effort").arg(CLAUDE_EFFORT);

    // Inject LLM proxy env vars if configured. In direct OAuth mode, refresh
    // AutoSam's isolated Claude credentials from the canonical Claude Code
    // login before spawning so the worker cannot drift into stale 401s.
    let proxy = LlmProxyConfig::load(&app);
    if proxy.is_none() {
        match sync_claude_oauth_credentials_if_needed(false) {
            Ok(true) => log::info!("[claude-code] refreshed worker Claude OAuth credentials before spawn"),
            Ok(false) => {}
            Err(e) => log::warn!("[claude-code] could not refresh worker Claude OAuth credentials: {}", e),
        }
    }
    inject_proxy_env(&mut command, &proxy);

    // Add extra args from the frontend (e.g. --resume). Note: --model is already
    // pinned above to CLAUDE_MODEL; frontend args take precedence if they set it again.
    for arg in &args {
        command.arg(arg);
    }

    // Set working directory
    let cwd_path = PathBuf::from(&cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        command.current_dir(&cwd_path);
    }

    command.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // CREATE_NO_WINDOW is already set by process::cmd() on Windows

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn claude: {}", e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    let alive = Arc::new(AtomicBool::new(true));
    let alive_stdout = Arc::clone(&alive);
    let alive_stderr = Arc::clone(&alive);

    let process = ClaudeCodeProcess {
        child,
        stdin,
        alive,
    };

    let state_arc = Arc::clone(&state.processes);

    {
        let mut processes = state.processes.lock();
        processes.insert(id.clone(), process);
    }

    // Spawn stdout reader thread
    if let Some(stdout) = stdout {
        let reader_id = id.clone();
        let app_handle = app.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if !alive_stdout.load(Ordering::Relaxed) {
                    break;
                }
                match line {
                    Ok(data) => {
                        if !data.is_empty() {
                            let _ = app_handle.emit(
                                "claude-code-output",
                                ClaudeCodeOutputPayload {
                                    id: reader_id.clone(),
                                    data,
                                },
                            );
                        }
                    }
                    Err(_) => break,
                }
            }
            // Process finished. Reap child and report exit status.
            let exit_code = if let Some(mut proc) = state_arc.lock().remove(&reader_id) {
                proc.child.wait().ok().and_then(|s| s.code())
            } else {
                None
            };
            let _ = app_handle.emit(
                "claude-code-closed",
                ClaudeCodeClosedPayload {
                    id: reader_id,
                    exit_code,
                },
            );
        });
    }

    // Spawn stderr reader thread
    if let Some(stderr) = stderr {
        let reader_id = id.clone();
        let app_handle = app.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if !alive_stderr.load(Ordering::Relaxed) {
                    break;
                }
                match line {
                    Ok(data) => {
                        if !data.is_empty() {
                            let _ = app_handle.emit(
                                "claude-code-output",
                                ClaudeCodeOutputPayload {
                                    id: reader_id.clone(),
                                    data: format!("[stderr] {}", data),
                                },
                            );
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    Ok(())
}

/// Run a one-shot Claude Code prompt and return the text output.
#[tauri::command]
pub fn claude_code_prompt(
    prompt: String,
    cwd: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let (claude_exe, prefix_args) = find_claude_command();
    let mut command = cmd(&claude_exe);
    for arg in &prefix_args {
        command.arg(arg);
    }

    command.arg("-p")
        .arg(&prompt)
        .arg("--output-format").arg("text")
        .arg("--max-turns").arg("1")
        .arg("--no-input")
        .arg("--model").arg(CLAUDE_MODEL)
        .arg("--effort").arg(CLAUDE_EFFORT);

    // Inject LLM proxy env vars if configured. In direct OAuth mode, refresh
    // AutoSam's isolated Claude credentials first.
    let proxy = LlmProxyConfig::load(&app);
    if proxy.is_none() {
        match sync_claude_oauth_credentials_if_needed(false) {
            Ok(true) => log::info!("[claude-code] refreshed worker Claude OAuth credentials before prompt"),
            Ok(false) => {}
            Err(e) => log::warn!("[claude-code] could not refresh worker Claude OAuth credentials: {}", e),
        }
    }
    inject_proxy_env(&mut command, &proxy);

    let cwd_path = std::path::PathBuf::from(&cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        command.current_dir(&cwd_path);
    }

    let output = command
        .output()
        .map_err(|e| format!("Claude Code not available: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude Code failed: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Kill the Claude Code process for a given session.
#[tauri::command]
pub fn close_claude_code(
    id: String,
    state: tauri::State<'_, ClaudeCodeState>,
) -> Result<(), String> {
    // Remove from map under lock, then kill/wait outside the lock
    // to avoid blocking other threads that need the mutex.
    let removed = {
        let mut processes = state.processes.lock();
        processes.remove(&id)
    };
    if let Some(mut process) = removed {
        process.alive.store(false, Ordering::Relaxed);
        drop(process.stdin.take());
        let _ = process.child.kill();
        let _ = process.child.wait();
    }
    Ok(())
}

/// Write a JSON message to the Claude Code process stdin.
#[tauri::command]
pub fn write_claude_code(
    id: String,
    message: String,
    state: tauri::State<'_, ClaudeCodeState>,
) -> Result<(), String> {
    use std::io::Write;
    let mut processes = state.processes.lock();
    let process = processes
        .get_mut(&id)
        .ok_or_else(|| format!("Claude Code session not found: {}", id))?;
    let stdin = process
        .stdin
        .as_mut()
        .ok_or_else(|| "Claude Code stdin not available".to_string())?;
    stdin
        .write_all(message.as_bytes())
        .map_err(|e| format!("stdin write failed: {}", e))?;
    stdin
        .write_all(b"\n")
        .map_err(|e| format!("stdin newline failed: {}", e))?;
    stdin
        .flush()
        .map_err(|e| format!("stdin flush failed: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn direct_oauth_mode_removes_stale_proxy_and_api_key_env() {
        let mut command = std::process::Command::new("true");
        for key in DIRECT_OAUTH_ENV_BLOCKERS {
            command.env(key, "stale");
        }

        inject_proxy_env(&mut command, &None);

        for key in DIRECT_OAUTH_ENV_BLOCKERS {
            let value = command
                .get_envs()
                .find(|(name, _)| *name == OsStr::new(key))
                .map(|(_, value)| value);
            assert_eq!(value, Some(None), "{} should be explicitly removed", key);
        }
    }
}
