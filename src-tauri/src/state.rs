use parking_lot::RwLock;

pub struct AppState {
    pub project_root: RwLock<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            project_root: RwLock::new(None),
        }
    }
}
