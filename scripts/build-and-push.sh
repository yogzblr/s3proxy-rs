#!/bin/bash
# Build and push Docker images for S3Proxy-RS

set -e

DOCKER_USER="${DOCKER_USER:-yogzblr}"
IMAGE_NAME="s3proxy-rs"
VERSION="${VERSION:-latest}"

echo "Building distroless image..."
docker build -t ${DOCKER_USER}/${IMAGE_NAME}:${VERSION} -f Dockerfile .

echo "Building debug image..."
docker build -t ${DOCKER_USER}/${IMAGE_NAME}:debug -f Dockerfile.debug .

echo "Tagging images..."
docker tag ${DOCKER_USER}/${IMAGE_NAME}:${VERSION} ${DOCKER_USER}/${IMAGE_NAME}:latest

echo "Images built successfully!"
echo ""
echo "To push to Docker Hub, run:"
echo "  docker login"
echo "  docker push ${DOCKER_USER}/${IMAGE_NAME}:latest"
echo "  docker push ${DOCKER_USER}/${IMAGE_NAME}:debug"
echo ""
echo "Or run this script with PUSH=true:"
echo "  PUSH=true ./scripts/build-and-push.sh"

if [ "${PUSH}" = "true" ]; then
    echo "Pushing images to Docker Hub..."
    docker push ${DOCKER_USER}/${IMAGE_NAME}:latest
    docker push ${DOCKER_USER}/${IMAGE_NAME}:debug
    echo "Images pushed successfully!"
fi

