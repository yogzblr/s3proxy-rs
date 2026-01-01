//! Azure Blob Storage backend implementation
//!
//! Uses object_store::azure::MicrosoftAzure with managed identity.
//! Supports:
//! - System-assigned managed identity
//! - User-assigned managed identity
//! - Workload identity federation in AKS
//!
//! Authentication is handled via azure_identity::DefaultAzureCredential
//! which automatically discovers credentials from:
//! - Environment variables (AZURE_CLIENT_ID, AZURE_TENANT_ID, etc.)
//! - Managed identity endpoint (in Azure VMs/containers)
//! - Azure CLI credentials
//! - Workload identity in AKS

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use object_store::azure::{MicrosoftAzure, MicrosoftAzureBuilder};
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore};
use std::sync::Arc;

use crate::config::Config;
use crate::storage::StorageBackend;

/// Azure Blob Storage backend
pub struct AzureBackend {
    store: Arc<MicrosoftAzure>,
    prefix: Option<String>,
}

impl AzureBackend {
    /// Create a new Azure Blob Storage backend
    ///
    /// Uses DefaultAzureCredential which supports:
    /// - Managed identity (system or user-assigned)
    /// - Workload identity in AKS
    /// - Environment variables
    /// - Azure CLI
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        // object_store's Azure builder uses DefaultAzureCredential internally
        // when no explicit credentials are provided
        let store = Arc::new(
            MicrosoftAzureBuilder::new()
                .with_container_name(&config.backend.container_or_bucket)
                // Use default credential chain (managed identity, env vars, etc.)
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
impl StorageBackend for AzureBackend {
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

