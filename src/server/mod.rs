//! HTTP server implementation
//!
//! Sets up the Axum HTTP server with:
//! - S3 API routes
//! - Middleware (logging, metrics, request ID, timeout)
//! - Graceful shutdown
//! - Health/readiness probes

use axum::Router;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info, instrument};

use crate::config::Config;
use crate::routes;
use crate::storage::StorageBackend;

/// HTTP server for S3Proxy
pub struct Server {
    config: Config,
    storage: Arc<dyn StorageBackend>,
}

impl Server {
    /// Create a new server instance
    pub fn new(
        config: Config,
        storage: Arc<dyn StorageBackend>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { config, storage })
    }

    /// Build the Axum router with all middleware
    fn build_router(&self) -> Router {
        routes::create_router(self.storage.clone())
            .layer(
                ServiceBuilder::new()
                    // Add request tracing (includes request ID via tracing)
                    .layer(TraceLayer::new_for_http())
                    // Add timeout
                    .layer(TimeoutLayer::new(
                        std::time::Duration::from_secs(self.config.server.timeout_secs),
                    ))
                    // Add compression
                    .layer(CompressionLayer::new())
                    .into_inner(),
            )
    }

    /// Start the server and run until shutdown signal
    pub async fn start<F>(&self, shutdown: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let app = self.build_router();

        let listener = tokio::net::TcpListener::bind(self.config.server.bind_address).await?;
        info!(address = %self.config.server.bind_address, "Server listening");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await?;

        Ok(())
    }
}

