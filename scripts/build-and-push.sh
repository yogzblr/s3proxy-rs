#!/bin/bash
set -euo pipefail

IMAGE_NAME="yogzblr/s3proxy-rs"
LATEST_TAG="${IMAGE_NAME}:latest"
DEBUG_TAG="${IMAGE_NAME}:debug"

echo "=========================================="
echo "Building S3Proxy Docker Images"
echo "=========================================="

echo ""
echo "Building distroless image: ${LATEST_TAG}"
docker build -t "${LATEST_TAG}" -f Dockerfile .

echo ""
echo "Building debug image: ${DEBUG_TAG}"
docker build -t "${DEBUG_TAG}" -f Dockerfile.debug .

echo ""
echo "=========================================="
echo "Pushing Docker Images to Docker Hub"
echo "=========================================="

echo ""
echo "Pushing distroless image: ${LATEST_TAG}"
docker push "${LATEST_TAG}"

echo ""
echo "Pushing debug image: ${DEBUG_TAG}"
docker push "${DEBUG_TAG}"

echo ""
echo "=========================================="
echo "Docker images built and pushed successfully!"
echo "=========================================="
echo ""
echo "Images:"
echo "  - ${LATEST_TAG}"
echo "  - ${DEBUG_TAG}"
echo ""
