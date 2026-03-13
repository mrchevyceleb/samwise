use axum::Router;
use std::path::PathBuf;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

pub struct PreviewServer {
    port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl PreviewServer {
    pub async fn start(serve_dir: PathBuf) -> Result<Self, String> {
        let port = super::port_allocator::find_preview_port()?;

        let app = Router::new()
            .fallback_service(
                ServeDir::new(&serve_dir).append_index_html_on_directories(true),
            )
            .layer(CorsLayer::permissive());

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| format!("Failed to bind preview server: {}", e))?;

        log::info!("Preview server starting on port {}", port);

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .ok();
        });

        Ok(Self {
            port,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            tx.send(()).ok();
        }
    }
}

impl Drop for PreviewServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
