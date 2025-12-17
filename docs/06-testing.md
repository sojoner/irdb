# Testing Guide

This document covers the comprehensive testing strategy for IRDB, including unit tests, integration tests, database tests, and code coverage analysis.

## Overview

The IRDB project uses a multi-layered testing approach:

| Test Type | Location | Features Required | Database Required |
|-----------|----------|-------------------|-------------------|
| Unit Tests | `src/**/*.rs` | `ssr` | No |
| Integration Tests | `tests/*.rs` | `db-tools` or `ssr` | Yes |
| SQL Tests | `sql_examples/*.sql` | N/A | Yes |
| Validation Tests | Makefile targets | N/A | Yes |

## Prerequisites

### 1. Database Setup

Before running tests, you need a PostgreSQL instance with ParadeDB and pgvector extensions:

```bash
# Option A: Docker Compose (recommended for local development)
make compose-up

# Option B: Kubernetes
make setup-all
```

### 2. Environment Configuration

Set the `DATABASE_URL` environment variable:

```bash
# Docker Compose (default)
export DATABASE_URL="postgresql://postgres:custom_secure_password_123@localhost:5432/database"

# Or create a .env file in pg_search_tests/
echo 'DATABASE_URL="postgresql://postgres:custom_secure_password_123@localhost:5432/database"' > pg_search_tests/.env
```

### 3. Install Test Dependencies

```bash
# Install cargo-llvm-cov for coverage reports
cargo install cargo-llvm-cov

# Verify installation
cargo llvm-cov --version
```

## Running Tests

### Quick Reference

```bash
# Navigate to the test directory
cd pg_search_tests

# Run all tests with SSR feature (most common)
cargo test --features ssr

# Run specific test file
cargo test --test products_hybrid_test --features ssr

# Run specific test with output
cargo test --test bm25_detailed_tests --features ssr test_bm25_fuzzy -- --nocapture

# Run unit tests only (no database)
cargo test --lib --features ssr
```

### Test Categories

#### Unit Tests (No Database Required)

Fast tests that verify component logic without database connections:

```bash
cargo test --lib --features ssr
```

**Test files covered:**
- `src/web_app/model/mod.rs` - Data model tests
- `src/web_app/components/*.rs` - UI component logic
- `src/web_app/api/queries.rs` - Query builder tests
- `src/web_app/pages/search.rs` - Page logic

**Example output:**
```
running 20 tests
test web_app::model::tests::test_search_mode_default ... ok
test web_app::components::common::tests::test_star_calculation ... ok
test web_app::api::queries::tests::test_generate_query_embedding_format ... ok
...
test result: ok. 19 passed; 0 failed; 1 ignored
```

#### Integration Tests (Database Required)

Tests that verify database queries and search functionality:

```bash
# All integration tests
cargo test --features ssr

# Specific test suites:
cargo test --test bm25_detailed_tests --features ssr     # BM25 full-text search
cargo test --test products_vector_test --features ssr    # Vector similarity search
cargo test --test products_hybrid_test --features ssr    # Hybrid search
cargo test --test products_facets_test --features ssr    # Facet aggregations
cargo test --test products_bm25_test --features ssr      # Product BM25 search
cargo test --test advanced_search_tests --features ssr   # Advanced query syntax
cargo test --test backend_search_tests --features ssr    # Backend API tests
cargo test --test dbtuning_test --features ssr           # PostgreSQL config verification
cargo test --test init_db_test --features ssr            # Database initialization
```

#### SQL Validation Tests

Direct SQL tests via Makefile:

```bash
# Run all SQL tests
make test-sql-all

# Individual SQL tests
make test-sql-bm25      # BM25 search tests
make test-sql-vector    # Vector search tests
make test-sql-hybrid    # Hybrid search tests
make test-sql-facets    # Facet aggregation tests
```

## Code Coverage

### Generating Coverage Reports

IRDB uses `cargo-llvm-cov` for accurate code coverage measurement.

#### HTML Report (Interactive)

```bash
cd pg_search_tests
cargo llvm-cov --features ssr --html
open target/llvm-cov/html/index.html
```

#### LCOV Report (For CI/Codecov)

```bash
cd pg_search_tests
cargo llvm-cov --features ssr --lcov --output-path lcov.info
```

#### Both Formats

```bash
cd pg_search_tests
cargo llvm-cov --features ssr --html --lcov --output-path lcov.info
```

### Understanding Coverage Reports

The HTML report shows:

1. **Summary View** - Overall coverage percentage for the project
2. **File View** - Per-file coverage breakdown
3. **Line View** - Green (covered), red (not covered), yellow (partially covered)

**Coverage Targets:**
- Unit tests: Aim for 80%+ coverage on business logic
- Integration tests: Focus on query correctness, not line coverage

### Excluding Files from Coverage

Some files are excluded from coverage by design:
- `src/bin/*.rs` - Binary entry points
- Test files themselves

## Test Suites in Detail

### BM25 Search Tests (`bm25_detailed_tests.rs`)

Tests ParadeDB BM25 full-text search capabilities:

| Test | Description |
|------|-------------|
| `test_basic_search` | Simple keyword matching |
| `test_bm25_fuzzy` | Fuzzy matching with typo tolerance |
| `test_bm25_phrase` | Phrase search with word order |
| `test_bm25_sorting` | Score-based sorting |
| `test_bm25_snippets` | Highlighted search snippets |
| `test_bm25_numeric_range` | Numeric range filtering |
| `test_bm25_boolean_filter` | Boolean field filtering |
| `test_category_filtering` | Category facet filtering |
| `test_field_specific_search` | Field-targeted search |
| `test_ranking` | BM25 score ranking verification |
| `test_special_characters` | Special character handling |
| `test_no_matches` | Empty result handling |

### Vector Search Tests (`products_vector_test.rs`)

Tests pgvector similarity search:

| Test | Description |
|------|-------------|
| `test_vector_cosine_similarity` | Cosine distance search |
| `test_vector_l2_distance` | Euclidean distance search |
| `test_vector_inner_product` | Inner product similarity |
| `test_vector_threshold_filter` | Similarity threshold filtering |
| `test_vector_with_category_filter` | Combined vector + category filter |
| `test_vector_with_price_filter` | Combined vector + price filter |
| `test_vector_with_rating_filter` | Combined vector + rating filter |
| `test_vector_featured_products` | Featured product boosting |
| `test_vector_hnsw_index_usage` | HNSW index verification |
| `test_vector_similarity_distribution` | Score distribution analysis |

### Hybrid Search Tests (`products_hybrid_test.rs`)

Tests combined BM25 + Vector search:

| Test | Description |
|------|-------------|
| `test_hybrid_weighted_combination` | 30/70 weight verification |
| `test_hybrid_balanced_weights` | 50/50 weight comparison |
| `test_hybrid_rrf_fusion` | Reciprocal Rank Fusion scoring |
| `test_hybrid_rrf_different_k` | RRF with different k values |
| `test_hybrid_score_distribution` | Combined score analysis |
| `test_hybrid_with_category_filter` | Hybrid + category filter |
| `test_hybrid_with_price_filter` | Hybrid + price filter |
| `test_hybrid_with_stock_filter` | Hybrid + stock filter |

### Database Configuration Tests (`dbtuning_test.rs`)

Validates PostgreSQL configuration settings:

| Test | Description |
|------|-------------|
| `test_shared_buffers_configured` | Memory buffer settings |
| `test_effective_cache_size_configured` | Cache size estimation |
| `test_work_mem_configured` | Per-operation memory |
| `test_maintenance_work_mem_configured` | Maintenance operation memory |
| `test_max_connections_configured` | Connection limit |
| `test_max_parallel_workers_configured` | Parallel query workers |
| `test_shared_preload_libraries_configured` | Extension loading |
| `test_checkpoint_completion_target_configured` | WAL checkpoint timing |
| `test_random_page_cost_configured` | Query planner cost |
| `test_effective_io_concurrency_configured` | I/O parallelism |

**Note:** The `shared_preload_libraries` test gracefully handles permission restrictions in CloudNativePG deployments where the `pg_read_all_settings` role is not available.

## Continuous Integration

### GitHub Actions Example

```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: sojoner/database:0.0.7
        env:
          POSTGRES_PASSWORD: custom_secure_password_123
          POSTGRES_DB: database
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Run tests with coverage
        env:
          DATABASE_URL: postgresql://postgres:custom_secure_password_123@localhost:5432/database
        run: |
          cd pg_search_tests
          cargo llvm-cov --features ssr --lcov --output-path lcov.info

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: pg_search_tests/lcov.info
```

## Debugging Test Failures

### View Test Output

```bash
# Show println! output
cargo test --features ssr -- --nocapture

# Show only failed test output
cargo test --features ssr -- --nocapture 2>&1 | grep -A 20 "FAILED"
```

### Run Single Test with Verbose Output

```bash
RUST_BACKTRACE=1 cargo test --test products_hybrid_test --features ssr test_hybrid_weighted_combination -- --nocapture
```

### Database Debugging

```bash
# Connect to the database
psql $DATABASE_URL

# Check extension status
SELECT * FROM pg_extension WHERE extname IN ('vector', 'pg_search');

# Verify test data exists
SELECT COUNT(*) FROM products.items;

# Test a search query directly
SELECT * FROM products.items WHERE description ||| 'wireless' LIMIT 5;
```

### Common Issues

#### "Connection refused"
- Ensure PostgreSQL is running: `make compose-ps` or `kubectl get pods`
- Check port forwarding: `make port-forward`

#### "Extension not found"
- Run initialization: `make compose-up` (waits for init scripts)
- Check logs: `make compose-logs`

#### "Permission denied" for `shared_preload_libraries`
- Expected in CloudNativePG - test handles this gracefully
- The parameter requires `pg_read_all_settings` role

#### Tests hang indefinitely
- Check for deadlocks in async tests
- Verify DATABASE_URL is correct
- Check connection pool limits

## Test Data

### Products Schema

The test database includes a `products` schema with sample e-commerce data:

```sql
-- Products table with BM25 + Vector indexes
SELECT
    id, name, brand, category,
    price, rating, in_stock
FROM products.items
LIMIT 5;
```

### Seeding Test Data

Test data is loaded during database initialization via:
- `docker-entrypoint-initdb.d/01-ai-extensions.sql`
- `sql_examples/09_products_data.sql`

To reload test data:

```bash
# Docker Compose
make compose-clean
make compose-up

# Kubernetes
make clean-all
make setup-all
```

## Performance Testing

### Benchmarking Search Modes

```bash
# Run with timing information
time cargo test --test products_hybrid_test --features ssr --release
```

### Database Query Analysis

```sql
-- Enable timing
\timing on

-- Analyze hybrid search query
EXPLAIN ANALYZE
WITH bm25_results AS (
    SELECT id, paradedb.score(id) AS score
    FROM products.items
    WHERE description ||| 'wireless headphones'
    LIMIT 100
),
vector_results AS (
    SELECT id, 1 - (description_embedding <=> $embedding) AS score
    FROM products.items
    ORDER BY description_embedding <=> $embedding
    LIMIT 100
)
SELECT * FROM bm25_results
FULL OUTER JOIN vector_results USING (id);
```

## Summary

| Command | Purpose |
|---------|---------|
| `cargo test --features ssr` | Run all tests |
| `cargo test --lib --features ssr` | Unit tests only |
| `cargo llvm-cov --features ssr --html` | HTML coverage report |
| `cargo llvm-cov --features ssr --lcov --output-path lcov.info` | LCOV coverage |
| `make test-sql-all` | SQL validation tests |
| `make validate-all` | Kubernetes validation tests |

---

**Last Updated:** 2024-12-17
