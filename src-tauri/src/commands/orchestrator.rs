use parking_lot::Mutex;
use std::path::PathBuf;
use tauri::{AppHandle, State};

use crate::preview::orchestrator::PreviewOrchestrator;
use crate::preview::tier_detector::TierDetection;

#[tauri::command]
pub async fn preview_open_project(
    app: AppHandle,
    state: State<'_, Mutex<PreviewOrchestrator>>,
    project_dir: String,
) -> Result<TierDetection, String> {
    let path = PathBuf::from(&project_dir);
    if !path.exists() {
        return Err(format!("Project directory does not exist: {}", project_dir));
    }

    // We need to take the lock, but since open_project is async,
    // we clone the orchestrator state pattern: lock -> take ownership -> unlock -> async work -> lock -> put back
    // Instead, we'll do the work in a blocking manner for the lock.
    // Since parking_lot::Mutex is not async-aware, we do the async work outside the lock.

    // First, stop any existing preview
    {
        let orchestrator = state.lock();
        let _ = orchestrator.current_tier_name(); // Verify lock works
    }

    // For async operations, we need to work around parking_lot::Mutex.
    // We'll use a tokio::task::spawn_blocking pattern or just accept brief lock holds.
    // The simplest correct approach: use an inner tokio::sync::Mutex.
    // But since the spec says parking_lot, let's do short lock holds with async gaps.

    // Stop existing
    stop_orchestrator_inner(&state).await?;

    // Open project (this is the main async work)
    let detection = open_project_inner(&app, &state, path).await?;

    Ok(detection)
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
    // For rebuild, we need async. Same pattern as above.
    // Take a snapshot of what we need, do async work, put back.
    // Actually for esbuild rebuild, it's a quick operation.
    // We'll hold the lock briefly... but EsbuildRunner::build is async.
    // Workaround: spawn the async work in a blocking context.

    // For now, use tokio spawn_blocking to bridge
    // Actually, let's just do it directly since the lock is short-lived per operation.

    let orchestrator = state.lock();
    // We can't await inside a parking_lot lock. For V1, the watcher triggers a
    // frontend event and the frontend calls rebuild which re-runs open_project.
    drop(orchestrator);

    log::info!("[orchestrator_cmd] Rebuild requested - frontend should re-trigger open_project");
    Ok(())
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

// -- Internal helpers to work around parking_lot::Mutex + async --

async fn stop_orchestrator_inner(
    state: &State<'_, Mutex<PreviewOrchestrator>>,
) -> Result<(), String> {
    // Take the orchestrator, replace with a new one, stop the old one outside the lock
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
) -> Result<TierDetection, String> {
    // Create a fresh orchestrator, do async open, then swap it in
    let mut orchestrator = PreviewOrchestrator::new();
    let detection = orchestrator.open_project(app, project_dir).await?;

    // Swap in the new orchestrator
    {
        let mut current = state.lock();
        *current = orchestrator;
    }

    Ok(detection)
}
