use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

pub struct AppState {
    pub project_root: RwLock<Option<String>>,
    pub preview_port: RwLock<Option<u16>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            project_root: RwLock::new(None),
            preview_port: RwLock::new(None),
        }
    }
}

// ---- Terminal State ----

#[derive(Default)]
pub struct TerminalState {
    pub sessions: Arc<Mutex<HashMap<String, crate::commands::terminal::PtySession>>>,
}

/// A running stdio MCP server session.
pub struct StdioMcpSession {
    pub alive: Arc<AtomicBool>,
    pub next_id: Arc<AtomicU64>,
    pub stdin_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub pending:
        Arc<tokio::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>>,
}

#[derive(Default)]
pub struct StdioMcpState {
    pub sessions: Arc<tokio::sync::Mutex<HashMap<String, StdioMcpSession>>>,
}
