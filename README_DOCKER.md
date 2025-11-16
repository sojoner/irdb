# Docker Compose Deployment Guide

Complete guide for running IR DB locally with Docker Compose.

## Overview

Docker Compose provides the simplest way to run IR DB for local development and testing. It includes:
- **PostgreSQL 17.5** with pgvector and ParadeDB extensions
- **pgAdmin** web interface for database management
- **Pre-configured** passwordless authentication
- **Persistent storage** for database data

## Prerequisites

- Docker Desktop or Docker Engine + Docker Compose
- 8GB+ RAM available
- 20GB+ disk space
- psql client (optional, for command-line access)

## Quick Start

### 1. Build the Image

```bash
# Navigate to project root
cd irdb

# Build the custom PostgreSQL image (takes 10-15 minutes)
docker build -t sojoner/database:0.0.7 .

# Watch for success message
# "Successfully tagged sojoner/database:0.0.7"
```

**Verify the build:**
```bash
docker images | grep sojoner/database
# Expected: sojoner/database   0.0.7   <IMAGE_ID>   <TIME>   ~850MB
```

### 2. Start the Services

```bash
# Start PostgreSQL and pgAdmin in detached mode
docker-compose up -d

# Wait 30-60 seconds for services to initialize
docker-compose ps
```

**Expected output:**
```
NAME                IMAGE                    STATUS
irdb-postgres-1     sojoner/database:0.0.7  Up
irdb-pgadmin-1      dpage/pgadmin4:latest   Up
```

### 3. Verify Database is Ready

```bash
# Check logs for successful startup
docker-compose logs postgres | grep "database system is ready to accept connections"

# Should see this message twice (initial startup + after loading extensions)
```

## Accessing the Database

### Option 1: psql Command Line

```bash
# Connect to PostgreSQL
psql -h localhost -U postgres -d database -p 5432

# When prompted, enter password
# Password: custom_secure_password_123
```

**Verify extensions:**
```sql
SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector', 'pg_search');
```

**Expected output:**
```
  extname  | extversion
-----------+------------
 pg_search | 0.17.2
 vector    | 0.8.0
```

### Option 2: pgAdmin Web Interface

1. Open browser to `http://localhost:5433`

2. Login with credentials:
   - Email: `admin@database.com`
   - Password: `custom_secure_password_123`

3. Database connection is pre-configured
   - Server name: "Database Server"
   - Passwordless authentication enabled
   - Automatically appears in server list

4. Navigate to: Servers → Database Server → Databases → database

## Verifying the Setup

### Check Schema and Tables

```sql
-- List schemas
\dn

-- Should show: ai_data, public, information_schema, pg_catalog

-- List tables in ai_data
\dt ai_data.*

-- Should show:
--   ai_data.documents
--   ai_data.chunks
```

### Test Hybrid Search

```sql
-- Insert test document
INSERT INTO ai_data.documents (title, content, embedding) VALUES
('Test Document', 'This is a test of the hybrid search functionality',
 ai_data.generate_random_vector(1536));

-- Run hybrid search
SELECT * FROM ai_data.hybrid_search(
  query_text => 'test search',
  query_embedding => ai_data.generate_random_vector(1536),
  similarity_threshold => 0.0,
  limit_count => 10
);
```

## Common Operations

### View Logs

```bash
# Follow logs for all services
docker-compose logs -f

# View only PostgreSQL logs
docker-compose logs -f postgres

# View only pgAdmin logs
docker-compose logs -f pgadmin
```

### Stop Services

```bash
# Stop services (keeps data)
docker-compose down

# Stop and remove volumes (deletes data)
docker-compose down -v
```

### Restart Services

```bash
# Restart all services
docker-compose restart

# Restart only PostgreSQL
docker-compose restart postgres
```

### Rebuild After Changes

When you modify initialization scripts in `docker-entrypoint-initdb.d/`:

```bash
# IMPORTANT: Must remove volumes to re-run init scripts
docker-compose down -v

# Rebuild and start
docker-compose up -d --build

# Verify init scripts ran
docker-compose logs postgres | grep "docker-entrypoint-initdb.d"
```

**Init scripts only run when `/var/lib/postgresql/data` is empty!**

## Configuration

### Services

**PostgreSQL:**
- Image: `sojoner/database:0.0.7`
- Port: `5432` (host) → `5432` (container)
- Volume: `./data/database` → `/var/lib/postgresql/data`
- Resources:
  - CPU: 8 cores max, 4 cores reserved
  - Memory: 32GB max, 16GB reserved
  - Shared Memory: 2GB

**pgAdmin:**
- Image: `dpage/pgadmin4:latest`
- Port: `5433` (host) → `80` (container)
- Volume: `./data/pgadmin` → `/var/lib/pgadmin`
- Auto-configured server connection
- Passwordless authentication via pgpass

### Default Credentials

| Service | Parameter | Value |
|---------|-----------|-------|
| PostgreSQL | Host | `localhost` |
| | Port | `5432` |
| | Database | `database` |
| | Username | `postgres` |
| | Password | `custom_secure_password_123` |
| pgAdmin | URL | `http://localhost:5433` |
| | Email | `admin@database.com` |
| | Password | `custom_secure_password_123` |

**Security Note:** Change these credentials before exposing to network or production use.

### Customizing Resources

Edit `docker-compose.yml`:

```yaml
services:
  postgres:
    deploy:
      resources:
        limits:
          cpus: '4'      # Reduce CPU limit
          memory: 16G    # Reduce memory limit
        reservations:
          cpus: '2'
          memory: 8G
```

### Customizing PostgreSQL Settings

Edit `postgresql.conf` before building:

```ini
shared_buffers = 128MB           # Reduce for lower memory
effective_cache_size = 512MB
max_connections = 100            # Reduce for fewer connections
```

Then rebuild:
```bash
docker-compose down
docker build -t sojoner/database:0.0.7 .
docker-compose up -d
```

## Troubleshooting

### Problem: Services Won't Start

```bash
# Check if ports are already in use
lsof -i :5432  # PostgreSQL
lsof -i :5433  # pgAdmin

# If ports are in use, stop conflicting services or change ports in docker-compose.yml
```

### Problem: Extensions Not Loading

```bash
# Check PostgreSQL logs for errors
docker-compose logs postgres | grep -i error

# Verify initialization scripts ran
docker-compose logs postgres | grep "docker-entrypoint-initdb.d"

# If scripts didn't run, database already existed
# Remove volumes and restart
docker-compose down -v
docker-compose up -d
```

### Problem: pgAdmin Can't Connect

```bash
# Verify PostgreSQL is running
docker-compose ps postgres

# Check PostgreSQL logs
docker-compose logs postgres

# Verify pgpass file was created
docker-compose exec pgadmin cat /var/lib/pgadmin/.pgpass

# Should show: postgres:5432:database:postgres:custom_secure_password_123
```

### Problem: Out of Memory

```bash
# Check Docker memory usage
docker stats

# Option 1: Reduce PostgreSQL settings (see Customizing PostgreSQL Settings)

# Option 2: Increase Docker memory limit
# Docker Desktop: Settings → Resources → Memory → Increase limit
```

### Problem: Init Scripts Failed

```bash
# View full PostgreSQL logs
docker-compose logs postgres > postgres.log

# Search for SQL errors
grep -i "error" postgres.log
grep -i "failed" postgres.log

# Common issues:
# - Syntax errors in SQL files
# - Missing dependencies between scripts
# - Insufficient permissions
```

### Problem: Database Data Persists After `down -v`

```bash
# Manually remove data directory
rm -rf ./data/database
rm -rf ./data/pgadmin

# Then start again
docker-compose up -d
```

## Performance Tips

### Bulk Data Loading

Use `COPY` for faster imports:

```sql
-- Prepare CSV file with embeddings
-- Format: id,title,content,embedding
-- Example: 1,"Title","Content","[0.1,0.2,0.3,...]"

-- Import from CSV
\copy ai_data.documents(title,content,embedding) FROM 'data.csv' WITH (FORMAT csv, HEADER true);
```

### Connection Pooling

For high-concurrency applications, use pgBouncer:

```bash
# Add to docker-compose.yml
services:
  pgbouncer:
    image: pgbouncer/pgbouncer:latest
    environment:
      DATABASES_HOST: postgres
      DATABASES_PORT: 5432
      DATABASES_DATABASE: database
      DATABASES_USER: postgres
      DATABASES_PASSWORD: custom_secure_password_123
    ports:
      - "6432:5432"
```

### Monitoring

Enable query statistics:

```sql
-- View slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- View table sizes
SELECT
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'ai_data'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

## Backup and Restore

### Backup Database

```bash
# Full database backup
docker-compose exec postgres pg_dump -U postgres database > backup_$(date +%Y%m%d).sql

# Schema only
docker-compose exec postgres pg_dump -U postgres -s database > schema.sql

# Data only
docker-compose exec postgres pg_dump -U postgres -a database > data.sql
```

### Restore Database

```bash
# Restore full backup
cat backup_20240101.sql | docker-compose exec -T postgres psql -U postgres database

# Or using psql directly
psql -h localhost -U postgres -d database -p 5432 < backup_20240101.sql
```

## Migrating to Kubernetes

When ready for production, migrate to Kubernetes using the Helm chart:

```bash
# Build and push image
docker build -t sojoner/database:0.0.7 .
docker push sojoner/database:0.0.7

# Deploy to Kubernetes
cd k8s/
helm dependency update
helm install irdb-postgres . -n databases --create-namespace -f values-prod.yaml
```

See [README_K8s.md](README_K8s.md) for complete Kubernetes deployment guide.

## Next Steps

- **[README.md](README.md)** - Project overview and architecture
- **[README_K8s.md](README_K8s.md)** - Kubernetes deployment guide
- **[.claude/CLAUDE.md](.claude/CLAUDE.md)** - Development guide

## Resources

- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [PostgreSQL Docker Official Image](https://hub.docker.com/_/postgres)
- [pgAdmin Documentation](https://www.pgadmin.org/docs/)
