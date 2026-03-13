use std::net::TcpListener;

pub fn find_free_port(range: std::ops::Range<u16>) -> Result<u16, String> {
    for port in range {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err("No free ports available in range".to_string())
}

pub fn find_preview_port() -> Result<u16, String> {
    find_free_port(3100..3200)
}

pub fn find_managed_port() -> Result<u16, String> {
    find_free_port(3000..3100)
}
