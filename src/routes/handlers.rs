//! Request handlers for S3 API endpoints

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use object_store::ObjectMeta;
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use tracing::{error, info, instrument};

use crate::errors::{Result, S3ProxyError};
use crate::s3;
use crate::storage::StorageBackend;

/// Health check endpoint
#[instrument]
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Readiness probe endpoint
#[instrument]
pub async fn ready() -> impl IntoResponse {
    // TODO: Add backend connectivity check
    (StatusCode::OK, "Ready")
}

/// Prometheus metrics endpoint
#[instrument]
pub async fn metrics() -> impl IntoResponse {
    use crate::metrics::REGISTRY;
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// GetObject - GET /{bucket}/{key}
#[instrument(skip(storage))]
pub async fn get_object(
    State(storage): State<Arc<dyn StorageBackend>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    info!(bucket = %bucket, key = %key, "GetObject request");

    let data = storage.get(&key).await.map_err(|e| {
        error!(error = %e, "Storage get failed");
        S3ProxyError::Storage(e)
    })?;

    // TODO: Add content-type detection based on file extension
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .header("content-length", data.len())
        .body(Body::from(data))
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// PutObject - PUT /{bucket}/{key}
#[instrument(skip(storage))]
pub async fn put_object(
    State(storage): State<Arc<dyn StorageBackend>>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    info!(bucket = %bucket, key = %key, size = body.len(), "PutObject request");

    // TODO: Extract and store metadata from x-amz-meta-* headers
    let _metadata = s3::extract_metadata(&headers);

    storage.put(&key, body).await.map_err(|e| {
        error!(error = %e, "Storage put failed");
        S3ProxyError::Storage(e)
    })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("etag", format!("\"{}\"", uuid::Uuid::new_v4()))
        .body(Body::empty())
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// DeleteObject - DELETE /{bucket}/{key}
#[instrument(skip(storage))]
pub async fn delete_object(
    State(storage): State<Arc<dyn StorageBackend>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    info!(bucket = %bucket, key = %key, "DeleteObject request");

    storage.delete(&key).await.map_err(|e| {
        error!(error = %e, "Storage delete failed");
        S3ProxyError::Storage(e)
    })?;

    let response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// HeadObject - HEAD /{bucket}/{key}
#[instrument(skip(storage))]
pub async fn head_object(
    State(storage): State<Arc<dyn StorageBackend>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response> {
    info!(bucket = %bucket, key = %key, "HeadObject request");

    let meta = storage.head(&key).await.map_err(|e| {
        error!(error = %e, "Storage head failed");
        S3ProxyError::Storage(e)
    })?;

    // ObjectMeta in object_store 0.10 doesn't have etag field directly
    // We'll generate a simple etag or leave it empty
    let etag = format!("\"{}\"", uuid::Uuid::new_v4());
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-length", meta.size)
        .header("last-modified", format!("{}", meta.last_modified.format("%a, %d %b %Y %H:%M:%S GMT")))
        .header("etag", etag)
        .body(Body::empty())
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// ListObjectsV2 - GET /{bucket}?prefix=...
#[instrument(skip(storage))]
pub async fn list_objects(
    State(storage): State<Arc<dyn StorageBackend>>,
    Path(bucket): Path<String>,
    Query(params): Query<crate::routes::ListObjectsQuery>,
) -> Result<Response> {
    info!(bucket = %bucket, prefix = ?params.prefix, "ListObjects request");

    let prefix = params.prefix.as_deref().unwrap_or("");
    let max_keys = params.max_keys.unwrap_or(1000);

    let objects = storage.list(prefix).await.map_err(|e| {
        error!(error = %e, "Storage list failed");
        S3ProxyError::Storage(e)
    })?;

    // Convert object_store::ObjectMeta to S3 Object format
    let mut s3_objects = Vec::new();
    for meta in objects.iter().take(max_keys as usize) {
        // Generate a simple etag since ObjectMeta doesn't expose it directly
        let etag = format!("\"{}\"", uuid::Uuid::new_v4());
        s3_objects.push(s3::Object {
            key: meta.location.to_string(),
            last_modified: meta.last_modified.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            etag,
            size: meta.size as u64,
            storage_class: "STANDARD".to_string(),
        });
    }

    let result = s3::ListObjectsV2Result {
        name: bucket,
        prefix: params.prefix,
        max_keys,
        is_truncated: objects.len() > max_keys as usize,
        contents: s3_objects,
        common_prefixes: None, // TODO: Implement delimiter support
    };

    let xml = result.to_xml().map_err(|e| {
        error!(error = %e, "XML serialization failed");
        S3ProxyError::Internal(format!("XML serialization failed: {}", e))
    })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/xml")
        .body(Body::from(xml))
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// CreateBucket - PUT /{bucket}
#[instrument]
pub async fn create_bucket(Path(bucket): Path<String>) -> Result<Response> {
    info!(bucket = %bucket, "CreateBucket request (noop)");
    
    // Bucket creation is a noop - the bucket/container should already exist
    // in the backend storage system
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// DeleteBucket - DELETE /{bucket}
#[instrument]
pub async fn delete_bucket(Path(bucket): Path<String>) -> Result<Response> {
    info!(bucket = %bucket, "DeleteBucket request (noop)");
    
    // Bucket deletion is a noop - buckets/containers are managed externally
    let response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .map_err(|e| S3ProxyError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

