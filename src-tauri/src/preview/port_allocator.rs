use std::net::TcpListener;

/// Check if a port is truly free on all interfaces (IPv4 + IPv6).
/// Returns true only if no existing process holds the port.
fn is_port_free(port: u16) -> bool {
    // Check IPv4 all-interfaces
    if TcpListener::bind(("0.0.0.0", port)).is_err() {
        return false;
    }
    // Check IPv6 all-interfaces (Next.js binds to :: by default)
    if TcpListener::bind(("[::]", port)).is_err() {
        return false;
    }
    true
}

pub fn find_free_port(range: std::ops::Range<u16>) -> Result<u16, String> {
    for port in range {
        if is_port_free(port) {
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
/// Uses high ephemeral range (9100-9999) to avoid conflicts with
/// common dev ports (3000, 5173, 8080, etc.).
pub fn find_managed_port() -> Result<u16, String> {
    find_free_port(9100..10000)
}
