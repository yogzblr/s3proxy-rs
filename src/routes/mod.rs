//! HTTP route handlers for S3-compatible API
//!
//! Implements the core S3 operations:
//! - GET /{bucket}/{key} - GetObject
//! - PUT /{bucket}/{key} - PutObject
//! - DELETE /{bucket}/{key} - DeleteObject
//! - HEAD /{bucket}/{key} - HeadObject
//! - GET /{bucket}?prefix=... - ListObjectsV2
//! - PUT /{bucket} - CreateBucket (noop)
//! - DELETE /{bucket} - DeleteBucket (noop)

mod handlers;

use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::storage::StorageBackend;

/// Query parameters for ListObjects operation
#[derive(Debug, serde::Deserialize)]
pub struct ListObjectsQuery {
    pub prefix: Option<String>,
    pub max_keys: Option<u32>,
    #[allow(dead_code)] // Reserved for future pagination support
    pub continuation_token: Option<String>,
}

/// Create the S3 API router
pub fn create_router(storage: Arc<dyn StorageBackend>) -> Router {
    use handlers;
    Router::new()
        .route("/healthz", get(handlers::health))
        .route("/ready", get(handlers::ready))
        .route("/metrics", get(handlers::metrics))
        .route("/:bucket", get(handlers::list_objects).put(handlers::create_bucket).delete(handlers::delete_bucket))
        .route("/:bucket/*key", get(handlers::get_object).put(handlers::put_object).delete(handlers::delete_object).head(handlers::head_object))
        .with_state(storage)
}

