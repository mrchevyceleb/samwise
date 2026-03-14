use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

use crate::preview::orchestrator::PreviewOrchestrator;
use crate::preview::tier_detector::TierDetection;

#[tauri::command]
pub async fn preview_open_project(
    app: AppHandle,
    state: State<'_, Mutex<PreviewOrchestrator>>,
    project_dir: String,
    env_vars: Option<HashMap<String, String>>,
) -> Result<TierDetection, String> {
    let path = PathBuf::from(&project_dir);
    if !path.exists() {
        return Err(format!("Project directory does not exist: {}", project_dir));
    }

    stop_orchestrator_inner(&state).await?;
    open_project_inner(&app, &state, path, env_vars.unwrap_or_default()).await
}

#[tauri::command]
pub async fn preview_scan_env_keys(
    project_dir: String,
) -> Result<Vec<String>, String> {
    let path = PathBuf::from(&project_dir);
    if !path.exists() {
        return Err(format!("Project directory does not exist: {}", project_dir));
    }

    let mut keys = Vec::new();
    let env_files = [
        ".env.example",
        ".env.local.example",
        ".env.sample",
        ".env.template",
        ".env",
        ".env.local",
        ".env.development",
        ".env.development.local",
    ];

    for filename in &env_files {
        let file_path = path.join(filename);
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some(eq_pos) = trimmed.find('=') {
                        let key = trimmed[..eq_pos].trim().to_string();
                        if !key.is_empty() && !keys.contains(&key) {
                            keys.push(key);
                        }
                    }
                }
            }
        }
    }

    // Also scan source files for process.env.* and import.meta.env.* references
    if keys.is_empty() {
        let re_process = regex::Regex::new(r"process\.env\.([A-Z][A-Z0-9_]+)").unwrap();
        let re_import = regex::Regex::new(r"import\.meta\.env\.([A-Z][A-Z0-9_]+)").unwrap();

        let src_dir = path.join("src");
        let dirs_to_scan = if src_dir.exists() {
            vec![src_dir, path.clone()]
        } else {
            vec![path.clone()]
        };

        for dir in dirs_to_scan {
            scan_dir_for_env_refs(&dir, &re_process, &re_import, &mut keys, 3);
        }
    }

    Ok(keys)
}

/// Recursively scan directory for process.env.* references in source files
fn scan_dir_for_env_refs(
    dir: &std::path::Path,
    re_process: &regex::Regex,
    re_import: &regex::Regex,
    keys: &mut Vec<String>,
    max_depth: usize,
) {
    if max_depth == 0 { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let skip_dirs = ["node_modules", ".git", ".next", "dist", "build", ".banana-preview"];
    let source_exts = ["ts", "tsx", "js", "jsx", "mts", "mjs"];

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if !skip_dirs.contains(&name.as_str()) {
                scan_dir_for_env_refs(&path, re_process, re_import, keys, max_depth - 1);
            }
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !source_exts.contains(&ext) { continue; }

        if let Ok(content) = std::fs::read_to_string(&path) {
            for cap in re_process.captures_iter(&content) {
                let key = cap[1].to_string();
                if !keys.contains(&key) && key != "NODE_ENV" {
                    keys.push(key);
                }
            }
            for cap in re_import.captures_iter(&content) {
                let key = cap[1].to_string();
                if !keys.contains(&key) && key != "NODE_ENV" {
                    keys.push(key);
                }
            }
        }

        // Cap at 20 keys to avoid scanning forever
        if keys.len() >= 20 { return; }
    }
}

#[tauri::command]
pub async fn preview_stop(
    state: State<'_, Mutex<PreviewOrchestrator>>,
) -> Result<(), String> {
    stop_orchestrator_inner(&state).await
}

#[tauri::command]
pub async fn preview_get_url(
    state: State<'_, Mutex<PreviewOrchestrator>>,
) -> Result<Option<String>, String> {
    let orchestrator = state.lock();
    Ok(orchestrator.current_url())
}

#[tauri::command]
pub async fn preview_get_tier(
    state: State<'_, Mutex<PreviewOrchestrator>>,
) -> Result<Option<String>, String> {
    let orchestrator = state.lock();
    Ok(orchestrator.current_tier_name())
}

#[tauri::command]
pub async fn preview_rebuild(
    state: State<'_, Mutex<PreviewOrchestrator>>,
) -> Result<(), String> {
    let mut orchestrator = {
        let mut current = state.lock();
        std::mem::replace(&mut *current, PreviewOrchestrator::new())
    };

    let result = orchestrator.rebuild().await;

    {
        let mut current = state.lock();
        *current = orchestrator;
    }

    result
}

#[tauri::command]
pub async fn preview_detect_tier(
    project_dir: String,
) -> Result<TierDetection, String> {
    let path = PathBuf::from(&project_dir);
    if !path.exists() {
        return Err(format!("Project directory does not exist: {}", project_dir));
    }
    Ok(crate::preview::tier_detector::detect_tier(&path))
}

#[tauri::command]
pub async fn preview_save_env_file(
    project_dir: String,
    env_vars: HashMap<String, String>,
) -> Result<(), String> {
    let path = PathBuf::from(&project_dir);
    if !path.exists() {
        return Err(format!("Project directory does not exist: {}", project_dir));
    }

    // Write .banana-env file
    let env_path = path.join(".banana-env");
    let mut content = String::from("# Banana Code IDE - Environment Variables\n# This file is auto-generated. Add to .gitignore.\n");
    let mut sorted_keys: Vec<&String> = env_vars.keys().collect();
    sorted_keys.sort();
    for key in sorted_keys {
        if let Some(value) = env_vars.get(key) {
            content.push_str(&format!("{}={}\n", key, value));
        }
    }
    std::fs::write(&env_path, &content)
        .map_err(|e| format!("Failed to write .banana-env: {}", e))?;

    // Auto-add .banana-env to .gitignore
    let gitignore_path = path.join(".gitignore");
    if gitignore_path.exists() {
        let gitignore_content = std::fs::read_to_string(&gitignore_path)
            .map_err(|e| format!("Failed to read .gitignore: {}", e))?;
        let already_listed = gitignore_content.lines().any(|line| line.trim() == ".banana-env");
        if !already_listed {
            let mut new_content = gitignore_content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(".banana-env\n");
            std::fs::write(&gitignore_path, &new_content)
                .map_err(|e| format!("Failed to update .gitignore: {}", e))?;
        }
    } else {
        std::fs::write(&gitignore_path, ".banana-env\n")
            .map_err(|e| format!("Failed to create .gitignore: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn preview_load_env_file(
    project_dir: String,
) -> Result<HashMap<String, String>, String> {
    let path = PathBuf::from(&project_dir);
    let env_path = path.join(".banana-env");

    if !env_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&env_path)
        .map_err(|e| format!("Failed to read .banana-env: {}", e))?;

    let mut vars = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let value = trimmed[eq_pos + 1..].trim().to_string();
            if !key.is_empty() {
                vars.insert(key, value);
            }
        }
    }

    Ok(vars)
}

// -- Internal helpers --

async fn stop_orchestrator_inner(
    state: &State<'_, Mutex<PreviewOrchestrator>>,
) -> Result<(), String> {
    let mut old = {
        let mut orchestrator = state.lock();
        std::mem::replace(&mut *orchestrator, PreviewOrchestrator::new())
    };
    old.stop().await
}

async fn open_project_inner(
    app: &AppHandle,
    state: &State<'_, Mutex<PreviewOrchestrator>>,
    project_dir: PathBuf,
    env_vars: HashMap<String, String>,
) -> Result<TierDetection, String> {
    let mut orchestrator = PreviewOrchestrator::new();
    orchestrator.set_env_vars(env_vars);

    // Resolve esbuild sidecar binary path
    if let Ok(resource_dir) = app.path().resource_dir() {
        let sidecar = if cfg!(windows) {
            resource_dir.join("binaries/esbuild-x86_64-pc-windows-msvc.exe")
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                resource_dir.join("binaries/esbuild-aarch64-apple-darwin")
            } else {
                resource_dir.join("binaries/esbuild-x86_64-apple-darwin")
            }
        } else {
            resource_dir.join("binaries/esbuild-x86_64-unknown-linux-gnu")
        };
        orchestrator.set_esbuild_sidecar(sidecar);
    }

    let detection = orchestrator.open_project(app, project_dir).await?;

    {
        let mut current = state.lock();
        *current = orchestrator;
    }

    Ok(detection)
}
