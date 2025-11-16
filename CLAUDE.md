# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an AI-enhanced PostgreSQL 17 platform with RAG (Retrieval Augmented Generation) capabilities, built using Docker. It combines pgvector for vector similarity search and ParadeDB's pg_search for full-text search, enabling hybrid search functionality optimized for AI/ML workloads.

## Build & Development Commands

### Building the Docker Image
```bash
# Build the image (uses multi-stage build)
docker build -t <USER>/database:0.0.1 .

# Push to registry (optional)
docker push <USER>/database:0.0.1
```

### Running the Services
```bash
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
```bash
# Connect to PostgreSQL directly
psql -h localhost -U postgres -d database -p 5432

# Access via pgAdmin web interface
# Open http://localhost:5433
# Email: admin@database.com
# Password: custom_secure_password_123
```

### Testing Changes

When modifying initialization scripts in `docker-entrypoint-initdb.d/`:
```bash
# Completely rebuild to test init scripts
docker-compose down -v  # Remove volumes to trigger re-initialization
docker-compose up -d --build
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
   - Compiles ParadeDB pg_search extension from source (v0.17.2)
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

## Important Notes

- The pgAdmin container runs as root to set up passwordless access via pgpass file
- Initialization scripts only run when the database is first created (empty `/var/lib/postgresql/data`)
- To re-run initialization scripts, must remove volumes: `docker-compose down -v`
- ParadeDB extensions are built from a specific version tag (v0.17.2) - update Dockerfile to change versions
- Default credentials are hardcoded in docker-compose.yml - change for production use
