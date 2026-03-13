use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct EsbuildRunner {
    output_dir: PathBuf,
}

impl EsbuildRunner {
    /// Attempt to bundle a project with esbuild.
    /// Returns the output directory containing bundled files on success.
    pub async fn build(project_dir: &Path, entry_point: &str) -> Result<Self, String> {
        let output_dir = project_dir.join(".banana-preview");

        // Create output directory
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create preview output dir: {}", e))?;

        let entry_path = project_dir.join(entry_point);
        if !entry_path.exists() {
            return Err(format!("Entry point not found: {}", entry_point));
        }

        // Find esbuild binary
        let esbuild_bin = find_esbuild(project_dir)?;

        log::info!(
            "[esbuild] Building {} with entry: {}",
            project_dir.display(),
            entry_point
        );

        let output = Command::new(&esbuild_bin)
            .arg(entry_path.to_string_lossy().to_string())
            .arg("--bundle")
            .arg(format!("--outdir={}", output_dir.display()))
            .arg("--format=esm")
            .arg("--jsx=automatic")
            .arg("--loader:.tsx=tsx")
            .arg("--loader:.ts=ts")
            .arg("--loader:.jsx=jsx")
            .arg("--loader:.css=css")
            .arg("--loader:.svg=dataurl")
            .arg("--loader:.png=dataurl")
            .arg("--loader:.jpg=dataurl")
            .arg("--loader:.gif=dataurl")
            .arg("--loader:.woff=file")
            .arg("--loader:.woff2=file")
            .arg("--sourcemap")
            .arg("--target=es2020")
            .current_dir(project_dir)
            .output()
            .await
            .map_err(|e| format!("Failed to run esbuild: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            log::error!("[esbuild] Build failed:\nstderr: {}\nstdout: {}", stderr, stdout);
            return Err(format!("esbuild build failed: {}", stderr));
        }

        log::info!("[esbuild] Build succeeded, output at {}", output_dir.display());

        // Generate an index.html that loads the bundle
        generate_index_html(&output_dir, entry_point)?;

        Ok(Self { output_dir })
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Clean up the output directory
    pub fn cleanup(&self) {
        if self.output_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.output_dir);
        }
    }
}

impl Drop for EsbuildRunner {
    fn drop(&mut self) {
        // Don't auto-cleanup on drop. Let the orchestrator decide.
    }
}

/// Find esbuild binary. Checks in order:
/// 1. Local node_modules/.bin/esbuild
/// 2. System PATH
fn find_esbuild(project_dir: &Path) -> Result<String, String> {
    // Check local node_modules
    let local_bin = if cfg!(windows) {
        project_dir.join("node_modules/.bin/esbuild.cmd")
    } else {
        project_dir.join("node_modules/.bin/esbuild")
    };

    if local_bin.exists() {
        return Ok(local_bin.to_string_lossy().to_string());
    }

    // Check system PATH
    let bin_name = if cfg!(windows) { "esbuild.cmd" } else { "esbuild" };
    match which_esbuild(bin_name) {
        Some(path) => Ok(path),
        None => Err(
            "esbuild not found. Install it locally (npm i esbuild) or globally (npm i -g esbuild)."
                .to_string(),
        ),
    }
}

fn which_esbuild(name: &str) -> Option<String> {
    // Simple PATH search
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in path_var.split(separator) {
            let candidate = Path::new(dir).join(name);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // On Windows, also check without .cmd extension
    if cfg!(windows) {
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(';') {
                let candidate = Path::new(dir).join("esbuild.exe");
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

/// Generate an index.html that loads the esbuild output bundle
fn generate_index_html(output_dir: &Path, entry_point: &str) -> Result<(), String> {
    // Determine the output JS filename from the entry point
    let entry_stem = Path::new(entry_point)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("index");

    let js_file = format!("{}.js", entry_stem);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Banana Code Preview</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        html, body, #root, #app {{ width: 100%; height: 100%; }}
    </style>
</head>
<body>
    <div id="root"></div>
    <div id="app"></div>
    <script type="module" src="./{}"></script>
</body>
</html>"#,
        js_file
    );

    // Also check if a CSS file was generated
    let css_file = format!("{}.css", entry_stem);
    let css_path = output_dir.join(&css_file);

    let html = if css_path.exists() {
        html.replace(
            "</head>",
            &format!("    <link rel=\"stylesheet\" href=\"./{}\" />\n</head>", css_file),
        )
    } else {
        html
    };

    std::fs::write(output_dir.join("index.html"), html)
        .map_err(|e| format!("Failed to write index.html: {}", e))?;

    Ok(())
}
