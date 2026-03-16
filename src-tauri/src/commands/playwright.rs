use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct ScreenshotResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn playwright_screenshot(
    url: String,
    viewport: Option<String>,
    output_path: String,
) -> Result<ScreenshotResult, String> {
    let viewport_str = viewport.unwrap_or_else(|| "1280,720".to_string());
    let parts: Vec<&str> = viewport_str.split(',').collect();
    let (width, height) = if parts.len() == 2 {
        let w: u32 = parts[0].trim().parse().unwrap_or(1280);
        let h: u32 = parts[1].trim().parse().unwrap_or(720);
        (w, h)
    } else {
        (1280, 720)
    };

    let viewport_arg = format!("{},{}", width, height);

    let output = tokio::process::Command::new("npx")
        .args([
            "playwright",
            "screenshot",
            &url,
            &output_path,
            "--viewport-size",
            &viewport_arg,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run playwright: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Playwright screenshot failed: {}", stderr.trim()));
    }

    Ok(ScreenshotResult {
        path: output_path,
        width,
        height,
    })
}

#[tauri::command]
pub async fn playwright_screenshot_mobile(
    url: String,
    output_path: String,
) -> Result<ScreenshotResult, String> {
    // iPhone 14 Pro viewport
    let width = 393;
    let height = 852;
    let viewport_arg = format!("{},{}", width, height);

    let output = tokio::process::Command::new("npx")
        .args([
            "playwright",
            "screenshot",
            &url,
            &output_path,
            "--viewport-size",
            &viewport_arg,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run playwright: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Playwright mobile screenshot failed: {}", stderr.trim()));
    }

    Ok(ScreenshotResult {
        path: output_path,
        width,
        height,
    })
}
