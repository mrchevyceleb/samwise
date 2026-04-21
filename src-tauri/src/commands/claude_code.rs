use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use crate::process::cmd;

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

/// The Claude model Sam always uses. Single source of truth so chat and worker
/// stay aligned. Change this string when bumping models.
pub const CLAUDE_MODEL: &str = "claude-opus-4-7";

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
        .arg("--model").arg(CLAUDE_MODEL);

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
pub fn claude_code_prompt(prompt: String, cwd: String) -> Result<String, String> {
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
        .arg("--model").arg(CLAUDE_MODEL);

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
