//! Storage backend abstraction layer
//!
//! Provides a unified interface for interacting with different object storage
//! backends (AWS S3, Azure Blob Storage, Google Cloud Storage) using the
//! object_store crate. Supports both explicit credentials and managed identity
//! for authentication.

mod aws;
mod azure;
mod gcp;

use async_trait::async_trait;
use bytes::Bytes;
use object_store::{ObjectMeta, ObjectStore};
use std::sync::Arc;

use crate::config::Config;

pub use aws::AwsBackend;
pub use azure::AzureBackend;
pub use gcp::GcpBackend;

/// Storage backend trait for unified object storage operations
///
/// All storage operations flow through this trait, which abstracts over
/// the different cloud providers. Implementations delegate to object_store
/// for the actual operations.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Get an object by path
    async fn get(&self, path: &str) -> Result<Bytes, object_store::Error>;

    /// Put an object at the given path
    async fn put(&self, path: &str, data: Bytes) -> Result<(), object_store::Error>;

    /// Delete an object at the given path
    async fn delete(&self, path: &str) -> Result<(), object_store::Error>;

    /// List objects with the given prefix
    async fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>, object_store::Error>;

    /// Get object metadata (HEAD operation)
    async fn head(&self, path: &str) -> Result<ObjectMeta, object_store::Error>;

    /// Get the underlying object store (for advanced operations)
    #[allow(dead_code)] // Part of trait interface for extensibility
    fn object_store(&self) -> &dyn ObjectStore;
}

/// Create a storage backend based on configuration
///
/// This function initializes the appropriate backend (AWS, Azure, or GCP)
/// using either explicit credentials or managed identity/workload identity
/// based on the configuration.
pub async fn create_backend(config: &Config) -> Result<Arc<dyn StorageBackend>, Box<dyn std::error::Error>> {
    match &config.backend {
        crate::config::BackendConfig::Aws(aws_config) => {
            let backend = AwsBackend::new(aws_config).await?;
            let backend = backend.with_prefix(config.prefix.clone());
            Ok(Arc::new(backend))
        }
        crate::config::BackendConfig::Azure(azure_config) => {
            let backend = AzureBackend::new(azure_config).await?;
            let backend = backend.with_prefix(config.prefix.clone());
            Ok(Arc::new(backend))
        }
        crate::config::BackendConfig::Gcp(gcp_config) => {
            let backend = GcpBackend::new(gcp_config).await?;
            let backend = backend.with_prefix(config.prefix.clone());
            Ok(Arc::new(backend))
        }
    }
}
