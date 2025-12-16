# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IR DB is an AI-enhanced PostgreSQL 17.5 platform with RAG (Retrieval Augmented Generation) capabilities. It combines:
- **pgvector (v0.8.0)** - Vector similarity search with 1536-dimension embeddings
- **ParadeDB pg_search (v0.20.x)** - Full-text search with BM25 ranking
- **PostgreSQL 17.5** - Latest stable PostgreSQL with custom optimizations

**Key Features:**
- Hybrid search combining vector similarity (70%) and text search (30%)
- Pre-configured schema, tables, and indexes for RAG workflows
- Multi-stage Docker build for minimal image size (~850MB)
- Production-ready Kubernetes Helm chart with CloudNativePG

**Default Credentials:**
- Database: `database`
- User: `postgres`
- Password: `custom_secure_password_123`
- Port: `5432`

## Quick Command Reference

| Task | Command |
|------|---------|
| **Docker Compose** |
| Build image | `make compose-build` |
| Start services | `make compose-up` |
| Stop services | `make compose-down` |
| Clean rebuild | `make compose-clean && make compose-build && make compose-up` |
| **Kubernetes (Local)** |
| Complete setup | `make setup-all` |
| Run all tests | `make test-all` |
| Connect to DB | `make connect` (requires port-forward) |
| View status | `make status` |
| Clean everything | `make clean-all` |
| **Validation** |
| Test BM25 search | `make validate-bm25` |
| Test vector search | `make validate-vector` |
| Test hybrid search | `make validate-hybrid` |
| All validations | `make validate-all` |

## Build & Development Commands

### Makefile Targets

The project includes a comprehensive Makefile for common operations. Run `make help` to see all available targets organized by category.

**Prerequisites:**
```bash
make check-prereqs           # Verify required tools (docker, kind, kubectl, helm)
make install-kind            # Install kind (Kubernetes in Docker)
make install-kubectl         # Install kubectl
make install-helm            # Install Helm
```

**Docker Compose Workflow:**
```bash
make compose-build           # Build Docker image
make compose-up              # Start PostgreSQL + pgAdmin
make compose-down            # Stop services
make compose-clean           # Stop and remove volumes
make compose-logs            # View logs
```

**Kubernetes Workflow (Local with kind):**
```bash
make setup-all               # Complete setup: create cluster, install operator, deploy DB
make test-all                # Run all validation tests
make validate-bm25           # Test BM25 full-text search
make validate-vector         # Test vector similarity search
make validate-hybrid         # Test hybrid search (vector + BM25)
make connect                 # Connect via psql (requires port-forward running)
make port-forward            # Setup port-forward to database
make logs                    # View database logs
make status                  # Show cluster and pod status
make clean-all               # Remove everything (cluster, operator, database)
```

**Advanced Kubernetes Operations:**
```bash
make create-cluster          # Create kind cluster only
make install-operator        # Install CloudNativePG operator only
make deploy-db               # Deploy database only
make retag-image             # Retag image with PostgreSQL version
make load-image              # Load image into kind cluster
```

### Manual Docker Commands

If not using the Makefile:

```bash
# Build the image (uses multi-stage build)
docker build -t sojoner/database:0.0.7 .

# Push to registry (optional)
docker push sojoner/database:0.0.7

# Start PostgreSQL + pgAdmin
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Rebuild and restart
docker-compose up -d --build
```

### Database Access

**Via psql:**
```bash
# Docker Compose
psql -h localhost -U postgres -d database -p 5432

# Kubernetes (after port-forward)
make port-forward    # In one terminal
make connect         # In another terminal
```

**Via pgAdmin (Docker Compose only):**
- URL: http://localhost:5433
- Email: admin@database.com
- Password: custom_secure_password_123

### Testing Changes

When modifying initialization scripts in `docker-entrypoint-initdb.d/`:

**Docker Compose:**
```bash
# Completely rebuild to test init scripts
docker-compose down -v  # Remove volumes to trigger re-initialization
docker-compose up -d --build

# Or use Makefile
make compose-clean
make compose-build
make compose-up
```

**Kubernetes:**
```bash
# Rebuild image and redeploy
docker build -t sojoner/database:0.0.7 .
make retag-image
make load-image
make undeploy-db
make deploy-db

# Or clean everything and start fresh
make clean-all
make setup-all
```

The initialization scripts run in alphabetical order:
1. `00-extensions.sql` - Core extensions
2. `01-ai-extensions.sql` - Schema, tables, functions
3. `02-validating-bm25.sql` - BM25 validation
4. `03-simple-vector-test.sql` - Vector search test
5. `05-comprehensive-test.sql` - Full test suite

## Architecture

### Multi-Stage Docker Build

The Dockerfile uses a two-stage build pattern:

1. **Builder Stage** (`postgres:17.5-bookworm AS builder`)
   - Installs Rust toolchain
   - Compiles ParadeDB pg_search extension from source (v0.20.x)
   - Uses cargo-pgrx (v0.15.0) for PostgreSQL extension building
   - Only pg_search is built (not pg_analytics)

2. **Runtime Stage** (`postgres:17.5-bookworm AS runtime`)
   - Minimal PostgreSQL 17 image
   - Copies only compiled extension files from builder
   - Installs pgvector from apt package
   - Copies custom `postgresql.conf` and initialization scripts

This approach keeps the final image small while supporting extensions that require compilation.

### Database Schema Structure

**Schema:** `ai_data`

**Core Tables:**
- `documents` - Main document storage with embeddings
  - `embedding vector(1536)` - OpenAI ada-002 dimension vectors
  - HNSW index for fast cosine similarity search
  - GIN index for full-text search on title + content

- `chunks` - Document chunks for RAG workflows
  - References parent document via `document_id`
  - Separate embeddings per chunk
  - HNSW index for vector search

**Key Functions:**
- `hybrid_search(query_text, query_embedding, similarity_threshold, limit_count)` - Combines vector similarity (70% weight) with text search (30% weight) using FULL OUTER JOIN
- `generate_random_vector(dimensions)` - Helper for generating test vectors

### Configuration Notes

The `postgresql.conf` is optimized for AI workloads:
- Uses `shared_preload_libraries` including pg_search and vectors
- Parallel workers configured for vector operations
- Memory settings tuned for vector similarity operations

Note: The shared_preload_libraries line in postgresql.conf references some extensions that may not be installed (pg_analytics, pg_cron, vectors, auto_explain). The system will start despite warnings about missing libraries.

### Service Configuration

**PostgreSQL Container:**
- Resource limits: 8 CPUs max, 4 CPUs reserved
- Memory limits: 32GB max, 16GB reserved
- 2GB shared memory for PostgreSQL operations
- Port 5432 exposed

**pgAdmin Container:**
- Pre-configured server connection (passwordless via pgpass file)
- Server definition embedded in docker-compose entrypoint
- Port 5433 (maps to internal port 80)

## Common Development Workflows

### Adding New Extensions
1. Add installation to builder stage in Dockerfile if compilation needed
2. Add extension creation to `00-extensions.sql` or `01-ai-extensions.sql`
3. Update `shared_preload_libraries` in `postgresql.conf` if required
4. Rebuild: `docker-compose down -v && docker-compose up -d --build`

### Modifying Database Schema
1. Edit `01-ai-extensions.sql` for schema/table changes
2. For existing deployments, create a new migration script (e.g., `06-migration.sql`)
3. Test with clean rebuild: `docker-compose down -v && docker-compose up -d --build`

### Testing Search Functionality

Vector search example:
```sql
SELECT id, title,
  1 - (embedding <=> '[0.1, 0.2, ...]'::vector(1536)) as similarity
FROM ai_data.documents
ORDER BY embedding <=> '[0.1, 0.2, ...]'::vector(1536)
LIMIT 10;
```

Text search example:
```sql
SELECT id, title,
  ts_rank(to_tsvector('english', title || ' ' || content),
          to_tsquery('english', 'search & terms')) as score
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content) @@
      to_tsquery('english', 'search & terms')
ORDER BY score DESC;
```

Hybrid search:
```sql
SELECT * FROM ai_data.hybrid_search(
  query_text => 'search terms',
  query_embedding => '[0.1, 0.2, ...]'::vector(1536),
  similarity_threshold => 0.5,
  limit_count => 10
);
```

## Kubernetes Deployment

The project includes a Helm chart for deploying IR DB using **CloudNativePG** operator.

### Quick Start

```bash
# Install CloudNativePG operator (once per cluster)
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update
helm install cnpg --namespace cnpg-system --create-namespace cnpg/cloudnative-pg

# Install dependencies
cd k8s/
helm dependency update

# Deploy for development (1 instance)
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-dev.yaml

# Deploy for production (3 instances, HA)
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-prod.yaml
```

### Helm Chart Structure

```
k8s/
├── Chart.yaml              # Depends on cloudnative-pg/cluster v0.4.0
├── values.yaml             # Default: 3 instances, production config
├── values-dev.yaml         # Override: 1 instance, minimal resources
├── values-prod.yaml        # Override: Enhanced resources, monitoring
└── templates/
    ├── _helpers.tpl        # Template functions
    ├── clusterimagecatalog.yaml  # Custom image catalog
    └── NOTES.txt           # Post-install instructions
```

### Accessing the Database

```bash
# Port-forward to primary instance
kubectl port-forward -n databases svc/postgres-rw 5432:5432

# Connect with psql
psql -h localhost -U postgres -d database -p 5432

# Get password
kubectl get secret postgres-superuser -n databases \
  -o jsonpath='{.data.password}' | base64 -d
```

### ArgoCD Deployment

Deploy from GitHub repository:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: irdb-postgres
  namespace: argocd
spec:
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
```

See `k8s/README.md` and `README_K8s.md` for detailed documentation.

## Important Notes

- **Init Scripts:** Only run when database is first created (empty `/var/lib/postgresql/data`)
  - To re-run: `docker-compose down -v && docker-compose up -d --build`
- **Helm Migration:** Project now uses Helm charts instead of Kustomize
- **Image Versioning:** Current version is `0.0.7`, CloudNativePG uses major version tag (`17`)
- **pgAdmin Security:** Runs as root to configure pgpass (local dev only)
- **Production Credentials:** Change default credentials before production deployment
- **High Availability:** Use 3+ instances with `minSyncReplicas: 1` for production
- **Makefile Variables:** The Makefile can be customized via environment variables:
  ```bash
  # Example: Use custom cluster name and namespace
  CLUSTER_NAME=my-cluster NAMESPACE=my-db make setup-all

  # Available variables: CLUSTER_NAME, NAMESPACE, IMAGE_NAME, IMAGE_TAG_CURRENT, DB_PASSWORD
  # See Makefile for complete list
  ```

## File Locations

| Purpose | Path |
|---------|------|
| Makefile | `/Makefile` |
| Dockerfile | `/Dockerfile` |
| PostgreSQL Config | `/postgresql.conf` |
| Init Scripts | `/docker-entrypoint-initdb.d/*.sql` |
| Docker Compose | `/docker-compose.yml` |
| Helm Chart | `/k8s/` |
| Chart Values (default) | `/k8s/values.yaml` |
| Chart Values (dev) | `/k8s/values-dev.yaml` |
| Chart Values (prod) | `/k8s/values-prod.yaml` |
| Image Catalog | `/k8s/templates/clusterimagecatalog.yaml` |

## Documentation

- `README.md` - Project overview and quick start
- `README_DOCKER.md` - Docker Compose deployment guide
- `README_K8s.md` - Kubernetes/Helm deployment guide
- `k8s/README.md` - Helm chart documentation
- `.claude/CLAUDE.md` - This file (Claude Code guidance)
