use std::process::Stdio;
use tokio::process::Child;
use crate::process::async_cmd;

pub struct DevServerHandle {
    pub child: Child,
    pub port: u16,
    pub url: String,
    pub pid: u32,
}

/// Detect the dev command and default port from package.json.
/// Returns (script_name, default_port) where script_name is "dev" or "start".
async fn detect_dev_command(repo_path: &str) -> Result<(String, u16), String> {
    let pkg_path = std::path::Path::new(repo_path).join("package.json");
    let content = tokio::fs::read_to_string(&pkg_path).await
        .map_err(|e| format!("Failed to read package.json: {}", e))?;
    let pkg: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    let scripts = pkg.get("scripts").and_then(|v| v.as_object())
        .ok_or_else(|| "No scripts in package.json".to_string())?;

    // Prefer "dev", then "start"
    let (script_name, script_val) = if let Some(dev) = scripts.get("dev") {
        ("dev", dev.as_str().unwrap_or(""))
    } else if let Some(start) = scripts.get("start") {
        ("start", start.as_str().unwrap_or(""))
    } else {
        return Err("No dev or start script in package.json".to_string());
    };

    let default_port = detect_port_from_script(script_val);

    Ok((script_name.to_string(), default_port))
}

/// Detect framework and default port from the script command string.
fn detect_port_from_script(script: &str) -> u16 {
    let s = script.to_lowercase();
    if s.contains("vite") || s.contains("svelte") || s.contains("sveltekit") {
        5173
    } else if s.contains("next") {
        3000
    } else if s.contains("nuxt") {
        3000
    } else if s.contains("react-scripts") {
        3000
    } else if s.contains("webpack") {
        8080
    } else {
        3000
    }
}

/// Detect the port CLI flag style for the framework.
enum PortStyle {
    /// --port {port} (vite, nuxt, sveltekit)
    DashDashPort,
    /// -p {port} (next)
    DashP,
    /// PORT={port} env var (react-scripts)
    EnvVar,
}

fn detect_port_style(script: &str) -> PortStyle {
    let s = script.to_lowercase();
    if s.contains("next") {
        PortStyle::DashP
    } else if s.contains("react-scripts") {
        PortStyle::EnvVar
    } else {
        // vite, nuxt, sveltekit, webpack, etc. all use --port
        PortStyle::DashDashPort
    }
}

/// Find an open port starting from `starting_from`, trying 20 consecutive ports.
/// Note: TOCTOU gap between bind check and actual server start is inherent to this pattern.
/// If the port gets claimed in the gap, the dev server will fail to start and we handle that gracefully.
pub fn find_open_port(starting_from: u16) -> u16 {
    for port in starting_from..=starting_from + 20 {
        if let Ok(listener) = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
            drop(listener);
            return port;
        }
    }
    // Fallback: try a random high port
    for port in 10000..10100 {
        if let Ok(listener) = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
            drop(listener);
            return port;
        }
    }
    // Last resort
    9876
}

/// Run `npm install` if node_modules doesn't exist. Ensures deps are available before starting the dev server.
pub async fn ensure_deps_installed(repo_path: &str) -> Result<(), String> {
    let node_modules = std::path::Path::new(repo_path).join("node_modules");
    if node_modules.exists() {
        return Ok(());
    }

    log::info!("[dev_server] node_modules missing in {}, running npm install", repo_path);

    let output = async_cmd("npm")
        .args(["install"])
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run npm install: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("npm install failed: {}", stderr.trim()));
    }

    log::info!("[dev_server] npm install complete in {}", repo_path);
    Ok(())
}

/// Start a dev server in the given repo directory.
/// If `dev_command` is provided (from ae_projects), use it directly. It may contain `{port}` placeholder.
/// Otherwise auto-detect from package.json.
pub async fn start_dev_server(
    repo_path: &str,
    dev_command: Option<&str>,
) -> Result<DevServerHandle, String> {
    let (npm_script, port) = if let Some(custom_cmd) = dev_command.filter(|s| !s.is_empty()) {
        // Custom command: detect port from the command text, find open port
        let default_port = detect_port_from_script(custom_cmd);
        let port = find_open_port(default_port);
        let resolved = custom_cmd.replace("{port}", &port.to_string());
        if !custom_cmd.contains("{port}") {
            log::warn!("[dev_server] Custom dev_command has no {{port}} placeholder. Server may start on a different port than {} and wait_for_ready will time out.", port);
        }
        (resolved, port)
    } else {
        // Auto-detect from package.json
        let (script_name, default_port) = detect_dev_command(repo_path).await?;
        let port = find_open_port(default_port);
        (script_name, port)
    };

    let is_custom = dev_command.filter(|s| !s.is_empty()).is_some();

    let mut cmd = if is_custom {
        // Custom command: execute via shell to handle paths with spaces and complex commands
        #[cfg(target_os = "windows")]
        {
            let mut c = async_cmd("cmd");
            c.args(["/C", &npm_script]);
            c
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut c = async_cmd("sh");
            c.args(["-c", &npm_script]);
            c
        }
    } else {
        // npm run {script} with port injection
        let script_content = read_script_content(repo_path, &npm_script).await?;
        let port_style = detect_port_style(&script_content);

        let mut c = async_cmd("npm");
        c.args(["run", &npm_script, "--"]);

        match port_style {
            PortStyle::DashDashPort => {
                c.args(["--port", &port.to_string()]);
            }
            PortStyle::DashP => {
                c.args(["-p", &port.to_string()]);
            }
            PortStyle::EnvVar => {
                // For react-scripts, remove the "--" separator and use env var
                let mut c2 = async_cmd("npm");
                c2.args(["run", &npm_script]);
                c2.env("PORT", port.to_string());
                c = c2;
            }
        }
        c
    };

    cmd.current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn dev server: {}", e))?;

    let pid = child.id().unwrap_or(0);

    // Drain stdout/stderr in background tasks to prevent pipe buffer exhaustion.
    // Without this, the dev server will deadlock once the OS pipe buffer fills (~64KB on Windows).
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut stdout = stdout;
            let mut buf = [0u8; 4096];
            loop {
                match stdout.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {} // discard output
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut stderr = stderr;
            let mut buf = [0u8; 4096];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {} // discard output
                }
            }
        });
    }

    log::info!("[dev_server] Started dev server (pid={}, port={}) in {}", pid, port, repo_path);

    Ok(DevServerHandle {
        child,
        port,
        url: format!("http://localhost:{}", port),
        pid,
    })
}

/// Read the actual script content from package.json for a given script name.
async fn read_script_content(repo_path: &str, script_name: &str) -> Result<String, String> {
    let pkg_path = std::path::Path::new(repo_path).join("package.json");
    let content = tokio::fs::read_to_string(&pkg_path).await
        .map_err(|e| format!("Failed to read package.json: {}", e))?;
    let pkg: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse package.json: {}", e))?;

    pkg.get("scripts")
        .and_then(|s| s.get(script_name))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Script '{}' not found in package.json", script_name))
}

/// Wait for the dev server to accept TCP connections on the given port.
pub async fn wait_for_ready(port: u16, timeout_secs: u64) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let addr = format!("127.0.0.1:{}", port);

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Dev server not ready after {}s on port {}", timeout_secs, port));
        }

        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => return Ok(()),
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
}

/// Kill the dev server and its entire process tree.
pub async fn kill_dev_server(mut handle: DevServerHandle) -> Result<(), String> {
    let pid = handle.pid;
    log::info!("[dev_server] Killing dev server (pid={}, port={})", pid, handle.port);

    // On Windows, use taskkill to kill the entire process tree.
    // npm spawns child processes (node, vite, etc.) that won't die from just killing the parent.
    #[cfg(target_os = "windows")]
    {
        if pid > 0 {
            let output = async_cmd("taskkill")
                .args(["/T", "/F", "/PID", &pid.to_string()])
                .output()
                .await;
            match output {
                Ok(o) if o.status.success() => {
                    log::info!("[dev_server] Process tree killed (pid={})", pid);
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    log::warn!("[dev_server] taskkill warning (pid={}): {}", pid, stderr.trim());
                }
                Err(e) => {
                    log::warn!("[dev_server] taskkill failed (pid={}): {}", pid, e);
                }
            }
        }
    }

    // Also try to kill the child directly as a fallback
    let _ = handle.child.kill().await;

    Ok(())
}
