# IR DB PostgreSQL pg_search Tests

Rust test suite for exploring IR DB features including pgvector, pg_search (BM25), and hybrid search capabilities.

## Project Structure

```
pg_search_tests/
├── Cargo.toml
├── src/
│   └── bin/
│       ├── connection_test.rs      # Database connectivity validation
│       ├── bm25_test.rs            # Full-text search with BM25 ranking
│       ├── vector_test.rs          # Vector similarity search
│       └── hybrid_search_test.rs    # Hybrid vector + BM25 search
└── README.md
```

## Prerequisites

- Rust 1.70+ (with Cargo)
- Running PostgreSQL 17.5 instance with:
  - pgvector extension
  - pg_search extension
  - ai_data schema initialized

The tests connect to the IR DB Kubernetes deployment via NodePort service at `192.168.178.181:30432`.

## Building

```bash
cd pg_search_tests
cargo build --release
```

## Running Tests

Run individual tests:

```bash
# Test basic connectivity
cargo run --bin connection_test

# Test BM25 full-text search
cargo run --bin bm25_test

# Test vector similarity search
cargo run --bin vector_test

# Test hybrid search
cargo run --bin hybrid_search_test
```

Run all tests:

```bash
cargo build --release
for bin in connection_test bm25_test vector_test hybrid_search_test; do
    echo "Running $bin..."
    cargo run --bin $bin --release
done
```

## Current Status

The basic connection test validates:
- ✓ Database connectivity
- ✓ PostgreSQL version
- ✗ pgvector extension (not yet initialized)
- ✗ pg_search extension (not yet initialized)
- ✗ ai_data schema (not yet initialized)

To initialize the database with extensions and schema, you need to run the initialization scripts from the parent project:

```bash
# Via Kubernetes init scripts (requires cluster rebuild)
cd ..
make clean-all
make setup-all
```

## Dependencies

- `tokio` - Async runtime
- `sqlx` - SQL toolkit with compile-time query checking
- `pgvector` - Vector type support
- `serde` / `serde_json` - JSON serialization
- `anyhow` - Error handling

## Next Steps

1. Initialize the database with extensions and schema
2. Implement BM25 search tests in `bm25_test.rs`
3. Implement vector similarity tests in `vector_test.rs`
4. Implement hybrid search tests in `hybrid_search_test.rs`
5. Add integration tests for complex search scenarios
