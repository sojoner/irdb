# AI-Enhanced PostgreSQL Platform

This project provides a Dockerized PostgreSQL environment with AI/ML extensions optimized for RAG (Retrieval Augmented Generation) applications.

## Features

- PostgreSQL 15 with AI extensions
- pgvector for vector similarity search
- ParadeDB extensions (pg_search, pg_analytics)
- Optimized configuration for AI workloads
- Pre-configured tables and indexes for RAG applications

## Building the Image

To build the Docker image with pgvector support:
```bash
docker build -t ai-postgres .
```

## Running the Container

You can run the container using Docker Compose:
```bash
docker-compose up -d
```

Or using Docker directly:
```bash
docker run -d \
  --name ai-postgres \
  -p 5432:5432 \
  -e POSTGRES_PASSWORD=mypassword \
  ai-postgres
```

## Connecting to the Database

Once the container is running, you can connect to your database using:
```bash
psql -h localhost -U myuser -d mydatabase -p 5432
```

## Enabling pgvector

After connecting to your database, enable the pgvector extension by running:
```sql
CREATE EXTENSION IF NOT EXISTS vector;
```

## Extensions Included

- `vector` - Vector data type and operators
- `pg_search` - Full-text search capabilities
- `pg_analytics` - Analytics extensions
- `pg_stat_statements` - Query performance monitoring
- `pg_trgm` - Text similarity functions
- `btree_gin` - Additional index types

## Database Structure

### Schema: `ai_data`

#### Tables:
1. `documents` - Stores documents with embeddings for vector search
2. `chunks` - Stores document chunks with embeddings

#### Functions:
1. `hybrid_search` - Combines vector and text search for better results

## Usage

Build the Docker image:
```bash
docker build -t ai-postgres .
```

Run the container:
```bash
docker run -d \
  --name ai-postgres \
  -p 5432:5432 \
  -e POSTGRES_PASSWORD=mypassword \
  ai-postgres
```

## Configuration

The PostgreSQL configuration (`postgresql.conf`) is optimized for AI/ML workloads with:
- Increased memory settings
- Parallel processing enabled
- Special indexes for vector and text search
