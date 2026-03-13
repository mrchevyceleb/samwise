use crate::state::TerminalState;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::Emitter;

/// PTY session stored per terminal instance.
pub struct PtySession {
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub child: Box<dyn portable_pty::Child + Send>,
    pub writer: Box<dyn Write + Send>,
    pub alive: Arc<Mutex<bool>>,
}

#[derive(Clone, Serialize)]
struct TerminalOutputPayload {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct TerminalClosedPayload {
    id: String,
    exit_code: Option<u32>,
}

#[derive(Clone, Serialize)]
pub struct TerminalInfo {
    pub id: String,
}

#[tauri::command]
pub fn spawn_terminal(
    id: String,
    cwd: String,
    rows: u16,
    cols: u16,
    shell: String,
    state: tauri::State<'_, TerminalState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Check if this terminal ID already has an active session
    {
        let sessions = state.sessions.lock();
        if sessions.contains_key(&id) {
            return Err(format!("Terminal session already exists: {}", id));
        }
    }

    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    // Determine shell
    let shell_trimmed = shell.trim();
    let mut cmd = if shell_trimmed.is_empty() || shell_trimmed.eq_ignore_ascii_case("auto") {
        if cfg!(target_os = "windows") {
            CommandBuilder::new("powershell.exe")
        } else {
            let default_shell =
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            CommandBuilder::new(default_shell)
        }
    } else if shell_trimmed.eq_ignore_ascii_case("powershell") {
        CommandBuilder::new("powershell.exe")
    } else if shell_trimmed.eq_ignore_ascii_case("bash") {
        if cfg!(target_os = "windows") {
            CommandBuilder::new("bash.exe")
        } else {
            CommandBuilder::new("bash")
        }
    } else if shell_trimmed.eq_ignore_ascii_case("cmd") {
        CommandBuilder::new("cmd.exe")
    } else {
        CommandBuilder::new(shell_trimmed)
    };

    // Set working directory
    let cwd_path = PathBuf::from(&cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        cmd.cwd(cwd_path);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

    let alive = Arc::new(Mutex::new(true));
    let alive_clone = Arc::clone(&alive);

    let session = PtySession {
        master: pair.master,
        child,
        writer,
        alive,
    };

    // Store the session
    {
        let mut sessions = state.sessions.lock();
        sessions.insert(id.clone(), session);
    }

    // Spawn a reader thread that emits terminal-output events
    let reader_id = id.clone();
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            {
                let is_alive = alive_clone.lock();
                if !*is_alive {
                    break;
                }
            }

            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = app_handle.emit(
                        "terminal-closed",
                        TerminalClosedPayload {
                            id: reader_id.clone(),
                            exit_code: None,
                        },
                    );
                    break;
                }
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_handle.emit(
                        "terminal-output",
                        TerminalOutputPayload {
                            id: reader_id.clone(),
                            data,
                        },
                    );
                }
                Err(e) => {
                    let _ = app_handle.emit(
                        "terminal-closed",
                        TerminalClosedPayload {
                            id: reader_id.clone(),
                            exit_code: None,
                        },
                    );
                    eprintln!("PTY reader error for {}: {}", reader_id, e);
                    break;
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn write_terminal(
    id: String,
    data: String,
    state: tauri::State<'_, TerminalState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock();
    let session = sessions
        .get_mut(&id)
        .ok_or_else(|| format!("Terminal session not found: {}", id))?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| format!("Failed to write to PTY: {}", e))?;
    session
        .writer
        .flush()
        .map_err(|e| format!("Failed to flush PTY writer: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn resize_terminal(
    id: String,
    rows: u16,
    cols: u16,
    state: tauri::State<'_, TerminalState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock();
    let session = sessions
        .get(&id)
        .ok_or_else(|| format!("Terminal session not found: {}", id))?;
    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to resize PTY: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn kill_terminal(id: String, state: tauri::State<'_, TerminalState>) -> Result<(), String> {
    let mut sessions = state.sessions.lock();
    if let Some(mut session) = sessions.remove(&id) {
        {
            let mut is_alive = session.alive.lock();
            *is_alive = false;
        }
        let _ = session.child.kill();
        let _ = session.child.wait();
    }
    Ok(())
}

#[tauri::command]
pub fn list_terminals(state: tauri::State<'_, TerminalState>) -> Result<Vec<TerminalInfo>, String> {
    let sessions = state.sessions.lock();
    let list: Vec<TerminalInfo> = sessions
        .keys()
        .map(|id| TerminalInfo { id: id.clone() })
        .collect();
    Ok(list)
}
