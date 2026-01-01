//! Google Cloud Storage backend implementation
//!
//! Uses object_store::gcp::GoogleCloudStorage with support for:
//! - Application Default Credentials (ADC) / Workload Identity
//! - Service account JSON key file
//! - Service account JSON key as string
//!
//! When using managed identity, authentication follows the ADC chain:
//! - Workload Identity in GKE
//! - GOOGLE_APPLICATION_CREDENTIALS environment variable
//! - GCE metadata server
//! - User credentials

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore};
use std::sync::Arc;

use crate::config::GcpConfig;
use crate::storage::StorageBackend;
use uuid::Uuid;

/// Google Cloud Storage backend
pub struct GcpBackend {
    store: Arc<GoogleCloudStorage>,
    prefix: Option<String>,
}

impl GcpBackend {
    /// Create a new GCP Cloud Storage backend
    ///
    /// Supports multiple authentication modes:
    /// 1. Managed identity (default): Uses Application Default Credentials (ADC)
    /// 2. Service account file: Uses service_account_path or GOOGLE_APPLICATION_CREDENTIALS env var
    /// 3. Service account key: Uses service_account_key (JSON string) via env var
    pub async fn new(config: &GcpConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Configure authentication
        if !config.use_managed_identity {
            // Use explicit service account credentials
            if let Some(service_account_path) = &config.service_account_path {
                // Set GOOGLE_APPLICATION_CREDENTIALS environment variable
                // object_store's GCP builder reads from this env var
                std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", service_account_path);
            } else if let Some(service_account_key) = &config.service_account_key {
                // For JSON key as string, write it to a temporary file
                // and set GOOGLE_APPLICATION_CREDENTIALS to point to it
                use std::io::Write;
                let temp_dir = std::env::temp_dir();
                let temp_file = temp_dir.join(format!("gcp-sa-key-{}.json", Uuid::new_v4()));
                let mut file = std::fs::File::create(&temp_file)?;
                file.write_all(service_account_key.as_bytes())?;
                file.sync_all()?;
                std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", temp_file.to_str().unwrap());
            } else {
                return Err("GCP service account credentials (service_account_path or service_account_key) are required when use_managed_identity is false".into());
            }
        }
        // If use_managed_identity is true, builder will use Application Default Credentials
        // (Workload Identity, GOOGLE_APPLICATION_CREDENTIALS, GCE metadata, etc.)

        // Build the store
        // The builder will use GOOGLE_APPLICATION_CREDENTIALS if set, or ADC if not
        let builder = GoogleCloudStorageBuilder::new()
            .with_bucket_name(&config.bucket_name);
        let store = Arc::new(builder.build()?);

        Ok(Self {
            store,
            prefix: None, // Prefix is applied at Config level
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

    /// Set the prefix for this backend
    pub fn with_prefix(mut self, prefix: Option<String>) -> Self {
        self.prefix = prefix;
        self
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

    #[allow(dead_code)] // Part of trait interface for extensibility
    fn object_store(&self) -> &dyn ObjectStore {
        self.store.as_ref()
    }
}
