# Multi-stage build for S3Proxy Rust implementation (Distroless)
# Uses Alpine Rust base image for smaller final image

# Build stage
FROM rust:1.92-alpine AS builder

# Install build dependencies and CA certificates for SSL
RUN echo "==> Installing build dependencies and CA certificates..." && \
    apk add --no-cache musl-dev gcc openssl-dev pkgconfig ca-certificates curl && \
    update-ca-certificates && \
    echo "==> Build dependencies installed"

WORKDIR /build

# Copy Zscaler CA certificate if available
COPY zscaler-ca.crt* ./

# Install Zscaler CA certificate if provided
# Alpine uses /usr/local/share/ca-certificates/ for custom certificates
RUN echo "==> Setting up SSL certificates..." && \
    if [ -f zscaler-ca.crt ]; then \
        echo "==> Installing Zscaler CA certificate..." && \
        mkdir -p /usr/local/share/ca-certificates && \
        cp zscaler-ca.crt /usr/local/share/ca-certificates/zscaler-ca.crt && \
        update-ca-certificates && \
        cat zscaler-ca.crt >> /etc/ssl/certs/ca-certificates.crt && \
        echo "==> Zscaler CA certificate installed"; \
    else \
        echo "==> No Zscaler certificate found, using default certificates"; \
    fi

# Copy Cargo config if available
COPY .cargo ./.cargo

# Set SSL certificate environment variables for Cargo (must be before any cargo commands)
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV CARGO_HTTP_CHECK_REVOKE=false
ENV CURL_CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt
ENV REQUESTS_CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt
ENV CURL_CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt
# Configure git to use system certificates
RUN echo "==> Configuring Git and Cargo SSL settings..." && \
    git config --global http.sslCAInfo /etc/ssl/certs/ca-certificates.crt || true && \
    echo "==> SSL configuration complete"

# Alpine already uses musl libc, so we use the default target
# No need for musl target or special linker configuration

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy src to build dependencies (for caching)
RUN echo "==> Building dependencies (this may take a while)..." && \
    mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src && \
    echo "==> Dependencies built successfully" || true

# Copy actual source code
COPY src ./src

# Build release binary (Alpine already uses musl, so we use default target)
RUN echo "==> Building S3Proxy release binary..." && \
    touch src/main.rs && \
    cargo build --release && \
    echo "==> Binary built successfully"

# Runtime stage - Distroless
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# Copy binary from builder
# Note: Distroless images don't have shell, so we can't add echo statements here
COPY --from=builder /build/target/release/s3proxy-rs /app/s3proxy

# Use non-root user (distroless images run as nonroot by default)
USER nonroot:nonroot

EXPOSE 8080

# Health check - distroless doesn't have shell/curl, so we check process
# Kubernetes should use HTTP probe to /healthz endpoint instead
# HEALTHCHECK removed - use Kubernetes liveness/readiness probes with HTTP GET /healthz

ENTRYPOINT ["/app/s3proxy"]
