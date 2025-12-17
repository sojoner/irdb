# Deployment Guide

This guide covers deploying IRDB using Docker Compose (local development) or Kubernetes (production).

## Prerequisites

### Required Tools
- Docker 20.10+
- Docker Compose 2.0+ (for local development)
- kubectl 1.28+ (for Kubernetes)
- Helm 3.12+ (for Kubernetes)
- kind 0.20+ (for local Kubernetes testing)

### Check Prerequisites

```bash
make check-prereqs
```

Or install missing tools:

```bash
make install-kind
make install-kubectl
make install-helm
```

## Docker Compose Deployment

Best for local development and testing.

### Quick Start

```bash
# Build the Docker image
make compose-build

# Start PostgreSQL + pgAdmin
make compose-up

# View logs
make compose-logs

# Connect to database
psql -h localhost -U postgres -d database -p 5432
# Password: custom_secure_password_123
```

### pgAdmin Access

- URL: http://localhost:5433
- Email: admin@database.com
- Password: custom_secure_password_123

The PostgreSQL server is pre-configured in pgAdmin (no manual setup needed).

### Rebuilding After Changes

When you modify initialization scripts in `docker-entrypoint-initdb.d/`:

```bash
# Clean rebuild (removes volumes to trigger re-initialization)
make compose-clean
make compose-build
make compose-up
```

Or manually:

```bash
docker-compose down -v
docker-compose up -d --build
```

### Docker Compose Configuration

The `docker-compose.yml` defines two services:

**PostgreSQL:**
- Image: `sojoner/database:0.0.7`
- Port: 5432
- Resources: 4-8 CPUs, 16-32GB RAM, 2GB shared memory
- Volume: `./postgres-data` (persistent storage)

**pgAdmin:**
- Image: `dpage/pgadmin4:latest`
- Port: 5433 (maps to internal 80)
- Pre-configured with PostgreSQL connection

### Environment Variables

Default credentials (change for production):

```bash
POSTGRES_DB=database
POSTGRES_USER=postgres
POSTGRES_PASSWORD=custom_secure_password_123
PGADMIN_DEFAULT_EMAIL=admin@database.com
PGADMIN_DEFAULT_PASSWORD=custom_secure_password_123
```

## Kubernetes Deployment

Production-ready deployment using Helm and CloudNativePG operator.

### Architecture

IRDB uses **CloudNativePG** for PostgreSQL management:
- Automated failover and high availability
- Continuous backup and point-in-time recovery
- Rolling updates with zero downtime
- Monitoring and metrics integration

Documentation: https://cloudnative-pg.io/documentation/

### Local Kubernetes (kind)

Create a local Kubernetes cluster for testing:

```bash
# Complete setup (cluster + operator + database)
make setup-all

# Or step by step
make create-cluster        # Create kind cluster
make install-operator      # Install CloudNativePG operator
make deploy-db             # Deploy database

# Check status
make status

# View logs
make logs

# Port-forward to access database
make port-forward          # In one terminal
make connect               # In another terminal
```

### Production Kubernetes

#### 1. Install CloudNativePG Operator

One-time installation per cluster:

```bash
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update
helm install cnpg \
  --namespace cnpg-system \
  --create-namespace \
  cnpg/cloudnative-pg
```

Verify installation:

```bash
kubectl get pods -n cnpg-system
```

#### 2. Deploy IRDB

**Development (1 instance):**

```bash
cd k8s/
helm dependency update
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-dev.yaml
```

**Production (3 instances with HA):**

```bash
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-prod.yaml
```

#### 3. Access the Database

Get the password:

```bash
kubectl get secret postgres-superuser -n databases \
  -o jsonpath='{.data.password}' | base64 -d
```

Port-forward to the primary instance:

```bash
kubectl port-forward -n databases svc/postgres-rw 5432:5432
```

Connect with psql:

```bash
psql -h localhost -U postgres -d database -p 5432
```

### Helm Chart Structure

```
k8s/
├── Chart.yaml              # Chart metadata, dependencies
├── values.yaml             # Default: 3 instances, production config
├── values-dev.yaml         # Override: 1 instance, minimal resources
├── values-prod.yaml        # Override: Enhanced resources, monitoring
└── templates/
    ├── _helpers.tpl        # Template functions
    ├── clusterimagecatalog.yaml  # Custom PostgreSQL image catalog
    └── NOTES.txt           # Post-install instructions
```

### Configuration Profiles

**Development (`values-dev.yaml`):**
- 1 PostgreSQL instance
- Minimal resources (2 CPU, 4GB RAM)
- No backup configured
- Quick startup for testing

**Production (`values-prod.yaml`):**
- 3 PostgreSQL instances (1 primary + 2 replicas)
- High resources (4 CPU, 8GB RAM per instance)
- Continuous backup to S3 or compatible storage
- `minSyncReplicas: 1` for synchronous replication
- Monitoring enabled (Prometheus metrics)

### Helm Values Customization

Key configuration options:

```yaml
cluster:
  instances: 3                    # Number of PostgreSQL instances
  primaryUpdateStrategy: unsupervised  # or "supervised" for manual failover

  storage:
    size: 20Gi                    # PVC size per instance

  postgresql:
    parameters:
      max_connections: "200"      # Connection limit
      shared_buffers: "2GB"       # Memory for caching
      work_mem: "256MB"           # Memory per operation

  monitoring:
    enabled: true                 # Prometheus metrics

  backup:
    enabled: true
    barmanObjectStore:
      destinationPath: s3://bucket/path
      s3Credentials:
        accessKeyId:
          name: backup-creds
          key: ACCESS_KEY_ID
        secretAccessKey:
          name: backup-creds
          key: SECRET_ACCESS_KEY
```

Full configuration reference: https://cloudnative-pg.io/documentation/1.24/

### ArgoCD Deployment

Deploy IRDB from a Git repository:

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

Documentation: https://argo-cd.readthedocs.io/

### High Availability Setup

For production, use 3+ instances with synchronous replication:

```yaml
cluster:
  instances: 3
  minSyncReplicas: 1        # At least 1 replica must confirm writes
  maxSyncReplicas: 2        # Max replicas for sync replication
```

**Benefits:**
- Automatic failover (typically 30-60 seconds)
- Zero data loss (synchronous replication)
- Read scaling (route reads to replicas)
- Rolling updates with no downtime

**Trade-offs:**
- Slightly higher write latency (waits for replica confirmation)
- Requires 3+ nodes for proper quorum

### Monitoring

CloudNativePG exposes Prometheus metrics:

```bash
# Port-forward to metrics endpoint
kubectl port-forward -n databases pod/postgres-1 9187:9187

# Scrape metrics
curl http://localhost:9187/metrics
```

Key metrics:
- `cnpg_pg_replication_lag` - Replication lag in bytes
- `cnpg_pg_database_size_bytes` - Database size
- `cnpg_pg_stat_database_*` - Connection and transaction stats

Grafana dashboard: https://grafana.com/grafana/dashboards/20417-cloudnativepg/

### Backup and Recovery

**Configure Backup:**

```yaml
backup:
  enabled: true
  retentionPolicy: "30d"          # Keep backups for 30 days
  barmanObjectStore:
    destinationPath: s3://bucket/postgres-backups/irdb
    s3Credentials:
      accessKeyId:
        name: backup-creds
        key: ACCESS_KEY_ID
      secretAccessKey:
        name: backup-creds
        key: SECRET_ACCESS_KEY
    wal:
      compression: gzip             # Compress WAL files
      maxParallel: 2                # Parallel upload
```

**Manual Backup:**

```bash
kubectl cnpg backup postgres -n databases
```

**Point-in-Time Recovery:**

```yaml
apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: postgres-restored
spec:
  instances: 3
  bootstrap:
    recovery:
      source: postgres
      recoveryTarget:
        targetTime: "2025-12-17 10:30:00"
```

Documentation: https://cloudnative-pg.io/documentation/1.24/backup_recovery/

## Validation

After deployment, run validation tests:

```bash
# All validation tests
make validate-all

# Individual tests
make validate-bm25      # Test BM25 full-text search
make validate-vector    # Test vector similarity search
make validate-hybrid    # Test hybrid search
```

Expected output:
```
✓ BM25 search validated
✓ Vector search validated
✓ Hybrid search validated
```

## Troubleshooting

### Docker Compose Issues

**Problem:** "Database not initialized"
```bash
# Remove volumes and rebuild
make compose-clean
make compose-build
make compose-up
```

**Problem:** "Extension not found"
```bash
# Check extension installation
docker exec -it irdb-postgres psql -U postgres -d database -c "\dx"
```

### Kubernetes Issues

**Problem:** "Pods not starting"
```bash
# Check events
kubectl describe pod postgres-1 -n databases

# Check logs
kubectl logs postgres-1 -n databases
```

**Problem:** "Cannot connect to database"
```bash
# Verify service
kubectl get svc -n databases

# Check pod status
kubectl get pods -n databases

# Port-forward and test
kubectl port-forward -n databases svc/postgres-rw 5432:5432
psql -h localhost -U postgres -d database
```

**Problem:** "Image pull errors"
```bash
# For kind, load image manually
make retag-image
make load-image
```

## Next Steps

- [Hybrid Search Deep Dive](./03-hybrid-search.md) - Learn how search works
- [Web Application Development](./04-web-app.md) - Build the UI
- [References & Resources](./05-references.md) - Upstream documentation
