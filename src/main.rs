//! S3Proxy - Production-grade S3-compatible proxy for cloud object stores
//!
//! This service provides an S3-compatible HTTP API that proxies requests
//! to backend object stores (AWS S3, Azure Blob Storage, Google Cloud Storage)
//! using managed identity/workload identity for authentication.

mod config;
mod errors;
mod metrics;
mod routes;
mod s3;
mod server;
mod storage;

use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::Config;
use crate::server::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with JSON output for structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Initialize Prometheus metrics
    crate::metrics::init_metrics();

    info!("Starting S3Proxy");

    // Load configuration from environment and optional config file
    let config = Config::from_env()?;
    info!(?config, "Configuration loaded");

    // Initialize storage backend based on configuration
    let storage = storage::create_backend(&config).await?;
    info!("Storage backend initialized");

    // Create and start the HTTP server
    let server = Server::new(config.clone(), storage)?;
    
    // Handle graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Received shutdown signal");
    };

    info!("Server starting on {}", config.server.bind_address);
    if let Err(e) = server.start(shutdown_signal).await {
        error!(error = %e, "Server error");
        return Err(e.into());
    }

    info!("Server shutdown complete");
    Ok(())
}

