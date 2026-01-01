//! Error types for S3Proxy
//!
//! Provides structured error handling using thiserror for all error cases
//! encountered in the proxy, including storage operations, HTTP handling,
//! and configuration errors.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Main error type for S3Proxy operations
#[derive(Error, Debug)]
pub enum S3ProxyError {
    /// Storage backend operation failed
    #[error("Storage error: {0}")]
    Storage(#[from] object_store::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    #[allow(dead_code)] // Reserved for future configuration validation
    Config(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    #[allow(dead_code)] // Part of public API for request validation
    InvalidRequest(String),

    /// Object not found
    #[error("Object not found: {path}")]
    #[allow(dead_code)] // Part of public API, used in error response mapping
    NotFound { path: String },

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// XML serialization error
    #[error("XML error: {0}")]
    #[allow(dead_code)] // Reserved for future XML error handling
    Xml(String),
}

impl IntoResponse for S3ProxyError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            S3ProxyError::NotFound { path } => (
                StatusCode::NOT_FOUND,
                "NoSuchKey",
                format!("The specified key does not exist: {}", path),
            ),
            S3ProxyError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "InvalidRequest",
                msg,
            ),
            S3ProxyError::Storage(e) => {
                // Map object_store errors to S3-compatible errors
                match e {
                    object_store::Error::NotFound { .. } => (
                        StatusCode::NOT_FOUND,
                        "NoSuchKey",
                        "The specified key does not exist".to_string(),
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "InternalError",
                        format!("Storage operation failed: {}", e),
                    ),
                }
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "InternalError",
                format!("{}", self),
            ),
        };

        // Return S3-compatible XML error response
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>{}</Code>
    <Message>{}</Message>
    <Resource></Resource>
    <RequestId></RequestId>
</Error>"#,
            error_code, message
        );

        (status, [("content-type", "application/xml")], xml).into_response()
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, S3ProxyError>;

