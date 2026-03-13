use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

pub struct ManagedProcess {
    child: Option<Child>,
    port: u16,
    project_dir: PathBuf,
}

/// Patterns that indicate a dev server is ready
const READY_PATTERNS: &[&str] = &[
    "ready on",
    "ready at",
    "ready in",
    "listening on",
    "listening at",
    "started on",
    "started at",
    "started server on",
    "local:",
    "localhost:",
    "127.0.0.1:",
    "server running",
    "compiled successfully",
    "compiled client",
    "webpack compiled",
    "built in",
];

impl ManagedProcess {
    /// Start a managed dev server process.
    ///
    /// The `command` should be the full dev command (e.g., "next dev", "npm run dev").
    /// We inject PORT env so the server listens on our chosen port.
    pub async fn start(
        project_dir: &Path,
        command: &str,
        env: HashMap<String, String>,
    ) -> Result<Self, String> {
        let port = super::port_allocator::find_managed_port()?;

        log::info!(
            "[process_manager] Starting managed process: '{}' on port {} in {}",
            command,
            port,
            project_dir.display()
        );

        // Parse command into program and args
        let (program, args) = parse_command(command);

        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .current_dir(project_dir)
            .env("PORT", port.to_string())
            .env("BROWSER", "none") // Prevent auto-opening browser
            .env("FORCE_COLOR", "0") // Clean output
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Inject custom environment variables
        for (key, value) in &env {
            cmd.env(key, value);
        }

        // On Windows, create a new process group so we can kill the tree
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x00000008); // CREATE_NO_WINDOW
        }

        let mut child = cmd.spawn().map_err(|e| {
            format!("Failed to start managed process '{}': {}", program, e)
        })?;

        // Wait for the server to be ready by watching stdout/stderr
        let ready = wait_for_ready(&mut child, port).await;

        if !ready {
            log::warn!(
                "[process_manager] Server may not be fully ready. Proceeding anyway after timeout."
            );
        }

        Ok(Self {
            child: Some(child),
            port,
            project_dir: project_dir.to_path_buf(),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// Stop the managed process gracefully
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            log::info!("[process_manager] Stopping managed process");

            // On Windows, we need to kill the process tree
            #[cfg(windows)]
            {
                if let Some(id) = child.id() {
                    // Use taskkill /T to kill the process tree
                    let _ = Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &id.to_string()])
                        .output()
                        .await;
                }
            }

            // On Unix, send SIGTERM
            #[cfg(not(windows))]
            {
                let _ = child.kill().await;
            }

            // Wait for exit
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                child.wait(),
            )
            .await;
        }

        Ok(())
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,  // Still running
                _ => false,        // Exited or error
            }
        } else {
            false
        }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        // Best-effort synchronous kill
        if let Some(ref mut child) = self.child {
            #[cfg(windows)]
            {
                if let Some(id) = child.id() {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &id.to_string()])
                        .output();
                }
            }
            #[cfg(not(windows))]
            {
                let _ = child.start_kill();
            }
        }
    }
}

/// Parse a command string into program and arguments.
/// Handles npm/npx scripts and direct commands.
fn parse_command(command: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return ("npm".to_string(), vec!["run".to_string(), "dev".to_string()]);
    }

    let program = parts[0].to_string();
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // If the command doesn't start with a known runner, wrap in npx
    let known_runners = ["npm", "npx", "yarn", "pnpm", "bun", "node", "deno", "tsx"];
    if known_runners.contains(&program.as_str()) {
        (program, args)
    } else {
        // e.g., "next dev" -> "npx next dev"
        let mut npx_args = vec![program];
        npx_args.extend(args);
        ("npx".to_string(), npx_args)
    }
}

/// Wait for the dev server to output a "ready" pattern, with a timeout
async fn wait_for_ready(child: &mut Child, port: u16) -> bool {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<bool>();
    let ready_tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(ready_tx)));

    // Spawn stdout reader
    if let Some(stdout) = stdout {
        let tx = ready_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[managed:stdout] {}", line);
                if is_ready_line(&line) {
                    if let Some(tx) = tx.lock().await.take() {
                        let _ = tx.send(true);
                    }
                }
            }
        });
    }

    // Spawn stderr reader
    if let Some(stderr) = stderr {
        let tx = ready_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[managed:stderr] {}", line);
                if is_ready_line(&line) {
                    if let Some(tx) = tx.lock().await.take() {
                        let _ = tx.send(true);
                    }
                }
            }
        });
    }

    // Wait with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(30), ready_rx).await {
        Ok(Ok(true)) => {
            log::info!("[process_manager] Server ready on port {}", port);
            true
        }
        _ => {
            // Timeout or channel closed. Check if port is actually listening.
            log::warn!("[process_manager] Timeout waiting for ready signal. Checking port...");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
        }
    }
}

fn is_ready_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    READY_PATTERNS.iter().any(|pat| lower.contains(pat))
}
