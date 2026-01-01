//! Google Cloud Storage backend implementation
//!
//! Uses object_store::gcp::GoogleCloudStorage with Application Default
//! Credentials (ADC). Supports:
//! - Workload Identity in GKE
//! - Service account keys via GOOGLE_APPLICATION_CREDENTIALS
//! - GCE metadata server
//! - User credentials (gcloud auth application-default login)
//!
//! Authentication follows the ADC chain automatically.

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore};
use std::sync::Arc;

use crate::config::Config;
use crate::storage::StorageBackend;

/// Google Cloud Storage backend
pub struct GcpBackend {
    store: Arc<GoogleCloudStorage>,
    prefix: Option<String>,
}

impl GcpBackend {
    /// Create a new GCP Cloud Storage backend
    ///
    /// Uses Application Default Credentials (ADC) which supports:
    /// - Workload Identity in GKE
    /// - GOOGLE_APPLICATION_CREDENTIALS environment variable
    /// - GCE metadata server
    /// - User credentials
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        // object_store's GCP builder uses ADC automatically
        // when no explicit credentials are provided
        let store = Arc::new(
            GoogleCloudStorageBuilder::new()
                .with_bucket_name(&config.backend.container_or_bucket)
                // Use Application Default Credentials
                .build()?,
        );

        Ok(Self {
            store,
            prefix: config.backend.prefix.clone(),
        })
    }

    /// Apply prefix to path if configured
    fn apply_prefix(&self, path: &str) -> Path {
        let full_path = if let Some(prefix) = &self.prefix {
            format!("{}/{}", prefix.trim_end_matches('/'), path)
        } else {
            path.to_string()
        };
        Path::from(full_path)
    }
}

#[async_trait]
impl StorageBackend for GcpBackend {
    async fn get(&self, path: &str) -> Result<Bytes, object_store::Error> {
        let path = self.apply_prefix(path);
        let data = self.store.get(&path).await?;
        let bytes = data.bytes().await?;
        Ok(bytes)
    }

    async fn put(&self, path: &str, data: Bytes) -> Result<(), object_store::Error> {
        let path = self.apply_prefix(path);
        self.store.put(&path, data.into()).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), object_store::Error> {
        let path = self.apply_prefix(path);
        self.store.delete(&path).await?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>, object_store::Error> {
        let prefix = self.apply_prefix(prefix);
        let mut results = vec![];
        let mut stream = self.store.list(Some(&prefix));

        while let Some(meta) = stream.next().await {
            results.push(meta?);
        }

        Ok(results)
    }

    async fn head(&self, path: &str) -> Result<ObjectMeta, object_store::Error> {
        let path = self.apply_prefix(path);
        self.store.head(&path).await
    }

    fn object_store(&self) -> &dyn ObjectStore {
        self.store.as_ref()
    }
}

