# AI-Enhanced PostgreSQL Platform

This project provides a Dockerized PostgreSQL environment with AI/ML extensions optimized for RAG (Retrieval Augmented Generation) applications.

## Features

- PostgreSQL 15 with AI extensions
- pgvector for vector similarity search
- ParadeDB extensions (pg_search only)
- Optimized configuration for AI workloads
- Pre-configured tables and indexes for RAG applications

## Extensions Included

1. **pgvector** - For vector similarity search
2. **pg_search** - For full-text search capabilities  
3. **pg_trgm** - Text similarity functions
4. **pg_stat_statements** - Query performance monitoring
5. **btree_gin** - Additional index types

## Database Structure

### Schema: `ai_data`

#### Tables:
1. **documents** - Stores documents with embeddings for vector search
2. **chunks** - Stores document chunks with embeddings

#### Functions:
1. **hybrid_search** - Combines vector and text search for better results
2. **generate_random_vector** - Helper function to generate test vectors

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

## BM25 Search Usage

### Basic Text Search
```sql
SELECT 
    id,
    title,
    ts_rank(to_tsvector('english', title || ' ' || content), to_tsquery('english', 'search & postgres')) as score
FROM ai_data.documents 
WHERE to_tsvector('english', title || ' ' || content) @@ to_tsquery('english', 'search & postgres')
ORDER BY score DESC;
```

### Using


## Vector Search Usage

### Simple Similarity Search
```sql
SELECT 
    d.id,
    d.title,
    1 - (d.embedding <=> '[0.1, 0.2, 0.3, 0.4, 0.5]'::vector(5)) as similarity
FROM ai_data.documents d
ORDER BY d.embedding <=> '[0.1, 0.2, 0.3, 0.4, 0.5]'::vector(5)
LIMIT 3;
```

## Hybrid Search Usage

### Combining Both Approaches
```sql
SELECT 
    id,
    title,
    vector_similarity,
    text_score,
    combined_score
FROM ai_data.hybrid_search(
    query_text => 'PostgreSQL search',
    query_embedding => '[0.1, 0.2, 0.3, 0.4, 0.5]'::vector(5),
    similarity_threshold => 0.5,
    limit_count => 5
)
ORDER BY combined_score DESC;
```

## Testing Both Features

### Validation Script
The container automatically runs a comprehensive validation script during initialization that tests all features:
1. Extension installation verification
2. Schema and table creation
3. Data insertion tests
4. BM25 search functionality
5. Vector search functionality
6. Hybrid search function
7. Index verification

You can also manually run the validation script at `/docker-entrypoint-initdb.d/02-validation-test.sql` to verify all functionality.

### Manual Testing
For manual testing of both features, you can run the test script in `irdb/test_bm25_and_vector.sql` which provides comprehensive validation of:
- Extension installation
- Table structure
- Data insertion
- BM25 search capabilities
- Vector search capabilities  
- Hybrid search function

## Running Tests Manually

To run these tests manually after connecting to your database:

```sql
-- Test 1: Verify extensions are installed
SELECT 
    extname,
    extversion
FROM pg_extension 
WHERE extname IN ('vector', 'pg_search', 'pg_analytics', 'pg_stat_statements', 'pg_trgm', 'btree_gin');

-- Test 2: Test BM25 search with various queries
SELECT 
    id,
    title,
    ts_rank(to_tsvector('english', title || ' ' || content), to_tsquery('english', 'ParadeDB')) as score
FROM ai_data.documents 
WHERE to_tsvector('english', title || ' ' || content) @@ to_tsquery('english', 'ParadeDB')
ORDER BY score DESC;


-- Test 3: Test vector similarity search
SELECT 
    d.id,
    d.title,
    1 - (d.embedding <=> '[0.6, 0.7, 0.8, 0.9, 0.1]'::vector(5)) as similarity
FROM ai_data.documents d
ORDER BY d.embedding <=> '[0.6, 0.7, 0.8, 0.9, 0.1]'::vector(5)
LIMIT 3;
```

## Configuration

The PostgreSQL configuration (`postgresql.conf`) is optimized for AI/ML workloads with:
- Increased memory settings
- Parallel processing enabled
- Special indexes for vector and text search

## Validation

The container automatically runs validation tests during initialization. You can also manually run the validation script at `/docker-entrypoint-initdb.d/02-validation-test.sql` to verify both BM25 and vector search functionality.
