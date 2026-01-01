# Multi-stage build for S3Proxy Rust implementation (Distroless)
# Uses Alpine Rust base image for smaller final image

# Build stage
FROM rust:1.92-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy src to build dependencies (for caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build release binary with static linking
RUN touch src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl

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
