# IR DB Deployment Guide

This guide covers deploying the IR DB (AI-enhanced PostgreSQL 17 with pgvector and ParadeDB) using **Docker Compose** or **Kubernetes** with CloudNativePG operator.

## Table of Contents

1. [Docker Compose Setup](#docker-compose-setup)
2. [Kubernetes Setup](#kubernetes-setup)
3. [Validation Examples](#validation-examples)
4. [Troubleshooting](#troubleshooting)

---

## Docker Compose Setup

Docker Compose provides the simplest way to run IR DB locally with PostgreSQL + pgAdmin.

### Prerequisites

- Docker and Docker Compose installed
- 8GB+ RAM available
- 20GB+ disk space

### Step-by-Step Guide

#### Step 1: Build the Docker Image

```bash
# Navigate to project root
cd irdb

# Build the custom PostgreSQL image
docker build -t sojoner/database:0.0.7 .

# This takes 10-15 minutes due to compiling ParadeDB extension
# Watch for "Successfully tagged sojoner/database:0.0.7"
```

**Verification:**
```bash
docker images | grep sojoner/database
# Should show: sojoner/database   0.0.7   <IMAGE_ID>   <TIME>   <SIZE>
```

#### Step 2: Start the Services

```bash
# Start PostgreSQL and pgAdmin in detached mode
docker-compose up -d

# Wait for services to be ready (30-60 seconds)
docker-compose ps
```

**Verification:**
```bash
# Both services should show "Up" status
docker-compose ps

# Check logs for successful startup
docker-compose logs postgres | grep "database system is ready to accept connections"
```

#### Step 3: Verify Extensions are Loaded

```bash
# Connect to PostgreSQL
psql -h localhost -U postgres -d database -p 5432
# Password: custom_secure_password_123

# Once connected, run:
SELECT extname, extversion FROM pg_extension;
```

**Expected output:**
```
    extname      | extversion
-----------------+------------
 plpgsql         | 1.0
 pg_search       | 0.17.2
 vector          | 0.8.0
 pg_stat_statements | 1.10
 pg_trgm         | 1.6
 btree_gin       | 1.3
```

#### Step 4: Verify AI Schema and Tables

```sql
-- List schemas
\dn

-- Should show ai_data schema
-- List tables in ai_data schema
\dt ai_data.*

-- Should show:
-- ai_data.documents
-- ai_data.chunks
```

#### Step 5: Access pgAdmin (Optional)

1. Open browser to `http://localhost:5433`
2. Login with:
   - Email: `admin@database.com`
   - Password: `custom_secure_password_123`
3. The "Database Server" connection is pre-configured
4. Expand: Servers → Database Server → Databases → database

### Common Docker Compose Commands

```bash
# View logs
docker-compose logs -f

# Stop services (keeps data)
docker-compose down

# Stop and remove all data
docker-compose down -v

# Restart services
docker-compose restart

# Rebuild and restart (after code changes)
docker-compose down -v
docker-compose up -d --build
```

### Testing Changes to Initialization Scripts

When you modify SQL scripts in `docker-entrypoint-initdb.d/`:

```bash
# Must remove volumes to trigger re-initialization
docker-compose down -v

# Rebuild and start
docker-compose up -d --build

# Verify scripts ran successfully
docker-compose logs postgres | grep "docker-entrypoint-initdb.d"
```

---

## Kubernetes Setup

Deploy IR DB on Kubernetes using kind (local) with CloudNativePG operator.

### Prerequisites

Install required tools:

```bash
# Check what's installed
make check-prereqs

# Install missing tools
make install-kind
make install-kubectl
make install-helm
```

Or install manually:
- [kind](https://kind.sigs.k8s.io/docs/user/quick-start/) - Kubernetes in Docker
- [kubectl](https://kubernetes.io/docs/tasks/tools/) - Kubernetes CLI
- [helm](https://helm.sh/docs/intro/install/) - Package manager

### Step-by-Step Guide (Using Makefile)

#### Option A: Automated Setup (Recommended)

Run the complete setup with a single command:

```bash
make setup-all
```

This will:
1. Create kind cluster
2. Install CloudNativePG operator
3. Retag and load Docker image
4. Deploy IR DB
5. Verify everything is ready

**Verification:**
```bash
make test-all
```

#### Option B: Manual Step-by-Step Setup

**Step 1: Prepare the Docker Image**

```bash
# Build the image (if not already built)
make compose-build

# Retag with PostgreSQL version (required by CloudNativePG)
make retag-image

# Verify the image
make verify-image
```

**Expected output:**
```
✓ Image found
REPOSITORY          TAG       IMAGE ID       CREATED        SIZE
sojoner/database    17        abc123def456   5 minutes ago  850MB
```

**Step 2: Create kind Cluster**

```bash
# Create cluster
make create-cluster

# Verify cluster is ready
make verify-cluster
```

**Expected output:**
```
✓ Cluster is ready
NAME                STATUS   ROLES           AGE   VERSION
irdb-cluster-control-plane   Ready    control-plane   1m    v1.27.0
irdb-cluster-worker          Ready    <none>          1m    v1.27.0
irdb-cluster-worker2         Ready    <none>          1m    v1.27.0
```

**Step 3: Add CloudNativePG Helm Repository**

```bash
# Add the repository
make add-helm-repo

# Verify repository
make verify-helm-repo
```

**Expected output:**
```
✓ Helm repository verified
NAME                    CHART VERSION   APP VERSION     DESCRIPTION
cnpg/cloudnative-pg     0.21.2          1.23.1          CloudNativePG Helm Chart
```

**Step 4: Install CloudNativePG Operator**

```bash
# Install operator
make install-operator

# Wait for operator to be ready (this may take 1-2 minutes)
make verify-operator
```

**Expected output:**
```
✓ Operator is ready
NAME                  READY   UP-TO-DATE   AVAILABLE   AGE
cnpg-cloudnative-pg   1/1     1            1           2m
```

**Step 5: Load Docker Image into kind**

```bash
# Load image into cluster
make load-image
```

**Expected output:**
```
✓ Image loaded into cluster
```

**Step 6: Deploy IR DB**

```bash
# Deploy database
make deploy-db

# Verify database is ready (this may take 2-3 minutes)
make verify-db
```

**Expected output:**
```
✓ Database is ready
NAME            AGE   INSTANCES   READY   STATUS                     PRIMARY
irdb-postgres   2m    1           1       Cluster in healthy state   irdb-postgres-1

NAME              READY   STATUS    RESTARTS   AGE
irdb-postgres-1   1/1     Running   0          2m
```

**Step 7: Validate Extensions and Functionality**

```bash
# Run all validation tests
make validate-all
```

This runs:
- Extension verification
- BM25 search test
- Vector search test
- Hybrid search test

### Step-by-Step Guide (Manual Commands)

If you prefer not to use the Makefile:

#### Step 1: Create kind Cluster

```bash
kind create cluster --config kind-config.yaml
kubectl cluster-info --context kind-irdb-cluster
kubectl wait --for=condition=Ready nodes --all --timeout=300s
```

#### Step 2: Install CloudNativePG Operator

```bash
# Add Helm repository
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update

# Install operator
helm install cnpg \
  --namespace cnpg-system \
  --create-namespace \
  cnpg/cloudnative-pg

# Wait for operator
kubectl wait --for=condition=Available \
  --timeout=300s \
  -n cnpg-system \
  deployment/cnpg-cloudnative-pg
```

#### Step 3: Prepare and Load Image

```bash
# Retag image
docker tag sojoner/database:0.0.7 sojoner/database:17

# Load into kind
kind load docker-image sojoner/database:17 --name irdb-cluster
```

#### Step 4: Deploy Database

```bash
# Deploy using kustomize
kubectl apply -k k8s/overlays/dev/

# Watch pods starting
kubectl get pods -n irdb -w

# Wait for ready
kubectl wait --for=condition=Ready \
  --timeout=600s \
  -n irdb \
  pod -l cnpg.io/cluster=irdb-postgres
```

#### Step 5: Verify Deployment

```bash
# Run verification script
./k8s/verify-extensions.sh
```

### Accessing the Database

#### Option 1: NodePort (via kind port mapping)

The kind cluster is configured to expose PostgreSQL on `localhost:5432`:

```bash
psql -h localhost -U postgres -d database -p 5432
# Password: custom_secure_password_123
```

#### Option 2: Port-Forward

```bash
# In one terminal, set up port-forward
make port-forward

# In another terminal, connect
make connect
```

Or manually:
```bash
# Terminal 1
kubectl port-forward -n irdb svc/irdb-postgres-rw 5432:5432

# Terminal 2
psql -h localhost -U postgres -d database -p 5432
```

### Viewing Logs

```bash
# Using Makefile
make logs

# Or manually
kubectl logs -n irdb -l cnpg.io/cluster=irdb-postgres --tail=100 -f
```

### Checking Status

```bash
# Using Makefile
make status

# Or manually
kubectl get cluster -n irdb
kubectl get pods -n irdb
```

### Scaling the Cluster

Edit the cluster to add more replicas:

```bash
kubectl edit cluster irdb-postgres -n irdb

# Change spec.instances from 1 to 3
# Save and exit
```

Or update the kustomize overlay and reapply:

```bash
# Edit k8s/overlays/dev/cluster-patch.yaml
# Change instances: 1 to instances: 3

kubectl apply -k k8s/overlays/dev/
```

### Cleanup

```bash
# Remove database only
make clean-db

# Remove operator only
make clean-operator

# Remove cluster only
make clean-cluster

# Remove everything
make clean-all
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
kubectl get events -n irdb --sort-by='.lastTimestamp'

# Check pod logs
kubectl logs -n irdb irdb-postgres-1

# Check operator logs
kubectl logs -n cnpg-system -l app.kubernetes.io/name=cloudnative-pg
```

**Problem: Image pull errors**
```bash
# If image is not in registry, must load into kind
kind load docker-image sojoner/database:17 --name irdb-cluster

# Verify image is loaded
docker exec -it irdb-cluster-control-plane crictl images | grep sojoner
```

**Problem: Database pods crash looping**
```bash
# Check pod status
kubectl describe pod -n irdb irdb-postgres-1

# Common issues:
# 1. Insufficient resources - reduce resource requests in cluster-patch.yaml
# 2. Image tag mismatch - must use version tag (17), not latest
# 3. Init script errors - check logs for SQL errors
```

**Problem: Can't connect to database**
```bash
# Verify service exists
kubectl get svc -n irdb

# Check if port-forward works
kubectl port-forward -n irdb svc/irdb-postgres-rw 5432:5432

# For NodePort, verify kind cluster port mapping
docker ps | grep irdb-cluster

# Should show: 0.0.0.0:5432->30432/tcp
```

**Problem: Extensions not found after deployment**
```bash
# Check if initialization scripts are in the image
kubectl exec -n irdb irdb-postgres-1 -- ls -la /docker-entrypoint-initdb.d/

# If missing, image wasn't built correctly
# Rebuild image:
make compose-build
make retag-image
make load-image

# Redeploy database
make undeploy-db
make deploy-db
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
| Kind cluster config | `/kind-config.yaml` |
| Kustomize base | `/k8s/base/` |
| Dev overlay | `/k8s/overlays/dev/` |

### Useful Commands

```bash
# Makefile (Kubernetes)
make help              # Show all available commands
make setup-all         # Complete setup
make test-all          # Run all tests
make clean-all         # Remove everything

# Docker Compose
docker-compose up -d   # Start services
docker-compose down -v # Stop and clean
docker-compose logs -f # View logs

# Kubernetes
kubectl get all -n irdb                    # Show all resources
kubectl logs -n irdb <pod-name> -f        # Follow logs
kubectl exec -n irdb <pod-name> -- psql -U postgres -d database  # Connect to DB
```
