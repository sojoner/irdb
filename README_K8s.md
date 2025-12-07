# Kubernetes Deployment Guide

Complete guide for deploying IR DB (AI-enhanced PostgreSQL 17 with pgvector and ParadeDB) on Kubernetes using CloudNativePG operator.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start-kind-cluster)
3. [Deploying the Database](#deploying-the-database)
4. [Customizing the Deployment](#customizing-the-deployment)
5. [Accessing the Database](#accessing-the-database)
6. [Managing the Deployment](#managing-the-deployment)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

Install required tools:
- [kubectl](https://kubernetes.io/docs/tasks/tools/) - Kubernetes CLI
- [helm](https://helm.sh/docs/intro/install/) - Package manager (v3+)
- A running Kubernetes cluster (local: kind/minikube/k3s, or remote: EKS/GKE/AKS)

### Quick Start (kind cluster)

For local testing with kind:

```bash
# Create kind cluster
kind create cluster --name irdb-cluster

# Add CloudNativePG Helm repository
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update

# Install CloudNativePG operator
helm install cnpg \
  --namespace cnpg-system \
  --create-namespace \
  cnpg/cloudnative-pg

# Wait for operator to be ready
kubectl wait --for=condition=Available \
  --timeout=300s \
  -n cnpg-system \
  deployment/cnpg-cloudnative-pg
```

### Deploying the Database

The `k8s/` directory contains a Helm chart that wraps the CloudNativePG cluster chart with a custom image catalog.

#### Chart Structure

```
k8s/
├── Chart.yaml              # Chart metadata and dependencies
├── values.yaml             # Default configuration (3 instances, production-ready)
├── values-dev.yaml         # Development overrides (1 instance, lower resources)
├── values-prod.yaml        # Production overrides (enhanced resources)
└── templates/
    ├── _helpers.tpl        # Template helpers
    ├── clusterimagecatalog.yaml  # Custom image catalog
    └── NOTES.txt           # Post-install instructions
```

#### Step 1: Install Dependencies

The chart depends on the upstream CloudNativePG cluster chart:

```bash
cd k8s/
helm dependency update
```

This downloads the `cloudnative-pg/cluster` chart into `charts/` directory.

#### Step 2: Review Configuration

Edit `values.yaml` to customize:
- Image version (`image.tag`)
- Instance count (`cnpg.cluster.instances`)
- Storage size (`cnpg.cluster.storage.size`)
- Resource requests/limits (`cnpg.cluster.resources`)
- PostgreSQL parameters (`cnpg.cluster.postgresql.parameters`)

#### Step 3: Deploy for Development

```bash
# Install with development values (single instance, low resources)
helm install irdb-postgres k8s/ \
  --namespace databases \
  --create-namespace \
  -f k8s/values-dev.yaml

# Watch the deployment
kubectl get pods -n databases -w
```

#### Step 4: Deploy for Production

```bash
# Install with production values (3 instances, high availability)
helm install irdb-postgres k8s/ \
  --namespace databases \
  --create-namespace \
  -f k8s/values-prod.yaml
```

#### Step 5: Verify Deployment

```bash
# Check cluster status
kubectl get cluster -n databases

# Check pods
kubectl get pods -n databases

# View post-install notes
helm get notes irdb-postgres -n databases
```

### Deploying from GitHub (ArgoCD)

If you host this chart in a public GitHub repository, you can deploy via ArgoCD:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: irdb-postgres
  namespace: argocd
spec:
  project: default

  source:
    repoURL: https://github.com/yourusername/irdb.git
    path: k8s
    targetRevision: main

    helm:
      releaseName: postgres
      valueFiles:
        - values-prod.yaml

  destination:
    server: https://kubernetes.default.svc
    namespace: databases

  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=true
```

### Customizing the Deployment

#### Environment-Specific Values

Create custom values files for different environments:

```yaml
# values-staging.yaml
cnpg:
  cluster:
    instances: 2
    storage:
      size: 20Gi
    resources:
      requests:
        memory: "2Gi"
        cpu: "1000m"
```

Deploy with:
```bash
helm upgrade --install irdb-postgres k8s/ \
  -f k8s/values-staging.yaml \
  -n databases
```

#### Updating Image Version

Update the image tag in `values.yaml` or override via command line:

```bash
helm upgrade irdb-postgres k8s/ \
  --set image.tag=0.0.8 \
  -n databases
```

#### Scaling the Cluster

```bash
# Scale to 5 instances
helm upgrade irdb-postgres k8s/ \
  --set cnpg.cluster.instances=5 \
  -n databases

# Or edit values.yaml and upgrade
helm upgrade irdb-postgres k8s/ -n databases
```

### Accessing the Database

The database is accessible via three service endpoints:
- `postgres-rw` - Primary instance (read-write)
- `postgres-ro` - Replica instances (read-only)
- `postgres-r` - Any instance (read)

#### Option 1: Port-Forward

```bash
# Port-forward to the primary instance
kubectl port-forward -n databases svc/postgres-rw 5432:5432

# In another terminal, connect
psql -h localhost -U postgres -d database -p 5432

# Get the password
kubectl get secret postgres-superuser -n databases \
  -o jsonpath='{.data.password}' | base64 -d
```

#### Option 2: From Within the Cluster

```bash
# Create a client pod
kubectl run -it --rm psql-client \
  --image=postgres:17 \
  --restart=Never \
  -n databases \
  -- psql -h postgres-rw -U postgres -d database
```

### Viewing Logs

```bash
# View cluster logs
kubectl logs -n databases -l cnpg.io/cluster=postgres --tail=100 -f

# View specific pod logs
kubectl logs -n databases postgres-1 -f
```

### Checking Status

```bash
# Check cluster status
kubectl get cluster -n databases

# Check pod status
kubectl get pods -n databases

# Get detailed cluster info (requires kubectl-cnpg plugin)
kubectl cnpg status postgres -n databases
```

### Managing the Deployment

#### Upgrade

```bash
# Upgrade with new values
helm upgrade irdb-postgres k8s/ \
  -f k8s/values-prod.yaml \
  -n databases

# Upgrade with inline overrides
helm upgrade irdb-postgres k8s/ \
  --set image.tag=0.0.8 \
  --set cnpg.cluster.instances=5 \
  -n databases
```

#### Rollback

```bash
# View release history
helm history irdb-postgres -n databases

# Rollback to previous version
helm rollback irdb-postgres -n databases

# Rollback to specific revision
helm rollback irdb-postgres 2 -n databases
```

#### Uninstall

```bash
# Uninstall the database (keeps PVCs)
helm uninstall irdb-postgres -n databases

# Delete PVCs if needed
kubectl delete pvc -n databases -l cnpg.io/cluster=postgres

# Delete namespace
kubectl delete namespace databases
```

**For database schema, SQL examples, and validation queries, see [README.md](README.md).**

### Validation with Makefile

The project includes Makefile targets for quick validation after deployment:

```bash
make validate-bm25     # Test BM25 full-text search
make validate-vector   # Test vector similarity search
make validate-hybrid   # Test hybrid search (vector + BM25)
make validate-all      # Run all validation tests
```

---

## Troubleshooting

### Kubernetes Issues

**Problem: Pods not starting**
```bash
# Check events
kubectl get events -n databases --sort-by='.lastTimestamp'

# Check pod logs
kubectl logs -n databases postgres-1

# Check operator logs
kubectl logs -n cnpg-system -l app.kubernetes.io/name=cloudnative-pg

# Check Helm release status
helm status irdb-postgres -n databases
```

**Problem: Image pull errors**
```bash
# If using local images with kind, must load into cluster
kind load docker-image sojoner/database:0.0.7 --name irdb-cluster

# Verify image is loaded
docker exec -it irdb-cluster-control-plane crictl images | grep sojoner

# For production, ensure image is pushed to registry
docker push sojoner/database:0.0.7
```

**Problem: Database pods crash looping**
```bash
# Check pod status
kubectl describe pod -n databases postgres-1

# Common issues:
# 1. Insufficient resources - reduce requests in values.yaml
# 2. Image tag mismatch - ensure tag matches majorVersion
# 3. Init script errors - check logs for SQL errors

# Update resources and upgrade
helm upgrade irdb-postgres k8s/ \
  --set cnpg.cluster.resources.requests.memory=512Mi \
  -n databases
```

**Problem: Can't connect to database**
```bash
# Verify service exists
kubectl get svc -n databases

# Check if port-forward works
kubectl port-forward -n databases svc/postgres-rw 5432:5432

# Get correct password
kubectl get secret postgres-superuser -n databases \
  -o jsonpath='{.data.password}' | base64 -d
```

**Problem: Extensions not found after deployment**
```bash
# Check if initialization scripts are in the image
kubectl exec -n databases postgres-1 -- ls -la /docker-entrypoint-initdb.d/

# If missing, image wasn't built correctly
# Rebuild and push image:
docker build -t sojoner/database:0.0.7 .
docker push sojoner/database:0.0.7

# Update Helm chart
helm upgrade irdb-postgres k8s/ -n databases
```

**Problem: Helm dependency issues**
```bash
# Update dependencies
cd k8s/
helm dependency update

# Verify dependencies
helm dependency list

# If issues persist, clear cache
rm -rf k8s/charts/
helm dependency build k8s/
```

### Validation Failures

**Problem: "relation does not exist" errors**
- This indicates initialization scripts didn't run properly
- For Kubernetes: Delete and recreate the cluster resource
- Check logs: `kubectl logs -n databases postgres-1 | grep "docker-entrypoint-initdb.d"`
- See [README.md](README.md) for database schema verification queries

**Problem: Vector dimension mismatch**
- If you need different dimensions, update `01-ai-extensions.sql`
- Rebuild the Docker image and redeploy
- See [README.md](README.md) for database validation queries

### Getting Help

- Check CloudNativePG docs: https://cloudnative-pg.io/documentation/
- View this project's issues: https://github.com/sojoner/irdb/issues
- PostgreSQL documentation: https://www.postgresql.org/docs/17/
- pgvector documentation: https://github.com/pgvector/pgvector
- ParadeDB documentation: https://docs.paradedb.com/

---

## Quick Reference

### Connection Details

| Parameter | Value |
|-----------|-------|
| Host | localhost (via port-forward) or service DNS |
| Port | 5432 |
| Database | database |
| Username | postgres |
| Password | Retrieved from secret |

### Important File Locations

| Purpose | Location |
|---------|----------|
| Dockerfile | `/Dockerfile` |
| PostgreSQL config | `/postgresql.conf` |
| Init scripts | `/docker-entrypoint-initdb.d/*.sql` |
| Docker Compose | `/docker-compose.yml` |
| Helm chart | `/k8s/` |
| Chart metadata | `/k8s/Chart.yaml` |
| Default values | `/k8s/values.yaml` |
| Dev values | `/k8s/values-dev.yaml` |
| Prod values | `/k8s/values-prod.yaml` |
| Image catalog template | `/k8s/templates/clusterimagecatalog.yaml` |

### Useful Commands

```bash
# Helm Commands
helm install irdb-postgres k8s/ -n databases -f k8s/values-dev.yaml   # Install
helm upgrade irdb-postgres k8s/ -n databases                          # Upgrade
helm uninstall irdb-postgres -n databases                             # Uninstall
helm list -n databases                                                # List releases
helm history irdb-postgres -n databases                               # Show history

# Kubernetes Commands
kubectl get all -n databases                                          # Show all resources
kubectl get cluster -n databases                                      # Show cluster status
kubectl logs -n databases postgres-1 -f                               # Follow logs
kubectl exec -it -n databases postgres-1 -- psql -U postgres -d database  # Connect to DB
kubectl port-forward -n databases svc/postgres-rw 5432:5432           # Port-forward

# Get database password
kubectl get secret postgres-superuser -n databases -o jsonpath='{.data.password}' | base64 -d
```

## Next Steps

- **[README.md](README.md)** - Project overview, architecture, and common database operations
- **[README_DOCKER.md](README_DOCKER.md)** - Docker Compose guide for local development
- **[k8s/README.md](k8s/README.md)** - Helm chart reference and CloudNativePG best practices
- **[.claude/CLAUDE.md](.claude/CLAUDE.md)** - Development guide for contributors

## Additional Resources

**CloudNativePG:**
- [Official Documentation](https://cloudnative-pg.io/documentation/)
- [Quickstart Guide](https://cloudnative-pg.io/documentation/current/quickstart/)
- [GitHub Repository](https://github.com/cloudnative-pg/cloudnative-pg)
- [Helm Charts](https://github.com/cloudnative-pg/charts)

**PostgreSQL & Extensions:**
- [PostgreSQL 17 Documentation](https://www.postgresql.org/docs/17/)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [ParadeDB Documentation](https://docs.paradedb.com/)

**Kubernetes:**
- [Helm Documentation](https://helm.sh/docs/)
- [kubectl Reference](https://kubernetes.io/docs/reference/kubectl/)
- [Kubernetes Storage](https://kubernetes.io/docs/concepts/storage/)
