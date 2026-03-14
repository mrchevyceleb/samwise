use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewTier {
    DirectServe,
    EsbuildBundle,
    ManagedProcess,
}

#[derive(Debug, Clone, Serialize)]
pub struct TierDetection {
    pub tier: PreviewTier,
    pub framework: Option<String>,
    pub entry_point: Option<String>,
    pub dev_command: Option<String>,
    pub reason: String,
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
    TierDetection { tier, framework, entry_point, dev_command, reason }
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

    // --- Priority 2: Client-side frameworks → Tier 2 (esbuild) ---
    // This is the key change: React/Solid/Preact/Lit go to esbuild
    // REGARDLESS of whether a dev script exists.
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

    // --- Priority 5: Has index.html → Tier 1 (instant static serve) ---
    // If a project has an index.html, serve it directly. This handles
    // Vite/Parcel/plain projects that have a working index.html entry point.
    // This is FASTER than spinning up a dev server.
    let has_index_html = project_dir.join("index.html").exists();
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
    let entry = if index_path.exists() {
        Some("index.html".to_string())
    } else {
        find_first_file(project_dir, "html")
    };

    make_detection(
        PreviewTier::DirectServe,
        None,
        entry,
        None,
        "Static project.".to_string(),
    )
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
