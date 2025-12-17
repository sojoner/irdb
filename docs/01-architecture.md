# Architecture

## Overview

IRDB is built on PostgreSQL 17.5 with two powerful extensions for information retrieval:
- **ParadeDB pg_search** - BM25 ranking for full-text search
- **pgvector** - Vector similarity search with HNSW indexing

The architecture follows functional programming principles with pure functions, strong typing, and data-oriented design.

## Technology Stack

### Database Layer
- **PostgreSQL 17.5** - Latest stable PostgreSQL release
- **ParadeDB pg_search 0.20.2** - BM25 full-text search with custom operators
  - Repository: https://github.com/paradedb/paradedb
  - Docs: https://docs.paradedb.com/search/overview
- **pgvector 0.8.0** - Vector similarity search with 1536-dimension embeddings
  - Repository: https://github.com/pgvector/pgvector
  - Docs: https://github.com/pgvector/pgvector#readme

### Application Layer
- **Rust** - Systems programming language with memory safety
- **sqlx 0.7** - Async PostgreSQL driver with compile-time query checking
- **Leptos 0.7+** - Reactive web framework compiling to WebAssembly
- **Actix-web 4.x** - High-performance HTTP server

### Infrastructure
- **Docker** - Multi-stage builds for minimal image size (~850MB)
- **Kubernetes** - Production deployment with Helm charts
- **CloudNativePG** - PostgreSQL operator for high availability

## Database Schema

### Core Schema: `ai_data`

**Documents Table:**
```sql
CREATE TABLE ai_data.documents (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),  -- OpenAI ada-002 compatible
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- HNSW index for fast vector similarity
CREATE INDEX documents_embedding_idx ON ai_data.documents
USING hnsw (embedding vector_cosine_ops);

-- GIN index for full-text search
CREATE INDEX documents_fts_idx ON ai_data.documents
USING gin(to_tsvector('english', title || ' ' || content));
```

**Chunks Table:**
```sql
CREATE TABLE ai_data.chunks (
    id SERIAL PRIMARY KEY,
    document_id INTEGER REFERENCES ai_data.documents(id) ON DELETE CASCADE,
    chunk_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX chunks_embedding_idx ON ai_data.chunks
USING hnsw (embedding vector_cosine_ops);
```

### Product Search Schema: `products`

**Items Table:**
```sql
CREATE TABLE products.items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    brand TEXT NOT NULL,
    category TEXT NOT NULL,
    subcategory TEXT,
    tags TEXT[],
    price NUMERIC(10, 2) NOT NULL,
    rating NUMERIC(3, 2) DEFAULT 0,
    review_count INTEGER DEFAULT 0,
    stock_quantity INTEGER DEFAULT 0,
    in_stock BOOLEAN DEFAULT true,
    featured BOOLEAN DEFAULT false,
    attributes JSONB,
    description_embedding vector(1536),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- ParadeDB BM25 index
CALL paradedb.create_bm25(
    index_name => 'products_bm25_idx',
    table_name => 'items',
    schema_name => 'products',
    key_field => 'id',
    text_fields => paradedb.field('description') || paradedb.field('name')
);

-- pgvector HNSW index
CREATE INDEX products_vector_idx ON products.items
USING hnsw (description_embedding vector_cosine_ops);

-- B-tree indexes for filtering
CREATE INDEX products_category_idx ON products.items (category);
CREATE INDEX products_price_idx ON products.items (price);
CREATE INDEX products_rating_idx ON products.items (rating);
```

## Hybrid Search Algorithm

The hybrid search function combines BM25 and vector scores using a weighted approach:

```sql
WITH bm25_results AS (
    -- Top 100 results from BM25 keyword search
    SELECT id, paradedb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| $query
    ORDER BY paradedb.score(id) DESC
    LIMIT 100
),
vector_results AS (
    -- Top 100 results from vector similarity
    SELECT
        id,
        (1 - (description_embedding <=> $embedding))::float8 AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> $embedding
    LIMIT 100
),
combined AS (
    -- FULL OUTER JOIN to include results from both methods
    SELECT
        COALESCE(b.id, v.id) AS id,
        COALESCE(b.bm25_score, 0)::float8 AS bm25_score,
        COALESCE(v.vector_score, 0)::float8 AS vector_score,
        -- Weighted combination: 30% BM25 + 70% Vector
        (COALESCE(b.bm25_score, 0) * 0.3 +
         COALESCE(v.vector_score, 0) * 0.7)::float8 AS combined_score
    FROM bm25_results b
    FULL OUTER JOIN vector_results v ON b.id = v.id
)
SELECT p.*, c.bm25_score, c.vector_score, c.combined_score
FROM combined c
JOIN products.items p ON c.id = p.id
WHERE /* filters */
ORDER BY c.combined_score DESC
LIMIT $limit OFFSET $offset;
```

**Why This Works:**
- **FULL OUTER JOIN** ensures results from either method are included
- **Top-100 pre-filtering** reduces computational cost
- **Weighted scoring** (30/70) favors semantic understanding while preserving keyword matches
- **COALESCE** handles cases where results only appear in one method

## Design Principles

### 1. Pure Functions Over Classes

All query functions are stateless and side-effect-free:

```rust
pub async fn search_hybrid(
    pool: &PgPool,       // Explicit dependency injection
    query: &str,         // Input
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error>  // Typed output, explicit error
```

**Benefits:**
- Easy to test (no mocking needed)
- Easy to compose and parallelize
- Clear data flow
- No hidden state

### 2. Type Safety Throughout

Strong typing at every layer:

```rust
// Enums for modes and options (not strings)
pub enum SearchMode { Bm25, Vector, Hybrid }
pub enum SortOption { Relevance, PriceAsc, PriceDesc, RatingDesc, Newest }

// Decimal for money (not f64)
pub price: rust_decimal::Decimal

// Compile-time query checking with sqlx
sqlx::query_as::<_, SearchResultRow>(sql)
```

### 3. Data-Oriented Design

Focus on data transformations, not object hierarchies:

```
Input (Query + Filters)
  ↓
SQL Query (Data transformation)
  ↓
SearchResultRow (Database representation)
  ↓
From trait conversion
  ↓
SearchResults (API representation)
```

### 4. Feature Flags for Conditional Compilation

Web components are optional to avoid WASM toolchain requirements:

```toml
[features]
default = []
web = ["leptos", "actix-web", "leptos_actix"]
hydrate = ["leptos/hydrate", "wasm-bindgen"]
```

## Multi-Stage Docker Build

The Dockerfile uses two stages to keep image size small:

**Builder Stage:**
```dockerfile
FROM postgres:17.5-bookworm AS builder
# Install Rust toolchain
# Compile ParadeDB pg_search from source
# Build with cargo-pgrx (PostgreSQL extension builder)
```

**Runtime Stage:**
```dockerfile
FROM postgres:17.5-bookworm AS runtime
# Copy only compiled extension files
# Install pgvector from apt
# Copy configuration and init scripts
# Final image: ~850MB
```

## Index Performance

| Search Mode | Index Type | Performance | Best For |
|-------------|-----------|-------------|----------|
| BM25 | Inverted Index | Fast (O(log n)) | Keywords, brand names, exact terms |
| Vector | HNSW (ANN) | Fast (O(log n)) | Semantic queries, concepts, natural language |
| Hybrid | Both | Moderate | General-purpose search with balanced relevance |

**HNSW (Hierarchical Navigable Small World):**
- Graph-based approximate nearest neighbor (ANN) algorithm
- O(log n) query time with high recall
- Configurable trade-offs between speed and accuracy
- Reference: [Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs](https://arxiv.org/abs/1603.09320)

**BM25 (Best Matching 25):**
- Probabilistic relevance ranking algorithm
- Considers term frequency, document frequency, and document length normalization
- Industry standard for text search (Elasticsearch, Solr)
- Reference: [The Probabilistic Relevance Framework: BM25 and Beyond](https://www.staff.city.ac.uk/~sbrp622/papers/foundations_bm25_review.pdf)

## Configuration

The system is optimized for AI workloads in `postgresql.conf`:

```ini
# Parallel query execution
max_parallel_workers_per_gather = 4
max_parallel_workers = 8

# Memory for vector operations
shared_buffers = 2GB
effective_cache_size = 8GB
work_mem = 256MB
maintenance_work_mem = 1GB

# Extension loading
shared_preload_libraries = 'pg_search,vector'

# HNSW index parameters
hnsw.ef_search = 100  # Higher = better recall, slower queries
```

## Next Steps

See the following documentation:
- [Deployment Guide](./02-deployment.md) - How to run IRDB
- [Hybrid Search Deep Dive](./03-hybrid-search.md) - Implementation details and examples
- [Web Application Development](./04-web-app.md) - Building the Leptos UI
