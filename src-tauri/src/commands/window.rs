use tauri::AppHandle;
use uuid::Uuid;

/// Helper: create a new project window with given workspace path and theme
fn create_project_window(app: &AppHandle, path: &str, theme: &str) -> Result<String, String> {
    let label = format!("project-{}", &Uuid::new_v4().to_string()[..8]);

    let encoded_path = urlencoding::encode(path);
    let encoded_theme = urlencoding::encode(theme);

    if cfg!(debug_assertions) {
        let url = format!(
            "http://localhost:5173/?workspace={}&theme={}",
            encoded_path, encoded_theme
        );
        let _window = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::External(url.parse().map_err(|e| format!("Invalid URL: {}", e))?),
        )
        .title(format!(
            "Banana Code - {}",
            path.split(['/', '\\']).last().unwrap_or(path)
        ))
        .inner_size(1440.0, 900.0)
        .min_inner_size(1024.0, 600.0)
        .decorations(false)
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;
    } else {
        let url = format!("index.html?workspace={}&theme={}", encoded_path, encoded_theme);
        let _window = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App(url.into()),
        )
        .title(format!(
            "Banana Code - {}",
            path.split(['/', '\\']).last().unwrap_or(path)
        ))
        .inner_size(1440.0, 900.0)
        .min_inner_size(1024.0, 600.0)
        .decorations(false)
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;
    }

    Ok(label)
}

#[tauri::command]
pub fn open_folder_in_new_window(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app
        .dialog()
        .file()
        .set_title("Open Folder in New Window")
        .blocking_pick_folder();

    match folder {
        Some(path) => {
            let path_str = path
                .as_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string());
            create_project_window(&app, &path_str, "banana")
        }
        None => Err("No folder selected".to_string()),
    }
}

#[tauri::command]
pub async fn open_path_in_new_window(
    app: AppHandle,
    path: String,
    theme: Option<String>,
) -> Result<String, String> {
    let theme = theme.unwrap_or_else(|| "banana".to_string());
    create_project_window(&app, &path, &theme)
}

#[tauri::command]
pub async fn git_clone_repo(url: String, target_dir: String) -> Result<String, String> {
    let output = tokio::process::Command::new("git")
        .args(["clone", &url, &target_dir])
        .output()
        .await
        .map_err(|e| format!("Failed to run git clone: {}", e))?;

    if output.status.success() {
        Ok(target_dir)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git clone failed: {}", stderr))
    }
}
