/// Helpers that wrap Command::new and apply CREATE_NO_WINDOW on Windows
/// so spawned processes never flash a console window.
///
/// Also handles CLI binary resolution. When Samwise runs under launchd (as an
/// always-on server) PATH can vary; rather than trust the PATH we resolve
/// common CLIs (gh, git, doppler, claude) to absolute paths once at first use.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Create a `std::process::Command` with CREATE_NO_WINDOW on Windows.
/// Resolves well-known CLI names (gh, git, doppler) to absolute paths so
/// launchd-spawned processes don't ENOENT on PATH misses.
pub fn cmd(program: &str) -> std::process::Command {
    let resolved = resolve_bin(program);
    let mut c = std::process::Command::new(resolved.as_deref().unwrap_or(program));
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    c
}

/// Create a `tokio::process::Command` with CREATE_NO_WINDOW on Windows.
pub fn async_cmd(program: &str) -> tokio::process::Command {
    let resolved = resolve_bin(program);
    let mut c = tokio::process::Command::new(resolved.as_deref().unwrap_or(program));
    #[cfg(target_os = "windows")]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    c
}

fn cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve a bare CLI name to an absolute path. Returns None if the program
/// already contains a path separator (already absolute) or if resolution fails.
fn resolve_bin(program: &str) -> Option<String> {
    if program.contains('/') || program.contains('\\') {
        return None;
    }
    if let Ok(cache) = cache().lock() {
        if let Some(hit) = cache.get(program) {
            return hit.clone();
        }
    }
    let found = lookup(program);
    if let Ok(mut cache) = cache().lock() {
        cache.insert(program.to_string(), found.clone());
    }
    found
}

fn lookup(program: &str) -> Option<String> {
    if let Ok(p) = which::which(program) {
        return Some(p.to_string_lossy().into_owned());
    }
    for base in common_bin_dirs() {
        let p = base.join(program);
        if p.exists() {
            return Some(p.to_string_lossy().into_owned());
        }
        #[cfg(target_os = "windows")]
        {
            let pexe = base.join(format!("{}.exe", program));
            if pexe.exists() {
                return Some(pexe.to_string_lossy().into_owned());
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn common_bin_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        v.insert(0, PathBuf::from(format!("{}/.local/bin", home)));
        v.push(PathBuf::from(format!("{}/.cargo/bin", home)));
    }
    v
}

#[cfg(target_os = "windows")]
fn common_bin_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = vec![];
    if let Ok(home) = std::env::var("USERPROFILE") {
        v.push(PathBuf::from(format!("{}\\.local\\bin", home)));
        v.push(PathBuf::from(format!("{}\\AppData\\Local\\Programs\\GitHub CLI", home)));
    }
    v.push(PathBuf::from("C:\\Program Files\\Git\\cmd"));
    v.push(PathBuf::from("C:\\Program Files\\GitHub CLI"));
    v
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn common_bin_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]
}
