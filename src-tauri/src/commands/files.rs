use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::models::{FileEntry, FileInfo, FileNode, SearchResult};

// ── Hidden directory filtering ──────────────────────────────────────

const HIDDEN_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".obsidian",
    ".svelte-kit",
    ".next",
    ".nuxt",
    ".vscode",
    ".idea",
    "__pycache__",
    ".DS_Store",
    "target",
    "dist",
    ".playwright-mcp",
    ".vault-pilot",
    "build",
];

fn should_skip(name: &str, show_hidden: bool) -> bool {
    if show_hidden {
        return false;
    }
    HIDDEN_DIRS.contains(&name) || name.starts_with('.')
}

// ── Tree building ───────────────────────────────────────────────────

fn build_tree(path: &Path, depth: usize, max_depth: usize, show_hidden: bool) -> Option<FileNode> {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if should_skip(&name, show_hidden) {
        return None;
    }

    if path.is_dir() {
        if depth >= max_depth {
            return Some(FileNode {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir: true,
                size: None,
                ext: None,
                children: Some(vec![]),
            });
        }

        let mut children: Vec<FileNode> = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a
                        .file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .cmp(&b.file_name().to_string_lossy().to_lowercase()),
                }
            });
            for entry in entries {
                if let Some(node) = build_tree(&entry.path(), depth + 1, max_depth, show_hidden) {
                    children.push(node);
                }
            }
        }

        Some(FileNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: true,
            size: None,
            ext: None,
            children: Some(children),
        })
    } else {
        let metadata = fs::metadata(path).ok();
        let size = metadata.map(|m| m.len());
        let ext = path.extension().map(|e| e.to_string_lossy().to_string());

        Some(FileNode {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: false,
            size,
            ext,
            children: None,
        })
    }
}

// ── Path helpers for import ─────────────────────────────────────────

fn copy_path_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    if source.is_dir() {
        fs::create_dir_all(destination).map_err(|e| e.to_string())?;
        let entries = fs::read_dir(source).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let child_source = entry.path();
            let child_destination = destination.join(entry.file_name());
            copy_path_recursive(&child_source, &child_destination)?;
        }
        Ok(())
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(source, destination).map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn remove_path_recursive(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

fn move_path_with_fallback(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    match fs::rename(source, destination) {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_path_recursive(source, destination)?;
            remove_path_recursive(source)?;
            Ok(())
        }
    }
}

fn unique_destination_path(destination_dir: &Path, source: &Path) -> Result<PathBuf, String> {
    let name = source
        .file_name()
        .ok_or_else(|| format!("Invalid source path: {}", source.display()))?
        .to_string_lossy()
        .to_string();

    let mut candidate = destination_dir.join(&name);
    if !candidate.exists() {
        return Ok(candidate);
    }

    let is_dir = source.is_dir();
    let (stem, ext) = if is_dir {
        (name.clone(), None)
    } else {
        let p = Path::new(&name);
        (
            p.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or(name.clone()),
            p.extension().map(|e| e.to_string_lossy().to_string()),
        )
    };

    for index in 1..10_000 {
        let suffix = if index == 1 {
            " copy".to_string()
        } else {
            format!(" copy {}", index)
        };

        let new_name = match &ext {
            Some(extension) => format!("{}{}.{}", stem, suffix, extension),
            None => format!("{}{}", stem, suffix),
        };

        candidate = destination_dir.join(new_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Unable to resolve non-conflicting destination path".to_string())
}

// ── Tauri commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn read_directory_tree(path: String, show_hidden: bool) -> Result<FileNode, String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }
    build_tree(&path, 0, 20, show_hidden)
        .ok_or_else(|| "Failed to build directory tree".to_string())
}

#[tauri::command]
pub fn read_directory_children(path: String, show_hidden: bool) -> Result<Vec<FileNode>, String> {
    let path = PathBuf::from(&path);
    if !path.is_dir() {
        return Err("Not a directory".to_string());
    }

    let mut children: Vec<FileNode> = Vec::new();
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&b.file_name().to_string_lossy().to_lowercase()),
        }
    });

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&name, show_hidden) {
            continue;
        }
        let entry_path = entry.path();
        let is_dir = entry_path.is_dir();
        let metadata = fs::metadata(&entry_path).ok();
        let size = if !is_dir {
            metadata.map(|m| m.len())
        } else {
            None
        };
        let ext = entry_path
            .extension()
            .map(|e| e.to_string_lossy().to_string());

        children.push(FileNode {
            name,
            path: entry_path.to_string_lossy().to_string(),
            is_dir,
            size,
            ext,
            children: if is_dir { Some(vec![]) } else { None },
        });
    }

    Ok(children)
}

#[tauri::command]
pub fn read_file_text(path: String) -> Result<String, String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    // Try UTF-8 first
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(_) => {
            // Fall back to encoding detection
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let (decoded, _, _) = encoding_rs::UTF_8.decode(&bytes);
            Ok(decoded.to_string())
        }
    }
}

#[tauri::command]
pub fn write_file_text(path: String, content: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_file(path: String, is_dir: bool) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if path.exists() {
        return Err("Path already exists".to_string());
    }
    if is_dir {
        fs::create_dir_all(&path).map_err(|e| e.to_string())
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, "").map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn rename_path(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_paths(
    paths: Vec<String>,
    destination_dir: String,
    move_items: bool,
) -> Result<Vec<String>, String> {
    let destination = PathBuf::from(&destination_dir);
    if !destination.exists() || !destination.is_dir() {
        return Err(format!(
            "Destination is not a directory: {}",
            destination.display()
        ));
    }

    let mut imported_paths: Vec<String> = Vec::new();
    let destination_abs = fs::canonicalize(&destination)
        .map_err(|e| format!("Failed to resolve destination path: {}", e))?;

    for source_str in paths {
        let source = PathBuf::from(&source_str);
        if !source.exists() {
            return Err(format!("Source does not exist: {}", source.display()));
        }

        if source.is_dir() {
            let source_abs = fs::canonicalize(&source)
                .map_err(|e| format!("Failed to resolve source path: {}", e))?;
            if destination_abs.starts_with(&source_abs) {
                return Err(format!(
                    "Cannot import a folder into itself: {} -> {}",
                    source.display(),
                    destination.display()
                ));
            }
        }

        let target = unique_destination_path(&destination, &source)?;
        if move_items {
            move_path_with_fallback(&source, &target)?;
        } else {
            copy_path_recursive(&source, &target)?;
        }

        imported_paths.push(target.to_string_lossy().to_string());
    }

    Ok(imported_paths)
}

#[tauri::command]
pub fn search_files(
    root: String,
    query: String,
    case_sensitive: bool,
    show_hidden: bool,
) -> Result<Vec<SearchResult>, String> {
    let mut results = Vec::new();
    let query_lower = if !case_sensitive {
        query.to_lowercase()
    } else {
        query.clone()
    };

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !should_skip(&name, show_hidden)
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            // Skip binary files by extension
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if matches!(
                ext.as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "svg"
                    | "ico"
                    | "webp"
                    | "mp4"
                    | "webm"
                    | "mov"
                    | "avi"
                    | "pdf"
                    | "zip"
                    | "tar"
                    | "gz"
                    | "exe"
                    | "dll"
                    | "so"
                    | "dylib"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "eot"
            ) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(path) {
                for (i, line) in content.lines().enumerate() {
                    let matches = if case_sensitive {
                        line.contains(&query)
                    } else {
                        line.to_lowercase().contains(&query_lower)
                    };
                    if matches {
                        results.push(SearchResult {
                            path: path.to_string_lossy().to_string(),
                            line_number: i + 1,
                            line_content: line.to_string(),
                        });
                        if results.len() >= 500 {
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn get_file_info(path: String) -> Result<FileInfo, String> {
    let path = PathBuf::from(&path);
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = path.extension().map(|e| e.to_string_lossy().to_string());
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    Ok(FileInfo {
        name,
        path: path.to_string_lossy().to_string(),
        size: metadata.len(),
        is_dir: metadata.is_dir(),
        ext,
        modified,
    })
}

#[tauri::command]
pub fn list_all_files(root: String, show_hidden: bool) -> Result<Vec<FileEntry>, String> {
    let root_path = PathBuf::from(&root);
    let mut files = Vec::new();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !should_skip(&name, show_hidden)
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let relative_path = path
                .strip_prefix(&root_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            let ext = path.extension().map(|e| e.to_string_lossy().to_string());

            files.push(FileEntry {
                name,
                path: path.to_string_lossy().to_string(),
                relative_path,
                ext,
            });
        }
    }

    Ok(files)
}
