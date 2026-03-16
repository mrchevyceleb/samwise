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

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok(data_dir.join("settings.json"))
}
