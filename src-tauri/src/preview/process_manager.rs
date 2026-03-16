use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

pub struct ManagedProcess {
    child: Option<Child>,
    port: u16,
    project_dir: PathBuf,
    health_monitor: Option<tokio::task::JoinHandle<()>>,
}

/// Patterns that indicate a dev server is ready AND contain a URL with the port.
/// These are high-confidence: when matched, the port has already been parsed from this line.
const READY_PATTERNS_WITH_URL: &[&str] = &[
    "localhost:",
    "127.0.0.1:",
    "0.0.0.0:",
    "local:",
];

/// Patterns that indicate a dev server is ready but the URL/port may appear on a
/// DIFFERENT line (printed after). When these match, we delay briefly to let the
/// port-bearing line be read before signaling ready.
const READY_PATTERNS_NO_URL: &[&str] = &[
    "ready on",
    "ready at",
    "ready in",
    "listening on",
    "listening at",
    "started on",
    "started at",
    "started server on",
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
        // In monorepos, node_modules may be hoisted to a parent directory
        let has_node_modules = {
            let mut dir = project_dir.to_path_buf();
            let mut found = false;
            for _ in 0..5 {
                if dir.join("node_modules").exists() {
                    found = true;
                    break;
                }
                if !dir.pop() { break; }
            }
            found
        };
        if !has_node_modules {
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

        // Pre-start: kill any orphan from a previous crash holding this port
        #[cfg(windows)]
        {
            Self::kill_port_holder(port).await;
        }

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

        let (ready, actual_port) = wait_for_ready(&mut child, port, &[], app.clone()).await;

        if !ready {
            // Give the process tree time to fully exit (cmd /C wrappers on Windows
            // can linger briefly after the child npm/node process exits)
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    return Err(format!(
                        "Process exited with code {:?} before ready. Command: '{}'",
                        exit_status.code(),
                        command
                    ));
                }
                Ok(None) => {
                    // Process is running but didn't signal ready - let the frontend HTTP check handle it
                    log::warn!("[process_manager] Server did not signal ready within timeout, proceeding to HTTP check.");
                }
                Err(e) => {
                    return Err(format!("Failed to check process status: {}", e));
                }
            }
        }

        log::info!("[process_manager] Using port {} (allocated: {})", actual_port, port);

        Ok(Self {
            child: Some(child),
            port: actual_port,
            project_dir: project_dir.to_path_buf(),
            health_monitor: None,
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
        self.stop_health_monitor();
        let port = self.port;
        if let Some(mut child) = self.child.take() {
            log::info!("[process_manager] Stopping managed process on port {}", port);

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

        // Safety net: kill any orphan process still holding the port (e.g. child spawned via cmd /C)
        #[cfg(windows)]
        {
            Self::kill_port_holder(port).await;
        }

        Ok(())
    }

    /// Kill any process holding a specific port (Windows only).
    /// This catches orphan child processes that survive after taskkill.
    #[cfg(windows)]
    async fn kill_port_holder(port: u16) {
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr \"LISTENING\" | findstr \":{port}\"")])
            .output()
            .await;

        if let Ok(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                // netstat output: "  TCP    0.0.0.0:3000    0.0.0.0:0    LISTENING    12345"
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if pid > 0 {
                            log::info!("[process_manager] Killing orphan PID {} on port {}", pid, port);
                            let _ = Command::new("taskkill")
                                .args(["/F", "/T", "/PID", &pid.to_string()])
                                .output()
                                .await;
                        }
                    }
                }
            }
        }
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

    /// Start a background health monitor that checks if the process is still alive.
    /// Emits `preview:server-died` if the process exits unexpectedly.
    pub fn start_health_monitor(&mut self, app: AppHandle) {
        let pid = self.child.as_ref().and_then(|c| c.id());
        if pid.is_none() {
            return;
        }
        let port = self.port;
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                // Check if process is still alive by trying to connect to the port
                match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                    Ok(_) => {} // Still alive
                    Err(_) => {
                        log::warn!("[health_monitor] Server on port {} is no longer responding", port);
                        let _ = app.emit("preview:server-died", serde_json::json!({
                            "port": port,
                            "message": "Dev server stopped unexpectedly"
                        }));
                        break;
                    }
                }
            }
        });
        self.health_monitor = Some(handle);
    }

    /// Stop the health monitor if running
    pub fn stop_health_monitor(&mut self) {
        if let Some(handle) = self.health_monitor.take() {
            handle.abort();
        }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        // Abort health monitor
        if let Some(handle) = self.health_monitor.take() {
            handle.abort();
        }
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
/// Returns (is_ready, actual_port) - actual_port is parsed from stdout if possible,
/// otherwise falls back to hint_port (the pre-allocated port).
/// When an AppHandle is provided, emits `preview:server-log` events for each line.
async fn wait_for_ready(child: &mut Child, hint_port: u16, extra_patterns: &[&str], app: Option<AppHandle>) -> (bool, u16) {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let extra_owned: Vec<String> = extra_patterns.iter().map(|s| s.to_string()).collect();

    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<bool>();
    let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));
    // Shared slot for the port detected from stdout/stderr
    let detected_port: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));

    // Spawn stdout reader
    if let Some(stdout) = stdout {
        let tx = ready_tx.clone();
        let extra = extra_owned.clone();
        let port_slot = detected_port.clone();
        let app_handle = app.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[managed:stdout] {}", line);
                if let Some(ref ah) = app_handle {
                    let _ = ah.emit("preview:server-log", serde_json::json!({
                        "stream": "stdout", "line": line
                    }));
                }
                if let Some(p) = parse_port_from_line(&line) {
                    *port_slot.lock().await = Some(p);
                }
                match is_ready_line(&line, &extra) {
                    Some(true) => {
                        // URL-bearing pattern: port already parsed from this line
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(true);
                        }
                    }
                    Some(false) => {
                        // Non-URL pattern (e.g. "vite", "▲ next"): delay to let the
                        // URL line arrive so port detection can read the actual port
                        let tx2 = tx.clone();
                        let port_slot2 = port_slot.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            log::info!("[process_manager] Delayed ready signal (port: {:?})", *port_slot2.lock().await);
                            if let Some(sender) = tx2.lock().await.take() {
                                let _ = sender.send(true);
                            }
                        });
                    }
                    None => {}
                }
            }
            // EOF - process closed stdout
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(false);
            }
        });
    }

    // Spawn stderr reader
    if let Some(stderr) = stderr {
        let tx = ready_tx.clone();
        let extra = extra_owned.clone();
        let port_slot = detected_port.clone();
        let app_handle = app.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[managed:stderr] {}", line);
                if let Some(ref ah) = app_handle {
                    let _ = ah.emit("preview:server-log", serde_json::json!({
                        "stream": "stderr", "line": line
                    }));
                }
                if let Some(p) = parse_port_from_line(&line) {
                    *port_slot.lock().await = Some(p);
                }
                match is_ready_line(&line, &extra) {
                    Some(true) => {
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(true);
                        }
                    }
                    Some(false) => {
                        let tx2 = tx.clone();
                        let port_slot2 = port_slot.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            log::info!("[process_manager] Delayed ready signal (port: {:?})", *port_slot2.lock().await);
                            if let Some(sender) = tx2.lock().await.take() {
                                let _ = sender.send(true);
                            }
                        });
                    }
                    None => {}
                }
            }
            // EOF - process closed stderr
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(false);
            }
        });
    }

    // Port poller fallback - checks hint_port (works for apps that DO use PORT env var)
    let poll_tx = ready_tx.clone();
    tokio::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", hint_port)).await.is_ok() {
                log::info!("[process_manager] Port {} open via polling", hint_port);
                if let Some(sender) = poll_tx.lock().await.take() {
                    let _ = sender.send(true);
                }
                return;
            }
        }
    });

    match tokio::time::timeout(std::time::Duration::from_secs(45), ready_rx).await {
        Ok(Ok(true)) => {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let port = detected_port.lock().await.unwrap_or(hint_port);
            log::info!("[process_manager] Server ready on port {}", port);
            (true, port)
        }
        Ok(Ok(false)) => {
            log::error!("[process_manager] Process exited before ready signal");
            (false, hint_port)
        }
        _ => {
            log::warn!("[process_manager] Timeout waiting for ready. Final check...");
            let port = detected_port.lock().await.unwrap_or(hint_port);
            let ready = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();
            (ready, port)
        }
    }
}

/// Strip ANSI escape codes from a string so port/pattern parsing works
/// even when dev servers emit colored output (Vite, Next.js, etc.).
fn strip_ansi(line: &str) -> String {
    let stripped = strip_ansi_escapes::strip(line.as_bytes());
    String::from_utf8_lossy(&stripped).to_string()
}

/// Extract a port number from a dev server log line.
/// Handles patterns like:
///   http://localhost:5173/
///   http://127.0.0.1:3000
///   Local:   http://localhost:5173/
fn parse_port_from_line(line: &str) -> Option<u16> {
    let clean = strip_ansi(line);
    for marker in &["localhost:", "127.0.0.1:", "0.0.0.0:"] {
        if let Some(pos) = clean.find(marker) {
            let after = &clean[pos + marker.len()..];
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                if let Ok(port) = digits.parse::<u16>() {
                    if port > 1023 {
                        return Some(port);
                    }
                }
            }
        }
    }
    None
}

/// Returns: None if not a ready line, Some(true) if URL-bearing, Some(false) if non-URL
fn is_ready_line(line: &str, extra_patterns: &[String]) -> Option<bool> {
    let lower = strip_ansi(line).to_lowercase();
    if READY_PATTERNS_WITH_URL.iter().any(|pat| lower.contains(pat)) {
        return Some(true);
    }
    if READY_PATTERNS_NO_URL.iter().any(|pat| lower.contains(pat)) {
        return Some(false);
    }
    if extra_patterns.iter().any(|pat| lower.contains(&pat.to_lowercase())) {
        return Some(false);
    }
    None
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
