//! AWS S3 storage backend implementation
//!
//! Uses object_store::aws::AmazonS3 with managed identity via IRSA
//! (IAM Role for Service Account) in Kubernetes. Relies on the default
//! AWS credential chain which automatically picks up:
//! - IRSA role annotations in Kubernetes
//! - Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
//! - EC2 instance metadata
//! - ECS task role
//!
//! No static credentials should be required in normal operation.

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore};
use std::sync::Arc;

use crate::config::Config;
use crate::storage::StorageBackend;

/// AWS S3 storage backend
pub struct AwsBackend {
    store: Arc<AmazonS3>,
    prefix: Option<String>,
}

impl AwsBackend {
    /// Create a new AWS S3 backend
    ///
    /// Uses the default AWS credential provider chain which supports:
    /// - IRSA (IAM Role for Service Account) in EKS
    /// - Environment variables
    /// - EC2 instance metadata
    /// - ECS task role
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let region = config
            .backend
            .region
            .as_deref()
            .unwrap_or("us-east-1");

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&config.backend.container_or_bucket)
            .with_region(region);

        // If custom endpoint is provided (e.g., for S3-compatible services)
        if let Some(endpoint) = &config.backend.endpoint {
            builder = builder.with_endpoint(endpoint);
        }

        // The builder will use the default credential provider chain
        // which automatically handles IRSA, environment variables, etc.
        let store = Arc::new(builder.build()?);

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
impl StorageBackend for AwsBackend {
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

