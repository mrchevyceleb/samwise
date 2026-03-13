use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Directories to ignore when watching for file changes
const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".svelte-kit",
    ".next",
    ".nuxt",
    ".output",
    ".cache",
    ".parcel-cache",
    ".banana-preview",
    "__pycache__",
];

/// File extensions to ignore
const IGNORE_EXTENSIONS: &[&str] = &[
    "lock",
    "log",
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub paths: Vec<String>,
}

pub struct PreviewWatcher {
    watcher: Option<RecommendedWatcher>,
    // Keep the thread handle so it stays alive
    _debounce_handle: Option<std::thread::JoinHandle<()>>,
}

impl PreviewWatcher {
    pub fn start(project_dir: PathBuf, app: AppHandle) -> Result<Self, String> {
        log::info!("[watcher] Starting file watcher on {}", project_dir.display());

        let (tx, rx) = mpsc::channel::<Event>();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        watcher
            .watch(&project_dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        // Spawn debounce thread
        let debounce_handle = std::thread::spawn(move || {
            debounce_loop(rx, app);
        });

        Ok(Self {
            watcher: Some(watcher),
            _debounce_handle: Some(debounce_handle),
        })
    }

    pub fn stop(&mut self) {
        log::info!("[watcher] Stopping file watcher");
        self.watcher = None;
        // The debounce thread will exit when the channel closes
    }
}

impl Drop for PreviewWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

fn debounce_loop(rx: mpsc::Receiver<Event>, app: AppHandle) {
    let debounce_duration = Duration::from_millis(200);
    let mut last_emit = Instant::now() - debounce_duration;
    let mut pending_paths: Vec<String> = Vec::new();

    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => {
                // Filter out ignored paths
                for path in &event.paths {
                    if should_ignore_path(path) {
                        continue;
                    }
                    if let Some(path_str) = path.to_str() {
                        if !pending_paths.contains(&path_str.to_string()) {
                            pending_paths.push(path_str.to_string());
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Check if we should emit
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                log::info!("[watcher] Channel disconnected, stopping debounce loop");
                break;
            }
        }

        // Emit if debounce period has passed and we have pending changes
        if !pending_paths.is_empty() && last_emit.elapsed() >= debounce_duration {
            let paths = std::mem::take(&mut pending_paths);
            log::info!("[watcher] File changes detected: {:?}", &paths[..paths.len().min(5)]);

            let _ = app.emit("preview:file-changed", FileChangeEvent { paths });
            last_emit = Instant::now();
        }
    }
}

fn should_ignore_path(path: &std::path::Path) -> bool {
    // Check directory components
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if IGNORE_DIRS.contains(&name_str) {
                    return true;
                }
            }
        }
    }

    // Check file extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if IGNORE_EXTENSIONS.contains(&ext) {
            return true;
        }
    }

    false
}
