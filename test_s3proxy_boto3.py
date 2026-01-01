#!/usr/bin/env python3
"""
Test script for S3Proxy using boto3.
Tests S3Proxy with MinIO backend using boto3 S3 client.

Usage:
    python3 test_s3proxy_boto3.py [--endpoint http://localhost:8080] [--bucket test-bucket]

Requirements:
    pip install boto3
"""

import argparse
import sys
import time
from typing import Optional

import boto3
from botocore.exceptions import ClientError, BotoCoreError


class S3ProxyTester:
    """Test S3Proxy functionality using boto3."""

    def __init__(
        self,
        endpoint: str = "http://localhost:8080",
        bucket: str = "test-bucket",
        access_key: str = "minioadmin",
        secret_key: str = "minioadmin",
        region: str = "us-east-1",
    ):
        """Initialize S3Proxy tester.

        Args:
            endpoint: S3Proxy endpoint URL
            bucket: Bucket name to use for testing
            access_key: AWS access key (MinIO default: minioadmin)
            secret_key: AWS secret key (MinIO default: minioadmin)
            region: AWS region (required by boto3, but not used by S3Proxy)
        """
        self.endpoint = endpoint
        self.bucket = bucket
        self.s3_client = boto3.client(
            "s3",
            endpoint_url=endpoint,
            aws_access_key_id=access_key,
            aws_secret_access_key=secret_key,
            region_name=region,
            use_ssl=False if endpoint.startswith("http://") else True,
            verify=False,  # Skip SSL verification for local testing
        )
        self.test_objects = []

    def wait_for_service(self, max_retries: int = 30, delay: int = 2) -> bool:
        """Wait for S3Proxy service to be ready.

        Args:
            max_retries: Maximum number of retry attempts
            delay: Delay between retries in seconds

        Returns:
            True if service is ready, False otherwise
        """
        print(f"Waiting for S3Proxy at {self.endpoint}...")
        for i in range(max_retries):
            try:
                # Try to list buckets (this will fail if service is not ready)
                self.s3_client.list_buckets()
                print("✓ S3Proxy is ready!")
                return True
            except (ClientError, BotoCoreError) as e:
                if i < max_retries - 1:
                    print(f"  Attempt {i+1}/{max_retries}: Service not ready, retrying in {delay}s...")
                    time.sleep(delay)
                else:
                    print(f"✗ Service not ready after {max_retries} attempts: {e}")
                    return False
        return False

    def test_create_bucket(self) -> bool:
        """Test bucket creation."""
        print("\n[TEST] Create Bucket")
        try:
            self.s3_client.create_bucket(Bucket=self.bucket)
            print(f"✓ Bucket '{self.bucket}' created successfully")
            return True
        except ClientError as e:
            error_code = e.response.get("Error", {}).get("Code", "Unknown")
            if error_code == "BucketAlreadyOwnedByYou":
                print(f"✓ Bucket '{self.bucket}' already exists (expected)")
                return True
            print(f"✗ Failed to create bucket: {e}")
            return False

    def test_put_object(self, key: str, content: bytes) -> bool:
        """Test object upload (PUT).

        Args:
            key: Object key
            content: Object content

        Returns:
            True if successful, False otherwise
        """
        print(f"\n[TEST] PUT Object: s3://{self.bucket}/{key}")
        try:
            self.s3_client.put_object(
                Bucket=self.bucket,
                Key=key,
                Body=content,
                Metadata={"test-meta": "test-value"},
            )
            print(f"✓ Object uploaded successfully (size: {len(content)} bytes)")
            self.test_objects.append(key)
            return True
        except ClientError as e:
            print(f"✗ Failed to upload object: {e}")
            return False

    def test_get_object(self, key: str, expected_content: bytes) -> bool:
        """Test object download (GET).

        Args:
            key: Object key
            expected_content: Expected object content

        Returns:
            True if successful and content matches, False otherwise
        """
        print(f"\n[TEST] GET Object: s3://{self.bucket}/{key}")
        try:
            response = self.s3_client.get_object(Bucket=self.bucket, Key=key)
            content = response["Body"].read()
            if content == expected_content:
                print(f"✓ Object downloaded successfully (size: {len(content)} bytes)")
                print(f"  Content matches: {content[:50].decode('utf-8', errors='ignore')}...")
                return True
            else:
                print(f"✗ Content mismatch!")
                print(f"  Expected: {expected_content[:50]}")
                print(f"  Got: {content[:50]}")
                return False
        except ClientError as e:
            print(f"✗ Failed to download object: {e}")
            return False

    def test_head_object(self, key: str) -> bool:
        """Test object metadata retrieval (HEAD).

        Args:
            key: Object key

        Returns:
            True if successful, False otherwise
        """
        print(f"\n[TEST] HEAD Object: s3://{self.bucket}/{key}")
        try:
            response = self.s3_client.head_object(Bucket=self.bucket, Key=key)
            size = response.get("ContentLength", 0)
            etag = response.get("ETag", "").strip('"')
            print(f"✓ Object metadata retrieved successfully")
            print(f"  Size: {size} bytes")
            print(f"  ETag: {etag}")
            return True
        except ClientError as e:
            print(f"✗ Failed to retrieve object metadata: {e}")
            return False

    def test_list_objects(self, prefix: Optional[str] = None) -> bool:
        """Test object listing (LIST).

        Args:
            prefix: Optional prefix filter

        Returns:
            True if successful, False otherwise
        """
        print(f"\n[TEST] LIST Objects: s3://{self.bucket}/" + (f"{prefix}" if prefix else ""))
        try:
            kwargs = {"Bucket": self.bucket}
            if prefix:
                kwargs["Prefix"] = prefix

            response = self.s3_client.list_objects_v2(**kwargs)
            objects = response.get("Contents", [])
            print(f"✓ Listed {len(objects)} object(s)")
            for obj in objects:
                print(f"  - {obj['Key']} ({obj['Size']} bytes, modified: {obj['LastModified']})")
            return True
        except ClientError as e:
            print(f"✗ Failed to list objects: {e}")
            return False

    def test_delete_object(self, key: str) -> bool:
        """Test object deletion (DELETE).

        Args:
            key: Object key

        Returns:
            True if successful, False otherwise
        """
        print(f"\n[TEST] DELETE Object: s3://{self.bucket}/{key}")
        try:
            self.s3_client.delete_object(Bucket=self.bucket, Key=key)
            print(f"✓ Object deleted successfully")
            if key in self.test_objects:
                self.test_objects.remove(key)
            return True
        except ClientError as e:
            print(f"✗ Failed to delete object: {e}")
            return False

    def test_delete_bucket(self) -> bool:
        """Test bucket deletion."""
        print(f"\n[TEST] Delete Bucket: {self.bucket}")
        try:
            # First, delete all objects in the bucket
            response = self.s3_client.list_objects_v2(Bucket=self.bucket)
            if "Contents" in response:
                for obj in response["Contents"]:
                    self.s3_client.delete_object(Bucket=self.bucket, Key=obj["Key"])
                    print(f"  Deleted object: {obj['Key']}")

            self.s3_client.delete_bucket(Bucket=self.bucket)
            print(f"✓ Bucket '{self.bucket}' deleted successfully")
            return True
        except ClientError as e:
            error_code = e.response.get("Error", {}).get("Code", "Unknown")
            if error_code == "NoSuchBucket":
                print(f"✓ Bucket '{self.bucket}' does not exist (expected)")
                return True
            print(f"✗ Failed to delete bucket: {e}")
            return False

    def cleanup(self):
        """Clean up test objects."""
        print("\n[CLEANUP] Removing test objects...")
        for key in self.test_objects[:]:
            try:
                self.s3_client.delete_object(Bucket=self.bucket, Key=key)
                print(f"  Deleted: {key}")
            except ClientError:
                pass

    def run_all_tests(self, cleanup: bool = True) -> bool:
        """Run all S3Proxy tests.

        Args:
            cleanup: Whether to clean up test objects after tests

        Returns:
            True if all tests passed, False otherwise
        """
        print("=" * 60)
        print("S3Proxy Boto3 Test Suite")
        print("=" * 60)
        print(f"Endpoint: {self.endpoint}")
        print(f"Bucket: {self.bucket}")

        if not self.wait_for_service():
            return False

        tests = []
        test_data = {
            "test1.txt": b"Hello, S3Proxy! This is test object 1.",
            "test2.txt": b"Hello, S3Proxy! This is test object 2 with some content.",
            "folder/test3.txt": b"Hello, S3Proxy! This is a nested object.",
        }

        # Test 1: Create bucket
        tests.append(("Create Bucket", self.test_create_bucket()))

        # Test 2: PUT objects
        for key, content in test_data.items():
            tests.append((f"PUT {key}", self.test_put_object(key, content)))

        # Test 3: GET objects
        for key, content in test_data.items():
            tests.append((f"GET {key}", self.test_get_object(key, content)))

        # Test 4: HEAD objects
        for key in test_data.keys():
            tests.append((f"HEAD {key}", self.test_head_object(key)))

        # Test 5: LIST objects
        tests.append(("LIST all objects", self.test_list_objects()))
        tests.append(("LIST with prefix", self.test_list_objects(prefix="folder/")))

        # Test 6: DELETE objects
        for key in test_data.keys():
            tests.append((f"DELETE {key}", self.test_delete_object(key)))

        # Test 7: Delete bucket (optional)
        # tests.append(("Delete Bucket", self.test_delete_bucket()))

        if cleanup:
            self.cleanup()

        # Print summary
        print("\n" + "=" * 60)
        print("Test Summary")
        print("=" * 60)
        passed = sum(1 for _, result in tests if result)
        total = len(tests)
        for test_name, result in tests:
            status = "✓ PASS" if result else "✗ FAIL"
            print(f"{status}: {test_name}")

        print(f"\nTotal: {passed}/{total} tests passed")
        if passed == total:
            print("🎉 All tests passed!")
            return True
        else:
            print("❌ Some tests failed")
            return False


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Test S3Proxy with boto3")
    parser.add_argument(
        "--endpoint",
        default="http://localhost:8080",
        help="S3Proxy endpoint URL (default: http://localhost:8080)",
    )
    parser.add_argument(
        "--bucket",
        default="test-bucket",
        help="Bucket name for testing (default: test-bucket)",
    )
    parser.add_argument(
        "--access-key",
        default="minioadmin",
        help="AWS access key (default: minioadmin)",
    )
    parser.add_argument(
        "--secret-key",
        default="minioadmin",
        help="AWS secret key (default: minioadmin)",
    )
    parser.add_argument(
        "--region",
        default="us-east-1",
        help="AWS region (default: us-east-1)",
    )
    parser.add_argument(
        "--no-cleanup",
        action="store_true",
        help="Don't clean up test objects after tests",
    )

    args = parser.parse_args()

    tester = S3ProxyTester(
        endpoint=args.endpoint,
        bucket=args.bucket,
        access_key=args.access_key,
        secret_key=args.secret_key,
        region=args.region,
    )

    success = tester.run_all_tests(cleanup=not args.no_cleanup)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()

