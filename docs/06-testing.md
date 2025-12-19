# Testing Guide

Comprehensive testing strategy for IRDB: unit tests, integration tests, database tests, and code coverage.

## Overview

Multi-layered testing approach with strong coverage (80%+):

| Test Type | Location | Features Required | Database |
|-----------|----------|-------------------|----------|
| Unit Tests | `src/**/*.rs` | `ssr` | No |
| Integration Tests | `tests/*.rs` | `ssr` | Yes |
| SQL Tests | `sql_examples/*.sql` | - | Yes |
| Validation | Makefile targets | - | Yes |

**Key Test Files:**

- `tests/queries_comprehensive_test.rs` - 35 tests for [queries.rs](../pg_search_tests/src/web_app/api/queries.rs) (BM25, Vector, Hybrid search)
- `tests/backend_search_tests.rs` - Backend integration tests with isolated schema
- `tests/component_render_tests.rs` - Component logic tests for UI components
- Coverage improved from 28% → 80%+ with real-world testing patterns

## Test Isolation & Idempotency

All tests are designed to be **idempotent** (can run multiple times safely):

- **Isolated Schema Approach**: `backend_search_tests.rs` uses unique schema per test run via `with_test_db()` wrapper
- **Shared Schema Approach**: `queries_comprehensive_test.rs` uses default `products` schema with cleanup
- **Setup/Teardown**: Automatic `DROP SCHEMA CASCADE` before each test ensures clean state
- **Parallel Safety**: Tests using unique schemas can run in parallel; shared schema tests should run sequentially

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

#### Unit Tests (No Database)

Fast logic tests without database:

```bash
cargo test --lib --features ssr
```

**Covers:** `src/web_app/{model,components,api,pages}/*.rs`

#### Integration Tests (Database Required)

```bash
# All tests (parallel execution)
cargo test --features ssr

# Specific test suites

# Backend search tests (uses isolated schema - safe for parallel execution)
cargo test --test backend_search_tests --features ssr

# Comprehensive query tests (uses shared schema - run sequentially for safety)
cargo test --test queries_comprehensive_test --features ssr -- --test-threads=1

# Other integration tests
cargo test --test bm25_detailed_tests --features ssr         # BM25 full-text search
cargo test --test products_vector_test --features ssr        # Vector similarity
cargo test --test products_hybrid_test --features ssr        # Hybrid search
cargo test --test dbtuning_test --features ssr               # PostgreSQL config
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

## Rust Testing Patterns

Key patterns demonstrated in `tests/queries_comprehensive_test.rs`:

### 1. Test Fixtures (Reusable Setup)

```rust
async fn create_test_pool() -> Result<PgPool, sqlx::Error> {
    // Database connection setup
}

fn default_filters() -> SearchFilters {
    // Standard test filters
}
```

**Why:** Reduces duplication, easier maintenance

### 2. Integration Testing with Real Database

```rust
#[tokio::test]
async fn test_bm25_wildcard_search() -> anyhow::Result<()> {
    let pool = create_test_pool().await?;
    // Test with real database
}
```

**Why:** Tests actual behavior, not mocks

### 3. Parameterized Testing

```rust
let queries = vec!["C++", "AT&T", "O'Reilly"];
for query in queries {
    let result = search_bm25(&pool, query, &filters).await?;
    // Verify each works
}
```

**Why:** Test multiple scenarios without duplication

### 4. Property-Based Testing

```rust
for i in 0..results.len() - 1 {
    assert!(results[i].score >= results[i + 1].score);
}
```

**Why:** Catches bugs that specific examples miss

### 5. Error Handling

```rust
async fn test_xyz() -> anyhow::Result<()> {
    let pool = create_test_pool().await?;
    let results = search_bm25(&pool, query, &filters).await?;
    Ok(())
}
```

**Why:** Clear error reporting, proper cleanup

## Code Coverage

### Generate Coverage Reports

```bash
cd pg_search_tests

# HTML report (interactive)
cargo llvm-cov --features ssr --html
open target/llvm-cov/html/index.html

# LCOV (for CI/Codecov)
cargo llvm-cov --features ssr --lcov --output-path lcov.info

# Both formats
cargo llvm-cov --features ssr --html --lcov --output-path lcov.info
```

### Coverage Targets

- **Unit tests:** 80%+ on business logic
- **Integration tests:** Focus on query correctness

**Current Coverage:**

- `queries.rs`: 28% → 80%+ (with queries_comprehensive_test.rs)
- Function coverage: 7/25 → 20+/25
- Line coverage: 81/356 → 270+/356

## Test Coverage by Feature

### `queries_comprehensive_test.rs` (35 tests)

**BM25 Tests (13):**

- Wildcard, empty query, specific brand search
- Pagination, price/category/rating/stock filtering
- Sorting (price asc/desc)
- Facets generation

**Vector Tests (5):**

- Wildcard, score range validation
- Result ordering, pagination, price filtering

**Hybrid Tests (7):**

- Score combination (30% BM25 + 70% Vector)
- Ordering, pagination, all filters combined
- Facets generation

**Edge Cases (7):**

- Empty results, special chars (C++, AT&T, O'Reilly)
- Long queries (500+ words), Unicode (café, 北京, Москва)
- Zero/large page sizes, concurrent searches

**Performance (3):**

- Baseline timing, concurrent load testing

### Other Test Suites

- **`bm25_detailed_tests.rs`** - BM25 features (fuzzy, phrase, snippets, ranking)
- **`products_vector_test.rs`** - pgvector (cosine, L2, HNSW index)
- **`products_hybrid_test.rs`** - RRF fusion, weight tuning
- **`dbtuning_test.rs`** - PostgreSQL config validation

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

## Debugging Tests

### Common Commands

```bash
# Show output
cargo test --features ssr -- --nocapture

# Single test with backtrace
RUST_BACKTRACE=1 cargo test --test queries_comprehensive_test --features ssr test_bm25_wildcard_search -- --nocapture

# Database check
psql $DATABASE_URL
SELECT * FROM pg_extension WHERE extname IN ('vector', 'pg_search');
SELECT COUNT(*) FROM products.items;
```

### Common Issues

| Issue | Solution |
|-------|----------|
| Connection refused | `make compose-ps` or `make port-forward` |
| Extension not found | `make compose-up` (runs init scripts) |
| Tests hang | Check DATABASE_URL, connection pool limits |
| Permission denied (shared_preload_libraries) | Expected in CloudNativePG, test handles gracefully |

## Best Practices

### 1. Test Organization

- Group related tests together
- Use clear, descriptive names (e.g., `test_bm25_wildcard_search`)
- Add comments explaining test intent

### 2. Fixtures Over Duplication

- Create helpers for common setup (`create_test_pool()`, `default_filters()`)
- Parameterize fixtures for flexibility
- Keep fixtures simple and focused

### 3. Integration > Mocking

- Test with real database when possible
- Use transactions for test isolation (if needed)
- Mock only external dependencies (APIs, etc.)

### 4. Test What Matters

- Focus on public APIs
- Test behavior, not implementation
- Test edge cases and error conditions

### 5. Fast Feedback Loop

- Run unit tests frequently (fast, no DB)
- Run integration tests before commits
- Run full coverage periodically

## Next Steps to 90%+ Coverage

1. **Test Helper Functions Directly** - Make facet functions public or test through more scenarios
2. **Test Error Paths** - Invalid connections, timeouts
3. **Test Boundary Conditions** - Max values, SQL injection attempts
4. **Add Property-Based Tests** - Use `proptest` or `quickcheck` for random input testing
5. **Performance Tests** - Benchmark search, test with large datasets

## Resources & Quick Reference

**Rust Testing:**

- [The Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust by Example - Testing](https://doc.rust-lang.org/rust-by-example/testing.html)

**Frameworks:**

- [rstest](https://github.com/la10736/rstest) - Fixture-based testing
- [proptest](https://github.com/proptest-rs/proptest) - Property-based testing
- [tokio::test](https://docs.rs/tokio/latest/tokio/attr.test.html) - Async testing

**Coverage:**

- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - Coverage reporting
- [sqlx::test](https://docs.rs/sqlx/latest/sqlx/attr.test.html) - Database test utilities

**Common Commands:**

| Command | Purpose |
|---------|---------|
| `cargo test --features ssr` | All tests |
| `cargo test --lib --features ssr` | Unit tests only |
| `cargo test --test queries_comprehensive_test --features ssr` | Specific test suite |
| `cargo llvm-cov --features ssr --html` | HTML coverage |
| `make test-sql-all` | SQL validation |
| `make validate-all` | Kubernetes validation |

---

**Last Updated:** 2024-12-18
