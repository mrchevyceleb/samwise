/// Helpers that wrap Command::new and apply CREATE_NO_WINDOW on Windows
/// so spawned processes never flash a console window.

/// Create a `std::process::Command` with CREATE_NO_WINDOW on Windows.
pub fn cmd(program: &str) -> std::process::Command {
    let mut c = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    c
}

/// Create a `tokio::process::Command` with CREATE_NO_WINDOW on Windows.
pub fn async_cmd(program: &str) -> tokio::process::Command {
    let mut c = tokio::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    c
}
