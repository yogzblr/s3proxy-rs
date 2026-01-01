//! Azure Blob Storage backend implementation
//!
//! Uses object_store::azure::MicrosoftAzure with support for:
//! - Managed identity (system or user-assigned)
//! - Workload identity federation in AKS
//! - Explicit credentials (storage account access key)
//!
//! When using managed identity, authentication is handled via
//! azure_identity::DefaultAzureCredential which automatically discovers:
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

use crate::config::AzureConfig;
use crate::storage::StorageBackend;

/// Azure Blob Storage backend
pub struct AzureBackend {
    store: Arc<MicrosoftAzure>,
    prefix: Option<String>,
}

impl AzureBackend {
    /// Create a new Azure Blob Storage backend
    ///
    /// Supports two authentication modes:
    /// 1. Managed identity (default): Uses DefaultAzureCredential
    /// 2. Explicit credentials: Uses provided access_key
    pub async fn new(config: &AzureConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut builder = MicrosoftAzureBuilder::new()
            .with_account(&config.account_name)
            .with_container_name(&config.container_name);

        // Configure authentication
        if !config.use_managed_identity {
            // Use explicit credentials
            // object_store's Azure builder supports with_access_key method
            if let Some(access_key) = &config.access_key {
                // Try to use with_access_key if available, otherwise set env var
                // Note: object_store may use different method names
                builder = builder.with_access_key(access_key);
            } else {
                return Err("Azure access_key is required when use_managed_identity is false".into());
            }
        }
        // If use_managed_identity is true, builder will use DefaultAzureCredential

        // Configure emulator (for local development)
        if config.use_emulator {
            builder = builder.with_use_emulator(true);
        }

        // Build the store
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

    #[allow(dead_code)] // Part of trait interface for extensibility
    fn object_store(&self) -> &dyn ObjectStore {
        self.store.as_ref()
    }
}
