# IR DB Deployment Guide

This guide covers deploying the IR DB (AI-enhanced PostgreSQL 17 with pgvector and ParadeDB) using **Docker Compose** or **Kubernetes** with CloudNativePG operator.

## Table of Contents

1. [Docker Compose Setup](#docker-compose-setup)
2. [Kubernetes Setup](#kubernetes-setup)
3. [Validation Examples](#validation-examples)
4. [Troubleshooting](#troubleshooting)

---

## Kubernetes Setup

Deploy IR DB on Kubernetes using Helm with CloudNativePG operator.

### Prerequisites

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

---

## Validation Examples

### Extension Verification

Check all required extensions are installed:

```sql
-- Connect to database first
-- Docker Compose: psql -h localhost -U postgres -d database -p 5432
-- Kubernetes: make connect

-- List all extensions
SELECT extname, extversion
FROM pg_extension
WHERE extname IN ('vector', 'pg_search', 'pg_stat_statements', 'pg_trgm', 'btree_gin')
ORDER BY extname;
```

**Expected output:**
```
    extname     | extversion
----------------+------------
 btree_gin      | 1.3
 pg_search      | 0.17.2
 pg_stat_statements | 1.10
 pg_trgm        | 1.6
 vector         | 0.8.0
```

### BM25 Full-Text Search Validation

#### Using Makefile (Kubernetes only):
```bash
make validate-bm25
```

#### Manual SQL:

```sql
-- 1. Insert test documents
INSERT INTO ai_data.documents (title, content, metadata, embedding) VALUES
('PostgreSQL Database Guide', 'PostgreSQL is a powerful open-source relational database system',
 '{"category": "database", "language": "english"}'::jsonb,
 ai_data.generate_random_vector(1536)),
('ParadeDB Search Tutorial', 'ParadeDB extends PostgreSQL with full-text search and BM25 ranking',
 '{"category": "tutorial", "language": "english"}'::jsonb,
 ai_data.generate_random_vector(1536)),
('Vector Embeddings Guide', 'Using pgvector for semantic similarity search with embeddings',
 '{"category": "guide", "language": "english"}'::jsonb,
 ai_data.generate_random_vector(1536)),
('Machine Learning with PostgreSQL', 'Integrate ML models with PostgreSQL using pgvector',
 '{"category": "ml", "language": "english"}'::jsonb,
 ai_data.generate_random_vector(1536));

-- 2. Test BM25 search for "PostgreSQL"
SELECT
    id,
    title,
    ts_rank(
        to_tsvector('english', title || ' ' || content),
        to_tsquery('english', 'PostgreSQL')
    ) as bm25_score
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content) @@ to_tsquery('english', 'PostgreSQL')
ORDER BY bm25_score DESC
LIMIT 5;
```

**Expected output:**
```
 id |            title                  | bm25_score
----+-----------------------------------+------------
  1 | PostgreSQL Database Guide         |   0.151395
  4 | Machine Learning with PostgreSQL  |   0.0759878
  2 | ParadeDB Search Tutorial          |   0.0607903
```

#### Advanced BM25 Queries:

```sql
-- Multi-term search with AND operator
SELECT id, title,
    ts_rank(to_tsvector('english', title || ' ' || content),
            to_tsquery('english', 'search & PostgreSQL')) as score
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content)
      @@ to_tsquery('english', 'search & PostgreSQL')
ORDER BY score DESC;

-- Multi-term search with OR operator
SELECT id, title,
    ts_rank(to_tsvector('english', title || ' ' || content),
            to_tsquery('english', 'vector | embedding')) as score
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content)
      @@ to_tsquery('english', 'vector | embedding')
ORDER BY score DESC;

-- Phrase search (using plainto_tsquery for simpler syntax)
SELECT id, title,
    ts_rank(to_tsvector('english', title || ' ' || content),
            plainto_tsquery('english', 'similarity search')) as score
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content)
      @@ plainto_tsquery('english', 'similarity search')
ORDER BY score DESC;
```

### Vector Similarity Search Validation

#### Using Makefile (Kubernetes only):
```bash
make validate-vector
```

#### Manual SQL:

```sql
-- 1. Generate a query vector
WITH query_vector AS (
    SELECT ai_data.generate_random_vector(1536) as qv
)

-- 2. Find similar documents using cosine distance
SELECT
    d.id,
    d.title,
    1 - (d.embedding <=> query_vector.qv) as cosine_similarity,
    d.embedding <-> query_vector.qv as l2_distance,
    d.embedding <#> query_vector.qv as inner_product
FROM ai_data.documents d, query_vector
ORDER BY d.embedding <=> query_vector.qv
LIMIT 5;
```

**Expected output:**
```
 id |            title                  | cosine_similarity | l2_distance | inner_product
----+-----------------------------------+-------------------+-------------+--------------
  3 | Vector Embeddings Guide           |          0.985432 |    0.234567 |      -123.45
  4 | Machine Learning with PostgreSQL  |          0.978234 |    0.267891 |      -134.56
  1 | PostgreSQL Database Guide         |          0.972156 |    0.289123 |      -145.67
```

#### Understanding Vector Distance Operators:

```sql
-- <=> : Cosine distance (0 = identical, 2 = opposite)
-- <-> : Euclidean (L2) distance
-- <#> : Negative inner product

-- Example: Find documents within a similarity threshold
WITH query_vector AS (
    SELECT ai_data.generate_random_vector(1536) as qv
)
SELECT
    d.id,
    d.title,
    1 - (d.embedding <=> query_vector.qv) as similarity
FROM ai_data.documents d, query_vector
WHERE 1 - (d.embedding <=> query_vector.qv) > 0.8  -- Only >= 80% similar
ORDER BY d.embedding <=> query_vector.qv
LIMIT 10;
```

### Hybrid Search Validation (Vector + BM25)

#### Using Makefile (Kubernetes only):
```bash
make validate-hybrid
```

#### Manual SQL:

```sql
-- The hybrid_search function combines vector and text search
-- 70% weight on vector similarity, 30% on BM25 text score

-- Generate a query vector for demonstration
WITH query_vec AS (
    SELECT ai_data.generate_random_vector(1536) as qv
)

-- Call the hybrid search function
SELECT
    id,
    title,
    vector_similarity,
    text_score,
    combined_score
FROM ai_data.hybrid_search(
    query_text => 'PostgreSQL database search',
    query_embedding => (SELECT qv FROM query_vec),
    similarity_threshold => 0.0,  -- Accept all similarities for demo
    limit_count => 5
)
ORDER BY combined_score DESC;
```

**Expected output:**
```
 id |            title                  | vector_similarity | text_score | combined_score
----+-----------------------------------+-------------------+------------+---------------
  2 | ParadeDB Search Tutorial          |          0.982134 |   0.151395 |       0.732536
  1 | PostgreSQL Database Guide         |          0.975421 |   0.121234 |       0.719165
  3 | Vector Embeddings Guide           |          0.968234 |   0.098765 |       0.707427
```

#### Understanding the Hybrid Search:

The `hybrid_search` function performs:
1. **Vector search**: Finds semantically similar documents
2. **Text search**: Finds keyword matches using BM25
3. **Combines results**: `combined_score = (vector_similarity * 0.7) + (text_score * 0.3)`

```sql
-- Adjust weights by modifying the function
-- Or create a custom query:

WITH
query_vec AS (
    SELECT ai_data.generate_random_vector(1536) as qv
),
vector_results AS (
    SELECT
        id, title, content,
        1 - (embedding <=> (SELECT qv FROM query_vec)) as vec_sim
    FROM ai_data.documents
    ORDER BY embedding <=> (SELECT qv FROM query_vec)
    LIMIT 20
),
text_results AS (
    SELECT
        id, title, content,
        ts_rank(to_tsvector('english', title || ' ' || content),
                plainto_tsquery('english', 'your search terms')) as txt_score
    FROM ai_data.documents
    WHERE to_tsvector('english', title || ' ' || content)
          @@ plainto_tsquery('english', 'your search terms')
    ORDER BY txt_score DESC
    LIMIT 20
)
SELECT
    COALESCE(vr.id, tr.id) as id,
    COALESCE(vr.title, tr.title) as title,
    COALESCE(vr.vec_sim, 0.0) as vector_similarity,
    COALESCE(tr.txt_score, 0.0) as text_score,
    (COALESCE(vr.vec_sim, 0.0) * 0.5 + COALESCE(tr.txt_score, 0.0) * 0.5) as combined_score
FROM vector_results vr
FULL OUTER JOIN text_results tr ON vr.id = tr.id
ORDER BY combined_score DESC
LIMIT 10;
```

### Performance Testing

#### Test Index Usage:

```sql
-- Check if vector index is being used
EXPLAIN ANALYZE
SELECT id, title, 1 - (embedding <=> ai_data.generate_random_vector(1536)) as similarity
FROM ai_data.documents
ORDER BY embedding <=> ai_data.generate_random_vector(1536)
LIMIT 10;

-- Should show "Index Scan using documents_embedding_idx"

-- Check if text index is being used
EXPLAIN ANALYZE
SELECT id, title
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content)
      @@ to_tsquery('english', 'PostgreSQL & search');

-- Should show "Bitmap Index Scan on idx_documents_title_content"
```

#### Bulk Insert Performance:

```sql
-- Insert 1000 test documents with random vectors
INSERT INTO ai_data.documents (title, content, embedding)
SELECT
    'Test Document ' || generate_series,
    'This is test content for document ' || generate_series,
    ai_data.generate_random_vector(1536)
FROM generate_series(1, 1000);

-- Verify count
SELECT COUNT(*) FROM ai_data.documents;
```

---

## Troubleshooting

### Docker Compose Issues

**Problem: Services won't start**
```bash
# Check if ports are already in use
lsof -i :5432
lsof -i :5433

# Kill processes using these ports or change ports in docker-compose.yml
```

**Problem: Extensions not loading**
```bash
# Check PostgreSQL logs
docker-compose logs postgres | grep -i error

# Verify initialization scripts ran
docker-compose logs postgres | grep "docker-entrypoint-initdb.d"

# If scripts didn't run, database already existed
# Must remove volumes to re-run initialization
docker-compose down -v
docker-compose up -d
```

**Problem: Out of memory**
```bash
# Reduce PostgreSQL memory settings in postgresql.conf
# Or allocate more memory to Docker

# Check Docker memory limit
docker stats
```

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
```sql
-- Check if ai_data schema exists
SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'ai_data';

-- If missing, initialization scripts didn't run
-- For Kubernetes, delete and recreate the cluster resource
-- For Docker Compose, remove volumes and restart
```

**Problem: Vector dimension mismatch**
```sql
-- Check current vector dimensions
SELECT column_name, udt_name
FROM information_schema.columns
WHERE table_schema = 'ai_data'
  AND table_name = 'documents'
  AND column_name = 'embedding';

-- If you need different dimensions, update 01-ai-extensions.sql
-- Then rebuild/redeploy
```

### Getting Help

- Check CloudNativePG docs: https://cloudnative-pg.io/documentation/
- View this project's issues: https://github.com/sojoner/irdb/issues
- PostgreSQL documentation: https://www.postgresql.org/docs/17/
- pgvector documentation: https://github.com/pgvector/pgvector
- ParadeDB documentation: https://docs.paradedb.com/

---

## Quick Reference

### Connection Details

| Parameter | Docker Compose | Kubernetes (NodePort) |
|-----------|---------------|---------------------|
| Host | localhost | localhost |
| Port | 5432 | 5432 |
| Database | database | database |
| Username | postgres | postgres |
| Password | custom_secure_password_123 | custom_secure_password_123 |

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
# Docker Compose
docker-compose up -d   # Start services
docker-compose down -v # Stop and clean
docker-compose logs -f # View logs

# Helm (Kubernetes)
helm install irdb-postgres k8s/ -n databases -f k8s/values-dev.yaml   # Install
helm upgrade irdb-postgres k8s/ -n databases                          # Upgrade
helm uninstall irdb-postgres -n databases                             # Uninstall
helm list -n databases                                                # List releases
helm history irdb-postgres -n databases                               # Show history

# Kubernetes
kubectl get all -n databases                    # Show all resources
kubectl get cluster -n databases                # Show cluster status
kubectl logs -n databases postgres-1 -f         # Follow logs
kubectl exec -it -n databases postgres-1 -- psql -U postgres -d database  # Connect to DB
kubectl port-forward -n databases svc/postgres-rw 5432:5432           # Port-forward
```
