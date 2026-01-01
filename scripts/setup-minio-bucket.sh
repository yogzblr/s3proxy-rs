#!/bin/bash
# Setup MinIO bucket for testing
# This script should be run after MinIO is up and running

set -e

MC_IMAGE="minio/mc:latest"
MINIO_ENDPOINT="${MINIO_ENDPOINT:-http://minio:9000}"
MINIO_USER="${MINIO_USER:-minioadmin}"
MINIO_PASSWORD="${MINIO_PASSWORD:-minioadmin}"
BUCKET_NAME="${BUCKET_NAME:-test-bucket}"

echo "Setting up MinIO bucket: ${BUCKET_NAME}"

# Check if running in docker-compose
if [ -f /.dockerenv ] || [ -n "${DOCKER_COMPOSE}" ]; then
    # We're in a container, use mc directly if available
    if command -v mc &> /dev/null; then
        mc alias set local ${MINIO_ENDPOINT} ${MINIO_USER} ${MINIO_PASSWORD}
        mc mb local/${BUCKET_NAME} || echo "Bucket may already exist"
        mc anonymous set download local/${BUCKET_NAME}
        echo "Bucket ${BUCKET_NAME} created and configured"
    else
        echo "mc not available in container. Bucket will be created on first access."
    fi
else
    # We're on the host, use docker run
    echo "Creating bucket using MinIO client..."
    docker run --rm --network s3proxy-rust_s3proxy-network \
        ${MC_IMAGE} alias set local ${MINIO_ENDPOINT} ${MINIO_USER} ${MINIO_PASSWORD} && \
    docker run --rm --network s3proxy-rust_s3proxy-network \
        ${MC_IMAGE} mb local/${BUCKET_NAME} || echo "Bucket may already exist" && \
    docker run --rm --network s3proxy-rust_s3proxy-network \
        ${MC_IMAGE} anonymous set download local/${BUCKET_NAME} && \
    echo "Bucket ${BUCKET_NAME} created and configured"
fi

