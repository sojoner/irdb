# Kubernetes Deployment for IR DB

This directory contains Kubernetes manifests for deploying the IR DB (AI-enhanced PostgreSQL with pgvector and ParadeDB) using **CloudNativePG**, the CNCF Sandbox PostgreSQL operator.

## Prerequisites

- [kind](https://kind.sigs.k8s.io/) - Kubernetes in Docker
- [kubectl](https://kubernetes.io/docs/tasks/tools/) - Kubernetes CLI
- [helm](https://helm.sh/) - Package manager for Kubernetes
- [kustomize](https://kustomize.io/) - Built into kubectl, but can be installed standalone
- Docker image `sojoner/database:17` pushed to registry (or available locally)

## Architecture

- **Operator**: CloudNativePG (CNCF Sandbox project)
- **Database**: PostgreSQL 17 with custom extensions (pgvector, ParadeDB pg_search)
- **Deployment**: Kustomize-based configuration with base + overlays
- **Storage**: Local persistent volumes via kind

## Directory Structure

```
k8s/
├── base/                    # Base Kustomize configuration
│   ├── namespace.yaml       # irdb namespace
│   ├── secret.yaml          # PostgreSQL credentials
│   ├── cluster.yaml         # CloudNativePG Cluster resource
│   ├── service.yaml         # NodePort service for external access
│   └── kustomization.yaml   # Base kustomization
├── overlays/
│   └── dev/                 # Development overlay
│       ├── cluster-patch.yaml   # Dev-specific settings (1 instance, lower resources)
│       └── kustomization.yaml
└── README.md
```

## Quick Start

### 1. Retag Docker Image

CloudNativePG requires the PostgreSQL version in the image tag:

```bash
# Pull the current image
docker pull sojoner/database:0.0.7

# Retag with PostgreSQL version
docker tag sojoner/database:0.0.7 sojoner/database:17

# Push the new tag
docker push sojoner/database:17
```

### 2. Create kind Cluster

```bash
# Create cluster from config
kind create cluster --config kind-config.yaml

# Verify cluster is running
kubectl cluster-info --context kind-irdb-cluster
```

### 3. Install CloudNativePG Operator

```bash
# Add the CloudNativePG Helm repository
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update

# Install the operator
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

### 4. Deploy IR DB

```bash
# Deploy using kustomize (development overlay)
kubectl apply -k k8s/overlays/dev/

# Watch the cluster come up
kubectl get pods -n irdb -w

# Check cluster status
kubectl get cluster -n irdb
```

### 5. Connect to Database

```bash
# Get the cluster status
kubectl cnpg status irdb-postgres -n irdb

# Port forward to access locally (alternative to NodePort)
kubectl port-forward -n irdb svc/irdb-postgres-rw 5432:5432

# Or use the NodePort (mapped to localhost:5432 via kind config)
psql -h localhost -U postgres -d database -p 5432
# Password: custom_secure_password_123
```

## Verifying Extensions

Once connected to the database:

```sql
-- Check installed extensions
SELECT extname, extversion FROM pg_extension;

-- Should see:
-- vector, pg_search, pg_stat_statements, pg_trgm, btree_gin

-- Verify the ai_data schema exists
\dn ai_data

-- Check the documents table
\d ai_data.documents

-- Test vector similarity
SELECT ai_data.generate_random_vector(1536);
```

## Managing the Cluster

### View Cluster Status
```bash
kubectl cnpg status irdb-postgres -n irdb
```

### View Logs
```bash
# Primary pod logs
kubectl logs -n irdb irdb-postgres-1 -f

# All pods
kubectl logs -n irdb -l cnpg.io/cluster=irdb-postgres --tail=100
```

### Scale the Cluster
```bash
# Edit the cluster
kubectl edit cluster irdb-postgres -n irdb

# Change spec.instances to desired number (e.g., 3 for HA)
```

### Backup and Restore

CloudNativePG supports Barman for backups. See the commented section in `cluster.yaml` for S3 backup configuration.

### Connection Pooling

To add PgBouncer connection pooling:

```bash
# Create a Pooler resource
kubectl apply -f - <<EOF
apiVersion: postgresql.cnpg.io/v1
kind: Pooler
metadata:
  name: irdb-postgres-pooler
  namespace: irdb
spec:
  cluster:
    name: irdb-postgres
  instances: 3
  type: rw
  pgbouncer:
    poolMode: session
    parameters:
      max_client_conn: "1000"
      default_pool_size: "25"
EOF
```

## Accessing from Outside the Cluster

### Option 1: NodePort (Already Configured)
The service is exposed on port 30432, mapped to localhost:5432 via kind configuration.

```bash
psql -h localhost -U postgres -d database -p 5432
```

### Option 2: Port Forward
```bash
kubectl port-forward -n irdb svc/irdb-postgres-rw 5432:5432
```

### Option 3: LoadBalancer (For cloud deployments)
Change the service type in `service.yaml` from `NodePort` to `LoadBalancer`.

## Cleanup

```bash
# Delete the IR DB deployment
kubectl delete -k k8s/overlays/dev/

# Delete the operator (optional)
helm uninstall cnpg -n cnpg-system

# Delete the kind cluster
kind delete cluster --name irdb-cluster
```

## Production Considerations

For production deployments:

1. **Use the base configuration** (3 instances for HA)
2. **Enable monitoring** (set `monitoring.enablePodMonitor: true`)
3. **Configure backups** (uncomment and configure S3 backup settings)
4. **Use proper secrets management** (e.g., Sealed Secrets, External Secrets Operator)
5. **Set resource limits** based on workload
6. **Use a proper storage class** with good I/O performance
7. **Enable TLS** for database connections
8. **Configure PgBouncer** for connection pooling
9. **Set up monitoring** with Prometheus/Grafana

## Troubleshooting

### Pods not starting
```bash
# Check events
kubectl get events -n irdb --sort-by='.lastTimestamp'

# Check pod logs
kubectl logs -n irdb irdb-postgres-1

# Check operator logs
kubectl logs -n cnpg-system -l app.kubernetes.io/name=cloudnative-pg
```

### Image pull errors
```bash
# If using a private registry, create an image pull secret
kubectl create secret docker-registry regcred \
  --docker-server=<your-registry> \
  --docker-username=<username> \
  --docker-password=<password> \
  -n irdb

# Add to cluster.yaml:
spec:
  imagePullSecrets:
    - name: regcred
```

### Extensions not loading
The initialization scripts from the Docker image should run automatically. If not:

```bash
# Exec into the pod
kubectl exec -it -n irdb irdb-postgres-1 -- psql -U postgres -d database

# Manually run the setup
\i /docker-entrypoint-initdb.d/00-extensions.sql
\i /docker-entrypoint-initdb.d/01-ai-extensions.sql
```

## Resources

- [CloudNativePG Documentation](https://cloudnative-pg.io/documentation/)
- [CloudNativePG GitHub](https://github.com/cloudnative-pg/cloudnative-pg)
- [PostgreSQL Operator Comparison](https://blog.palark.com/comparing-kubernetes-operators-for-postgresql/)
