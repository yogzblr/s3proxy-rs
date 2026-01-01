//! AWS S3 storage backend implementation
//!
//! Uses object_store::aws::AmazonS3 with support for:
//! - Managed identity via IRSA (IAM Role for Service Account) in Kubernetes
//! - Explicit credentials (access key ID and secret access key)
//!
//! When using managed identity, relies on the default AWS credential chain:
//! - IRSA role annotations in Kubernetes
//! - Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
//! - EC2 instance metadata
//! - ECS task role

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore};
use std::sync::Arc;

use crate::config::AwsConfig;
use crate::storage::StorageBackend;

/// AWS S3 storage backend
pub struct AwsBackend {
    store: Arc<AmazonS3>,
    prefix: Option<String>,
}

impl AwsBackend {
    /// Create a new AWS S3 backend
    ///
    /// Supports two authentication modes:
    /// 1. Managed identity (default): Uses default AWS credential provider chain
    /// 2. Explicit credentials: Sets AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY env vars
    pub async fn new(config: &AwsConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Configure authentication
        if !config.use_managed_identity {
            // Use explicit credentials via environment variables
            // object_store uses the AWS SDK which reads from environment variables
            if let (Some(access_key_id), Some(secret_access_key)) =
                (&config.access_key_id, &config.secret_access_key)
            {
                std::env::set_var("AWS_ACCESS_KEY_ID", access_key_id);
                std::env::set_var("AWS_SECRET_ACCESS_KEY", secret_access_key);
            } else {
                return Err("AWS credentials (access_key_id and secret_access_key) are required when use_managed_identity is false".into());
            }
        }
        // If use_managed_identity is true, builder will use default credential chain
        // (IRSA, environment variables, EC2 metadata, etc.)

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&config.bucket_name)
            .with_region(&config.region);

        // Configure endpoint (for S3-compatible services like MinIO)
        if let Some(endpoint) = &config.endpoint {
            builder = builder.with_endpoint(endpoint);
        }

        // Configure HTTP/HTTPS
        if config.allow_http {
            builder = builder.with_allow_http(true);
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

    #[allow(dead_code)] // Part of trait interface for extensibility
    fn object_store(&self) -> &dyn ObjectStore {
        self.store.as_ref()
    }
}
