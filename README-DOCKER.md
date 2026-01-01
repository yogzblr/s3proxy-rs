# Docker Build and Deployment Guide

## Quick Start with Docker Compose

### 1. Build and Start Services

```bash
# Start MinIO and S3Proxy
docker-compose up -d

# View logs
docker-compose logs -f s3proxy

# Check status
docker-compose ps
```

### 2. Access Services

- **S3Proxy**: http://localhost:8080
- **MinIO Console**: http://localhost:9001 (minioadmin/minioadmin)
- **MinIO API**: http://localhost:9000

### 3. Create MinIO Bucket

```bash
# Using MinIO client
docker run --rm --network s3proxy-rust_s3proxy-network \
  minio/mc:latest alias set local http://minio:9000 minioadmin minioadmin

docker run --rm --network s3proxy-rust_s3proxy-network \
  minio/mc:latest mb local/test-bucket

# Or use the setup script
./scripts/setup-minio-bucket.sh
```

### 4. Test S3Proxy

```bash
# Run test script
./scripts/test-minio.sh

# Or test manually
curl http://localhost:8080/healthz
curl -X PUT http://localhost:8080/test-bucket/test.txt --data-binary "Hello, World!"
curl http://localhost:8080/test-bucket/test.txt
```

## Building Docker Images

### Build Locally

```bash
# Build distroless image
docker build -t yogzblr/s3proxy-rs:latest -f Dockerfile .

# Build debug image
docker build -t yogzblr/s3proxy-rs:debug -f Dockerfile.debug .

# Or use the build script
./scripts/build-and-push.sh
```

### Push to Docker Hub

```bash
# Login to Docker Hub
docker login

# Tag and push
docker push yogzblr/s3proxy-rs:latest
docker push yogzblr/s3proxy-rs:debug

# Or use the script with PUSH=true
PUSH=true ./scripts/build-and-push.sh
```

## Docker Compose Configuration

The `docker-compose.yaml` includes:

1. **MinIO**: S3-compatible object storage for testing
   - API: port 9000
   - Console: port 9001
   - Default credentials: minioadmin/minioadmin

2. **S3Proxy**: Production distroless image
   - Port: 8080
   - Configured to use MinIO as backend

3. **S3Proxy Debug**: Debug image with debugging tools
   - Port: 8081
   - Only starts with `--profile debug`

### Environment Variables

S3Proxy is configured via environment variables in `docker-compose.yaml`:

```yaml
S3PROXY_BACKEND_TYPE: aws
S3PROXY_AWS_BUCKET: test-bucket
S3PROXY_AWS_ENDPOINT: http://minio:9000
S3PROXY_AWS_ACCESS_KEY_ID: minioadmin
S3PROXY_AWS_SECRET_ACCESS_KEY: minioadmin
S3PROXY_AWS_ALLOW_HTTP: "true"
```

### Using TOML Config File

To use a TOML config file instead:

1. Copy `docker-compose.override.yaml.example` to `docker-compose.override.yaml`
2. Mount your config file as a volume
3. Set `S3PROXY_CONFIG_FILE` environment variable

## Image Details

### Distroless Image (`yogzblr/s3proxy-rs:latest`)

- **Base**: `gcr.io/distroless/cc-debian12:nonroot`
- **Size**: ~20-30MB (very small)
- **Security**: Minimal attack surface, no shell
- **Use case**: Production deployments

### Debug Image (`yogzblr/s3proxy-rs:debug`)

- **Base**: `rust:1.92-alpine`
- **Size**: ~500MB+ (includes debugging tools)
- **Tools**: gdb, strace, curl, wget
- **Use case**: Development and debugging

## Troubleshooting

### Build Issues

If build fails due to Cargo.lock version:
```bash
# Regenerate Cargo.lock with compatible version
cargo update
```

### MinIO Connection Issues

Check MinIO is healthy:
```bash
docker-compose ps minio
docker-compose logs minio
```

### S3Proxy Connection Issues

Check S3Proxy logs:
```bash
docker-compose logs s3proxy
```

Verify MinIO bucket exists:
```bash
# Access MinIO console at http://localhost:9001
# Or use mc client
docker run --rm --network s3proxy-rust_s3proxy-network \
  minio/mc:latest ls local/
```

## Production Deployment

For production, use the distroless image:

```bash
docker pull yogzblr/s3proxy-rs:latest
docker run -d \
  -p 8080:8080 \
  -e S3PROXY_BACKEND_TYPE=aws \
  -e S3PROXY_AWS_BUCKET=your-bucket \
  -e S3PROXY_AWS_REGION=us-east-1 \
  -e S3PROXY_AWS_USE_MANAGED_IDENTITY=true \
  yogzblr/s3proxy-rs:latest
```

