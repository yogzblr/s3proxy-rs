# S3Proxy-RS

Production-grade S3-compatible HTTP proxy for cloud object stores, written in Rust.

## Overview

S3Proxy-RS is a high-performance, cloud-native rewrite of [S3Proxy](https://github.com/gaul/s3proxy) in Rust. It provides an S3-compatible HTTP API that proxies requests to backend object stores (AWS S3, Azure Blob Storage, Google Cloud Storage) using managed identity/workload identity for authentication.

### Key Features

- **S3-Compatible API**: Implements core S3 operations (GET, PUT, DELETE, HEAD, ListObjects)
- **Multi-Cloud Support**: AWS S3, Azure Blob Storage, Google Cloud Storage
- **Managed Identity**: Uses IRSA (AWS), Workload Identity (Azure/GCP) - no static credentials
- **Production-Ready**: Async-first with Tokio, structured logging, Prometheus metrics
- **Kubernetes-Native**: Designed for Kubernetes with health probes, graceful shutdown
- **High Performance**: Zero-copy streaming, efficient async I/O

## Architecture

```
┌─────────────┐
│   Client    │
│ (aws-cli,   │
│  SDK, etc)  │
└──────┬──────┘
       │ S3 HTTP API
       │
┌──────▼──────────────────┐
│   S3Proxy-RS (Rust)      │
│  ┌────────────────────┐ │
│  │  Axum HTTP Server  │ │
│  └──────────┬─────────┘ │
│  ┌──────────▼─────────┐ │
│  │  Storage Backend   │ │
│  │     Abstraction    │ │
│  └──────────┬─────────┘ │
└─────────────┼───────────┘
              │
    ┌─────────┼─────────┐
    │         │         │
┌───▼───┐ ┌──▼───┐ ┌───▼───┐
│  AWS  │ │Azure │ │  GCP  │
│  S3   │ │ Blob │ │  GCS  │
└───────┘ └──────┘ └───────┘
```

## Quick Start

### Build

```bash
cargo build --release
```

### Run Locally

```bash
# AWS S3 backend
export S3PROXY_BACKEND_TYPE=aws
export S3PROXY_BACKEND_CONTAINER=my-bucket
export S3PROXY_BACKEND_REGION=us-east-1

./target/release/s3proxy-rs
```

### Docker

```bash
docker build -t s3proxy-rs:latest .
docker run -p 8080:8080 \
  -e S3PROXY_BACKEND_TYPE=aws \
  -e S3PROXY_BACKEND_CONTAINER=my-bucket \
  -e S3PROXY_BACKEND_REGION=us-east-1 \
  s3proxy-rs:latest
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `S3PROXY_BACKEND_TYPE` | Backend type: `aws`, `azure`, `gcp` | `aws` |
| `S3PROXY_BACKEND_CONTAINER` | Bucket/container name | Required |
| `S3PROXY_BACKEND_PREFIX` | Optional path prefix | None |
| `S3PROXY_BACKEND_REGION` | AWS region (AWS only) | `us-east-1` |
| `S3PROXY_BACKEND_ENDPOINT` | Custom endpoint URL | None |
| `S3PROXY_BIND_ADDRESS` | Server bind address | `0.0.0.0:8080` |
| `S3PROXY_TIMEOUT_SECS` | Request timeout | `300` |
| `S3PROXY_MAX_BODY_SIZE` | Max request size (bytes) | `5368709120` (5GB) |
| `S3PROXY_LOG_LEVEL` | Log level | `info` |
| `S3PROXY_CONFIG_FILE` | Optional TOML config file | None |

### Config File (TOML)

Create `config.toml`:

```toml
[server]
bind_address = "0.0.0.0:8080"
timeout_secs = 300
max_body_size = 5368709120

[backend]
type = "aws"
container_or_bucket = "my-bucket"
prefix = "optional/prefix"
region = "us-east-1"
```

Set `S3PROXY_CONFIG_FILE=/path/to/config.toml` to use it.

## Cloud Provider Setup

### AWS (IRSA)

S3Proxy uses IAM Role for Service Account (IRSA) in EKS. See [deploy/aws-irsa-setup.md](deploy/aws-irsa-setup.md) for detailed setup.

**Quick setup:**

1. Create IAM role with S3 permissions
2. Annotate ServiceAccount:
   ```yaml
   annotations:
     eks.amazonaws.com/role-arn: arn:aws:iam::ACCOUNT:role/s3proxy-role
   ```
3. Deploy with AWS backend configuration

### Azure (Workload Identity)

S3Proxy uses Azure Workload Identity in AKS. See [deploy/azure-workload-identity-setup.md](deploy/azure-workload-identity-setup.md) for detailed setup.

**Quick setup:**

1. Create managed identity
2. Grant "Storage Blob Data Contributor" role
3. Configure federated identity credential
4. Annotate ServiceAccount and Pod:
   ```yaml
   serviceAccount:
     annotations:
       azure.workload.identity/client-id: CLIENT_ID
   pod:
     annotations:
       azure.workload.identity/use: "true"
   ```

### GCP (Workload Identity)

S3Proxy uses GCP Workload Identity in GKE. See [deploy/gcp-workload-identity-setup.md](deploy/gcp-workload-identity-setup.md) for detailed setup.

**Quick setup:**

1. Create GCP service account
2. Grant "Storage Object Admin" role
3. Bind Kubernetes SA to GCP SA
4. Annotate ServiceAccount:
   ```yaml
   annotations:
     iam.gke.io/gcp-service-account: SA@PROJECT.iam.gserviceaccount.com
   ```

## Kubernetes Deployment

### Deploy

```bash
# Edit deploy/k8s.yaml with your configuration
kubectl apply -f deploy/k8s.yaml
```

### Verify

```bash
# Check pods
kubectl get pods -l app=s3proxy

# Check logs
kubectl logs -l app=s3proxy

# Test health endpoint
kubectl port-forward svc/s3proxy 8080:80
curl http://localhost:8080/healthz
```

## API Endpoints

### S3 Operations

- `GET /{bucket}/{key}` - GetObject
- `PUT /{bucket}/{key}` - PutObject
- `DELETE /{bucket}/{key}` - DeleteObject
- `HEAD /{bucket}/{key}` - HeadObject
- `GET /{bucket}?prefix=...` - ListObjectsV2
- `PUT /{bucket}` - CreateBucket (noop)
- `DELETE /{bucket}` - DeleteBucket (noop)

### System Endpoints

- `GET /healthz` - Liveness probe
- `GET /ready` - Readiness probe
- `GET /metrics` - Prometheus metrics

## Testing

### Using AWS CLI

```bash
# Configure endpoint
export AWS_ENDPOINT_URL=http://localhost:8080

# List objects
aws s3 ls s3://my-bucket/ --endpoint-url $AWS_ENDPOINT_URL

# Upload file
aws s3 cp file.txt s3://my-bucket/file.txt --endpoint-url $AWS_ENDPOINT_URL

# Download file
aws s3 cp s3://my-bucket/file.txt file.txt --endpoint-url $AWS_ENDPOINT_URL
```

### Using s3cmd

```bash
# Configure
s3cmd --configure --host=localhost:8080 --host-bucket=localhost:8080

# Use normally
s3cmd ls s3://my-bucket/
s3cmd put file.txt s3://my-bucket/
s3cmd get s3://my-bucket/file.txt
```

### Using curl

```bash
# Put object
curl -X PUT http://localhost:8080/my-bucket/test.txt \
  --data-binary @file.txt

# Get object
curl http://localhost:8080/my-bucket/test.txt

# List objects
curl "http://localhost:8080/my-bucket?prefix=test"
```

## Observability

### Logging

Structured JSON logs via `tracing`:

```json
{"level":"info","message":"GetObject request","bucket":"my-bucket","key":"test.txt","target":"s3proxy::routes::handlers","spans":[{"bucket":"my-bucket","key":"test.txt"}]}
```

Set log level:
```bash
export RUST_LOG=debug
```

### Metrics

Prometheus metrics available at `/metrics`:

- `s3proxy_http_requests_total` - HTTP request count by method/status
- `s3proxy_http_request_duration_seconds` - HTTP request latency
- `s3proxy_storage_operations_total` - Storage operation count
- `s3proxy_storage_operation_duration_seconds` - Storage operation latency

### Request IDs

All requests include a unique request ID in headers for tracing.

## Performance

- **Async I/O**: Fully async using Tokio
- **Zero-Copy**: Streaming uploads/downloads where possible
- **Backpressure**: Proper handling of slow clients
- **Connection Pooling**: Efficient backend connections

## Limitations & TODOs

### Current Limitations

- Signature verification is not implemented (focus on proxying)
- Multipart uploads not yet supported
- Delimiter support in ListObjects not implemented
- Content-type detection based on extension not implemented

### Extensibility Points

The codebase includes TODO markers for:
- Signature verification (optional/pluggable)
- Multipart upload support
- Advanced metadata handling
- Custom authentication middleware
- Request/response transformation

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Run

```bash
cargo run
```

### Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy
```

## Project Structure

```
s3proxy-rs/
├── Cargo.toml          # Dependencies
├── Dockerfile          # Container image
├── src/
│   ├── main.rs         # Entry point
│   ├── config.rs       # Configuration
│   ├── errors.rs       # Error types
│   ├── metrics.rs      # Prometheus metrics
│   ├── routes/         # HTTP handlers
│   ├── s3/             # S3 API types
│   ├── server/         # HTTP server
│   └── storage/        # Storage backends
│       ├── mod.rs
│       ├── aws.rs
│       ├── azure.rs
│       └── gcp.rs
├── deploy/             # Kubernetes manifests
│   ├── k8s.yaml
│   ├── rbac.yaml
│   └── *-setup.md     # Cloud provider setup guides
└── README-RUST.md      # This file
```

## License

Apache 2.0 (same as original S3Proxy)

## Contributing

This is a production rewrite. Contributions should maintain:
- Production-grade code quality
- Comprehensive error handling
- Extensive documentation
- Cloud-native best practices

## Differences from Java S3Proxy

- **Language**: Rust instead of Java
- **Runtime**: Tokio async instead of Jetty
- **Storage**: object_store crate instead of jclouds
- **Identity**: Native managed identity support (no credential injection)
- **Performance**: Zero-copy streaming, efficient async I/O
- **Deployment**: Kubernetes-first design

## Support

For issues and questions, please open an issue in the repository.

