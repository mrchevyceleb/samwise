use std::path::Path;
use std::process::Stdio;
use tokio::process::Child;
use crate::process::async_cmd;

/// Single-quote a string for safe embedding in `sh -c` / `cmd /C` input.
/// Used for the --scope argument on the custom-command path where we're
/// building a shell string. Paths with spaces or special characters stay
/// intact.
fn shell_quote(s: &str) -> String {
    // Simple POSIX-style single-quote. Replace any embedded single quote
    // with the '\'' escape sequence. Works for both sh -c and cmd /C since
    // Windows interprets the whole thing as one token either way.
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// Resolve the "main" repo that hosts the Doppler scope. Sam runs dev
/// servers in per-task worktrees under `~/samwise/worktrees/<repo>/<id>`,
/// but Matt's Doppler configs are scoped to the original checkout paths
/// (e.g. `/Users/mjohnst/Documents/KG-Apps/operly`). Worktrees share the
/// same git metadata as the main repo, so `git rev-parse --git-common-dir`
/// returns `<main>/.git`; its parent is the path Doppler knows about.
///
/// Returns the worktree path unchanged when git resolution fails (e.g. no
/// git, or the path isn't a worktree at all).
async fn resolve_main_repo(worktree: &str) -> String {
    let out = async_cmd("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(worktree)
        .output()
        .await;
    let Ok(o) = out else { return worktree.to_string(); };
    if !o.status.success() { return worktree.to_string(); }
    let common = String::from_utf8_lossy(&o.stdout).trim().to_string();
    if common.is_empty() { return worktree.to_string(); }
    let abs = if Path::new(&common).is_absolute() {
        std::path::PathBuf::from(&common)
    } else {
        Path::new(worktree).join(&common)
    };
    abs.parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| worktree.to_string())
}

/// Check whether Doppler is configured for the given scope path.
/// Uses `doppler configure get enclave.project --scope <path> --plain` which
/// returns the project name for that scope (or nothing if no scope exists).
async fn scope_has_doppler_project(scope: &str) -> bool {
    if which::which("doppler").is_err() { return false; }
    for key in ["enclave.project", "project"] {
        let out = async_cmd("doppler")
            .args(["configure", "get", key, "--scope", scope, "--plain"])
            .output()
            .await
            .ok();
        let Some(out) = out else { continue; };
        if !out.status.success() { continue; }
        let project = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !project.is_empty() {
            return true;
        }
    }
    false
}

/// Check likely Doppler scopes for a worktree. Samwise keeps worktrees under
/// `~/samwise/worktrees/<repo>/<task>`, while Matt's Doppler scopes are often
/// saved on the original checkout under `~/Documents/<app-family>/<repo>`.
/// Checking only the git-common-dir path silently launches apps without
/// Supabase/Vite env, which produces loading-splash screenshots and false
/// visual QA failures.
async fn doppler_scope_for(main_repo: &str, worktree: &str) -> Option<String> {
    let explicitly_configured_scopes = configured_doppler_scopes().await;
    for scope in candidate_doppler_scopes(main_repo, worktree, &explicitly_configured_scopes) {
        if explicitly_configured_scopes.iter().any(|configured| configured == &scope)
            || scope_has_doppler_project(&scope).await
        {
            return Some(scope);
        }
    }
    None
}

/// Resolve the Doppler scope Sam should use when running commands for a repo
/// checkout or Samwise worktree.
pub async fn doppler_scope_for_checkout(repo_path: &str) -> Option<String> {
    let main_repo = resolve_main_repo(repo_path).await;
    doppler_scope_for(&main_repo, repo_path).await
}

async fn configured_doppler_scopes() -> Vec<String> {
    if which::which("doppler").is_err() { return Vec::new(); }
    let out = async_cmd("doppler")
        .args(["configure", "--all", "--json"])
        .output()
        .await
        .ok();
    let Some(out) = out else { return Vec::new(); };
    if !out.status.success() { return Vec::new(); }

    let parsed: serde_json::Value = match serde_json::from_slice(&out.stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let Some(obj) = parsed.as_object() else { return Vec::new(); };
    obj.iter()
        .filter_map(|(scope, config)| {
            if scope == "/" || !doppler_config_can_run(config) {
                return None;
            }
            Some(scope.to_string())
        })
        .collect()
}

fn doppler_config_can_run(config: &serde_json::Value) -> bool {
    if ["enclave.project", "project"]
        .iter()
        .any(|key| config.get(*key).and_then(|v| v.as_str()).map(|s| !s.trim().is_empty()).unwrap_or(false))
    {
        return true;
    }

    config
        .get("token")
        .and_then(|v| v.as_str())
        .map(|token| token.starts_with("dp.st."))
        .unwrap_or(false)
}

fn candidate_doppler_scopes(
    main_repo: &str,
    worktree: &str,
    explicitly_configured_scopes: &[String],
) -> Vec<String> {
    let mut scopes = Vec::new();
    push_unique(&mut scopes, main_repo.to_string());
    push_unique(&mut scopes, worktree.to_string());

    let repo_names = repo_names_for_doppler(main_repo, worktree);
    add_matching_configured_scopes(&mut scopes, &repo_names, explicitly_configured_scopes);
    add_documents_repo_scopes(&mut scopes, &repo_names);

    for path in [main_repo, worktree] {
        add_mirrored_documents_scope(&mut scopes, path);
    }

    scopes
}

fn push_unique(scopes: &mut Vec<String>, scope: String) {
    if !scope.is_empty() && !scopes.iter().any(|s| s == &scope) {
        scopes.push(scope);
    }
}

fn repo_names_for_doppler(main_repo: &str, worktree: &str) -> Vec<String> {
    let mut names = Vec::new();
    for path in [main_repo, worktree] {
        if let Some(name) = Path::new(path).file_name().and_then(|s| s.to_str()) {
            push_unique(&mut names, name.to_string());
        }
        if let Some(name) = samwise_worktree_repo_name(path) {
            push_unique(&mut names, name);
        }
    }
    names
}

fn samwise_worktree_repo_name(path: &str) -> Option<String> {
    let Ok(home) = std::env::var("HOME") else { return None; };
    let marker = format!("{}/samwise/worktrees/", home);
    let rest = path.strip_prefix(&marker)?;
    let repo_name = rest.split('/').next().unwrap_or("").trim();
    if repo_name.is_empty() { return None; }
    Some(repo_name.to_string())
}

fn add_matching_configured_scopes(
    scopes: &mut Vec<String>,
    repo_names: &[String],
    explicitly_configured_scopes: &[String],
) {
    for scope in explicitly_configured_scopes {
        let Some(name) = Path::new(scope).file_name().and_then(|s| s.to_str()) else { continue; };
        if repo_names.iter().any(|repo_name| repo_name == name) {
            push_unique(scopes, scope.to_string());
        }
    }
}

fn add_documents_repo_scopes(scopes: &mut Vec<String>, repo_names: &[String]) {
    let Some(home) = dirs::home_dir() else { return; };
    let documents = home.join("Documents");
    let Ok(entries) = std::fs::read_dir(documents) else { return; };

    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else { continue; };
        if !metadata.is_dir() { continue; }
        for repo_name in repo_names {
            let candidate = entry.path().join(repo_name);
            if candidate.is_dir() {
                push_unique(scopes, candidate.to_string_lossy().into_owned());
            }
        }
    }
}

fn add_mirrored_documents_scope(scopes: &mut Vec<String>, path: &str) {
    let Ok(home) = std::env::var("HOME") else { return; };
    let marker = format!("{}/samwise/", home);
    let Some(rest) = path.strip_prefix(&marker) else { return; };
    let mut parts = rest.split('/');
    let Some(bucket) = parts.next().filter(|s| !s.is_empty() && *s != "worktrees") else { return; };
    let Some(repo_name) = parts.next().filter(|s| !s.is_empty()) else { return; };

    push_unique(scopes, format!("{}/Documents/{}/{}", home, bucket, repo_name));

    if let Some(prefix) = bucket.strip_suffix("-Apps") {
        let project_bucket = format!("{}-PROJECTS", prefix.to_ascii_uppercase());
        push_unique(scopes, format!("{}/Documents/{}/{}", home, project_bucket, repo_name));
    }
}

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

    // Sam runs dev in a per-task worktree, but Doppler's scope is keyed to
    // the main repo path. Resolve the main, then ask Doppler whether it has
    // a scope for it. When present, every spawn is wrapped with
    // `doppler run --scope <main> --` so env vars resolve correctly.
    let main_repo = resolve_main_repo(repo_path).await;
    let doppler_scope = doppler_scope_for(&main_repo, repo_path).await;
    if let Some(scope) = doppler_scope.as_deref() {
        log::info!("[dev_server] wrapping dev server with `doppler run --scope {}` (main repo of worktree {})", scope, repo_path);
    } else {
        log::info!("[dev_server] no doppler scope found for main repo {}; running dev server bare", main_repo);
    }

    let mut cmd = if is_custom {
        // Custom command: execute via shell to handle paths with spaces and complex commands.
        // Prefix with `doppler run --scope <path> --` when the main repo has a Doppler scope.
        let shell_script = if let Some(scope) = doppler_scope.as_deref() {
            format!("doppler run --scope {} -- {}", shell_quote(scope), npm_script)
        } else {
            npm_script.clone()
        };
        #[cfg(target_os = "windows")]
        {
            let mut c = async_cmd("cmd");
            c.args(["/C", &shell_script]);
            c
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut c = async_cmd("sh");
            c.args(["-c", &shell_script]);
            c
        }
    } else {
        // npm run {script} with port injection.
        let script_content = read_script_content(repo_path, &npm_script).await?;
        let port_style = detect_port_style(&script_content);

        // When Doppler has a scope, the outer program is `doppler run --scope
        // <main> --` and the `npm` invocation moves into its args. Port
        // flags / env handling are preserved.
        let mut c = if let Some(scope) = doppler_scope.as_deref() {
            let mut c0 = async_cmd("doppler");
            c0.args(["run", "--scope", scope, "--", "npm", "run", &npm_script, "--"]);
            c0
        } else {
            let mut c0 = async_cmd("npm");
            c0.args(["run", &npm_script, "--"]);
            c0
        };

        match port_style {
            PortStyle::DashDashPort => {
                c.args(["--port", &port.to_string()]);
            }
            PortStyle::DashP => {
                c.args(["-p", &port.to_string()]);
            }
            PortStyle::EnvVar => {
                // react-scripts style: no `--` separator, port comes via env var.
                // Rebuild without the trailing `--` so react-scripts doesn't see
                // an unexpected empty argument.
                let mut c2 = if let Some(scope) = doppler_scope.as_deref() {
                    let mut cc = async_cmd("doppler");
                    cc.args(["run", "--scope", scope, "--", "npm", "run", &npm_script]);
                    cc
                } else {
                    let mut cc = async_cmd("npm");
                    cc.args(["run", &npm_script]);
                    cc
                };
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
