# IR DB PostgreSQL Helm Chart

This Helm chart deploys an AI-enhanced PostgreSQL 17 database with pgvector and ParadeDB extensions using CloudNativePG operator.

## Overview

This chart wraps the official CloudNativePG `cluster` chart and adds a `ClusterImageCatalog` for our custom PostgreSQL image. It simplifies deployment by managing both the image catalog and cluster configuration through a single Helm release.

## CloudNativePG Best Practices

Understanding CloudNativePG's architecture and best practices is essential for running production-grade PostgreSQL clusters in Kubernetes.

### Architecture & Design Principles

CloudNativePG implements a **primary/standby architecture** using PostgreSQL's native streaming replication. Unlike other operators (Patroni, repmgr, Stolon), CloudNativePG integrates directly with the Kubernetes API without requiring external failover management tools.

**Key Design Concepts:**

- **Instances Parameter**: The single parameter `instances` controls cluster size. Setting `instances: 1` creates a standalone primary; `instances: 3` creates one primary plus two replicas
- **Automated Failover**: The operator detects failures and promotes replicas automatically without external tools
- **Rolling Updates**: Configuration changes trigger switchover operations for zero-downtime updates
- **Direct PVC Management**: CloudNativePG manages PersistentVolumeClaims directly rather than using StatefulSets

### High Availability Configuration

For production deployments, follow these HA best practices:

#### Minimum Instance Count

```yaml
cnpg:
  cluster:
    instances: 3  # Minimum for production HA
```

- **Single instance** (`instances: 1`): Development/testing only, no HA
- **Three instances** (`instances: 3`): Production minimum, survives one node failure
- **Five instances** (`instances: 5`): Enhanced resilience, survives two node failures

#### Synchronous Replication

Enable synchronous replication for zero data loss:

```yaml
cnpg:
  cluster:
    minSyncReplicas: 1      # Minimum replicas that must confirm writes
    maxSyncReplicas: 2      # Maximum replicas in sync group
```

**Trade-offs:**
- **Disabled** (default): Maximum performance, potential data loss on failover
- **Enabled**: Zero data loss, reduced write performance (waits for replica confirmation)

#### Replication Slot Management

CloudNativePG automatically manages physical replication slots for each standby replica. This ensures WAL files required by standby instances are retained on the primary's storage, even after failover.

**Default behavior** (recommended):
- Replication slots enabled automatically
- Managed by the operator
- Ensures WAL retention for all replicas

To disable (not recommended):
```yaml
cnpg:
  cluster:
    replicationSlots:
      highAvailability:
        enabled: false
```

### Storage Best Practices

#### Storage Class Selection

Choose appropriate storage classes based on workload:

```yaml
cnpg:
  cluster:
    storage:
      storageClass: fast-ssd  # e.g., gp3, pd-ssd, longhorn
      size: 100Gi
```

**Recommendations:**
- **Development**: Standard HDD storage classes acceptable
- **Production**: Use SSD-backed storage (AWS gp3, GCP pd-ssd, Azure Premium SSD)
- **Critical workloads**: NVMe-based local storage for maximum IOPS

#### WAL Storage Separation

For better performance and reliability, store Write-Ahead Logs on a separate volume:

```yaml
cnpg:
  cluster:
    walStorage:
      storageClass: fast-nvme  # Faster than PGDATA storage
      size: 20Gi
```

**Benefits:**
- **Parallel I/O**: WAL writes don't compete with data file operations
- **Reliability**: Dedicated WAL space prevents PGDATA exhaustion from affecting log writes
- **Independent sizing**: Optimize storage size and class separately for each volume
- **Better monitoring**: Track PGDATA and pg_wal usage independently

#### Storage Sizing Guidelines

**PGDATA Volume:**
- Calculate based on: actual data + indexes + temporary files + 20% growth buffer
- AI/RAG workloads: Plan for vector embeddings (1536-dim float = ~6KB per document)

**WAL Volume** (if separated):
- Minimum: 3x checkpoint segments (typically 10-20Gi)
- With continuous backup: Can be smaller as archived WALs are removed
- Without backup: Size for maximum retention between cleanups

#### Volume Expansion

Ensure your StorageClass supports expansion:

```bash
# Check if storage class allows expansion
kubectl get storageclass <class-name> -o jsonpath='{.allowVolumeExpansion}'
```

To expand a cluster's storage:

```yaml
cnpg:
  cluster:
    storage:
      size: 200Gi  # Increased from 100Gi
```

**Note**: Expansion is one-way only (cannot shrink). Some storage classes require pod deletion for offline resizing.

#### Recommendations for Block Storage

When using replicated block storage (Ceph, Longhorn):

1. **Reduce storage-level replication** to 1 (rely on PostgreSQL replication instead)
2. **Enable pod anti-affinity** to spread instances across nodes
3. **Use strict-local data locality** (Longhorn) to prevent single points of failure

```yaml
cnpg:
  cluster:
    affinity:
      podAntiAffinityType: required  # Spread pods across nodes
```

### Resource Management

#### QoS Class Configuration

For **Guaranteed QoS** (recommended for production), set requests equal to limits:

```yaml
cnpg:
  cluster:
    resources:
      requests:
        memory: "4Gi"
        cpu: "2000m"
      limits:
        memory: "4Gi"    # Same as request
        cpu: "2000m"     # Same as request
```

**QoS Classes:**
- **Guaranteed**: Requests = Limits (production recommended)
- **Burstable**: Requests < Limits (acceptable for non-critical workloads)
- **BestEffort**: No requests/limits (development only)

#### Memory Sizing

Follow the PostgreSQL memory hierarchy:

```yaml
cnpg:
  cluster:
    resources:
      requests:
        memory: "4Gi"
    postgresql:
      parameters:
        shared_buffers: "1024MB"       # 25% of total memory
        effective_cache_size: "3GB"    # 75% of total memory
        work_mem: "32MB"                # Per-operation memory
        maintenance_work_mem: "512MB"  # For VACUUM, CREATE INDEX
```

**Sizing Formula:**
- **Total container memory**: Your workload + PostgreSQL buffers + OS overhead
- **shared_buffers**: 25% of container memory (reasonable starting point)
- **effective_cache_size**: 75% of container memory (for query planner)
- **work_mem**: Total memory ÷ max_connections ÷ 4 (conservative estimate)

**Example**: For a 4Gi container:
- `shared_buffers`: 1GB (25%)
- `effective_cache_size`: 3GB (75%)
- `work_mem`: For 100 connections = 4096MB ÷ 100 ÷ 4 = ~10MB per operation

#### CPU Sizing

```yaml
cnpg:
  cluster:
    resources:
      requests:
        cpu: "2000m"  # 2 cores
    postgresql:
      parameters:
        max_parallel_workers: "8"
        max_parallel_workers_per_gather: "4"
        max_worker_processes: "16"
```

**Guidelines:**
- **Development**: 500m-1000m (0.5-1 core)
- **Production**: 2000m-4000m (2-4 cores) minimum
- **AI/RAG workloads**: 4000m+ for vector search operations
- **max_parallel_workers**: Should not exceed CPU core count

### PostgreSQL Configuration

CloudNativePG uses **declarative configuration** through the Cluster manifest. Never use `ALTER SYSTEM` as it bypasses replication and cluster-wide consistency.

#### Configuration Changes

```yaml
cnpg:
  cluster:
    postgresql:
      parameters:
        max_connections: "200"
        shared_buffers: "2GB"
        # ... other parameters
```

**Behavior:**
- **Hot reload**: Parameters that don't require restart are applied immediately
- **Rolling restart**: Parameters requiring restart trigger automated rolling updates
- **Cluster-wide**: All instances receive the same configuration

#### Essential Production Parameters

```yaml
cnpg:
  cluster:
    postgresql:
      parameters:
        # Connection settings
        max_connections: "200"

        # Memory settings
        shared_buffers: "2GB"
        effective_cache_size: "6GB"
        work_mem: "32MB"
        maintenance_work_mem: "512MB"

        # WAL settings
        wal_level: "logical"                    # Enable logical replication
        max_wal_size: "2GB"
        min_wal_size: "1GB"
        wal_buffers: "16MB"

        # Checkpoint settings
        checkpoint_timeout: "15min"
        checkpoint_completion_target: "0.9"

        # Query planner
        random_page_cost: "1.1"                 # For SSD storage
        effective_io_concurrency: "200"         # For SSD storage

        # Parallel query
        max_parallel_workers: "8"
        max_parallel_workers_per_gather: "4"

        # Logging
        log_min_duration_statement: "1000"      # Log queries > 1s
        log_checkpoints: "on"
        log_connections: "on"
        log_disconnections: "on"
        log_lock_waits: "on"

        # Security
        ssl_min_protocol_version: "TLSv1.3"
```

### Security Best Practices

#### TLS/SSL Configuration

CloudNativePG enables TLS by default with auto-generated certificates:

```yaml
cnpg:
  cluster:
    certificates:
      serverTLSSecret: ""          # Auto-generated if empty
      serverCASecret: ""           # Auto-generated if empty
      clientCASecret: ""           # For client cert authentication
      replicationTLSSecret: ""     # For replica connections
```

**Production recommendations:**
1. **Use cert-manager** for automated certificate management
2. **Bring your own certificates** for custom CA integration
3. **Enable client certificate authentication** for enhanced security

#### Authentication Configuration

Configure `pg_hba` rules for client authentication:

```yaml
cnpg:
  cluster:
    postgresql:
      pg_hba:
        # Allow superuser from localhost only
        - host all postgres 127.0.0.1/32 scram-sha-256
        # Application access with SCRAM-SHA-256
        - hostssl all all 0.0.0.0/0 scram-sha-256
        # Require client certificates for admin users
        - hostssl all admin 0.0.0.0/0 cert
```

**Best practices:**
- Use `scram-sha-256` instead of `md5` for password authentication
- Require SSL (`hostssl`) for all network connections
- Use client certificates for administrative access
- Limit superuser access to localhost only

#### Secrets Management

CloudNativePG automatically creates and manages secrets:

```bash
# Superuser secret
kubectl get secret <cluster>-superuser -n <namespace>

# Application user secret
kubectl get secret <cluster>-app -n <namespace>
```

**For production:**
1. **Integrate with external secret managers** (HashiCorp Vault, AWS Secrets Manager)
2. **Use External Secrets Operator** to sync secrets from external sources
3. **Rotate credentials regularly** using the operator's credential rotation features

### Backup and Recovery

#### Continuous Backup Configuration

Enable continuous backup to object storage:

```yaml
cnpg:
  cluster:
    backup:
      barmanObjectStore:
        destinationPath: "s3://my-bucket/postgres-backups/"
        s3Credentials:
          accessKeyId:
            name: aws-credentials
            key: ACCESS_KEY_ID
          secretAccessKey:
            name: aws-credentials
            key: SECRET_ACCESS_KEY
        wal:
          compression: gzip
          encryption: AES256
      retentionPolicy: "30d"
```

**Supported object stores:**
- AWS S3
- Azure Blob Storage
- Google Cloud Storage
- MinIO
- S3-compatible stores

#### Backup Strategies

**Base Backups:**
```yaml
apiVersion: postgresql.cnpg.io/v1
kind: ScheduledBackup
metadata:
  name: daily-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  backupOwnerReference: self
  cluster:
    name: postgres
```

**Best practices:**
- **Backup from standby**: CloudNativePG takes backups from the most up-to-date replica by default
- **Schedule during low-traffic periods**: Typically 2-4 AM
- **Test restores regularly**: Verify backup integrity monthly
- **Retention policies**: Keep daily for 7 days, weekly for 4 weeks, monthly for 12 months

#### Point-in-Time Recovery (PITR)

With continuous backup enabled, you can restore to any point in time:

```yaml
apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: restored-cluster
spec:
  instances: 3

  bootstrap:
    recovery:
      source: postgres
      recoveryTarget:
        targetTime: "2025-12-07 14:30:00.00000+00"

  externalClusters:
    - name: postgres
      barmanObjectStore:
        destinationPath: "s3://my-bucket/postgres-backups/"
        s3Credentials:
          # ... credentials
```

### Monitoring and Observability

#### Prometheus Integration

Enable the built-in Prometheus exporter:

```yaml
cnpg:
  cluster:
    monitoring:
      enablePodMonitor: true
```

This creates a `PodMonitor` resource that Prometheus Operator discovers automatically.

#### Custom Metrics

Define custom metrics in SQL:

```yaml
cnpg:
  cluster:
    monitoring:
      customQueriesConfigMap:
        - name: custom-metrics
          key: queries.yaml
```

**Example custom metrics** (queries.yaml):
```yaml
queries:
  - name: "pg_database_size"
    query: "SELECT pg_database.datname, pg_database_size(pg_database.datname) as size FROM pg_database"
    metrics:
      - datname:
          usage: "LABEL"
          description: "Database name"
      - size:
          usage: "GAUGE"
          description: "Database size in bytes"
```

#### Logging

CloudNativePG sends logs to stdout in JSON format for Kubernetes-native log aggregation:

```yaml
cnpg:
  cluster:
    postgresql:
      parameters:
        log_destination: "stderr"
        logging_collector: "off"
        log_min_duration_statement: "1000"   # Log slow queries
        log_checkpoints: "on"
        log_connections: "on"
        log_disconnections: "on"
        log_lock_waits: "on"
```

**Integrate with:**
- Elasticsearch/Fluentd/Kibana (EFK)
- Grafana Loki
- CloudWatch Logs (AWS)
- Cloud Logging (GCP)

### Production Deployment Checklist

Before deploying to production, verify:

#### Infrastructure
- [ ] Kubernetes version 1.25 or higher
- [ ] CloudNativePG operator installed in dedicated namespace
- [ ] SSD-backed StorageClass available
- [ ] StorageClass supports volume expansion (`allowVolumeExpansion: true`)
- [ ] Sufficient nodes for pod anti-affinity (3+ nodes for 3 instances)

#### High Availability
- [ ] At least 3 instances configured (`instances: 3`)
- [ ] Synchronous replication configured (`minSyncReplicas: 1`)
- [ ] Pod anti-affinity enabled to spread instances across nodes
- [ ] Node affinity rules if using specific node pools

#### Storage
- [ ] PGDATA storage sized appropriately (data + 20% buffer)
- [ ] WAL storage separated for better performance
- [ ] Storage class matches performance requirements (SSD for production)
- [ ] Backup retention policy defined

#### Resources
- [ ] QoS class set to Guaranteed (requests = limits)
- [ ] Memory sized according to PostgreSQL parameters
- [ ] CPU requests appropriate for workload
- [ ] PostgreSQL parameters tuned for container resources

#### Security
- [ ] TLS enabled for all connections
- [ ] Client certificate authentication configured for admin access
- [ ] pg_hba rules reviewed and restricted
- [ ] Secrets management integrated (External Secrets, Vault, etc.)
- [ ] Network policies applied to restrict database access
- [ ] Superuser access limited to localhost

#### Backup & Recovery
- [ ] Continuous backup to object storage configured
- [ ] Scheduled backups configured (daily minimum)
- [ ] Backup credentials stored securely
- [ ] Retention policy matches compliance requirements
- [ ] Recovery tested in non-production environment
- [ ] PITR capability verified

#### Monitoring
- [ ] Prometheus PodMonitor enabled
- [ ] Grafana dashboards imported
- [ ] Alert rules configured (disk space, replication lag, connections)
- [ ] Log aggregation configured
- [ ] Custom metrics for application-specific monitoring

#### Configuration
- [ ] PostgreSQL parameters tuned for production
- [ ] Connection pooling configured (PgBouncer if needed)
- [ ] Maintenance windows defined for updates
- [ ] Extension requirements documented
- [ ] Database initialization scripts tested

#### Documentation
- [ ] Connection strings documented
- [ ] Backup/restore procedures documented
- [ ] Failover procedures documented
- [ ] Escalation contacts defined
- [ ] Monitoring runbook created

### References

- [CloudNativePG Documentation](https://cloudnative-pg.io/documentation/current/)
- [Resource Management](https://cloudnative-pg.io/documentation/current/resource_management/)
- [Storage Configuration](https://cloudnative-pg.io/documentation/current/storage/)
- [PostgreSQL Configuration](https://cloudnative-pg.io/documentation/current/postgresql_conf/)
- [Replication](https://cloudnative-pg.io/documentation/current/replication/)
- [Backup](https://cloudnative-pg.io/documentation/current/backup/)
- [Operator Capability Levels](https://cloudnative-pg.io/documentation/current/operator_capability_levels/)

## Prerequisites

- Kubernetes cluster (1.25+)
- Helm 3.x
- CloudNativePG operator installed in the cluster
- Custom PostgreSQL image available (`sojoner/database`)

## Installing CloudNativePG Operator

The operator must be installed before deploying this chart:

```bash
# Add CloudNativePG Helm repository
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update

# Install operator
helm install cnpg \
  --namespace cnpg-system \
  --create-namespace \
  cnpg/cloudnative-pg
```

## Chart Structure

```
.
├── Chart.yaml                      # Chart metadata and dependencies
├── values.yaml                     # Default values (production config)
├── values-dev.yaml                 # Development overrides
├── values-prod.yaml                # Production enhancements
├── .helmignore                     # Files to exclude from chart package
└── templates/
    ├── _helpers.tpl                # Template helper functions
    ├── clusterimagecatalog.yaml    # Custom image catalog resource
    └── NOTES.txt                   # Post-install instructions
```

## Installing the Chart

**Using Makefile** (recommended for local development):

The project includes a comprehensive Makefile that simplifies common operations. See the root [Makefile](../Makefile) and [CLAUDE.md](../.claude/CLAUDE.md) for details.

```bash
# Complete local setup with kind
make setup-all

# Or individual steps
make create-cluster        # Create kind cluster
make install-operator      # Install CloudNativePG operator
make deploy-db             # Deploy database
```

**Manual Helm Installation:**

### 1. Install Dependencies

```bash
cd k8s/
helm dependency update
```

This downloads the `cloudnative-pg/cluster` chart into the `charts/` directory.

### 2. Install for Development

Single instance with minimal resources:

```bash
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-dev.yaml
```

### 3. Install for Production

Three instances with high availability:

```bash
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-prod.yaml
```

### 4. Custom Installation

Override specific values:

```bash
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  --set image.tag=0.0.8 \
  --set cnpg.cluster.instances=5 \
  --set cnpg.cluster.storage.size=100Gi
```

## Configuration

### Key Configuration Values

| Parameter | Description | Default |
|-----------|-------------|---------|
| `image.registry` | Docker registry | `docker.io` |
| `image.repository` | Image repository | `sojoner/database` |
| `image.tag` | Image tag | `0.0.7` |
| `image.majorVersion` | PostgreSQL major version | `17` |
| `catalog.enabled` | Create ClusterImageCatalog | `true` |
| `catalog.name` | Catalog name | `sojoner-catalog` |
| `cnpg.cluster.name` | Cluster name | `postgres` |
| `cnpg.cluster.instances` | Number of instances | `3` |
| `cnpg.cluster.storage.size` | Storage size per instance | `10Gi` |
| `cnpg.cluster.resources.requests.memory` | Memory request | `2Gi` |
| `cnpg.cluster.resources.requests.cpu` | CPU request | `1000m` |
| `cnpg.cluster.minSyncReplicas` | Minimum sync replicas | `1` |
| `cnpg.cluster.monitoring.enablePodMonitor` | Enable Prometheus monitoring | `false` |

### PostgreSQL Configuration

The chart includes optimized PostgreSQL parameters for AI workloads:

```yaml
cnpg:
  cluster:
    postgresql:
      parameters:
        # Memory settings
        shared_buffers: "256MB"
        effective_cache_size: "1GB"
        work_mem: "16MB"

        # Vector search optimization
        max_parallel_workers: "4"
        max_parallel_workers_per_gather: "2"

        # Extensions
        shared_preload_libraries: "pg_stat_statements,pg_search,vector"
```

See `values.yaml` for the complete list of parameters.

## Upgrading the Chart

### Upgrade Image Version

```bash
helm upgrade irdb-postgres . \
  --set image.tag=0.0.8 \
  -n databases
```

### Scale the Cluster

```bash
helm upgrade irdb-postgres . \
  --set cnpg.cluster.instances=5 \
  -n databases
```

### Apply New Configuration

```bash
# Edit values.yaml or values-prod.yaml
helm upgrade irdb-postgres . \
  -f values-prod.yaml \
  -n databases
```

## Uninstalling the Chart

```bash
# Uninstall release (keeps PVCs)
helm uninstall irdb-postgres -n databases

# Delete PVCs if needed
kubectl delete pvc -n databases -l cnpg.io/cluster=postgres
```

## Environment-Specific Deployments

### Development (values-dev.yaml)

- Single instance
- Lower resource requests (512Mi memory, 500m CPU)
- Smaller storage (5Gi)
- No synchronous replication
- Verbose logging

### Production (values-prod.yaml)

- Three instances for high availability
- Higher resource limits (8Gi memory, 4 CPU)
- Larger storage (50Gi)
- Synchronous replication enabled
- Pod anti-affinity rules
- Prometheus monitoring enabled

### Creating Custom Environments

Create your own values file:

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
    postgresql:
      parameters:
        max_connections: "150"
```

Deploy with:

```bash
helm install irdb-postgres . \
  -f values-staging.yaml \
  -n databases
```

## Accessing the Database

The chart creates three service endpoints:

- `postgres-rw` - Primary instance (read-write)
- `postgres-ro` - Replica instances (read-only)
- `postgres-r` - Any instance (read)

### Get Password

```bash
kubectl get secret postgres-superuser -n databases \
  -o jsonpath='{.data.password}' | base64 -d
```

### Port-Forward

```bash
kubectl port-forward -n databases svc/postgres-rw 5432:5432
psql -h localhost -U postgres -d database
```

### From Within Cluster

Connect using service DNS names:

```bash
# Read-write connection
postgres-rw.databases.svc.cluster.local:5432

# Read-only connection
postgres-ro.databases.svc.cluster.local:5432
```

## Monitoring

### Enable Prometheus Monitoring

```bash
helm upgrade irdb-postgres . \
  --set cnpg.cluster.monitoring.enablePodMonitor=true \
  -n databases
```

This creates a `PodMonitor` resource that Prometheus Operator can discover.

### View Metrics

CloudNativePG exposes metrics on port 9187:

```bash
kubectl port-forward -n databases postgres-1 9187:9187
curl http://localhost:9187/metrics
```

## Troubleshooting

### View Release Status

```bash
helm status irdb-postgres -n databases
```

### View Rendered Templates

```bash
helm get manifest irdb-postgres -n databases
```

### Debug Installation

```bash
helm install irdb-postgres . \
  -f values-dev.yaml \
  -n databases \
  --dry-run --debug
```

### Check Cluster Status

```bash
kubectl get cluster -n databases
kubectl get pods -n databases
```

### View Logs

```bash
kubectl logs -n databases postgres-1 -f
```

## ArgoCD Integration

Deploy this chart via ArgoCD:

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

## Dependencies

This chart depends on:

- `cloudnative-pg/cluster` version `0.4.0`

Dependencies are managed in `Chart.yaml` and downloaded with `helm dependency update`.

## License

See parent project LICENSE file.

## Additional Documentation

- **Root Documentation**:
  - [Main README](../README.md) - Project overview
  - [CLAUDE.md](../.claude/CLAUDE.md) - Complete development guide with Makefile commands
  - [Docker Compose Guide](../README_DOCKER.md) - Local Docker deployment
  - [Kubernetes Guide](../README_K8s.md) - Detailed Kubernetes deployment guide

- **CloudNativePG Resources**:
  - [CloudNativePG Documentation](https://cloudnative-pg.io/documentation/)
  - [CloudNativePG GitHub](https://github.com/cloudnative-pg/cloudnative-pg)
  - [CloudNativePG Cluster Chart](https://github.com/cloudnative-pg/charts/tree/main/charts/cluster)
  - [Quickstart Guide](https://cloudnative-pg.io/documentation/current/quickstart/)

- **PostgreSQL Extensions**:
  - [pgvector GitHub](https://github.com/pgvector/pgvector)
  - [ParadeDB Documentation](https://docs.paradedb.com/)

- **Related Tools**:
  - [External Secrets Operator](https://external-secrets.io/)
  - [cert-manager](https://cert-manager.io/)
  - [Prometheus Operator](https://prometheus-operator.dev/)
