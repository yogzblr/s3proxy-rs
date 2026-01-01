# GCP Workload Identity Setup

This guide explains how to configure S3Proxy to use GCP Workload Identity in GKE.

## Prerequisites

- GKE cluster with Workload Identity enabled
- gcloud CLI configured
- kubectl configured

## Steps

### 1. Enable Workload Identity (if not already enabled)

```bash
gcloud container clusters update CLUSTER_NAME \
  --workload-pool=PROJECT_ID.svc.id.goog \
  --region=REGION
```

### 2. Create GCP Service Account

```bash
# Set variables
PROJECT_ID=your-project-id
SERVICE_ACCOUNT_NAME=s3proxy-gsa
K8S_NAMESPACE=default
K8S_SERVICE_ACCOUNT=s3proxy

# Create service account
gcloud iam service-accounts create $SERVICE_ACCOUNT_NAME \
  --display-name="S3Proxy Service Account" \
  --project=$PROJECT_ID

# Grant Storage Object Admin role
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/storage.objectAdmin"
```

### 3. Allow Kubernetes Service Account to Impersonate GCP Service Account

```bash
gcloud iam service-accounts add-iam-policy-binding \
  ${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com \
  --role roles/iam.workloadIdentityUser \
  --member "serviceAccount:${PROJECT_ID}.svc.id.goog[${K8S_NAMESPACE}/${K8S_SERVICE_ACCOUNT}]"
```

### 4. Annotate Kubernetes ServiceAccount

Update the ServiceAccount in `k8s.yaml`:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: s3proxy
  annotations:
    iam.gke.io/gcp-service-account: ${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com
```

### 5. Deploy

```bash
kubectl apply -f deploy/k8s.yaml
```

## Verification

Check that the pod can access GCS:

```bash
kubectl exec -it deployment/s3proxy -- env | grep GOOGLE
```

You should see Google Cloud credentials configured.

