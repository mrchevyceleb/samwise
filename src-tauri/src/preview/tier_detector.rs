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

/// Frameworks that need their own dev server (Tier 3: ManagedProcess)
const MANAGED_FRAMEWORKS: &[(&str, &str)] = &[
    ("next", "Next.js"),
    ("nuxt", "Nuxt"),
    ("@remix-run/dev", "Remix"),
    ("express", "Express"),
    ("fastify", "Fastify"),
    ("@nestjs/core", "NestJS"),
    ("hono", "Hono"),
    ("koa", "Koa"),
    ("astro", "Astro"),
    ("gatsby", "Gatsby"),
];

/// Frameworks that can be bundled with esbuild (Tier 2: EsbuildBundle)
const BUNDLE_FRAMEWORKS: &[(&str, &str)] = &[
    ("react", "React"),
    ("react-dom", "React"),
    ("svelte", "Svelte"),
    ("vue", "Vue"),
    ("solid-js", "Solid"),
    ("preact", "Preact"),
    ("lit", "Lit"),
];

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

    // Check for managed frameworks first (Tier 3)
    for (dep, framework_name) in MANAGED_FRAMEWORKS {
        if all_deps.iter().any(|d| d == dep) {
            let dev_command = resolve_dev_command(&pkg, framework_name);
            return TierDetection {
                tier: PreviewTier::ManagedProcess,
                framework: Some(framework_name.to_string()),
                entry_point: None,
                dev_command: Some(dev_command),
                reason: format!("Found {} in dependencies. Requires managed dev server.", framework_name),
            };
        }
    }

    // Check for bundleable frameworks (Tier 2)
    for (dep, framework_name) in BUNDLE_FRAMEWORKS {
        if all_deps.iter().any(|d| d == dep) {
            let entry = find_entry_point(project_dir, framework_name);
            return TierDetection {
                tier: PreviewTier::EsbuildBundle,
                framework: Some(framework_name.to_string()),
                entry_point: Some(entry),
                dev_command: None,
                reason: format!("Found {} in dependencies. Can be bundled with esbuild.", framework_name),
            };
        }
    }

    // Has TypeScript/JSX files? -> EsbuildBundle
    if has_files_with_extensions(project_dir, &["tsx", "jsx", "ts"]) {
        let entry = find_entry_point(project_dir, "TypeScript");
        return TierDetection {
            tier: PreviewTier::EsbuildBundle,
            framework: None,
            entry_point: Some(entry),
            dev_command: None,
            reason: "Found TypeScript/JSX files. Bundling with esbuild.".to_string(),
        };
    }

    // Fallback: serve static
    detect_static_tier(project_dir)
}

fn detect_static_tier(project_dir: &Path) -> TierDetection {
    // Look for an index.html
    let index_path = project_dir.join("index.html");
    let entry = if index_path.exists() {
        Some("index.html".to_string())
    } else {
        // Check for any .html file
        find_first_file(project_dir, "html")
    };

    TierDetection {
        tier: PreviewTier::DirectServe,
        framework: None,
        entry_point: entry,
        dev_command: None,
        reason: "Static project. Serving files directly.".to_string(),
    }
}

fn resolve_dev_command(pkg: &serde_json::Value, framework: &str) -> String {
    // Try to find "dev" script in package.json
    if let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
        if let Some(dev) = scripts.get("dev").and_then(|s| s.as_str()) {
            return dev.to_string();
        }
        if let Some(start) = scripts.get("start").and_then(|s| s.as_str()) {
            return start.to_string();
        }
    }

    // Default dev commands per framework
    match framework {
        "Next.js" => "next dev".to_string(),
        "Nuxt" => "nuxt dev".to_string(),
        "Remix" => "remix dev".to_string(),
        "Express" | "Fastify" | "NestJS" | "Hono" | "Koa" => "node .".to_string(),
        "Astro" => "astro dev".to_string(),
        "Gatsby" => "gatsby develop".to_string(),
        _ => "npm run dev".to_string(),
    }
}

fn find_entry_point(project_dir: &Path, framework: &str) -> String {
    let candidates = match framework {
        "React" => vec![
            "src/index.tsx", "src/index.jsx", "src/main.tsx", "src/main.jsx",
            "src/App.tsx", "src/App.jsx", "index.tsx", "index.jsx",
        ],
        "Svelte" => vec![
            "src/main.ts", "src/main.js", "src/App.svelte", "index.ts", "index.js",
        ],
        "Vue" => vec![
            "src/main.ts", "src/main.js", "src/App.vue", "index.ts", "index.js",
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

    // Fallback
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
