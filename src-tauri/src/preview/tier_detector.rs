use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewTier {
    DirectServe,
    EsbuildBundle,
    ManagedProcess,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
pub struct TierDetection {
    pub tier: PreviewTier,
    pub framework: Option<String>,
    pub entry_point: Option<String>,
    pub dev_command: Option<String>,
    pub reason: String,
    pub message: Option<String>,
}

/// Frameworks that MUST have their own dev server (SSR, server-side, backend).
/// These cannot be bundled by esbuild alone.
const SSR_FRAMEWORKS: &[(&str, &str)] = &[
    ("next", "Next.js"),
    ("nuxt", "Nuxt"),
    ("@remix-run/dev", "Remix"),
    ("astro", "Astro"),
    ("gatsby", "Gatsby"),
];

/// Backend frameworks that need their own server process.
const BACKEND_FRAMEWORKS: &[(&str, &str)] = &[
    ("express", "Express"),
    ("fastify", "Fastify"),
    ("@nestjs/core", "NestJS"),
    ("hono", "Hono"),
    ("koa", "Koa"),
];

/// Client-side frameworks that esbuild can bundle directly.
/// These go to Tier 2 REGARDLESS of whether a dev script exists.
const ESBUILD_CAPABLE: &[(&str, &str)] = &[
    ("react", "React"),
    ("react-dom", "React"),
    ("solid-js", "Solid"),
    ("preact", "Preact"),
    ("lit", "Lit"),
];

/// Frameworks that need plugins esbuild CLI can't provide.
/// These go to Tier 3 (silent managed process).
const NEEDS_SERVER: &[(&str, &str)] = &[
    ("svelte", "Svelte"),
    ("vue", "Vue"),
];

fn make_detection(
    tier: PreviewTier,
    framework: Option<String>,
    entry_point: Option<String>,
    dev_command: Option<String>,
    reason: String,
) -> TierDetection {
    TierDetection { tier, framework, entry_point, dev_command, reason, message: None }
}

fn make_unsupported(framework: &str, message: &str, reason: &str) -> TierDetection {
    TierDetection {
        tier: PreviewTier::Unsupported,
        framework: Some(framework.to_string()),
        entry_point: None,
        dev_command: None,
        reason: reason.to_string(),
        message: Some(message.to_string()),
    }
}

pub fn detect_tier(project_dir: &Path) -> TierDetection {
    let pkg_path = project_dir.join("package.json");

    // No package.json: check for raw files
    if !pkg_path.exists() {
        return detect_static_tier(project_dir);
    }

    // Parse package.json
    let pkg_content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return detect_static_tier(project_dir),
    };

    let pkg: serde_json::Value = match serde_json::from_str(&pkg_content) {
        Ok(v) => v,
        Err(_) => return detect_static_tier(project_dir),
    };

    // Collect all dependency names
    let mut all_deps: Vec<String> = Vec::new();
    for key in &["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(deps) = pkg.get(key).and_then(|d| d.as_object()) {
            for dep_name in deps.keys() {
                all_deps.push(dep_name.clone());
            }
        }
    }

    // --- Priority 0: React Native / Expo detection ---
    let has_expo = all_deps.iter().any(|d| d == "expo");
    let has_react_native = all_deps.iter().any(|d| d == "react-native");
    let has_react_native_web = all_deps.iter().any(|d| d == "react-native-web");
    let has_react_dom = all_deps.iter().any(|d| d == "react-dom");
    let has_expo_webpack = all_deps.iter().any(|d| d == "@expo/webpack-config");

    if has_expo {
        // Check app.json for web platform support as additional signal
        let app_json_has_web = check_expo_web_support(project_dir);
        let can_web = has_react_native_web || has_react_dom || has_expo_webpack || app_json_has_web;

        if can_web {
            let dev_command = resolve_dev_command(&pkg, "Expo");
            return make_detection(
                PreviewTier::ManagedProcess,
                Some("Expo".to_string()),
                None,
                Some(dev_command),
                "Expo project with web support.".to_string(),
            );
        } else {
            return make_unsupported(
                "Expo",
                "Mobile-only Expo project. Add react-native-web for web preview, or use Expo Go on your device.",
                "Expo project without web dependencies.",
            );
        }
    }

    if has_react_native && !has_react_dom && !has_react_native_web {
        return make_unsupported(
            "React Native",
            "React Native project without web support. Use a device emulator to preview.",
            "React Native project without react-dom or react-native-web.",
        );
    }

    // --- Priority 1: SSR/Backend frameworks → Tier 3 (silent managed process) ---
    for (dep, framework_name) in SSR_FRAMEWORKS {
        if all_deps.iter().any(|d| d == dep) {
            let dev_command = resolve_dev_command(&pkg, framework_name);
            return make_detection(
                PreviewTier::ManagedProcess,
                Some(framework_name.to_string()),
                None,
                Some(dev_command),
                format!("{} requires its own server.", framework_name),
            );
        }
    }

    for (dep, framework_name) in BACKEND_FRAMEWORKS {
        if all_deps.iter().any(|d| d == dep) {
            let dev_command = resolve_dev_command(&pkg, framework_name);
            return make_detection(
                PreviewTier::ManagedProcess,
                Some(framework_name.to_string()),
                None,
                Some(dev_command),
                format!("{} is a backend framework.", framework_name),
            );
        }
    }

    // --- Priority 2a: Vite projects → Tier 3 (managed process) ---
    // Vite projects MUST use their dev server. esbuild alone can't handle
    // PostCSS, Tailwind plugins, CSS modules, HMR, or Vite-specific imports.
    let has_vite_config = ["vite.config.ts", "vite.config.js", "vite.config.mts", "vite.config.mjs"]
        .iter()
        .any(|f| project_dir.join(f).exists())
        || all_deps.iter().any(|d| d == "vite");
    if has_vite_config {
        let dev_command = resolve_dev_command(&pkg, "Vite");
        return make_detection(
            PreviewTier::ManagedProcess,
            Some("Vite".to_string()),
            None,
            Some(dev_command),
            "Vite project - using dev server for plugins and CSS processing.".to_string(),
        );
    }

    // --- Priority 2b: Client-side frameworks without a bundler → Tier 2 (esbuild) ---
    // Only use esbuild for projects that DON'T have Vite/Webpack/Parcel.
    for (dep, framework_name) in ESBUILD_CAPABLE {
        if all_deps.iter().any(|d| d == dep) {
            let entry = find_entry_point(project_dir, framework_name);
            return make_detection(
                PreviewTier::EsbuildBundle,
                Some(framework_name.to_string()),
                Some(entry),
                None,
                format!("{} project. Bundling instantly with esbuild.", framework_name),
            );
        }
    }

    // --- Priority 3: Frameworks needing plugins → Tier 3 (silent) ---
    for (dep, framework_name) in NEEDS_SERVER {
        if all_deps.iter().any(|d| d == dep) {
            let dev_command = resolve_dev_command(&pkg, framework_name);
            return make_detection(
                PreviewTier::ManagedProcess,
                Some(framework_name.to_string()),
                None,
                Some(dev_command),
                format!("{} needs its dev server for component compilation.", framework_name),
            );
        }
    }

    // --- Priority 4: TypeScript/JSX files (no framework) → Tier 2 ---
    if has_files_with_extensions(project_dir, &["tsx", "jsx", "ts"]) {
        let entry = find_entry_point(project_dir, "TypeScript");
        return make_detection(
            PreviewTier::EsbuildBundle,
            None,
            Some(entry),
            None,
            "TypeScript/JSX project. Bundling with esbuild.".to_string(),
        );
    }

    // --- Priority 5: Has index.html (or public/index.html) → Tier 1 (instant static serve) ---
    // If a project has an index.html, serve it directly. This handles
    // Vite/Parcel/plain projects that have a working index.html entry point.
    // This is FASTER than spinning up a dev server.
    let has_index_html = project_dir.join("index.html").exists()
        || project_dir.join("public").join("index.html").exists();
    if has_index_html {
        return make_detection(
            PreviewTier::DirectServe,
            None,
            Some("index.html".to_string()),
            None,
            "Project has index.html. Serving directly.".to_string(),
        );
    }

    // --- Priority 6: Has dev script but no index.html → Tier 3 (silent) ---
    let has_dev_script = pkg.get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts| scripts.contains_key("dev") || scripts.contains_key("start"))
        .unwrap_or(false);

    if has_dev_script {
        let dev_command = resolve_dev_command(&pkg, "Unknown");
        return make_detection(
            PreviewTier::ManagedProcess,
            None,
            None,
            Some(dev_command),
            "Project has a dev script.".to_string(),
        );
    }

    // --- Priority 7: Fallback static ---
    detect_static_tier(project_dir)
}

fn detect_static_tier(project_dir: &Path) -> TierDetection {
    let index_path = project_dir.join("index.html");
    let public_index = project_dir.join("public").join("index.html");

    let (serve_dir, entry) = if index_path.exists() {
        (None, Some("index.html".to_string()))
    } else if public_index.exists() {
        // Serve the public/ subdirectory directly (common for static sites)
        (Some("public"), Some("index.html".to_string()))
    } else {
        (None, find_first_file(project_dir, "html"))
    };

    // If the real content is in public/, update the reason to note it
    let reason = if serve_dir.is_some() {
        "Static project (public/ directory).".to_string()
    } else {
        "Static project.".to_string()
    };

    make_detection(
        PreviewTier::DirectServe,
        None,
        entry,
        None,
        reason,
    )
}

/// Determine the actual directory to serve for a project.
/// If the static tier detected a public/ subdirectory, serve that instead.
pub fn resolve_serve_dir(project_dir: &Path) -> PathBuf {
    // If there's no index.html at root but public/index.html exists, serve public/
    if !project_dir.join("index.html").exists()
        && project_dir.join("public").join("index.html").exists()
    {
        return project_dir.join("public");
    }
    project_dir.to_path_buf()
}

fn resolve_dev_command(pkg: &serde_json::Value, framework: &str) -> String {
    if let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
        if let Some(dev) = scripts.get("dev").and_then(|s| s.as_str()) {
            return dev.to_string();
        }
        if let Some(start) = scripts.get("start").and_then(|s| s.as_str()) {
            return start.to_string();
        }
    }

    match framework {
        "Next.js" => "next dev".to_string(),
        "Nuxt" => "nuxt dev".to_string(),
        "Remix" => "remix dev".to_string(),
        "Express" | "Fastify" | "NestJS" | "Hono" | "Koa" => "node .".to_string(),
        "Astro" => "astro dev".to_string(),
        "Gatsby" => "gatsby develop".to_string(),
        "Expo" => "npx expo start --web".to_string(),
        "Vite" => "npm run dev".to_string(),
        "Svelte" => "npm run dev".to_string(),
        "Vue" => "npm run dev".to_string(),
        _ => "npm run dev".to_string(),
    }
}

fn find_entry_point(project_dir: &Path, framework: &str) -> String {
    let candidates = match framework {
        "React" => vec![
            "src/index.tsx", "src/index.jsx", "src/main.tsx", "src/main.jsx",
            "src/App.tsx", "src/App.jsx", "index.tsx", "index.jsx",
        ],
        "Solid" => vec![
            "src/index.tsx", "src/index.jsx", "src/main.tsx", "src/main.jsx",
        ],
        _ => vec![
            "src/index.tsx", "src/index.ts", "src/index.jsx", "src/index.js",
            "src/main.tsx", "src/main.ts", "src/main.jsx", "src/main.js",
            "index.tsx", "index.ts", "index.jsx", "index.js",
        ],
    };

    for candidate in candidates {
        if project_dir.join(candidate).exists() {
            return candidate.to_string();
        }
    }

    "src/index.tsx".to_string()
}

fn has_files_with_extensions(dir: &Path, extensions: &[&str]) -> bool {
    let src_dir = dir.join("src");
    let dirs_to_check = if src_dir.exists() {
        vec![src_dir, dir.to_path_buf()]
    } else {
        vec![dir.to_path_buf()]
    };

    for check_dir in dirs_to_check {
        if let Ok(entries) = std::fs::read_dir(&check_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn find_first_file(dir: &Path, extension: &str) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if ext == extension {
                    return entry.file_name().to_str().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

/// Check app.json / app.config.json for Expo web platform support
fn check_expo_web_support(project_dir: &Path) -> bool {
    for filename in &["app.json", "app.config.json"] {
        let path = project_dir.join(filename);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Check expo.platforms contains "web"
                if let Some(platforms) = json.get("expo")
                    .and_then(|e| e.get("platforms"))
                    .and_then(|p| p.as_array())
                {
                    if platforms.iter().any(|v| v.as_str() == Some("web")) {
                        return true;
                    }
                }
                // Check expo.web key exists
                if json.get("expo").and_then(|e| e.get("web")).is_some() {
                    return true;
                }
            }
        }
    }
    false
}

/// Resolve the best previewable directory in a monorepo.
/// If the root has workspaces, find the most web-friendly package.
/// Returns (resolved_dir, is_monorepo) - is_monorepo is true if resolved_dir != root.
pub fn resolve_project_dir(root: &Path) -> PathBuf {
    let candidates = collect_workspace_candidates(root);
    if candidates.is_empty() {
        return root.to_path_buf();
    }

    let mut best_path = root.to_path_buf();
    let mut best_score: i32 = 0;

    for candidate in &candidates {
        let score = score_web_previewability(candidate);
        log::debug!(
            "[tier_detector] Workspace candidate: {} (score: {})",
            candidate.display(),
            score
        );
        if score > best_score {
            best_score = score;
            best_path = candidate.clone();
        }
    }

    // Only use a candidate if it scored positively
    if best_score > 0 {
        best_path
    } else {
        root.to_path_buf()
    }
}

/// Collect workspace package directories from package.json workspaces or pnpm-workspace.yaml
fn collect_workspace_candidates(root: &Path) -> Vec<PathBuf> {
    let mut globs: Vec<String> = Vec::new();

    // Check package.json workspaces
    let pkg_path = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(workspaces) = pkg.get("workspaces") {
                // Format: "workspaces": ["apps/*", "packages/*"]
                if let Some(arr) = workspaces.as_array() {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            globs.push(s.to_string());
                        }
                    }
                }
                // Format: "workspaces": { "packages": ["apps/*", "packages/*"] }
                if let Some(obj) = workspaces.as_object() {
                    if let Some(pkgs) = obj.get("packages").and_then(|p| p.as_array()) {
                        for item in pkgs {
                            if let Some(s) = item.as_str() {
                                globs.push(s.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check pnpm-workspace.yaml
    let pnpm_path = root.join("pnpm-workspace.yaml");
    if let Ok(content) = std::fs::read_to_string(&pnpm_path) {
        // Simple YAML parsing for packages list - avoid adding a yaml dep
        // Format:
        // packages:
        //   - 'apps/*'
        //   - 'packages/*'
        let mut in_packages = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("packages:") {
                in_packages = true;
                continue;
            }
            if in_packages {
                if trimmed.starts_with('-') {
                    let val = trimmed.trim_start_matches('-').trim()
                        .trim_matches('\'').trim_matches('"');
                    if !val.is_empty() {
                        globs.push(val.to_string());
                    }
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    // New top-level key, stop
                    break;
                }
            }
        }
    }

    // Heuristic fallback: if no workspaces field found, scan common monorepo dirs
    // for subdirectories that could be previewable (have package.json, index.html, or web content)
    if globs.is_empty() {
        let heuristic_dirs = ["apps", "packages", "projects", "services", "libs"];
        let mut heuristic_candidates: Vec<PathBuf> = Vec::new();

        for dir_name in &heuristic_dirs {
            let dir = root.join(dir_name);
            if dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && is_previewable_dir(&path) {
                            heuristic_candidates.push(path);
                        }
                    }
                }
            }
        }

        // Also check top-level dirs that could be previewable
        // (e.g., root/web/, root/website/)
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() { continue; }
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                // Skip node_modules, hidden dirs, and dirs already scanned above
                if name == "node_modules" || name.starts_with('.')
                    || heuristic_dirs.contains(&name) {
                    continue;
                }
                if is_previewable_dir(&path) && !heuristic_candidates.contains(&path) {
                    heuristic_candidates.push(path);
                }
            }
        }

        if !heuristic_candidates.is_empty() {
            log::info!(
                "[tier_detector] No workspaces field, found {} heuristic candidates",
                heuristic_candidates.len()
            );
            return heuristic_candidates;
        }

        return Vec::new();
    }

    // Expand simple globs (e.g., "apps/*") to actual directories
    let mut candidates: Vec<PathBuf> = Vec::new();
    for pattern in &globs {
        if let Some(star_pos) = pattern.find('*') {
            // "apps/*" -> list directories under "apps/"
            let prefix = &pattern[..star_pos];
            let parent = root.join(prefix);
            if let Ok(entries) = std::fs::read_dir(&parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        candidates.push(path);
                    }
                }
            }
        } else {
            // Exact path
            let path = root.join(pattern);
            if path.is_dir() {
                candidates.push(path);
            }
        }
    }

    candidates
}

/// Check if a directory has any web-previewable content
/// (package.json, index.html, or HTML files in a public/ subfolder)
fn is_previewable_dir(dir: &Path) -> bool {
    if dir.join("package.json").exists() {
        return true;
    }
    if dir.join("index.html").exists() {
        return true;
    }
    // Check public/ subfolder (common for static sites, Expo web, etc.)
    let public_dir = dir.join("public");
    if public_dir.is_dir() && public_dir.join("index.html").exists() {
        return true;
    }
    // Check for any HTML files directly in the dir
    if has_files_with_extensions(dir, &["html"]) {
        return true;
    }
    false
}

/// Score a directory for web-previewability (higher = more likely a web app)
fn score_web_previewability(dir: &Path) -> i32 {
    let mut score: i32 = 0;

    let dir_name = dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Name-based scoring
    for name in &["web", "frontend", "app", "client", "site", "website"] {
        if dir_name.contains(name) {
            score += 10;
            break;
        }
    }
    for name in &["api", "server", "backend", "mobile", "native"] {
        if dir_name.contains(name) {
            score -= 10;
            break;
        }
    }

    // Check package.json for web framework deps
    let pkg_path = dir.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            let mut all_deps: Vec<String> = Vec::new();
            for key in &["dependencies", "devDependencies"] {
                if let Some(deps) = pkg.get(key).and_then(|d| d.as_object()) {
                    for dep_name in deps.keys() {
                        all_deps.push(dep_name.clone());
                    }
                }
            }

            let web_deps = ["react-dom", "next", "vite", "svelte", "vue", "nuxt", "@sveltejs/kit"];
            if all_deps.iter().any(|d| web_deps.contains(&d.as_str())) {
                score += 5;
            }

            // Penalize mobile-only
            let has_rn = all_deps.iter().any(|d| d == "react-native");
            let has_rd = all_deps.iter().any(|d| d == "react-dom");
            if has_rn && !has_rd {
                score -= 10;
            }
        }
    }

    // Bonus for index.html (direct or in public/)
    if dir.join("index.html").exists() {
        score += 3;
    }
    if dir.join("public").join("index.html").exists() {
        score += 3;
    }

    score
}
