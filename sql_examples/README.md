# IRDB SQL Examples

This directory contains SQL examples demonstrating various search and graph capabilities of the IRDB multi-model database:
- **Full-text search**: ParadeDB pg_search v0.20+
- **Vector similarity**: pgvector v0.8.0
- **Graph relationships**: Apache AGE v1.7.0

## Files Overview

### Test Utilities
| File | Description |
|------|-------------|
| `test_utils.sql` | **Shared utilities** - Setup/teardown functions for idempotent testing |

### Setup & Examples (Files 00-09)
| File | Description |
|------|-------------|
| `00_setup_extensions.sql` | Initializes extensions, schema, tables, and functions |
| `01_fuzzy_search.sql` | Fuzzy term matching with `paradedb.fuzzy_term()` |
| `02_exact_term_search.sql` | Field-specific exact term matching with `field:term` syntax |
| `03_boolean_search.sql` | Boolean operators (OR, AND, NOT) for combining searches |
| `04_phrase_search.sql` | Exact phrase matching with `paradedb.phrase()` |
| `05_complete_setup.sql` | Complete workflow: table creation, indexing, all search types |
| `06_numeric_range_search.sql` | Numeric field searches with ranges and comparisons |
| `07_snippet_highlighting.sql` | Search result highlighting with `paradedb.snippet()` |
| `08_products_schema.sql` | Products schema and indexes (reference, not for direct use) |
| `09_products_data.sql` | Product data loader (reference, not for direct use) |

### Self-Contained Test Suites (Files 10-13): Text & Vector Search
| File | Description |
|------|-------------|
| `10_bm25_search_tests.sql` | **Self-contained** BM25 full-text search tests |
| `11_vector_search_tests.sql` | **Self-contained** Vector similarity search tests |
| `12_hybrid_search_tests.sql` | **Self-contained** Hybrid search (BM25 + vector) tests |
| `13_facet_aggregation_tests.sql` | **Self-contained** Faceted search and aggregation tests |

### Apache AGE Graph Database Tests (Files 14+)
| File | Description |
|------|-------------|
| `14_age_graph_setup.sql` | **Self-contained** AGE graph creation and basic Cypher queries (13 tests) |
| `15_age_cypher_tests.sql` | **Self-contained** Advanced Cypher operations (MERGE, DELETE, path matching, aggregations) |
| `16_age_hybrid_queries.sql` | **Self-contained** Hybrid SQL+Cypher integration (combining relational and graph queries) |
| `17_age_knowledge_graph_tests.sql` | **Self-contained** Full knowledge graph with multi-label entities and relationship types |
| `18_combined_ir_db_tests.sql` | **Self-contained** Full integration: BM25 + vectors + graphs in single queries |

### Test Data
| File | Description |
|------|-------------|
| `../data/products.json` | 40 product entries used by all test suites |

## Idempotent Test Architecture

Files 10-13 are designed to be **idempotent and self-contained**:

- Each test file includes its own setup and teardown
- No dependencies between test files
- Uses shared `test_utils.sql` for consistent setup/cleanup
- All tests use the same product data from `products.json`
- Tests can be run in any order, multiple times

### How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    test_utils.sql                           │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  test_utils.setup()                                 │   │
│  │  - Creates test_products schema                     │   │
│  │  - Creates items table with all columns             │   │
│  │  - Loads 40 products from embedded data             │   │
│  │  - Creates BM25 index (pg_search)                   │   │
│  │  - Creates HNSW vector index (pgvector)             │   │
│  │  - Creates B-tree and GIN indexes                   │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  test_utils.teardown()                              │   │
│  │  - Drops test_products schema (CASCADE)             │   │
│  │  - Cleans up temp tables                            │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  test_utils.create_test_embeddings()                │   │
│  │  - Creates temp table with query embeddings         │   │
│  │  - Used by vector and hybrid search tests           │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Run Individual Test Suites

Each test file is completely self-contained:

```bash
# Change to the sql_examples directory
cd sql_examples

# Run BM25 search tests (creates data, runs tests, cleans up)
psql -h localhost -U postgres -d database -f 10_bm25_search_tests.sql

# Run vector search tests (independent of BM25 tests)
psql -h localhost -U postgres -d database -f 11_vector_search_tests.sql

# Run hybrid search tests
psql -h localhost -U postgres -d database -f 12_hybrid_search_tests.sql

# Run facet aggregation tests
psql -h localhost -U postgres -d database -f 13_facet_aggregation_tests.sql

# Run AGE graph setup tests
psql -h localhost -U postgres -d database -f 14_age_graph_setup.sql

# Run advanced AGE Cypher tests
psql -h localhost -U postgres -d database -f 15_age_cypher_tests.sql

# Run AGE + SQL hybrid queries
psql -h localhost -U postgres -d database -f 16_age_hybrid_queries.sql

# Run knowledge graph tests
psql -h localhost -U postgres -d database -f 17_age_knowledge_graph_tests.sql

# Run combined IR database integration tests
psql -h localhost -U postgres -d database -f 18_combined_ir_db_tests.sql
```

### Run All Tests

```bash
# Run all self-contained test suites (BM25, Vector, Hybrid, Facets, AGE)
for f in {10..18}_*.sql; do
    echo "=========================================="
    echo "Running $f..."
    echo "=========================================="
    psql -h localhost -U postgres -d database -f "$f"
done
```

### Kubernetes Deployment

```bash
# Port-forward first
kubectl port-forward -n databases svc/irdb-postgres-rw 5432:5432

# Run tests (use 'app' database for k8s)
psql -h localhost -U postgres -d app -f 10_bm25_search_tests.sql
```

## Test Data

All tests use the same 40 products from `products.json`:

| Category | Products |
|----------|----------|
| Electronics | Headphones, Mice, Keyboards, Monitors, Cameras, Accessories |
| Home & Garden | Furniture, Kitchen appliances |
| Clothing | Jackets, Shoes, Pants, T-Shirts |
| Sports | Fitness equipment, Outdoor gear |
| Books | Fiction, Non-Fiction, Technical |

Each product includes:
- Text fields: name, description, brand, category, subcategory, tags
- Numeric fields: price, rating, review_count, stock_quantity
- Boolean fields: in_stock, featured
- JSONB: attributes (color, size, specs, etc.)
- Vector: description_embedding (1536 dimensions)

## Test Coverage

### Text & Vector Search (Files 10-13)

#### 10_bm25_search_tests.sql (16 tests)
- Disjunction (`|||`) and conjunction (`&&&`) operators
- Field-specific search
- Numeric range filters
- Category/brand filters
- Featured/in-stock filters
- Combined scoring
- EXPLAIN ANALYZE

#### 11_vector_search_tests.sql (15 tests)
- Cosine similarity (`<=>`)
- L2/Euclidean distance (`<->`)
- Inner product (`<#>`)
- Similarity thresholds
- Category/price/rating filters
- Cross-category analysis
- Index statistics

#### 12_hybrid_search_tests.sql (10 tests)
- Weighted score combination (70/30, 50/50, 40/60)
- Reciprocal Rank Fusion (RRF) with k=30, k=60
- Multi-filter hybrid search
- Multi-field BM25 + vector
- Score distribution analysis
- Performance comparison

#### 13_facet_aggregation_tests.sql (20 tests)
- Value counts and histograms
- Category/brand/subcategory facets
- Price range buckets
- Rating distributions
- Stock availability facets
- JSONB attribute facets
- ROLLUP aggregations
- Pagination simulation

### Graph Database (Files 14-18)

#### 14_age_graph_setup.sql (13 tests)
- Graph creation and listing
- Document and Entity vertex creation with properties
- MENTIONS relationship creation with confidence scores
- Reference relationship creation
- Vertex/edge counting by type
- Property query and manipulation
- Graph statistics (vertex/edge counts, label distributions)
- Path queries (2-hop paths)
- Data retention verification

#### 15_age_cypher_tests.sql (Advanced Cypher - Coming)
- MERGE operations for upserts
- DELETE vertex/edge operations
- Complex path matching with variable-length paths
- Aggregation functions (count, min, max, collect)
- Pattern exploration (triangle detection, cycles)
- Performance optimization with indexes

#### 16_age_hybrid_queries.sql (SQL+Cypher Integration - Coming)
- BEGIN/COMMIT transaction blocks combining SQL and Cypher
- JOIN results from Cypher queries with SQL tables
- Update SQL tables based on graph analysis results
- Graph-based filtering for SQL queries
- Denormalization of graph results into relational tables

#### 17_age_knowledge_graph_tests.sql (Knowledge Graph - Coming)
- Multi-label entity support (Person, Organization, Location)
- Rich relationship types (AUTHORED, MENTIONS, REFERENCES, BELONGS_TO)
- Entity disambiguation and merging
- Relationship confidence scoring
- Full knowledge graph traversal
- Query expansion via graph paths

#### 18_combined_ir_db_tests.sql (Full Integration - Coming)
- Document retrieval with BM25 + vectors + graph context
- Entity linking via graph relationships
- Relationship discovery through hybrid search
- Ranking by combined signals (text match + vector similarity + graph centrality)
- Knowledge graph-enhanced search results

## Key Concepts

### BM25 Search (pg_search v0.20+)

```sql
-- Disjunction: match ANY token
WHERE description ||| 'wireless headphones'

-- Conjunction: match ALL tokens
WHERE description &&& 'wireless noise cancellation'

-- Get BM25 score
SELECT pdb.score(id) AS bm25_score FROM items WHERE ...
```

### Vector Search (pgvector v0.8.0)

```sql
-- Cosine similarity (0 = identical, 2 = opposite)
SELECT 1 - (embedding <=> query_embedding) AS similarity
ORDER BY embedding <=> query_embedding

-- L2/Euclidean distance
ORDER BY embedding <-> query_embedding

-- Inner product (for normalized vectors)
ORDER BY embedding <#> query_embedding
```

### Hybrid Search

```sql
-- Weighted combination (70% vector, 30% BM25)
(bm25_score * 0.3 + vector_score * 0.7) AS combined_score

-- Reciprocal Rank Fusion
(1/(60 + bm25_rank) + 1/(60 + vector_rank)) AS rrf_score
```

## Manual Setup/Teardown

If you need to set up or tear down manually:

```sql
-- Load utilities first
\i test_utils.sql

-- Run setup (returns progress table)
SELECT * FROM test_utils.setup();

-- Run your custom queries against test_products.items
SELECT * FROM test_products.items LIMIT 5;

-- Clean up when done
SELECT * FROM test_utils.teardown();
```

## Troubleshooting

### "relation 'test_products.items' does not exist"

Run setup first:
```sql
\i test_utils.sql
SELECT * FROM test_utils.setup();
```

### "access method 'bm25' does not exist"

The pg_search extension is not loaded:
```sql
CREATE EXTENSION IF NOT EXISTS pg_search;
```

### "type 'vector' does not exist"

The pgvector extension is not loaded:
```sql
CREATE EXTENSION IF NOT EXISTS vector;
```

### Tests leave data behind

If teardown fails, manually clean up:
```sql
DROP SCHEMA IF EXISTS test_products CASCADE;
DROP SCHEMA IF EXISTS test_utils CASCADE;
```

## Related Documentation

### Full-Text & Vector Search
- [ParadeDB Official Docs](https://docs.paradedb.com/)
- [pg_search GitHub](https://github.com/paradedb/paradedb/tree/dev/pg_search)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [BM25 Algorithm](https://en.wikipedia.org/wiki/Okapi_BM25)

### Graph Database
- [Apache AGE GitHub](https://github.com/apache/age)
- [Apache AGE Documentation](https://age.apache.org/)
- [OpenCypher Query Language](https://opencypher.org/)
- [AGE Cypher Syntax](https://age.apache.org/for-users/cypher/)

### IRDB Project
- [PLAN-AGE-INTEGRATION.md](../PLAN-AGE-INTEGRATION.md) - Detailed integration plan and architecture
- [Architecture Documentation](../docs/01-architecture.md)
- [Hybrid Search Documentation](../docs/03-hybrid-search.md)

## License

These examples are part of the IRDB project test suite.
