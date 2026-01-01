#!/bin/bash
# Test S3Proxy with MinIO

set -e

PROXY_ENDPOINT="${PROXY_ENDPOINT:-http://localhost:8080}"
BUCKET="${BUCKET:-test-bucket}"

echo "Testing S3Proxy at ${PROXY_ENDPOINT}"

# Test health endpoint
echo "1. Testing health endpoint..."
curl -f ${PROXY_ENDPOINT}/healthz && echo " ✓ Health check passed" || echo " ✗ Health check failed"

# Test readiness endpoint
echo "2. Testing readiness endpoint..."
curl -f ${PROXY_ENDPOINT}/ready && echo " ✓ Readiness check passed" || echo " ✗ Readiness check failed"

# Test metrics endpoint
echo "3. Testing metrics endpoint..."
curl -f ${PROXY_ENDPOINT}/metrics > /dev/null && echo " ✓ Metrics endpoint accessible" || echo " ✗ Metrics endpoint failed"

# Test PUT object
echo "4. Testing PUT object..."
echo "Hello, S3Proxy!" > /tmp/test-file.txt
curl -X PUT ${PROXY_ENDPOINT}/${BUCKET}/test.txt \
    --data-binary @/tmp/test-file.txt && echo " ✓ PUT object succeeded" || echo " ✗ PUT object failed"

# Test GET object
echo "5. Testing GET object..."
curl -f ${PROXY_ENDPOINT}/${BUCKET}/test.txt > /tmp/retrieved-file.txt && \
    diff /tmp/test-file.txt /tmp/retrieved-file.txt && echo " ✓ GET object succeeded" || echo " ✗ GET object failed"

# Test LIST objects
echo "6. Testing LIST objects..."
curl -f "${PROXY_ENDPOINT}/${BUCKET}?prefix=" > /dev/null && echo " ✓ LIST objects succeeded" || echo " ✗ LIST objects failed"

# Test HEAD object
echo "7. Testing HEAD object..."
curl -f -I ${PROXY_ENDPOINT}/${BUCKET}/test.txt > /dev/null && echo " ✓ HEAD object succeeded" || echo " ✗ HEAD object failed"

# Test DELETE object
echo "8. Testing DELETE object..."
curl -X DELETE ${PROXY_ENDPOINT}/${BUCKET}/test.txt && echo " ✓ DELETE object succeeded" || echo " ✗ DELETE object failed"

# Cleanup
rm -f /tmp/test-file.txt /tmp/retrieved-file.txt

echo ""
echo "All tests completed!"

