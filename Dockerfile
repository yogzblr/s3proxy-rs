# Multi-stage build for S3Proxy Rust implementation (Distroless)
# Uses Alpine Rust base image for smaller final image

# Build stage
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary with static linking
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - Distroless
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/s3proxy-rs /app/s3proxy

# Use non-root user (distroless images run as nonroot by default)
USER nonroot:nonroot

EXPOSE 8080

# Health check - distroless doesn't have shell/curl, so we check process
# Kubernetes should use HTTP probe to /healthz endpoint instead
# HEALTHCHECK removed - use Kubernetes liveness/readiness probes with HTTP GET /healthz

ENTRYPOINT ["/app/s3proxy"]
