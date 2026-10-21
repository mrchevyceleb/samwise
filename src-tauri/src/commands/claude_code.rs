use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use sha2::{Digest, Sha256};
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

/// Secrets that must never reach a child process env. The worker agent's own
/// diagnostic commands (e.g. `printenv | grep -E '^(SAM|AUTO|PI_MODEL)'`, plain
/// `env` dumps) print its env lines straight into the pi session transcript,
/// so any secret left in the child env lands in ~/.pi/agent/sessions (the
/// LiteLLM master key surfaced in ~45 historical session files this way, Oct
/// 16 2026). No child needs these: pi authenticates itself from
/// ~/.pi/agent/auth.json, and the Claude path injects proxy credentials
/// explicitly (ANTHROPIC_BASE_URL/ANTHROPIC_API_KEY) when a proxy is
/// configured. The user-session keys land here because the systemd user unit
/// passes the desktop session env through; a task that genuinely needs one
/// of these secrets must be handed it explicitly.
const WORKER_SECRET_ENV_BLOCKERS: &[&str] = &[
    "AUTOSAM_LLM_PROXY_URL",
    "AUTOSAM_LLM_PROXY_API_KEY",
    "SAM_CALLBACK_SECRET",
    "DOPPLER_TOKEN",
    "FIREWORKS_PERSONAL_API_KEY",
    "GROK_API_KEY",
    "META_PERSONAL_API_KEY",
    "Z_AI_API_KEY",
];

fn strip_direct_oauth_blockers(cmd: &mut std::process::Command) {
    for key in DIRECT_OAUTH_ENV_BLOCKERS.iter().chain(WORKER_SECRET_ENV_BLOCKERS) {
        cmd.env_remove(key);
    }
}

/// Scrub Anthropic/proxy routing vars from a child command.
///
/// Also used for pi spawns: pi has its own `anthropic` provider that reads
/// `ANTHROPIC_API_KEY`/`ANTHROPIC_BASE_URL`, so leaving agent-one's LiteLLM
/// values in the environment would quietly route pi through the very proxy the
/// pi backend exists to remove.
pub fn strip_direct_oauth_blockers_async(cmd: &mut tokio::process::Command) {
    for key in DIRECT_OAUTH_ENV_BLOCKERS.iter().chain(WORKER_SECRET_ENV_BLOCKERS) {
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
        // The managed shell shim calls us. Never resolve back to it when the
        // real CLI disappears, or an auth probe would recursively spawn itself.
        if path != std::path::Path::new("/home/mrchevyceleb/.local/lib/autosam/bin/claude") {
            return (path.to_string_lossy().into_owned(), vec![]);
        }
    }
    ("/home/mrchevyceleb/.local/bin/claude".to_string(), vec![])
}

/// AutoSam never reads or copies the personal Claude profile. An inherited
/// override cannot redirect either the auth probe or the actual child.
tokio::task_local! {
    static AUTH_TASK: AuthTask;
}

struct AuthTask {
    config: super::supabase::SupabaseConfig,
    id: String,
    failure: std::sync::Mutex<Option<String>>,
}

pub fn team_auth_task_status() -> Result<(), String> {
    match AUTH_TASK.try_with(|task| task.failure.lock().unwrap_or_else(|p| p.into_inner()).clone()).ok().flatten() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Secondary calls can be handled by their caller without losing this failure.
/// The task stays held even if a summary/repair caller uses a fallback result.
async fn note_team_auth_failure(error: &str) {
    let context = AUTH_TASK.try_with(|task| {
        let mut failure = task.failure.lock().unwrap_or_else(|p| p.into_inner());
        if failure.is_some() { return None; }
        *failure = Some(error.to_string());
        Some((task.config.clone(), task.id.clone()))
    }).ok().flatten();
    if let Some((config, id)) = context {
        if super::supabase::update_task(&config, &id, &serde_json::json!({
            "on_hold": true, "updated_at": chrono::Utc::now().to_rfc3339(),
        })).await.is_err() {
            log::error!("[worker] could not save team-auth hold for task {}", id);
        }
    }
}

pub async fn with_team_auth_task<T, F>(config: &super::supabase::SupabaseConfig, id: &str, future: F) -> Result<T, String>
where F: std::future::Future<Output = Result<T, String>> {
    if AUTH_TASK.try_with(|task| task.id == id).unwrap_or(false) { return future.await; }
    AUTH_TASK.scope(AuthTask { config: config.clone(), id: id.to_string(), failure: std::sync::Mutex::new(None) }, async {
        let result = future.await;
        let failure = AUTH_TASK.with(|task| task.failure.lock().unwrap_or_else(|p| p.into_inner()).clone());
        if let Some(error) = failure {
            super::supabase::update_task(config, id, &serde_json::json!({
                "status": "queued", "on_hold": true,
                "worker_id": serde_json::Value::Null, "claimed_at": serde_json::Value::Null,
                "updated_at": chrono::Utc::now().to_rfc3339(),
            })).await.map_err(|_| team_auth_error("Work is preserved, but saving its account hold needs attention"))?;
            return Err(error);
        }
        result
    }).await
}

pub async fn guard_pi_provider(provider: &str) -> Result<(), String> {
    if provider.eq_ignore_ascii_case("anthropic") {
        let error = team_auth_error("Direct pi Anthropic login is disabled; use the guarded Claude coder");
        note_team_auth_failure(&error).await;
        return Err(error);
    }
    Ok(())
}

const TEAM_AUTH_BLOCKED: &str = "TEAM_AUTH_BLOCKED";
const TEAM_ACCOUNT_SHA256: &str = "a0c253a7ea9f80b2e4ba9cd2131e690d5de903a6661ef5bf84da3b27e3dc8723";
static TEAM_AUTH_CHECK: Mutex<()> = Mutex::new(());

fn team_auth_error(reason: &str) -> String {
    format!("{}: {}. Refusing to use another Claude account.", TEAM_AUTH_BLOCKED, reason)
}

pub fn is_team_auth_blocked(error: &str) -> bool { error.starts_with(TEAM_AUTH_BLOCKED) }

fn team_claude_profile() -> Result<PathBuf, String> {
    let profile = PathBuf::from("/home/mrchevyceleb/.claude-tardis");
    let credentials = profile.join(".credentials.json");
    if !profile.is_dir() || !credentials.is_file()
        || std::fs::symlink_metadata(&profile).map(|m| m.file_type().is_symlink()).unwrap_or(true)
        || std::fs::symlink_metadata(&credentials).map(|m| m.file_type().is_symlink()).unwrap_or(true) {
        return Err(team_auth_error("Team profile is missing or symlinked"));
    }
    Ok(profile)
}

fn is_claude_auth_override(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    name.starts_with("ANTHROPIC_") || name.starts_with("CLAUDE_CODE_OAUTH_")
        || name.starts_with("CLAUDE_CODE_USE_")
        || matches!(name.as_str(), "CLAUDE_CODE_SIMPLE" | "CLAUDECODE")
        || TEAM_CLAUDE_NETWORK_ENV.iter().any(|key| key.eq_ignore_ascii_case(&name))
}

const TEAM_CLAUDE_NETWORK_ENV: &[&str] = &[
    "HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "https_proxy", "http_proxy", "all_proxy",
    "NODE_EXTRA_CA_CERTS", "NODE_TLS_REJECT_UNAUTHORIZED",
    "SSL_CERT_FILE", "SSL_CERT_DIR", "CLAUDE_CODE_CLIENT_CERT", "CLAUDE_CODE_CLIENT_KEY",
    "CLAUDE_CODE_CLIENT_KEY_PASSPHRASE", "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
    "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
];

fn team_claude_env(command: &mut std::process::Command, profile: &std::path::Path) {
    let keys: Vec<_> = std::env::vars_os().map(|(key, _)| key)
        .chain(command.get_envs().map(|(key, _)| key.to_os_string())).collect();
    for key in keys {
        if is_claude_auth_override(&key.to_string_lossy()) { command.env_remove(key); }
    }
    strip_direct_oauth_blockers(command);
    command.env("CLAUDE_CONFIG_DIR", profile);
}

/// Keep native global, project and local instructions while CLI settings
/// override authentication. Filtering settings sources also hides their rules.
fn team_claude_settings(command: &mut std::process::Command, profile: &std::path::Path) {
    let mut env = serde_json::Map::new();
    for key in [
        "ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_BASE_URL",
        "ANTHROPIC_CUSTOM_HEADERS", "ANTHROPIC_MODEL", "ANTHROPIC_DEFAULT_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL", "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL", "CLAUDE_CODE_OAUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_TOKEN_FILE", "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY", "CLAUDE_CODE_SIMPLE",
    ] { env.insert(key.to_string(), serde_json::json!("")); }
    for key in TEAM_CLAUDE_NETWORK_ENV { env.insert((*key).to_string(), serde_json::json!("")); }
    // Keep the worker's build/runtime options, while file settings cannot
    // inject a Node preload or replace its memory allowance.
    env.insert("NODE_OPTIONS".into(), serde_json::json!(std::env::var("NODE_OPTIONS").unwrap_or_default()));
    env.insert("NODE_TLS_REJECT_UNAUTHORIZED".into(), serde_json::json!("1"));
    // Empty OpenSSL paths suppress its system trust store in Bash children.
    // Pin Moria's normal OpenSSL defaults while refusing repo CA replacement.
    env.insert("SSL_CERT_FILE".into(), serde_json::json!("/usr/lib/ssl/cert.pem"));
    env.insert("SSL_CERT_DIR".into(), serde_json::json!("/usr/lib/ssl/certs"));
    env.insert("CLAUDE_CONFIG_DIR".into(), serde_json::json!(profile));
    command.args(["--setting-sources", "user,project,local", "--settings"])
        .arg(serde_json::json!({
            "env": env, "apiKeyHelper": "", "awsAuthRefresh": "",
            "awsCredentialExport": "", "forceLoginMethod": "claudeai",
            // Keep Matt's team-profile guards even when a repo disables hooks.
            "disableAllHooks": false, "enableAllProjectMcpServers": false,
            "otelHeadersHelper": "", "statusLine": { "type": "command", "command": "" },
        }).to_string());
}

fn checked_team_claude_profile(cwd: Option<PathBuf>) -> Result<PathBuf, String> {
    // Serialize checks because the CLI may refresh its shared subscription token.
    let _check = TEAM_AUTH_CHECK.lock().map_err(|_| team_auth_error("Team auth check is blocked"))?;
    let profile = team_claude_profile()?;
    let (exe, prefix) = find_claude_command();
    let mut probe = cmd(&exe);
    team_claude_env(&mut probe, &profile);
    probe.args(prefix);
    team_claude_settings(&mut probe, &profile);
    if let Some(cwd) = cwd { probe.current_dir(cwd); }
    let mut child = probe.args(["auth", "status"])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null()).spawn()
        .map_err(|_| team_auth_error("Cannot verify team authentication"))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(50)),
            _ => {
                let _ = child.kill(); let _ = child.wait();
                return Err(team_auth_error("Team auth check timed out"));
            }
        }
    }
    let output = child.wait_with_output().map_err(|_| team_auth_error("Cannot read team auth status"))?;
    let status: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| team_auth_error("Team auth status is invalid"))?;
    if !output.status.success() || status["loggedIn"].as_bool() != Some(true)
        || status["authMethod"].as_str() != Some("claude.ai")
        || status["apiProvider"].as_str() != Some("firstParty") {
        return Err(team_auth_error("Team subscription is not authenticated"));
    }
    let email = status["email"].as_str().ok_or_else(|| team_auth_error("Team identity is missing"))?;
    let org = status["orgId"].as_str().ok_or_else(|| team_auth_error("Team organization is missing"))?;
    let identity = format!("{:x}", Sha256::digest(format!("{}|{}", email, org).as_bytes()));
    if identity != TEAM_ACCOUNT_SHA256 { return Err(team_auth_error("Profile belongs to a different account")); }
    Ok(profile)
}

/// Called before any worker or UI thread starts. Shell descendants inherit the
/// same team profile even when their immediate parent is the pi harness.
pub fn initialize_team_claude_account() -> Result<(), String> {
    let profile = checked_team_claude_profile(None)?;
    let keys: Vec<_> = std::env::vars_os().map(|(key, _)| key).collect();
    for key in keys {
        if is_claude_auth_override(&key.to_string_lossy()) { std::env::remove_var(key); }
    }
    std::env::set_var("CLAUDE_CONFIG_DIR", profile);
    let current_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("/home/mrchevyceleb/.local/lib/autosam/bin:{}", current_path));
    Ok(())
}

pub fn prepare_team_claude(command: &mut std::process::Command) -> Result<(), String> {
    let profile = checked_team_claude_profile(command.get_current_dir().map(PathBuf::from))?;
    team_claude_env(command, &profile);
    team_claude_settings(command, &profile);
    Ok(())
}

pub async fn prepare_team_claude_async(command: &mut tokio::process::Command) -> Result<(), String> {
    team_auth_task_status()?;
    let cwd = command.as_std().get_current_dir().map(PathBuf::from);
    let checked = tokio::task::spawn_blocking(move || checked_team_claude_profile(cwd)).await
        .map_err(|_| team_auth_error("Team auth check failed")).and_then(|result| result);
    let profile = match checked {
        Ok(profile) => profile,
        Err(error) => { note_team_auth_failure(&error).await; return Err(error); }
    };
    team_claude_env(command.as_std_mut(), &profile);
    team_claude_settings(command.as_std_mut(), &profile);
    Ok(())
}

pub fn team_shell_guard_ready() -> Result<(), String> {
    let path = std::path::Path::new("/home/mrchevyceleb/.local/lib/autosam/bin/claude");
    let valid = std::fs::symlink_metadata(path).map(|m| {
        #[cfg(unix)]
        { use std::os::unix::fs::PermissionsExt; m.is_file() && m.permissions().mode() & 0o111 != 0 }
        #[cfg(not(unix))]
        { m.is_file() }
    }).unwrap_or(false);
    if !valid || std::fs::read_to_string(path).ok().as_deref() != Some(include_str!("../../../ops/bin/claude")) {
        return Err(team_auth_error("Team Claude shell launcher is missing or changed"));
    }
    Ok(())
}

/// Entry point used by the shell shim. Project rules load, but auth stays pinned.
pub fn run_team_claude_shell(args: Vec<std::ffi::OsString>) -> ! {
    if args.iter().filter_map(|a| a.to_str()).any(|a| matches!(a.split('=').next(), Some("--settings" | "--setting-sources"))) {
        eprintln!("{}", team_auth_error("Shell arguments cannot override account settings"));
        std::process::exit(78);
    }
    let (exe, prefix) = find_claude_command();
    let mut command = cmd(&exe);
    command.args(prefix).args(args);
    if let Err(error) = prepare_team_claude(&mut command) {
        eprintln!("{}", error); std::process::exit(78);
    }
    #[cfg(unix)]
    { use std::os::unix::process::CommandExt; let _ = command.exec(); }
    #[cfg(not(unix))]
    if let Ok(status) = command.status() { std::process::exit(status.code().unwrap_or(1)); }
    eprintln!("{}", team_auth_error("Cannot launch the verified team Claude CLI"));
    std::process::exit(78);
}

// ── Commands ─────────────────────────────────────────────────────────

/// Spawn a persistent Claude Code process with stream-json on stdin/stdout.
#[tauri::command]
pub async fn spawn_claude_code(
    id: String,
    cwd: String,
    args: Vec<String>,
    state: tauri::State<'_, ClaudeCodeState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
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

    // The team subscription policy wins over saved legacy proxy settings.
    let proxy = LlmProxyConfig::load(&app);
    inject_proxy_env(&mut command, &proxy);

    // Add extra args from the frontend (e.g. --resume). Note: --model is already
    // pinned above to CLAUDE_MODEL; frontend args take precedence if they set it again.
    if args.iter().any(|arg| matches!(arg.split('=').next(), Some("--settings" | "--setting-sources"))) {
        return Err("Claude account settings cannot be overridden by session arguments".into());
    }
    for arg in &args { command.arg(arg); }

    // Set working directory
    let cwd_path = PathBuf::from(&cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        command.current_dir(&cwd_path);
    }

    let mut command = tokio::task::spawn_blocking(move || {
        prepare_team_claude(&mut command)?;
        Ok::<_, String>(command)
    }).await.map_err(|_| team_auth_error("Team auth check failed"))??;

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
pub async fn claude_code_prompt(
    prompt: String,
    cwd: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let (claude_exe, prefix_args) = find_claude_command();
    let mut command = crate::process::async_cmd(&claude_exe);
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

    // The team subscription policy wins over saved legacy proxy settings.
    let proxy = LlmProxyConfig::load(&app);
    inject_proxy_env_async(&mut command, &proxy);

    let cwd_path = std::path::PathBuf::from(&cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        command.current_dir(&cwd_path);
    }

    prepare_team_claude_async(&mut command).await?;

    let output = command
        .output().await
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
