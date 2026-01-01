# Quick Start Guide

Get S3Proxy-RS running in 5 minutes.

## Prerequisites

- Rust 1.75+ (or Docker)
- Access to a cloud object store (AWS S3, Azure Blob, or GCS)
- Kubernetes cluster (optional, for production)

## Local Development

### 1. Clone and Build

```bash
cd s3proxy
cargo build --release
```

### 2. Configure

Set environment variables for your backend:

**AWS S3:**
```bash
export S3PROXY_BACKEND_TYPE=aws
export S3PROXY_BACKEND_CONTAINER=my-bucket
export S3PROXY_BACKEND_REGION=us-east-1
```

**Azure Blob:**
```bash
export S3PROXY_BACKEND_TYPE=azure
export S3PROXY_BACKEND_CONTAINER=my-container
```

**GCP:**
```bash
export S3PROXY_BACKEND_TYPE=gcp
export S3PROXY_BACKEND_CONTAINER=my-bucket
```

### 3. Run

```bash
./target/release/s3proxy-rs
```

Server starts on `http://localhost:8080`

### 4. Test

```bash
# Health check
curl http://localhost:8080/healthz

# Put a file
echo "Hello, S3Proxy!" > test.txt
curl -X PUT http://localhost:8080/my-bucket/test.txt --data-binary @test.txt

# Get the file
curl http://localhost:8080/my-bucket/test.txt

# List objects
curl "http://localhost:8080/my-bucket?prefix="
```

## Docker

### Build Image

```bash
docker build -t s3proxy-rs:latest .
```

### Run Container

```bash
docker run -p 8080:8080 \
  -e S3PROXY_BACKEND_TYPE=aws \
  -e S3PROXY_BACKEND_CONTAINER=my-bucket \
  -e S3PROXY_BACKEND_REGION=us-east-1 \
  -e AWS_ACCESS_KEY_ID=your-key \
  -e AWS_SECRET_ACCESS_KEY=your-secret \
  s3proxy-rs:latest
```

## Kubernetes

### 1. Configure Workload Identity

See setup guides:
- [AWS IRSA](deploy/aws-irsa-setup.md)
- [Azure Workload Identity](deploy/azure-workload-identity-setup.md)
- [GCP Workload Identity](deploy/gcp-workload-identity-setup.md)

### 2. Edit Configuration

Edit `deploy/k8s.yaml`:

```yaml
env:
- name: S3PROXY_BACKEND_TYPE
  value: "aws"  # or azure, gcp
- name: S3PROXY_BACKEND_CONTAINER
  value: "my-bucket"
```

### 3. Deploy

```bash
kubectl apply -f deploy/k8s.yaml
```

### 4. Verify

```bash
kubectl get pods -l app=s3proxy
kubectl logs -l app=s3proxy
kubectl port-forward svc/s3proxy 8080:80
curl http://localhost:8080/healthz
```

## Using with AWS CLI

```bash
# Set endpoint
export AWS_ENDPOINT_URL=http://localhost:8080

# Use normally
aws s3 ls s3://my-bucket/ --endpoint-url $AWS_ENDPOINT_URL
aws s3 cp file.txt s3://my-bucket/ --endpoint-url $AWS_ENDPOINT_URL
```

## Troubleshooting

### Authentication Errors

- **AWS**: Check IRSA role annotation, IAM permissions
- **Azure**: Verify managed identity, federated credential
- **GCP**: Confirm workload identity binding, service account permissions

### Connection Errors

- Verify backend container/bucket exists
- Check network connectivity
- Review logs: `kubectl logs -l app=s3proxy`

### Performance Issues

- Adjust `S3PROXY_MAX_BODY_SIZE` for large files
- Increase `S3PROXY_TIMEOUT_SECS` for slow operations
- Scale replicas: `kubectl scale deployment s3proxy --replicas=3`

## Next Steps

- Read [README-RUST.md](README-RUST.md) for full documentation
- Review [ARCHITECTURE.md](ARCHITECTURE.md) for design details
- Check [examples/](examples/) for configuration samples

