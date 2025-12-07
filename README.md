# IR DB - AI-Enhanced PostgreSQL Platform

PostgreSQL 17.5 with pgvector and ParadeDB extensions, optimized for RAG (Retrieval Augmented Generation) applications.

## Features

- **PostgreSQL 17.5** - Latest stable release
- **pgvector v0.8.0** - Vector similarity search (1536 dimensions for OpenAI embeddings)
- **ParadeDB pg_search v0.17.2** - Full-text search with BM25 ranking
- **Hybrid Search** - Combines vector similarity (70%) and text search (30%)
- **Pre-configured RAG Schema** - Ready-to-use tables, indexes, and functions
- **Multi-stage Docker Build** - Optimized ~850MB final image
- **Production-Ready Helm Chart** - CloudNativePG for Kubernetes

## Quick Start

### Docker Compose (Recommended for Local Development)

```bash
# Build the image
docker build -t sojoner/database:0.0.7 .

# Start PostgreSQL + pgAdmin
docker-compose up -d

# Connect to database
psql -h localhost -U postgres -d database -p 5432
# Password: custom_secure_password_123

# Access pgAdmin web interface
open http://localhost:5433
# Email: admin@database.com
# Password: custom_secure_password_123
```

**See [README_DOCKER.md](README_DOCKER.md) for complete Docker guide.**

### Kubernetes (Production Deployment)

```bash
# Install CloudNativePG operator
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm install cnpg --namespace cnpg-system --create-namespace cnpg/cloudnative-pg

# Deploy IR DB (development)
cd k8s/
helm dependency update
helm install irdb-postgres . -n databases --create-namespace -f values-dev.yaml

# Deploy IR DB (production with HA)
helm install irdb-postgres . -n databases --create-namespace -f values-prod.yaml
```

**See [README_K8s.md](README_K8s.md) for complete Kubernetes guide.**

## Database Schema

### ai_data.documents

Main table for document storage with embeddings:

```sql
CREATE TABLE ai_data.documents (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),  -- OpenAI ada-002 dimensions
    metadata JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

**Indexes:**
- HNSW index on `embedding` for fast cosine similarity search
- GIN index on `to_tsvector(title || content)` for full-text search

### ai_data.chunks

Document chunks for RAG workflows:

```sql
CREATE TABLE ai_data.chunks (
    id SERIAL PRIMARY KEY,
    document_id INTEGER REFERENCES documents(id),
    chunk_text TEXT NOT NULL,
    embedding vector(1536),
    chunk_index INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Search Examples

### Vector Similarity Search

```sql
SELECT id, title,
  1 - (embedding <=> '[0.1, 0.2, ...]'::vector(1536)) as similarity
FROM ai_data.documents
ORDER BY embedding <=> '[0.1, 0.2, ...]'::vector(1536)
LIMIT 10;
```

### Full-Text Search (BM25)

```sql
SELECT id, title,
  ts_rank(to_tsvector('english', title || ' ' || content),
          to_tsquery('english', 'search & terms')) as score
FROM ai_data.documents
WHERE to_tsvector('english', title || ' ' || content) @@
      to_tsquery('english', 'search & terms')
ORDER BY score DESC;
```

### Hybrid Search (Vector + Text)

```sql
SELECT * FROM ai_data.hybrid_search(
  query_text => 'search terms',
  query_embedding => '[0.1, 0.2, ...]'::vector(1536),
  similarity_threshold => 0.5,
  limit_count => 10
)
ORDER BY combined_score DESC;
```

The `hybrid_search` function combines:
- 70% weight on vector similarity
- 30% weight on BM25 text score

## Architecture

### Multi-Stage Docker Build

**Builder Stage:**
- Compiles ParadeDB pg_search extension from source (Rust)
- Uses cargo-pgrx for PostgreSQL extension building
- ~10-15 minutes build time

**Runtime Stage:**
- Minimal PostgreSQL 17.5 image
- Copies compiled pg_search extension
- Installs pgvector from apt
- Includes custom postgresql.conf and init scripts
- Final size: ~850MB

### Initialization Scripts

Scripts run in alphabetical order during first database creation:

1. `00-extensions.sql` - Creates extensions (pgvector, pg_search, pg_trgm, etc.)
2. `01-ai-extensions.sql` - Creates ai_data schema, tables, functions, indexes
3. `02-validating-bm25.sql` - BM25 search validation tests
4. `03-simple-vector-test.sql` - Vector search validation tests
5. `05-comprehensive-test.sql` - Full integration tests

**Note:** Init scripts only run when database data directory is empty.

## Extensions Included

| Extension | Version | Purpose |
|-----------|---------|---------|
| pgvector | 0.8.0 | Vector similarity search with HNSW index |
| pg_search | 0.17.2 | Full-text search with BM25 ranking |
| pg_stat_statements | 1.10 | Query performance monitoring |
| pg_trgm | 1.6 | Trigram similarity for fuzzy matching |
| btree_gin | 1.3 | Additional GIN index support |

## Configuration

### PostgreSQL Settings

Optimized for AI workloads in `postgresql.conf`:

```ini
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 16MB
maintenance_work_mem = 512MB

# Parallel workers for vector operations
max_parallel_workers = 4
max_parallel_workers_per_gather = 2

# Extensions
shared_preload_libraries = 'pg_stat_statements,pg_search,vector'
```

### Resource Allocation

**Docker Compose:**
- CPU: 8 cores max, 4 cores reserved
- Memory: 32GB max, 16GB reserved
- Shared Memory: 2GB for PostgreSQL operations

**Kubernetes (Production):**
- CPU: 4 cores max, 2 cores reserved
- Memory: 8GB max, 4GB reserved
- 3 instances for high availability

## Deployment Options

| Method | Use Case | Documentation |
|--------|----------|---------------|
| Docker Compose | Local development, testing | [README_DOCKER.md](README_DOCKER.md) |
| Kubernetes Helm | Production, staging, multi-node | [README_K8s.md](README_K8s.md) |
| ArgoCD GitOps | Production CD pipeline | [README_K8s.md](README_K8s.md#deploying-from-github-argocd) |

## Common Tasks

### Rebuild After Schema Changes

```bash
# Docker Compose (removes all data)
docker-compose down -v
docker-compose up -d --build

# Kubernetes (rolling update)
helm upgrade irdb-postgres k8s/ -n databases
```

### Scale Kubernetes Deployment

```bash
# Scale to 5 instances
helm upgrade irdb-postgres k8s/ \
  --set cnpg.cluster.instances=5 \
  -n databases
```

### Update Image Version

```bash
# Docker Compose
docker build -t sojoner/database:0.0.8 .
docker-compose up -d --build

# Kubernetes
helm upgrade irdb-postgres k8s/ \
  --set image.tag=0.0.8 \
  -n databases
```

## Project Structure

```
irdb/
├── Dockerfile                          # Multi-stage build
├── postgresql.conf                     # Optimized PostgreSQL config
├── docker-compose.yml                  # Local development setup
├── docker-entrypoint-initdb.d/         # Initialization scripts
│   ├── 00-extensions.sql
│   ├── 01-ai-extensions.sql
│   ├── 02-validating-bm25.sql
│   ├── 03-simple-vector-test.sql
│   └── 05-comprehensive-test.sql
├── k8s/                                # Helm chart
│   ├── Chart.yaml
│   ├── values.yaml                     # Default (production)
│   ├── values-dev.yaml                 # Development overrides
│   ├── values-prod.yaml                # Production enhancements
│   ├── templates/
│   │   ├── clusterimagecatalog.yaml    # Custom image catalog
│   │   ├── _helpers.tpl
│   │   └── NOTES.txt
│   └── README.md                       # Chart documentation
├── README.md                           # This file
├── README_DOCKER.md                    # Docker Compose guide
├── README_K8s.md                       # Kubernetes guide
└── .claude/
    └── CLAUDE.md                       # Claude Code guidance

```

## Security Notes

- Default credentials are hardcoded for development convenience
- **Change credentials before production deployment**
- pgAdmin container runs as root (local dev only)
- For production, use Kubernetes secrets management
- CloudNativePG automatically manages TLS certificates

## Performance Tips

1. **Vector Search:** Use HNSW index for fast approximate nearest neighbor search
2. **Text Search:** GIN indexes are automatically used for `@@` queries
3. **Hybrid Search:** Pre-filters with text search, then ranks by combined score
4. **Batch Inserts:** Use `COPY` or multi-row `INSERT` for bulk loading
5. **Connection Pooling:** Use pgBouncer for high-concurrency applications

## Troubleshooting

### Extensions Not Found

```bash
# Verify extensions are installed
psql -h localhost -U postgres -d database -c "SELECT extname, extversion FROM pg_extension;"

# If missing, rebuild without cache
docker-compose down -v
docker build --no-cache -t sojoner/database:0.0.7 .
docker-compose up -d
```

### Init Scripts Not Running

```sql
-- Check if ai_data schema exists
SELECT schema_name FROM information_schema.schemata WHERE schema_name = 'ai_data';

-- If missing, volumes weren't removed
-- Docker Compose:
docker-compose down -v  # This removes volumes
docker-compose up -d

-- Kubernetes:
helm uninstall irdb-postgres -n databases
kubectl delete pvc -n databases -l cnpg.io/cluster=postgres
helm install irdb-postgres k8s/ -n databases
```

### Out of Memory

```bash
# Reduce PostgreSQL memory settings in postgresql.conf
shared_buffers = 128MB
effective_cache_size = 512MB
work_mem = 8MB

# Or increase Docker memory limit
# Docker Desktop: Settings → Resources → Memory
```

## Documentation

- **[README_DOCKER.md](README_DOCKER.md)** - Complete Docker Compose deployment guide
- **[README_K8s.md](README_K8s.md)** - Complete Kubernetes/Helm deployment guide
- **[k8s/README.md](k8s/README.md)** - Helm chart reference documentation
- **[.claude/CLAUDE.md](.claude/CLAUDE.md)** - Development guide for Claude Code

## Resources

- [PostgreSQL 17 Documentation](https://www.postgresql.org/docs/17/)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [ParadeDB Documentation](https://docs.paradedb.com/)
- [CloudNativePG Documentation](https://cloudnative-pg.io/documentation/)

## License

[Add your license here]

## Contributing

[Add contributing guidelines here]
