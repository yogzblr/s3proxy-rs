# S3Proxy-RS Architecture

## Overview

S3Proxy-RS is a production-grade S3-compatible HTTP proxy written in Rust. It provides a unified S3 API interface over multiple cloud object storage backends (AWS S3, Azure Blob Storage, Google Cloud Storage) using managed identity for authentication.

## Design Principles

1. **Cloud-Native**: Designed for Kubernetes with workload identity
2. **Zero Secrets**: No static credentials - uses managed identity exclusively
3. **Async-First**: Fully async using Tokio for high performance
4. **Production-Grade**: Comprehensive error handling, observability, graceful shutdown
5. **Modular**: Clean separation of concerns with trait-based abstractions

## Architecture Layers

### 1. HTTP Layer (`src/server/`, `src/routes/`)

- **Framework**: Axum on top of Hyper
- **Middleware**: Tower middleware stack
  - Request ID propagation
  - Timeout handling
  - Request tracing
  - Compression
- **Routes**: S3-compatible endpoints
  - Object operations (GET, PUT, DELETE, HEAD)
  - Bucket operations (LIST, CREATE, DELETE - noop)
  - System endpoints (health, ready, metrics)

### 2. Storage Abstraction Layer (`src/storage/`)

- **Trait**: `StorageBackend` provides unified interface
- **Implementations**:
  - `AwsBackend`: AWS S3 via `object_store::aws::AmazonS3`
  - `AzureBackend`: Azure Blob via `object_store::azure::MicrosoftAzure`
  - `GcpBackend`: GCS via `object_store::gcp::GoogleCloudStorage`
- **Path Mapping**: Optional prefix support for multi-tenant scenarios

### 3. Configuration Layer (`src/config.rs`)

- **Sources**: Environment variables (primary) + optional TOML file
- **Backend Config**: Type, container/bucket, prefix, region, endpoint
- **Server Config**: Bind address, timeouts, limits

### 4. Observability Layer (`src/metrics.rs`)

- **Logging**: Structured JSON logs via `tracing`
- **Metrics**: Prometheus metrics for HTTP and storage operations
- **Request IDs**: Unique ID per request for distributed tracing

### 5. Error Handling (`src/errors.rs`)

- **Error Types**: Structured errors using `thiserror`
- **S3 Compatibility**: Errors formatted as S3 XML responses
- **Status Mapping**: Proper HTTP status codes

## Data Flow

### GetObject Request

```
Client → Axum Router → Handler → StorageBackend → object_store → Cloud Provider
                                                                    ↓
Client ← Axum Router ← Handler ← StorageBackend ← object_store ← Response
```

### Authentication Flow

```
Kubernetes Pod
  ↓
ServiceAccount (annotated with workload identity)
  ↓
Cloud Provider Identity Service
  ↓
object_store credential provider
  ↓
Storage Backend API
```

## Storage Backend Details

### AWS S3

- **Credential Chain**: Default AWS credential provider
  - IRSA role (via `eks.amazonaws.com/role-arn` annotation)
  - Environment variables
  - EC2 instance metadata
  - ECS task role
- **Implementation**: `object_store::aws::AmazonS3`
- **Configuration**: Bucket name, region, optional endpoint

### Azure Blob Storage

- **Credential Chain**: `DefaultAzureCredential`
  - Managed identity (system/user-assigned)
  - Workload identity (AKS)
  - Environment variables
  - Azure CLI
- **Implementation**: `object_store::azure::MicrosoftAzure`
- **Configuration**: Container name, optional prefix

### Google Cloud Storage

- **Credential Chain**: Application Default Credentials (ADC)
  - Workload Identity (GKE)
  - `GOOGLE_APPLICATION_CREDENTIALS`
  - GCE metadata server
  - User credentials
- **Implementation**: `object_store::gcp::GoogleCloudStorage`
- **Configuration**: Bucket name, optional prefix

## Request Processing

1. **Receive**: Axum receives HTTP request
2. **Route**: Match to handler based on path
3. **Extract**: Parse bucket/key from path
4. **Authenticate**: Storage backend uses managed identity
5. **Execute**: Call storage backend operation
6. **Transform**: Convert response to S3-compatible format
7. **Return**: Send HTTP response to client

## Error Handling Strategy

- **Storage Errors**: Mapped to S3 error codes
- **HTTP Errors**: Proper status codes with S3 XML format
- **Internal Errors**: Logged with context, returned as 500
- **Validation Errors**: 400 Bad Request with descriptive message

## Performance Considerations

- **Async I/O**: All operations are async, no blocking calls
- **Streaming**: Large objects streamed (via object_store)
- **Connection Pooling**: Handled by object_store and HTTP clients
- **Zero-Copy**: Where possible, data passed without copying
- **Backpressure**: Proper handling via async streams

## Extensibility Points

### TODO: Signature Verification

Current implementation does not verify S3 signatures. This can be added as:
- Optional middleware
- Pluggable authentication layer
- Configuration flag

### TODO: Multipart Uploads

Not yet implemented. Can be added by:
- Extending `StorageBackend` trait
- Adding handlers for multipart operations
- Managing upload state

### TODO: Advanced Metadata

Currently basic metadata extraction. Can be enhanced:
- Custom metadata headers
- Metadata storage/retrieval
- Content-type detection

## Security Considerations

- **No Static Credentials**: All authentication via managed identity
- **TLS**: Should be terminated at ingress/LB
- **Input Validation**: Path sanitization, size limits
- **Rate Limiting**: Can be added via middleware
- **Audit Logging**: All operations logged with request IDs

## Deployment Architecture

```
┌─────────────────────────────────────┐
│         Kubernetes Cluster          │
│  ┌───────────────────────────────┐ │
│  │      Ingress / LoadBalancer   │ │
│  └───────────────┬───────────────┘ │
│  ┌───────────────▼───────────────┐ │
│  │      S3Proxy Service           │ │
│  └───────────────┬───────────────┘ │
│  ┌───────────────▼───────────────┐ │
│  │   S3Proxy Pods (2+ replicas)  │ │
│  │  ┌──────────────────────────┐  │ │
│  │  │  ServiceAccount         │  │ │
│  │  │  (Workload Identity)    │  │ │
│  │  └──────────────────────────┘  │ │
│  └────────────────────────────────┘ │
└─────────────────────────────────────┘
         │
         │ Managed Identity
         │
┌────────▼────────────────────────────┐
│      Cloud Provider                 │
│  (AWS / Azure / GCP)               │
└─────────────────────────────────────┘
```

## Future Enhancements

1. **Caching Layer**: Optional Redis/Memcached for frequently accessed objects
2. **Multi-Backend**: Support multiple backends with routing rules
3. **Transformation**: On-the-fly compression, encryption
4. **Analytics**: Request analytics and reporting
5. **Webhooks**: Event notifications for object operations

