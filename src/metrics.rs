//! Prometheus metrics for S3Proxy
//!
//! Defines metrics for:
//! - Request counts by method and status
//! - Request latency
//! - Storage operation duration
//! - Error counts

use lazy_static::lazy_static;
use prometheus::{Counter, Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry};

lazy_static! {
    /// Registry for all metrics
    pub static ref REGISTRY: Registry = Registry::new();

    /// HTTP request counter by method and status
    pub static ref HTTP_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("s3proxy_http_requests_total", "Total HTTP requests"),
        &["method", "status"]
    )
    .expect("Failed to create HTTP_REQUESTS metric");

    /// HTTP request latency histogram
    pub static ref HTTP_REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "s3proxy_http_request_duration_seconds",
            "HTTP request duration in seconds"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    )
    .expect("Failed to create HTTP_REQUEST_DURATION metric");

    /// Storage operation counter by operation and status
    pub static ref STORAGE_OPERATIONS: IntCounterVec = IntCounterVec::new(
        Opts::new("s3proxy_storage_operations_total", "Total storage operations"),
        &["operation", "status"]
    )
    .expect("Failed to create STORAGE_OPERATIONS metric");

    /// Storage operation duration histogram
    pub static ref STORAGE_OPERATION_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "s3proxy_storage_operation_duration_seconds",
            "Storage operation duration in seconds"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    )
    .expect("Failed to create STORAGE_OPERATION_DURATION metric");
}

/// Initialize metrics and register with the global registry
pub fn init_metrics() {
    REGISTRY.register(Box::new(HTTP_REQUESTS.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(STORAGE_OPERATIONS.clone())).unwrap();
    REGISTRY.register(Box::new(STORAGE_OPERATION_DURATION.clone())).unwrap();
}

