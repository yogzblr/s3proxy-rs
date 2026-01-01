//! S3-compatible API response types and utilities
//!
//! Provides XML response generation for S3-compatible operations
//! including ListObjectsV2, error responses, and metadata handling.

use quick_xml::se::to_string;
use serde::Serialize;
use std::collections::HashMap;

/// S3 error response structure
#[derive(Debug, Serialize)]
#[serde(rename = "Error")]
#[allow(dead_code)] // Used by error_xml function
pub struct S3Error {
    pub code: String,
    pub message: String,
    pub resource: Option<String>,
    pub request_id: Option<String>,
}

/// ListObjectsV2 response structure
#[derive(Debug, Serialize)]
#[serde(rename = "ListBucketResult", rename_all = "PascalCase")]
pub struct ListObjectsV2Result {
    pub name: String,
    pub prefix: Option<String>,
    pub max_keys: u32,
    pub is_truncated: bool,
    pub contents: Vec<Object>,
    pub common_prefixes: Option<Vec<CommonPrefix>>,
}

/// Object entry in ListObjects response
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Object {
    pub key: String,
    pub last_modified: String,
    pub etag: String,
    pub size: u64,
    #[serde(rename = "StorageClass")]
    pub storage_class: String,
}

/// Common prefix entry in ListObjects response
#[derive(Debug, Serialize)]
pub struct CommonPrefix {
    pub prefix: String,
}

impl ListObjectsV2Result {
    /// Create a new ListObjectsV2 result
    #[allow(dead_code)] // Reserved for future use
    pub fn new(bucket: String, prefix: Option<String>, max_keys: u32) -> Self {
        Self {
            name: bucket,
            prefix,
            max_keys,
            is_truncated: false,
            contents: vec![],
            common_prefixes: None,
        }
    }

    /// Convert to XML string
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>{}"#,
            to_string(self)?
        );
        Ok(xml)
    }
}

/// Generate S3-compatible error XML
#[allow(dead_code)] // Utility function for future error handling
pub fn error_xml(code: &str, message: &str) -> String {
    let error = S3Error {
        code: code.to_string(),
        message: message.to_string(),
        resource: None,
        request_id: None,
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>{}</Code>
    <Message>{}</Message>
</Error>"#,
        error.code, error.message
    )
}

/// Extract metadata from HTTP headers
pub fn extract_metadata(headers: &axum::http::HeaderMap) -> HashMap<String, String> {
    let mut metadata = HashMap::new();
    
    for (key, value) in headers.iter() {
        if let Some(key_str) = key.as_str().strip_prefix("x-amz-meta-") {
            if let Ok(value_str) = value.to_str() {
                metadata.insert(key_str.to_string(), value_str.to_string());
            }
        }
    }
    
    metadata
}

