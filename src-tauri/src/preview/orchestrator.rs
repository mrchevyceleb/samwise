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
    esbuild_sidecar_path: Option<PathBuf>,
    env_vars: HashMap<String, String>,
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
            esbuild_sidecar_path: None,
            env_vars: HashMap::new(),
        }
    }

    /// Set the esbuild sidecar binary path (resolved from Tauri resource dir)
    pub fn set_esbuild_sidecar(&mut self, path: PathBuf) {
        self.esbuild_sidecar_path = Some(path);
    }

    /// Set environment variables to inject into the preview process
    pub fn set_env_vars(&mut self, vars: HashMap<String, String>) {
        self.env_vars = vars;
    }

    /// Open a project for preview. Detects tier and starts the appropriate server.
    /// All tiers are silent - no user-facing status messages about infrastructure.
    pub async fn open_project(
        &mut self,
        app: &AppHandle,
        project_dir: PathBuf,
    ) -> Result<TierDetection, String> {
        self.stop().await?;

        log::info!(
            "[orchestrator] Opening project: {}",
            project_dir.display()
        );

        let detection = tier_detector::detect_tier(&project_dir);
        log::info!(
            "[orchestrator] Tier: {:?} ({})",
            detection.tier,
            detection.reason
        );

        // Start the appropriate server
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

        // Fallback: if esbuild or managed process fails, try direct serve
        if result.is_err() && detection.tier != PreviewTier::DirectServe {
            let err = result.as_ref().unwrap_err().clone();
            log::warn!("[orchestrator] {:?} failed ({}), falling back to DirectServe", detection.tier, err);

            match self.start_direct_serve(&project_dir).await {
                Ok(_) => {
                    let fallback = TierDetection {
                        tier: PreviewTier::DirectServe,
                        framework: detection.framework.clone(),
                        entry_point: detection.entry_point.clone(),
                        dev_command: detection.dev_command.clone(),
                        reason: format!("Fallback: {}", err),
                    };

                    self.current_tier = Some(PreviewTier::DirectServe);
                    self.detection = Some(fallback.clone());
                    self.project_dir = Some(project_dir.clone());
                    self.start_watcher(project_dir, app)?;
                    return Ok(fallback);
                }
                Err(_) => return Err(err),
            }
        }

        result?;

        self.current_tier = Some(detection.tier.clone());
        self.detection = Some(detection.clone());
        self.project_dir = Some(project_dir.clone());
        self.start_watcher(project_dir, app)?;

        Ok(detection)
    }

    /// Stop the current preview session
    pub async fn stop(&mut self) -> Result<(), String> {
        log::info!("[orchestrator] Stopping preview");

        if let Some(ref mut watcher) = self.watcher {
            watcher.stop();
        }
        self.watcher = None;

        if let Some(ref mut server) = self.http_server {
            server.shutdown();
        }
        self.http_server = None;

        if let Some(ref esbuild) = self.esbuild {
            esbuild.cleanup();
        }
        self.esbuild = None;

        if let Some(ref mut process) = self.managed_process {
            process.stop().await?;
        }
        self.managed_process = None;

        self.current_tier = None;
        self.detection = None;
        self.project_dir = None;

        Ok(())
    }

    pub fn current_url(&self) -> Option<String> {
        if let Some(ref server) = self.http_server {
            return Some(server.url());
        }
        if let Some(ref process) = self.managed_process {
            return Some(process.url());
        }
        None
    }

    pub fn current_tier_name(&self) -> Option<String> {
        self.current_tier.as_ref().map(|t| match t {
            PreviewTier::DirectServe => "direct".to_string(),
            PreviewTier::EsbuildBundle => "esbuild".to_string(),
            PreviewTier::ManagedProcess => "managed".to_string(),
        })
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
                let runner = EsbuildRunner::build(
                    &project_dir,
                    entry,
                    self.esbuild_sidecar_path.as_deref(),
                    &self.env_vars,
                ).await?;
                self.esbuild = Some(runner);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    // -- Private helpers --

    async fn start_direct_serve(&mut self, project_dir: &PathBuf) -> Result<(), String> {
        let server = PreviewServer::start(project_dir.clone()).await?;
        log::info!("[orchestrator] DirectServe at {}", server.url());
        self.http_server = Some(server);
        Ok(())
    }

    async fn start_esbuild_bundle(
        &mut self,
        project_dir: &PathBuf,
        detection: &TierDetection,
    ) -> Result<(), String> {
        let entry = detection.entry_point.as_deref().unwrap_or("src/index.tsx");
        log::info!("[orchestrator] esbuild bundle: {}", entry);

        let runner = EsbuildRunner::build(
            project_dir,
            entry,
            self.esbuild_sidecar_path.as_deref(),
            &self.env_vars,
        ).await?;
        let output_dir = runner.output_dir().to_path_buf();

        let server = PreviewServer::start(output_dir).await?;
        log::info!("[orchestrator] esbuild serving at {}", server.url());

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

        log::info!("[orchestrator] ManagedProcess: {}", command);

        let env = self.env_vars.clone();
        let process = ManagedProcess::start(project_dir, command, env, Some(app.clone())).await?;
        log::info!("[orchestrator] ManagedProcess at {}", process.url());

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
                log::warn!("[orchestrator] File watcher failed: {}", e);
                Ok(())
            }
        }
    }
}
