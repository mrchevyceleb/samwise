use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
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
    "0.0.0.0:",
    "server running",
    "compiled successfully",
    "compiled client",
    "webpack compiled",
    "built in",
    "vite",
    "➜",
    "▲ next",
    "ready started server",
    "app is running",
    "serving on",
    "dev server running",
    "press h + enter",
];

impl ManagedProcess {
    /// Start a managed dev server process.
    /// Silently installs deps if missing, spawns the dev command, waits for ready.
    pub async fn start(
        project_dir: &Path,
        command: &str,
        env: HashMap<String, String>,
        app: Option<AppHandle>,
    ) -> Result<Self, String> {
        // Auto-install deps if node_modules is missing (silently)
        let node_modules = project_dir.join("node_modules");
        if !node_modules.exists() {
            log::info!("[process_manager] node_modules missing, running install silently...");
            match run_package_install(project_dir).await {
                Ok(_) => log::info!("[process_manager] Install completed"),
                Err(e) => {
                    log::error!("[process_manager] Install failed: {}", e);
                    return Err(format!(
                        "Dependencies not installed: {}",
                        e
                    ));
                }
            }
        }

        let port = super::port_allocator::find_managed_port()?;

        log::info!(
            "[process_manager] Starting: '{}' on port {} in {}",
            command,
            port,
            project_dir.display()
        );

        let (program, args) = parse_command_platform(command);

        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .current_dir(project_dir)
            .env("PORT", port.to_string())
            .env("BROWSER", "none")
            .env("FORCE_COLOR", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (key, value) in &env {
            cmd.env(key, value);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = cmd.spawn().map_err(|e| {
            format!("Failed to start '{}': {}", program, e)
        })?;

        let ready = wait_for_ready(&mut child, port, &[]).await;

        if !ready {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    return Err(format!(
                        "Process exited with code {:?} before ready. Command: '{}'",
                        exit_status.code(),
                        command
                    ));
                }
                _ => {
                    log::warn!("[process_manager] Server may not be fully ready, proceeding.");
                }
            }
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

/// Platform-aware command parsing. On Windows, wraps shell commands through
/// cmd.exe /C so that .cmd/.bat scripts (npm, npx, yarn, pnpm) resolve correctly.
fn parse_command_platform(command: &str) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let (program, args) = parse_command(command);
        let shell_commands = ["npm", "npx", "yarn", "pnpm", "bun"];
        if shell_commands.contains(&program.as_str()) {
            // On Windows, npm/npx/etc. are .cmd files. Use cmd /C for reliable resolution.
            let mut full_cmd = program;
            for arg in &args {
                full_cmd.push(' ');
                full_cmd.push_str(arg);
            }
            return ("cmd".to_string(), vec!["/C".to_string(), full_cmd]);
        }
        (program, args)
    }
    #[cfg(not(windows))]
    {
        parse_command(command)
    }
}

/// Wait for the dev server to output a "ready" pattern, with a timeout.
/// Uses a multi-signal approach: ready pattern in stdout/stderr, port polling, or process exit.
async fn wait_for_ready(child: &mut Child, port: u16, extra_patterns: &[&str]) -> bool {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Collect extra patterns into owned strings for the async tasks
    let extra_owned: Vec<String> = extra_patterns.iter().map(|s| s.to_string()).collect();

    // Channel for ready signal OR process death
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<bool>();
    let ready_tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(ready_tx)));

    // Spawn stdout reader
    if let Some(stdout) = stdout {
        let tx = ready_tx.clone();
        let extra = extra_owned.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[managed:stdout] {}", line);
                if is_ready_line(&line, &extra) {
                    if let Some(tx) = tx.lock().await.take() {
                        let _ = tx.send(true);
                    }
                }
            }
            // EOF means process closed stdout - signal not ready
            if let Some(tx) = tx.lock().await.take() {
                let _ = tx.send(false);
            }
        });
    }

    // Spawn stderr reader
    if let Some(stderr) = stderr {
        let tx = ready_tx.clone();
        let extra = extra_owned.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[managed:stderr] {}", line);
                if is_ready_line(&line, &extra) {
                    if let Some(tx) = tx.lock().await.take() {
                        let _ = tx.send(true);
                    }
                }
            }
        });
    }

    // Spawn port poller as a parallel fallback - checks every 2s if port is listening
    let poll_tx = ready_tx.clone();
    let poll_port = port;
    tokio::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", poll_port)).await.is_ok() {
                log::info!("[process_manager] Port {} detected open via polling", poll_port);
                if let Some(tx) = poll_tx.lock().await.take() {
                    let _ = tx.send(true);
                }
                return;
            }
        }
    });

    // Wait with timeout (45s to give install/compile time)
    match tokio::time::timeout(std::time::Duration::from_secs(45), ready_rx).await {
        Ok(Ok(true)) => {
            log::info!("[process_manager] Server ready on port {}", port);
            // Give the server a moment to fully initialize after the ready signal
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            true
        }
        Ok(Ok(false)) => {
            // Process exited before ready signal
            log::error!("[process_manager] Process exited before ready signal");
            false
        }
        _ => {
            // Timeout or channel closed. Final port check.
            log::warn!("[process_manager] Timeout waiting for ready signal. Final port check...");
            std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
        }
    }
}

fn is_ready_line(line: &str, extra_patterns: &[String]) -> bool {
    let lower = line.to_lowercase();
    if READY_PATTERNS.iter().any(|pat| lower.contains(pat)) {
        return true;
    }
    extra_patterns.iter().any(|pat| lower.contains(&pat.to_lowercase()))
}

/// Detect the package manager and run install.
/// Checks for lock files to determine which package manager to use.
async fn run_package_install(project_dir: &Path) -> Result<(), String> {
    let command = if project_dir.join("bun.lockb").exists() || project_dir.join("bun.lock").exists() {
        "bun install"
    } else if project_dir.join("pnpm-lock.yaml").exists() {
        "pnpm install"
    } else if project_dir.join("yarn.lock").exists() {
        "yarn install"
    } else {
        "npm install"
    };

    log::info!("[process_manager] Running '{}' in {}", command, project_dir.display());
    run_install_command(project_dir, command).await
}

/// Execute an install command string in the project directory.
async fn run_install_command(project_dir: &Path, command: &str) -> Result<(), String> {
    let (program, args) = {
        #[cfg(windows)]
        {
            ("cmd".to_string(), vec!["/C".to_string(), command.to_string()])
        }
        #[cfg(not(windows))]
        {
            let parts: Vec<&str> = command.split_whitespace().collect();
            let prog = parts.first().unwrap_or(&"npm").to_string();
            let a: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            (prog, a)
        }
    };

    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .current_dir(project_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to run '{}': {}", command, e))?;

    match tokio::time::timeout(
        std::time::Duration::from_secs(180),
        child.wait(),
    ).await {
        Ok(Ok(exit_status)) => {
            if !exit_status.success() {
                return Err(format!("'{}' failed with exit code: {:?}", command, exit_status.code()));
            }
        }
        Ok(Err(e)) => {
            return Err(format!("'{}' failed: {}", command, e));
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(format!("'{}' timed out after 180 seconds", command));
        }
    };

    Ok(())
}
