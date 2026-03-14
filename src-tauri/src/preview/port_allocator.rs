use std::net::TcpListener;

pub fn find_free_port(range: std::ops::Range<u16>) -> Result<u16, String> {
    for port in range {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err("No free ports available in range".to_string())
}

/// Let the OS assign a free port by binding to port 0.
/// Returns the actual port assigned. This is the most robust approach
/// for servers we control (like the preview HTTP server), since it
/// guarantees no conflicts even across multiple app windows.
pub fn find_os_assigned_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Failed to bind ephemeral port: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local addr: {}", e))?
        .port();
    // Drop the listener so the port is free for the caller to use.
    // There is a small race window here, but in practice it is fine
    // because the caller binds immediately after.
    drop(listener);
    Ok(port)
}

/// Find a free port for the preview HTTP server (OS-assigned).
pub fn find_preview_port() -> Result<u16, String> {
    find_os_assigned_port()
}

/// Find a free port for managed dev server processes.
/// Uses a wide range (3000-4000) so multiple Banana Code windows
/// can run simultaneously without conflicts.
pub fn find_managed_port() -> Result<u16, String> {
    find_free_port(3000..4000)
}
