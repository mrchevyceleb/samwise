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
///
/// Binds to both IPv4 and IPv6 loopback to catch dual-stack conflicts. On macOS,
/// `TcpListener::bind("127.0.0.1:port")` will succeed even when something else is
/// listening on `[::1]:port` — so we have to check both. This was the cause of
/// the "screenshots show a different app" bug: find_open_port returned a port that
/// was actually owned by another app on the IPv6 side.
fn port_is_free(port: u16) -> bool {
    let v4 = std::net::TcpListener::bind(format!("127.0.0.1:{}", port));
    let v6 = std::net::TcpListener::bind(format!("[::1]:{}", port));
    v4.is_ok() && v6.is_ok()
}

pub fn find_open_port(starting_from: u16) -> u16 {
    for port in starting_from..=starting_from + 20 {
        if port_is_free(port) {
            return port;
        }
    }
    for port in 10000..10100 {
        if port_is_free(port) {
            return port;
        }
    }
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
    // A custom dev_command is only honored when it has a {port} placeholder.
    // Without the placeholder we can't guarantee the server binds where we're watching,
    // which leads to screenshots of whatever random process happens to own the port.
    // Sam always binds in his own dedicated high-port range (47100+). Framework
    // defaults like 5173 (Vite) or 3000 (Next) are collision magnets with other apps
    // Matt runs on the mini. We still care about framework detection for the port
    // *flag style* (--port vs -p vs PORT env) but the port number is always ours.
    const SAM_PORT_START: u16 = 47100;
    let port = find_open_port(SAM_PORT_START);

    let custom_with_port = dev_command
        .filter(|s| !s.is_empty() && s.contains("{port}"));

    let npm_script = if let Some(custom_cmd) = custom_with_port {
        custom_cmd.replace("{port}", &port.to_string())
    } else {
        if dev_command.filter(|s| !s.is_empty()).is_some() {
            log::info!("[dev_server] dev_command has no {{port}} placeholder; falling back to package.json auto-detect so we can inject port {}.", port);
        }
        let (script_name, _) = detect_dev_command(repo_path).await?;
        script_name
    };

    let is_custom = custom_with_port.is_some();

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

/// Wait until the dev server responds at `url` with a non-error HTTP status.
///
/// TCP-accept is not enough: some other process on the machine can be listening
/// on the port we think we reserved (e.g. because our dev server bound a different
/// port than expected and a stray app owns this one). Hitting HTTP and checking
/// status ensures we're actually talking to a real app, not a random listener.
///
/// 2xx/3xx = ready. 4xx/5xx keeps polling until timeout. A 404 is still "someone's
/// answering, just not with our app yet" — if it persists the whole timeout window
/// the caller will skip screenshots rather than capture the wrong thing.
pub async fn wait_for_ready(url: &str, timeout_secs: u64) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("reqwest build: {}", e))?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Dev server didn't return 2xx/3xx at {} within {}s", url, timeout_secs));
        }
        if let Ok(resp) = client.get(url).send().await {
            let s = resp.status();
            if s.is_success() || s.is_redirection() {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// Kill the dev server and its entire process tree.
pub async fn kill_dev_server(mut handle: DevServerHandle) -> Result<(), String> {
    let pid = handle.pid;
    log::info!("[dev_server] Killing dev server (pid={}, port={})", pid, handle.port);

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

    // On macOS/Linux, npm -> sh -> vite is 2-3 levels deep; directly killing the child
    // only gets npm. Walk the descendant tree via pgrep -P and TERM each PID.
    #[cfg(not(target_os = "windows"))]
    {
        if pid > 0 {
            kill_tree_unix(pid).await;
        }
    }

    let _ = handle.child.kill().await;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn kill_tree_unix(pid: u32) {
    // BFS through the descendant tree using pgrep -P.
    let mut frontier = vec![pid];
    let mut all = vec![pid];
    while let Some(parent) = frontier.pop() {
        if let Ok(out) = async_cmd("pgrep").args(["-P", &parent.to_string()]).output().await {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    if let Ok(child) = line.trim().parse::<u32>() {
                        all.push(child);
                        frontier.push(child);
                    }
                }
            }
        }
    }
    // TERM leaves first so parents don't respawn children, then mop up with KILL.
    for p in all.iter().rev() {
        let _ = async_cmd("kill").args(["-TERM", &p.to_string()]).output().await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    for p in all.iter().rev() {
        let _ = async_cmd("kill").args(["-KILL", &p.to_string()]).output().await;
    }
}
