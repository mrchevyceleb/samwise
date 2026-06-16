use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Save settings as a raw JSON string.
/// The frontend owns the schema; we just persist the blob.
#[tauri::command]
pub async fn save_settings(app: AppHandle, data: String) -> Result<(), String> {
    let path = settings_path(&app)?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

/// Load settings as a raw JSON string.
/// Returns the stored JSON or an empty string if no file exists.
#[tauri::command]
pub async fn load_settings(app: AppHandle) -> Result<String, String> {
    let path = settings_path(&app)?;
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

/// True when the WebView is running with GPU compositing disabled
/// (software rendering). On the Spark the systemd service sets
/// `WEBKIT_DISABLE_COMPOSITING_MODE=1` / `WEBKIT_DISABLE_DMABUF_RENDERER=1`
/// because the GPU path black-screens on this NVIDIA box. With no compositor
/// layers, every CSS animation frame forces a full software repaint on the
/// WebKit main thread, so a few always-on `infinite` animations were enough to
/// pin that thread at ~100% CPU and make clicks lag for seconds. The frontend
/// reads this to enable "perf-lite" (stop continuous animations, drop blur).
/// GPU-composited hosts (e.g. the Windows workstation) report `false` and keep
/// the full animation set.
#[tauri::command]
pub fn perf_lite_mode() -> bool {
    let enabled = |k: &str| matches!(std::env::var(k).ok().as_deref(), Some("1") | Some("true"));
    enabled("WEBKIT_DISABLE_COMPOSITING_MODE") || enabled("WEBKIT_DISABLE_DMABUF_RENDERER")
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok(data_dir.join("settings.json"))
}
