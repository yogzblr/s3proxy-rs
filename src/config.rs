//! Configuration management for S3Proxy
//!
//! Supports configuration via:
//! - Environment variables (primary)
//! - Optional TOML config file (secondary)
//!
//! Environment variables take precedence over config file values.
//!
//! Supports two authentication modes:
//! 1. Explicit credentials (access keys, service account keys)
//! 2. Managed identity (IRSA for AWS, Workload Identity for Azure/GCP)

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

/// AWS S3 specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    /// S3 bucket name (required)
    pub bucket_name: String,

    /// AWS region (required, e.g., "us-east-1")
    pub region: String,

    /// Optional custom endpoint URL (for S3-compatible services like MinIO)
    #[serde(default)]
    pub endpoint: Option<String>,

    /// Use managed identity (IRSA) instead of explicit credentials
    /// If true, access_key_id and secret_access_key are ignored
    #[serde(default = "default_true")]
    pub use_managed_identity: bool,

    /// AWS access key ID (optional, required if use_managed_identity is false)
    #[serde(default)]
    pub access_key_id: Option<String>,

    /// AWS secret access key (optional, required if use_managed_identity is false)
    #[serde(default)]
    pub secret_access_key: Option<String>,

    /// Allow HTTP connections (default: false, only HTTPS allowed)
    #[serde(default)]
    pub allow_http: bool,
}

fn default_true() -> bool {
    true
}

/// Azure Blob Storage specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    /// Azure storage account name (required)
    pub account_name: String,

    /// Container name (required)
    pub container_name: String,

    /// Use managed identity (Workload Identity) instead of explicit credentials
    /// If true, access_key is ignored
    #[serde(default = "default_true")]
    pub use_managed_identity: bool,

    /// Azure storage account access key (optional, required if use_managed_identity is false)
    #[serde(default)]
    pub access_key: Option<String>,

    /// Use Azure Storage Emulator (for local development)
    #[serde(default)]
    pub use_emulator: bool,
}

/// Google Cloud Storage specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    /// GCS bucket name (required)
    pub bucket_name: String,

    /// Use managed identity (Workload Identity) or Application Default Credentials
    /// If true, service_account_path and service_account_key are ignored
    #[serde(default = "default_true")]
    pub use_managed_identity: bool,

    /// Path to service account JSON key file (optional, used if use_managed_identity is false)
    /// Alternative to service_account_key
    #[serde(default)]
    pub service_account_path: Option<String>,

    /// Service account JSON key as string (optional, used if use_managed_identity is false)
    /// Alternative to service_account_path
    #[serde(default)]
    pub service_account_key: Option<String>,
}

/// Provider-specific backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    /// AWS S3 configuration
    #[serde(rename = "aws")]
    Aws(AwsConfig),

    /// Azure Blob Storage configuration
    #[serde(rename = "azure")]
    Azure(AzureConfig),

    /// Google Cloud Storage configuration
    #[serde(rename = "gcp")]
    Gcp(GcpConfig),
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

    /// Optional path prefix for all objects (applied to all backends)
    #[serde(default)]
    pub prefix: Option<String>,

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
    /// - S3PROXY_BACKEND_CONTAINER: container/bucket name (legacy, use provider-specific vars)
    /// - S3PROXY_BACKEND_PREFIX: optional path prefix
    /// - S3PROXY_BIND_ADDRESS: server bind address (default: 0.0.0.0:8080)
    /// - S3PROXY_TIMEOUT_SECS: request timeout (default: 300)
    /// - S3PROXY_MAX_BODY_SIZE: max request size in bytes (default: 5GB)
    /// - S3PROXY_LOG_LEVEL: log level (default: info)
    /// - S3PROXY_CONFIG_FILE: optional path to TOML config file
    ///
    /// AWS-specific:
    /// - S3PROXY_AWS_BUCKET: bucket name
    /// - S3PROXY_AWS_REGION: region (e.g., us-east-1)
    /// - S3PROXY_AWS_ENDPOINT: optional custom endpoint
    /// - S3PROXY_AWS_USE_MANAGED_IDENTITY: true|false (default: true)
    /// - S3PROXY_AWS_ACCESS_KEY_ID: access key (if not using managed identity)
    /// - S3PROXY_AWS_SECRET_ACCESS_KEY: secret key (if not using managed identity)
    ///
    /// Azure-specific:
    /// - S3PROXY_AZURE_ACCOUNT_NAME: storage account name
    /// - S3PROXY_AZURE_CONTAINER_NAME: container name
    /// - S3PROXY_AZURE_USE_MANAGED_IDENTITY: true|false (default: true)
    /// - S3PROXY_AZURE_ACCESS_KEY: access key (if not using managed identity)
    ///
    /// GCP-specific:
    /// - S3PROXY_GCP_BUCKET: bucket name
    /// - S3PROXY_GCP_USE_MANAGED_IDENTITY: true|false (default: true)
    /// - S3PROXY_GCP_SERVICE_ACCOUNT_PATH: path to service account JSON file
    /// - S3PROXY_GCP_SERVICE_ACCOUNT_KEY: service account JSON key as string
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load from config file first if specified
        let config_file = std::env::var("S3PROXY_CONFIG_FILE").ok();
        let mut config = if let Some(path) = &config_file {
            Self::from_file(path)?
        } else {
            // Build config from environment variables
            Self::from_env_only()?
        };

        // Override with environment variables (env vars take precedence)
        config.apply_env_overrides()?;

        Ok(config)
    }

    /// Build configuration from environment variables only
    fn from_env_only() -> Result<Self, Box<dyn std::error::Error>> {
        let backend_type = std::env::var("S3PROXY_BACKEND_TYPE")
            .unwrap_or_else(|_| "aws".to_string());
        let backend_type = BackendType::from_str(&backend_type)?;

        let backend = match backend_type {
            BackendType::Aws => {
                let bucket_name = std::env::var("S3PROXY_AWS_BUCKET")
                    .or_else(|_| std::env::var("S3PROXY_BACKEND_CONTAINER"))
                    .map_err(|_| "S3PROXY_AWS_BUCKET or S3PROXY_BACKEND_CONTAINER must be set")?;
                let region = std::env::var("S3PROXY_AWS_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string());
                let use_managed_identity = std::env::var("S3PROXY_AWS_USE_MANAGED_IDENTITY")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse::<bool>()
                    .unwrap_or(true);

                BackendConfig::Aws(AwsConfig {
                    bucket_name,
                    region,
                    endpoint: std::env::var("S3PROXY_AWS_ENDPOINT").ok(),
                    use_managed_identity,
                    access_key_id: std::env::var("S3PROXY_AWS_ACCESS_KEY_ID").ok(),
                    secret_access_key: std::env::var("S3PROXY_AWS_SECRET_ACCESS_KEY").ok(),
                    allow_http: std::env::var("S3PROXY_AWS_ALLOW_HTTP")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse::<bool>()
                        .unwrap_or(false),
                })
            }
            BackendType::Azure => {
                let account_name = std::env::var("S3PROXY_AZURE_ACCOUNT_NAME")
                    .map_err(|_| "S3PROXY_AZURE_ACCOUNT_NAME must be set")?;
                let container_name = std::env::var("S3PROXY_AZURE_CONTAINER_NAME")
                    .or_else(|_| std::env::var("S3PROXY_BACKEND_CONTAINER"))
                    .map_err(|_| "S3PROXY_AZURE_CONTAINER_NAME or S3PROXY_BACKEND_CONTAINER must be set")?;
                let use_managed_identity = std::env::var("S3PROXY_AZURE_USE_MANAGED_IDENTITY")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse::<bool>()
                    .unwrap_or(true);

                BackendConfig::Azure(AzureConfig {
                    account_name,
                    container_name,
                    use_managed_identity,
                    access_key: std::env::var("S3PROXY_AZURE_ACCESS_KEY").ok(),
                    use_emulator: std::env::var("S3PROXY_AZURE_USE_EMULATOR")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse::<bool>()
                        .unwrap_or(false),
                })
            }
            BackendType::Gcp => {
                let bucket_name = std::env::var("S3PROXY_GCP_BUCKET")
                    .or_else(|_| std::env::var("S3PROXY_BACKEND_CONTAINER"))
                    .map_err(|_| "S3PROXY_GCP_BUCKET or S3PROXY_BACKEND_CONTAINER must be set")?;
                let use_managed_identity = std::env::var("S3PROXY_GCP_USE_MANAGED_IDENTITY")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse::<bool>()
                    .unwrap_or(true);

                BackendConfig::Gcp(GcpConfig {
                    bucket_name,
                    use_managed_identity,
                    service_account_path: std::env::var("S3PROXY_GCP_SERVICE_ACCOUNT_PATH").ok(),
                    service_account_key: std::env::var("S3PROXY_GCP_SERVICE_ACCOUNT_KEY").ok(),
                })
            }
        };

        Ok(Config {
            server: ServerConfig {
                bind_address: std::env::var("S3PROXY_BIND_ADDRESS")
                    .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
                    .parse()
                    .unwrap_or_else(|_| default_bind_address()),
                timeout_secs: std::env::var("S3PROXY_TIMEOUT_SECS")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
                max_body_size: std::env::var("S3PROXY_MAX_BODY_SIZE")
                    .unwrap_or_else(|_| "5368709120".to_string())
                    .parse()
                    .unwrap_or(5 * 1024 * 1024 * 1024),
            },
            backend,
            prefix: std::env::var("S3PROXY_BACKEND_PREFIX").ok(),
            log_level: std::env::var("S3PROXY_LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }

    /// Apply environment variable overrides to existing config
    fn apply_env_overrides(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Server config overrides
        if let Ok(addr) = std::env::var("S3PROXY_BIND_ADDRESS") {
            self.server.bind_address = addr.parse()?;
        }
        if let Ok(timeout) = std::env::var("S3PROXY_TIMEOUT_SECS") {
            self.server.timeout_secs = timeout.parse()?;
        }
        if let Ok(size) = std::env::var("S3PROXY_MAX_BODY_SIZE") {
            self.server.max_body_size = size.parse()?;
        }
        if let Ok(level) = std::env::var("S3PROXY_LOG_LEVEL") {
            self.log_level = level;
        }
        if let Ok(prefix) = std::env::var("S3PROXY_BACKEND_PREFIX") {
            self.prefix = Some(prefix);
        }

        // Backend-specific overrides
        match &mut self.backend {
            BackendConfig::Aws(aws) => {
                if let Ok(bucket) = std::env::var("S3PROXY_AWS_BUCKET") {
                    aws.bucket_name = bucket;
                }
                if let Ok(region) = std::env::var("S3PROXY_AWS_REGION") {
                    aws.region = region;
                }
                if let Ok(endpoint) = std::env::var("S3PROXY_AWS_ENDPOINT") {
                    aws.endpoint = Some(endpoint);
                }
                if let Ok(use_mi) = std::env::var("S3PROXY_AWS_USE_MANAGED_IDENTITY") {
                    aws.use_managed_identity = use_mi.parse().unwrap_or(true);
                }
                if let Ok(key_id) = std::env::var("S3PROXY_AWS_ACCESS_KEY_ID") {
                    aws.access_key_id = Some(key_id);
                }
                if let Ok(secret) = std::env::var("S3PROXY_AWS_SECRET_ACCESS_KEY") {
                    aws.secret_access_key = Some(secret);
                }
            }
            BackendConfig::Azure(azure) => {
                if let Ok(account) = std::env::var("S3PROXY_AZURE_ACCOUNT_NAME") {
                    azure.account_name = account;
                }
                if let Ok(container) = std::env::var("S3PROXY_AZURE_CONTAINER_NAME") {
                    azure.container_name = container;
                }
                if let Ok(use_mi) = std::env::var("S3PROXY_AZURE_USE_MANAGED_IDENTITY") {
                    azure.use_managed_identity = use_mi.parse().unwrap_or(true);
                }
                if let Ok(key) = std::env::var("S3PROXY_AZURE_ACCESS_KEY") {
                    azure.access_key = Some(key);
                }
            }
            BackendConfig::Gcp(gcp) => {
                if let Ok(bucket) = std::env::var("S3PROXY_GCP_BUCKET") {
                    gcp.bucket_name = bucket;
                }
                if let Ok(use_mi) = std::env::var("S3PROXY_GCP_USE_MANAGED_IDENTITY") {
                    gcp.use_managed_identity = use_mi.parse().unwrap_or(true);
                }
                if let Ok(path) = std::env::var("S3PROXY_GCP_SERVICE_ACCOUNT_PATH") {
                    gcp.service_account_path = Some(path);
                }
                if let Ok(key) = std::env::var("S3PROXY_GCP_SERVICE_ACCOUNT_KEY") {
                    gcp.service_account_key = Some(key);
                }
            }
        }

        Ok(())
    }

    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get backend type
    #[allow(dead_code)] // Useful for logging/debugging
    pub fn backend_type(&self) -> BackendType {
        match self.backend {
            BackendConfig::Aws(_) => BackendType::Aws,
            BackendConfig::Azure(_) => BackendType::Azure,
            BackendConfig::Gcp(_) => BackendType::Gcp,
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
