/// Health check commands for the setup wizard.
/// Verifies that external dependencies (Claude Code CLI, gh CLI) are available.

use crate::process::async_cmd;

/// Check if Claude Code CLI is installed and return the version string.
#[tauri::command]
pub async fn check_claude_code() -> Result<String, String> {
    let (exe, prefix_args) = super::claude_code::find_claude_command();

    let mut cmd = async_cmd(&exe);
    for arg in &prefix_args {
        cmd.arg(arg);
    }
    cmd.arg("--version");

    let output = cmd.output().await.map_err(|e| format!("Claude Code not found: {}", e))?;

    if !output.status.success() {
        return Err("Claude Code CLI not installed or not in PATH".to_string());
    }

    // Some CLI wrappers print version to stderr
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(combined.trim().to_string())
}

/// Check if GitHub CLI (gh) is authenticated.
#[tauri::command]
pub async fn check_gh_auth() -> Result<String, String> {
    let output = async_cmd("gh")
        .args(["auth", "status"])
        .output()
        .await
        .map_err(|e| format!("gh CLI not found: {}", e))?;

    // gh auth status outputs to stderr on success
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() {
        return Err(format!("Not authenticated: {}", combined.trim()));
    }

    Ok(combined.trim().to_string())
}

/// Check if the LLM proxy (LiteLLM) is reachable and healthy.
/// Returns the proxy URL on success, or an error message.
#[tauri::command]
pub async fn check_llm_proxy(proxy_url: String) -> Result<String, String> {
    if proxy_url.trim().is_empty() {
        return Err("No proxy URL configured".to_string());
    }

    let health_url = format!("{}/health", proxy_url.trim().trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(&health_url)
        .send()
        .await
        .map_err(|e| format!("Proxy at {} unreachable: {}", proxy_url, e))?;

    if resp.status().is_success() {
        Ok(format!("Proxy healthy at {}", proxy_url))
    } else {
        Err(format!("Proxy returned status {}", resp.status()))
    }
}

/// Check if Doppler CLI is available.
#[tauri::command]
pub async fn check_doppler() -> Result<String, String> {
    let output = async_cmd("doppler")
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Doppler CLI not found: {}", e))?;

    if !output.status.success() {
        return Err("Doppler CLI not installed or not in PATH".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
