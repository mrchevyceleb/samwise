use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct EsbuildRunner {
    output_dir: PathBuf,
}

impl EsbuildRunner {
    /// Bundle a project with esbuild.
    /// `sidecar_path` is the resolved path to the esbuild sidecar binary (from Tauri resource dir).
    /// Falls back to local node_modules and system PATH if sidecar is not available.
    /// `env_vars` are injected as --define flags for process.env.KEY replacements.
    pub async fn build(
        project_dir: &Path,
        entry_point: &str,
        sidecar_path: Option<&Path>,
        env_vars: &HashMap<String, String>,
    ) -> Result<Self, String> {
        let output_dir = project_dir.join(".banana-preview");

        // Create output directory
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create preview output dir: {}", e))?;

        let entry_path = project_dir.join(entry_point);
        if !entry_path.exists() {
            return Err(format!("Entry point not found: {}", entry_point));
        }

        // Find esbuild binary: sidecar first, then local, then PATH
        let esbuild_bin = find_esbuild(project_dir, sidecar_path)?;

        log::info!(
            "[esbuild] Building {} with entry: {} using: {}",
            project_dir.display(),
            entry_point,
            esbuild_bin
        );

        // Check if project uses Tailwind
        let has_tailwind = project_has_tailwind(project_dir);

        let mut cmd = Command::new(&esbuild_bin);
        cmd.arg(entry_path.to_string_lossy().to_string())
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
            .arg("--loader:.json=json")
            .arg("--sourcemap")
            .arg("--target=es2020")
            .arg("--define:process.env.NODE_ENV=\"development\"");

        // Inject user-provided env vars as process.env.KEY defines
        // These must come BEFORE the catch-all process.env={} so specific keys take precedence
        for (key, value) in env_vars {
            // Escape the value as a JSON string for esbuild --define
            let escaped = serde_json::to_string(value).unwrap_or_else(|_| format!("\"{}\"", value));
            cmd.arg(format!("--define:process.env.{}={}", key, escaped));
            // Also define import.meta.env.KEY for Vite-style projects
            cmd.arg(format!("--define:import.meta.env.{}={}", key, escaped));
        }

        // Catch-all for any unset process.env / import.meta.env references
        // (specific per-key defines above take precedence over these catch-alls)
        cmd.arg("--define:process.env={}");
        cmd.arg("--define:import.meta.env={}");

        // esbuild resolves node_modules automatically when run from project dir
        cmd.current_dir(project_dir);

        let output = cmd
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
        generate_index_html(&output_dir, entry_point, has_tailwind)?;

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
/// 1. Sidecar binary (shipped with the app)
/// 2. Local node_modules/.bin/esbuild
/// 3. System PATH
fn find_esbuild(project_dir: &Path, sidecar_path: Option<&Path>) -> Result<String, String> {
    // Check sidecar binary first (guaranteed to exist in production builds)
    if let Some(sidecar) = sidecar_path {
        if sidecar.exists() {
            log::info!("[esbuild] Using sidecar binary: {}", sidecar.display());
            return Ok(sidecar.to_string_lossy().to_string());
        }
    }

    // Check local node_modules
    let local_bin = if cfg!(windows) {
        project_dir.join("node_modules/.bin/esbuild.cmd")
    } else {
        project_dir.join("node_modules/.bin/esbuild")
    };

    if local_bin.exists() {
        log::info!("[esbuild] Using local binary: {}", local_bin.display());
        return Ok(local_bin.to_string_lossy().to_string());
    }

    // Check system PATH
    let bin_name = if cfg!(windows) { "esbuild.cmd" } else { "esbuild" };
    match which_esbuild(bin_name) {
        Some(path) => {
            log::info!("[esbuild] Using system binary: {}", path);
            Ok(path)
        }
        None => Err(
            "esbuild not found. The bundled sidecar binary may be missing."
                .to_string(),
        ),
    }
}

fn which_esbuild(name: &str) -> Option<String> {
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in path_var.split(separator) {
            let candidate = Path::new(dir).join(name);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // On Windows, also check .exe extension
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

/// Check if a project uses Tailwind CSS
fn project_has_tailwind(project_dir: &Path) -> bool {
    // Check tailwind config files
    let config_files = [
        "tailwind.config.js",
        "tailwind.config.ts",
        "tailwind.config.mjs",
        "tailwind.config.cjs",
    ];
    for f in &config_files {
        if project_dir.join(f).exists() {
            return true;
        }
    }

    // Check package.json for tailwindcss dependency
    if let Ok(content) = std::fs::read_to_string(project_dir.join("package.json")) {
        if content.contains("\"tailwindcss\"") {
            return true;
        }
    }

    false
}

/// Generate an index.html that loads the esbuild output bundle.
/// Includes Tailwind Play CDN if the project uses Tailwind, and an error overlay.
fn generate_index_html(
    output_dir: &Path,
    entry_point: &str,
    has_tailwind: bool,
) -> Result<(), String> {
    let entry_stem = Path::new(entry_point)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("index");

    let js_file = format!("{}.js", entry_stem);

    let tailwind_script = if has_tailwind {
        "    <script src=\"https://cdn.tailwindcss.com\"></script>\n"
    } else {
        ""
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Preview</title>
{tailwind}    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        html, body, #root, #app {{ width: 100%; height: 100%; }}
        .banana-error-overlay {{
            position: fixed; inset: 0; z-index: 99999;
            background: rgba(13, 17, 23, 0.95); color: #f0f0f0;
            font-family: 'JetBrains Mono', monospace; font-size: 13px;
            padding: 32px; overflow: auto;
        }}
        .banana-error-overlay h2 {{
            color: #ff6b6b; font-size: 16px; margin-bottom: 12px;
        }}
        .banana-error-overlay pre {{
            background: rgba(255,255,255,0.05); padding: 16px;
            border-radius: 8px; overflow-x: auto; line-height: 1.5;
            border: 1px solid rgba(255,107,107,0.2);
        }}
        .banana-error-overlay button {{
            margin-top: 16px; padding: 8px 20px;
            background: #FFD60A; color: #0D1117; border: none;
            border-radius: 6px; font-weight: 600; cursor: pointer;
        }}
    </style>
</head>
<body>
    <div id="root"></div>
    <div id="app"></div>
    <script type="module" src="./{js_file}"></script>
    <script>
        // Error overlay - catch uncaught errors and display them in-preview
        window.onerror = function(msg, src, line, col, err) {{
            showError(msg, err ? err.stack : src + ':' + line);
        }};
        window.onunhandledrejection = function(e) {{
            showError('Unhandled Promise Rejection', e.reason ? (e.reason.stack || String(e.reason)) : 'Unknown error');
        }};
        function showError(title, detail) {{
            var el = document.createElement('div');
            el.className = 'banana-error-overlay';
            el.innerHTML = '<h2>' + escHtml(String(title)) + '</h2><pre>' + escHtml(String(detail || '')) + '</pre><button onclick="this.parentElement.remove()">Dismiss</button>';
            document.body.appendChild(el);
        }}
        function escHtml(s) {{ var d = document.createElement('div'); d.textContent = s; return d.innerHTML; }}
    </script>
</body>
</html>"#,
        tailwind = tailwind_script,
        js_file = js_file
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
