# Azure Workload Identity Setup

This guide explains how to configure S3Proxy to use Azure Workload Identity in AKS.

## Prerequisites

- AKS cluster with OIDC issuer enabled
- Azure CLI configured
- kubectl configured

## Steps

### 1. Enable OIDC Issuer (if not already enabled)

```bash
az aks update \
  --resource-group RESOURCE_GROUP \
  --name CLUSTER_NAME \
  --enable-oidc-issuer \
  --enable-workload-identity
```

### 2. Create User-Assigned Managed Identity

```bash
# Create managed identity
az identity create \
  --resource-group RESOURCE_GROUP \
  --name s3proxy-identity

# Get identity details
IDENTITY_CLIENT_ID=$(az identity show \
  --resource-group RESOURCE_GROUP \
  --name s3proxy-identity \
  --query clientId -o tsv)

IDENTITY_RESOURCE_ID=$(az identity show \
  --resource-group RESOURCE_GROUP \
  --name s3proxy-identity \
  --query id -o tsv)
```

### 3. Grant Storage Blob Data Contributor Role

```bash
# Get storage account resource ID
STORAGE_ACCOUNT_ID=$(az storage account show \
  --resource-group RESOURCE_GROUP \
  --name STORAGE_ACCOUNT_NAME \
  --query id -o tsv)

# Assign role
az role assignment create \
  --assignee $IDENTITY_CLIENT_ID \
  --role "Storage Blob Data Contributor" \
  --scope $STORAGE_ACCOUNT_ID
```

### 4. Configure Federated Identity Credential

```bash
# Get OIDC issuer URL
OIDC_ISSUER=$(az aks show \
  --resource-group RESOURCE_GROUP \
  --name CLUSTER_NAME \
  --query "oidcIssuerProfile.issuerUrl" -o tsv)

# Create federated identity credential
az identity federated-credential create \
  --name s3proxy-federated-credential \
  --identity-name s3proxy-identity \
  --resource-group RESOURCE_GROUP \
  --issuer $OIDC_ISSUER \
  --subject system:serviceaccount:default:s3proxy \
  --audience api://AzureADTokenExchange
```

### 5. Annotate ServiceAccount

Update the ServiceAccount in `k8s.yaml`:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: s3proxy
  annotations:
    azure.workload.identity/client-id: $IDENTITY_CLIENT_ID
```

### 6. Annotate Pod Template

Update the Deployment in `k8s.yaml`:

```yaml
spec:
  template:
    metadata:
      annotations:
        azure.workload.identity/use: "true"
```

### 7. Deploy

```bash
kubectl apply -f deploy/k8s.yaml
```

## Verification

Check that the pod has the Azure identity token mounted:

```bash
kubectl exec -it deployment/s3proxy -- ls -la /var/run/secrets/azure/tokens/
```

