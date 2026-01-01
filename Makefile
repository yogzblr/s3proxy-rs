.PHONY: build test run clean docker-build docker-run fmt clippy

# Build release binary
build:
	cargo build --release

# Run tests
test:
	cargo test

# Run in development mode
run:
	cargo run

# Clean build artifacts
clean:
	cargo clean

# Build Docker image
docker-build:
	docker build -t s3proxy-rs:latest .

# Run Docker container
docker-run:
	docker run -p 8080:8080 \
		-e S3PROXY_BACKEND_TYPE=aws \
		-e S3PROXY_BACKEND_CONTAINER=my-bucket \
		-e S3PROXY_BACKEND_REGION=us-east-1 \
		s3proxy-rs:latest

# Format code
fmt:
	cargo fmt

# Lint code
clippy:
	cargo clippy -- -D warnings

# Check compilation
check:
	cargo check

# Build and test
all: fmt clippy test build

