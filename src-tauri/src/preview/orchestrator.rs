use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use super::esbuild_runner::EsbuildRunner;
use super::http_server::PreviewServer;
use super::process_manager::ManagedProcess;
use super::tier_detector::{self, PreviewTier, TierDetection};
use super::watcher::PreviewWatcher;

pub struct PreviewOrchestrator {
    current_tier: Option<PreviewTier>,
    detection: Option<TierDetection>,
    http_server: Option<PreviewServer>,
    esbuild: Option<EsbuildRunner>,
    managed_process: Option<ManagedProcess>,
    watcher: Option<PreviewWatcher>,
    project_dir: Option<PathBuf>,
}

impl PreviewOrchestrator {
    pub fn new() -> Self {
        Self {
            current_tier: None,
            detection: None,
            http_server: None,
            esbuild: None,
            managed_process: None,
            watcher: None,
            project_dir: None,
        }
    }

    /// Open a project for preview. Detects tier and starts the appropriate server.
    pub async fn open_project(
        &mut self,
        app: &AppHandle,
        project_dir: PathBuf,
    ) -> Result<TierDetection, String> {
        // 1. Stop any existing preview
        self.stop().await?;

        log::info!(
            "[orchestrator] Opening project for preview: {}",
            project_dir.display()
        );

        // 2. Detect tier
        let detection = tier_detector::detect_tier(&project_dir);
        log::info!(
            "[orchestrator] Detected tier: {:?} ({})",
            detection.tier,
            detection.reason
        );

        // Emit status so frontend can show progress
        let _ = app.emit("preview:status", serde_json::json!({
            "phase": "starting",
            "tier": format!("{:?}", detection.tier),
            "framework": &detection.framework,
            "message": match detection.tier {
                PreviewTier::DirectServe => "Starting static file server...",
                PreviewTier::EsbuildBundle => "Bundling with esbuild...",
                PreviewTier::ManagedProcess => "Starting dev server...",
            }
        }));

        // 3. Start the appropriate server based on tier
        let result = match detection.tier {
            PreviewTier::DirectServe => {
                self.start_direct_serve(&project_dir).await
            }
            PreviewTier::EsbuildBundle => {
                self.start_esbuild_bundle(&project_dir, &detection).await
            }
            PreviewTier::ManagedProcess => {
                self.start_managed_process(app, &project_dir, &detection).await
            }
        };

        // If esbuild fails, fall back to direct serve
        if result.is_err() && detection.tier == PreviewTier::EsbuildBundle {
            log::warn!(
                "[orchestrator] Esbuild bundle failed, falling back to DirectServe: {}",
                result.as_ref().unwrap_err()
            );
            self.start_direct_serve(&project_dir).await?;

            let fallback_detection = TierDetection {
                tier: PreviewTier::DirectServe,
                framework: detection.framework.clone(),
                entry_point: detection.entry_point.clone(),
                dev_command: None,
                reason: format!(
                    "Esbuild unavailable, serving static files. Original: {}",
                    detection.reason
                ),
            };

            self.current_tier = Some(PreviewTier::DirectServe);
            self.detection = Some(fallback_detection.clone());
            self.project_dir = Some(project_dir.clone());

            self.start_watcher(project_dir, app)?;

            return Ok(fallback_detection);
        }

        // If managed process fails, fall back to direct serve as last resort
        if result.is_err() && detection.tier == PreviewTier::ManagedProcess {
            let err_msg = result.as_ref().unwrap_err().clone();
            log::warn!(
                "[orchestrator] ManagedProcess failed, falling back to DirectServe: {}",
                err_msg
            );

            // Try direct serve as last resort
            match self.start_direct_serve(&project_dir).await {
                Ok(_) => {
                    let fallback_detection = TierDetection {
                        tier: PreviewTier::DirectServe,
                        framework: detection.framework.clone(),
                        entry_point: detection.entry_point.clone(),
                        dev_command: detection.dev_command.clone(),
                        reason: format!(
                            "Dev server failed ({}). Serving static files as fallback.",
                            err_msg
                        ),
                    };

                    self.current_tier = Some(PreviewTier::DirectServe);
                    self.detection = Some(fallback_detection.clone());
                    self.project_dir = Some(project_dir.clone());
                    self.start_watcher(project_dir, app)?;
                    return Ok(fallback_detection);
                }
                Err(_) => {
                    // Even static serve failed, return the original error
                    return Err(err_msg);
                }
            }
        }

        result?;

        // 4. Start file watcher
        self.current_tier = Some(detection.tier.clone());
        self.detection = Some(detection.clone());
        self.project_dir = Some(project_dir.clone());

        self.start_watcher(project_dir, app)?;

        Ok(detection)
    }

    /// Stop the current preview session
    pub async fn stop(&mut self) -> Result<(), String> {
        log::info!("[orchestrator] Stopping preview");

        // Stop watcher
        if let Some(ref mut watcher) = self.watcher {
            watcher.stop();
        }
        self.watcher = None;

        // Stop HTTP server
        if let Some(ref mut server) = self.http_server {
            server.shutdown();
        }
        self.http_server = None;

        // Stop esbuild (cleanup output)
        if let Some(ref esbuild) = self.esbuild {
            esbuild.cleanup();
        }
        self.esbuild = None;

        // Stop managed process
        if let Some(ref mut process) = self.managed_process {
            process.stop().await?;
        }
        self.managed_process = None;

        self.current_tier = None;
        self.detection = None;
        self.project_dir = None;

        Ok(())
    }

    /// Get the current preview URL
    pub fn current_url(&self) -> Option<String> {
        if let Some(ref server) = self.http_server {
            return Some(server.url());
        }
        if let Some(ref process) = self.managed_process {
            return Some(process.url());
        }
        None
    }

    /// Get the current tier as a string
    pub fn current_tier_name(&self) -> Option<String> {
        self.current_tier.as_ref().map(|t| match t {
            PreviewTier::DirectServe => "direct".to_string(),
            PreviewTier::EsbuildBundle => "esbuild".to_string(),
            PreviewTier::ManagedProcess => "managed".to_string(),
        })
    }

    /// Get the current detection result
    pub fn current_detection(&self) -> Option<&TierDetection> {
        self.detection.as_ref()
    }

    /// Rebuild the esbuild bundle (called on file change for Tier 2)
    pub async fn rebuild(&mut self) -> Result<(), String> {
        let project_dir = self.project_dir.clone()
            .ok_or_else(|| "No project open".to_string())?;

        match self.current_tier {
            Some(PreviewTier::EsbuildBundle) => {
                let detection = self.detection.clone()
                    .ok_or_else(|| "No detection info".to_string())?;
                let entry = detection.entry_point.as_deref().unwrap_or("src/index.tsx");

                log::info!("[orchestrator] Rebuilding esbuild bundle");
                let runner = EsbuildRunner::build(&project_dir, entry).await?;
                self.esbuild = Some(runner);
                Ok(())
            }
            _ => Ok(()), // Other tiers handle their own rebuilds
        }
    }

    // -- Private helpers --

    async fn start_direct_serve(&mut self, project_dir: &PathBuf) -> Result<(), String> {
        log::info!("[orchestrator] Starting DirectServe for {}", project_dir.display());
        let server = PreviewServer::start(project_dir.clone()).await?;
        log::info!("[orchestrator] DirectServe running at {}", server.url());
        self.http_server = Some(server);
        Ok(())
    }

    async fn start_esbuild_bundle(
        &mut self,
        project_dir: &PathBuf,
        detection: &TierDetection,
    ) -> Result<(), String> {
        let entry = detection.entry_point.as_deref().unwrap_or("src/index.tsx");

        log::info!(
            "[orchestrator] Starting EsbuildBundle for {} (entry: {})",
            project_dir.display(),
            entry
        );

        let runner = EsbuildRunner::build(project_dir, entry).await?;
        let output_dir = runner.output_dir().to_path_buf();

        // Serve the esbuild output
        let server = PreviewServer::start(output_dir).await?;
        log::info!("[orchestrator] EsbuildBundle serving at {}", server.url());

        self.esbuild = Some(runner);
        self.http_server = Some(server);
        Ok(())
    }

    async fn start_managed_process(
        &mut self,
        app: &AppHandle,
        project_dir: &PathBuf,
        detection: &TierDetection,
    ) -> Result<(), String> {
        let command = detection
            .dev_command
            .as_deref()
            .unwrap_or("npm run dev");

        log::info!(
            "[orchestrator] Starting ManagedProcess for {} (command: {})",
            project_dir.display(),
            command
        );

        let env = HashMap::new();
        let process = ManagedProcess::start(project_dir, command, env, Some(app.clone())).await?;
        log::info!("[orchestrator] ManagedProcess running at {}", process.url());

        self.managed_process = Some(process);
        Ok(())
    }

    fn start_watcher(&mut self, project_dir: PathBuf, app: &AppHandle) -> Result<(), String> {
        match PreviewWatcher::start(project_dir, app.clone()) {
            Ok(watcher) => {
                self.watcher = Some(watcher);
                Ok(())
            }
            Err(e) => {
                // Watcher failure is non-fatal
                log::warn!("[orchestrator] File watcher failed to start: {}", e);
                Ok(())
            }
        }
    }
}
