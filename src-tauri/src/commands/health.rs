/// Health check commands for the setup wizard.
/// Verifies that external dependencies (Claude Code CLI, gh CLI) are available.

/// Check if Claude Code CLI is installed and return the version string.
#[tauri::command]
pub async fn check_claude_code() -> Result<String, String> {
    let (exe, prefix_args) = super::worker::find_claude_command();

    let mut cmd = tokio::process::Command::new(&exe);
    for arg in &prefix_args {
        cmd.arg(arg);
    }
    cmd.arg("--version");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

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
    let output = tokio::process::Command::new("gh")
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

/// Check if Doppler CLI is available.
#[tauri::command]
pub async fn check_doppler() -> Result<String, String> {
    let output = tokio::process::Command::new("doppler")
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Doppler CLI not found: {}", e))?;

    if !output.status.success() {
        return Err("Doppler CLI not installed or not in PATH".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
