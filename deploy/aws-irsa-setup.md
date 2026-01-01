# AWS IRSA (IAM Role for Service Account) Setup

This guide explains how to configure S3Proxy to use AWS IAM Role for Service Account (IRSA) in EKS.

## Prerequisites

- EKS cluster with OIDC provider configured
- AWS CLI configured
- kubectl configured

## Steps

### 1. Create IAM Role

Create an IAM role with permissions to access your S3 bucket:

```bash
# Set variables
CLUSTER_NAME=your-eks-cluster
NAMESPACE=default
SERVICE_ACCOUNT=s3proxy
ROLE_NAME=s3proxy-role
BUCKET_NAME=your-bucket-name

# Get OIDC issuer URL
OIDC_ISSUER=$(aws eks describe-cluster --name $CLUSTER_NAME --query "cluster.identity.oidc.issuer" --output text | sed 's|https://||')

# Create trust policy
cat > trust-policy.json <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::ACCOUNT_ID:oidc-provider/${OIDC_ISSUER}"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "${OIDC_ISSUER}:sub": "system:serviceaccount:${NAMESPACE}:${SERVICE_ACCOUNT}",
          "${OIDC_ISSUER}:aud": "sts.amazonaws.com"
        }
      }
    }
  ]
}
EOF

# Create IAM role
aws iam create-role \
  --role-name $ROLE_NAME \
  --assume-role-policy-document file://trust-policy.json

# Attach S3 access policy
aws iam attach-role-policy \
  --role-name $ROLE_NAME \
  --policy-arn arn:aws:iam::aws:policy/AmazonS3ReadOnlyAccess  # Or create custom policy
```

### 2. Annotate ServiceAccount

Update the ServiceAccount in `k8s.yaml`:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: s3proxy
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::ACCOUNT_ID:role/s3proxy-role
```

### 3. Deploy

```bash
kubectl apply -f deploy/k8s.yaml
```

## Verification

Check that the pod can assume the role:

```bash
kubectl exec -it deployment/s3proxy -- env | grep AWS
```

You should see AWS environment variables set by the IRSA webhook.

