# ☸️ IRDB Helm Chart

Production-ready **PostgreSQL 17.5** with **ParadeDB** and **pgvector** on Kubernetes using **CloudNativePG**.

## 🚀 Quick Start

```bash
# Add CloudNativePG operator (once per cluster)
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update
helm install cnpg --namespace cnpg-system --create-namespace cnpg/cloudnative-pg

# Install dependencies
cd k8s/
helm dependency update

# Deploy (development)
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-dev.yaml

# Deploy (production)
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-prod.yaml
```

## 📦 What's Included

- **PostgreSQL 17.5** - Latest stable release
- **ParadeDB pg_search** - BM25 full-text search
- **pgvector** - Vector similarity search (1536 dims)
- **CloudNativePG** - Automated HA and failover
- **Continuous Backup** - PITR to S3-compatible storage

## 🎯 Deployment Profiles

### Development (`values-dev.yaml`)

```yaml
instances: 1              # Single instance, no HA
storage: 10Gi             # Minimal storage
backup: disabled          # No backup
```

**Use for**: Local testing, CI/CD pipelines

### Production (`values-prod.yaml`)

```yaml
instances: 3              # High availability
minSyncReplicas: 1        # Zero data loss
storage: 100Gi            # Production storage
backup: enabled           # Continuous backup
monitoring: enabled       # Prometheus metrics
```

**Use for**: Production workloads, critical data

## ⚙️ Key Configuration

### High Availability

```yaml
cnpg:
  cluster:
    instances: 3              # 1 primary + 2 replicas
    minSyncReplicas: 1        # Wait for 1 replica confirmation
    maxSyncReplicas: 2        # Max replicas in sync group
    primaryUpdateStrategy: unsupervised  # Auto failover
```

**Failover time**: ~30-60 seconds

### Storage

```yaml
cnpg:
  cluster:
    storage:
      size: 100Gi
      storageClass: fast-ssd  # Use fast storage for production
```

**Best practices**:
- Use SSD storage classes (`gp3`, `premium-ssd`)
- Size for 3-6 months growth
- Monitor usage with Prometheus

### Backup & Recovery

```yaml
cnpg:
  cluster:
    backup:
      enabled: true
      retentionPolicy: "30d"
      barmanObjectStore:
        destinationPath: s3://bucket/postgres-backups/
        s3Credentials:
          accessKeyId:
            name: backup-creds
            key: ACCESS_KEY_ID
          secretAccessKey:
            name: backup-creds
            key: SECRET_ACCESS_KEY
```

**Point-in-Time Recovery**:

```yaml
bootstrap:
  recovery:
    source: backup-source
    recoveryTarget:
      targetTime: "2025-12-17 10:30:00"
```

### Resources

```yaml
cnpg:
  cluster:
    resources:
      requests:
        cpu: "2"
        memory: "4Gi"
      limits:
        cpu: "4"
        memory: "8Gi"
```

**Sizing guidelines**:
- **Small**: 2 CPU, 4Gi RAM (< 10GB data)
- **Medium**: 4 CPU, 8Gi RAM (10-100GB data)
- **Large**: 8+ CPU, 16Gi+ RAM (> 100GB data)

## 🔍 Accessing the Database

### Port Forward

```bash
# Forward to primary (read-write)
kubectl port-forward -n databases svc/postgres-rw 5432:5432

# Forward to any replica (read-only)
kubectl port-forward -n databases svc/postgres-ro 5432:5432
```

### Get Password

```bash
kubectl get secret postgres-superuser -n databases \
  -o jsonpath='{.data.password}' | base64 -d
```

### Connect

```bash
psql -h localhost -U postgres -d database -p 5432
```

## 📊 Monitoring

CloudNativePG exposes Prometheus metrics on port `9187`:

```yaml
cnpg:
  cluster:
    monitoring:
      enabled: true
      podMonitorEnabled: true
```

**Key metrics**:
- `cnpg_pg_replication_lag` - Replication lag in bytes
- `cnpg_pg_database_size_bytes` - Database size
- `cnpg_pg_stat_database_xact_commit` - Transaction rate

**Grafana Dashboard**: <https://grafana.com/grafana/dashboards/20417>

## 🔧 Management

### Upgrade Database

```bash
# Update values
helm upgrade irdb-postgres . \
  --namespace databases \
  -f values-prod.yaml \
  --set cnpg.cluster.imageName=sojoner/database:0.0.8
```

### Manual Backup

```bash
kubectl cnpg backup postgres -n databases
```

### View Logs

```bash
# Primary logs
kubectl logs -n databases postgres-1 -f

# All instances
kubectl logs -n databases -l cnpg.io/cluster=postgres -f
```

### Scale Replicas

```bash
helm upgrade irdb-postgres . \
  --namespace databases \
  -f values-prod.yaml \
  --set cnpg.cluster.instances=5
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────┐
│  CloudNativePG Operator (cnpg-system)           │
│  Manages lifecycle, failover, backup            │
└─────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────┐
│  PostgreSQL Cluster (databases namespace)       │
│                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ Primary  │─▶│ Replica  │  │ Replica  │     │
│  │ (RW)     │  │ (RO)     │  │ (RO)     │     │
│  └────┬─────┘  └──────────┘  └──────────┘     │
│       │                                         │
│       ▼                                         │
│  ┌──────────┐                                  │
│  │ Backup   │                                   │
│  │ to S3    │                                   │
│  └──────────┘                                  │
└─────────────────────────────────────────────────┘
```

## 🎓 CloudNativePG Best Practices

### 1. Always Use 3+ Instances in Production

```yaml
instances: 3  # Minimum for HA
```

**Why**: Survives single node failure, enables automatic failover

### 2. Enable Synchronous Replication

```yaml
minSyncReplicas: 1  # Zero data loss
```

**Trade-off**: Slightly slower writes, zero data loss on failover

### 3. Configure Backup Immediately

```yaml
backup:
  enabled: true
  retentionPolicy: "30d"
```

**Why**: Point-in-time recovery, disaster recovery

### 4. Use Anti-Affinity

```yaml
affinity:
  podAntiAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      - topologyKey: kubernetes.io/hostname
```

**Why**: Spreads replicas across nodes, prevents single point of failure

### 5. Monitor Everything

```yaml
monitoring:
  enabled: true
```

**Why**: Catch issues early, capacity planning, performance tuning

## 📚 Documentation

- **[CloudNativePG Docs](https://cloudnative-pg.io/documentation/)** - Official documentation
- **[Deployment Guide](../docs/02-deployment.md)** - Complete deployment guide
- **[Architecture](../docs/01-architecture.md)** - System design

## 🐛 Troubleshooting

### Pods Not Starting

```bash
kubectl describe pod postgres-1 -n databases
kubectl logs postgres-1 -n databases
```

### Replication Lag

```bash
kubectl exec -it postgres-1 -n databases -- psql -U postgres -c "SELECT * FROM pg_stat_replication;"
```

### Backup Failures

```bash
kubectl get backup -n databases
kubectl describe backup <backup-name> -n databases
```

## 🚀 Advanced Features

### Connection Pooling

Use [PgBouncer](https://cloudnative-pg.io/documentation/current/connection_pooling/) for connection pooling:

```yaml
cnpg:
  pooler:
    enabled: true
    instances: 3
    type: rw
```

### Custom TLS Certificates

```yaml
cnpg:
  cluster:
    certificates:
      serverTLSSecret: postgres-tls
      serverCASecret: postgres-ca
```

### Custom PostgreSQL Configuration

```yaml
cnpg:
  cluster:
    postgresql:
      parameters:
        max_connections: "500"
        shared_buffers: "4GB"
        effective_cache_size: "12GB"
        work_mem: "64MB"
```

## 📦 Chart Values Reference

See [CloudNativePG Cluster Chart](https://github.com/cloudnative-pg/charts/tree/main/charts/cluster) for all available values.

---

**Status**: ✅ Production ready | 📊 Battle tested
