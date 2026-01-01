//! Configuration management for S3Proxy
//!
//! Supports configuration via:
//! - Environment variables (primary)
//! - Optional TOML config file (secondary)
//!
//! Environment variables take precedence over config file values.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;

/// Backend storage type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// AWS S3
    Aws,
    /// Azure Blob Storage
    Azure,
    /// Google Cloud Storage
    Gcp,
}

impl FromStr for BackendType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aws" | "s3" => Ok(BackendType::Aws),
            "azure" => Ok(BackendType::Azure),
            "gcp" | "gcs" | "google" => Ok(BackendType::Gcp),
            _ => Err(format!("Unknown backend type: {}", s)),
        }
    }
}

/// Backend storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Backend type (aws, azure, gcp)
    #[serde(rename = "type")]
    pub backend_type: BackendType,

    /// Container/bucket name
    pub container_or_bucket: String,

    /// Optional path prefix for all objects
    #[serde(default)]
    pub prefix: Option<String>,

    /// AWS-specific: region (defaults to us-east-1)
    #[serde(default)]
    pub region: Option<String>,

    /// AWS-specific: endpoint URL (for S3-compatible services)
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address (default: 0.0.0.0:8080)
    #[serde(default = "default_bind_address")]
    pub bind_address: SocketAddr,

    /// Request timeout in seconds (default: 300)
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// Max request body size in bytes (default: 5GB)
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

fn default_bind_address() -> SocketAddr {
    "0.0.0.0:8080".parse().unwrap()
}

fn default_timeout_secs() -> u64 {
    300
}

fn default_max_body_size() -> usize {
    5 * 1024 * 1024 * 1024 // 5GB
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,

    /// Backend storage configuration
    pub backend: BackendConfig,

    /// Log level (default: info)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - S3PROXY_BACKEND_TYPE: aws|azure|gcp
    /// - S3PROXY_BACKEND_CONTAINER: container/bucket name
    /// - S3PROXY_BACKEND_PREFIX: optional path prefix
    /// - S3PROXY_BACKEND_REGION: AWS region (optional)
    /// - S3PROXY_BACKEND_ENDPOINT: custom endpoint URL (optional)
    /// - S3PROXY_BIND_ADDRESS: server bind address (default: 0.0.0.0:8080)
    /// - S3PROXY_TIMEOUT_SECS: request timeout (default: 300)
    /// - S3PROXY_MAX_BODY_SIZE: max request size in bytes (default: 5GB)
    /// - S3PROXY_LOG_LEVEL: log level (default: info)
    /// - S3PROXY_CONFIG_FILE: optional path to TOML config file
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load from config file first if specified
        let config_file = std::env::var("S3PROXY_CONFIG_FILE").ok();
        let mut config = if let Some(path) = &config_file {
            Self::from_file(path)?
        } else {
            Self::default()
        };

        // Override with environment variables
        if let Ok(backend_type) = std::env::var("S3PROXY_BACKEND_TYPE") {
            config.backend.backend_type = BackendType::from_str(&backend_type)?;
        }

        if let Ok(container) = std::env::var("S3PROXY_BACKEND_CONTAINER") {
            config.backend.container_or_bucket = container;
        }

        if let Ok(prefix) = std::env::var("S3PROXY_BACKEND_PREFIX") {
            config.backend.prefix = Some(prefix);
        }

        if let Ok(region) = std::env::var("S3PROXY_BACKEND_REGION") {
            config.backend.region = Some(region);
        }

        if let Ok(endpoint) = std::env::var("S3PROXY_BACKEND_ENDPOINT") {
            config.backend.endpoint = Some(endpoint);
        }

        if let Ok(addr) = std::env::var("S3PROXY_BIND_ADDRESS") {
            config.server.bind_address = addr.parse()?;
        }

        if let Ok(timeout) = std::env::var("S3PROXY_TIMEOUT_SECS") {
            config.server.timeout_secs = timeout.parse()?;
        }

        if let Ok(size) = std::env::var("S3PROXY_MAX_BODY_SIZE") {
            config.server.max_body_size = size.parse()?;
        }

        if let Ok(level) = std::env::var("S3PROXY_LOG_LEVEL") {
            config.log_level = level;
        }

        Ok(config)
    }

    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the default configuration
    pub fn default() -> Self {
        Self {
            server: ServerConfig {
                bind_address: default_bind_address(),
                timeout_secs: default_timeout_secs(),
                max_body_size: default_max_body_size(),
            },
            backend: BackendConfig {
                backend_type: BackendType::Aws,
                container_or_bucket: "default-bucket".to_string(),
                prefix: None,
                region: Some("us-east-1".to_string()),
                endpoint: None,
            },
            log_level: default_log_level(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_type_parsing() {
        assert_eq!(BackendType::from_str("aws").unwrap(), BackendType::Aws);
        assert_eq!(BackendType::from_str("azure").unwrap(), BackendType::Azure);
        assert_eq!(BackendType::from_str("gcp").unwrap(), BackendType::Gcp);
    }
}

