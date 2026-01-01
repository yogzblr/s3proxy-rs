# Multi-stage build for S3Proxy Rust implementation
FROM rust:1.75 as builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for TLS
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/s3proxy-rs /app/s3proxy

# Create non-root user
RUN useradd -r -s /bin/false s3proxy && \
    chown -R s3proxy:s3proxy /app

USER s3proxy

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD ["/bin/sh", "-c", "wget --quiet --tries=1 --spider http://localhost:8080/healthz || exit 1"]

ENTRYPOINT ["/app/s3proxy"]
