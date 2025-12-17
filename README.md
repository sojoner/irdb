# IRDB - Learning Hybrid Search in PostgreSQL & Rust

A study project exploring **hybrid search** by combining BM25 full-text search with vector similarity search in PostgreSQL.

## What is This?

This project demonstrates how to combine two complementary search approaches:

- **Lexical search (BM25)** - Keyword matching for brands, models, technical terms
- **Semantic search (Vector)** - Meaning-based similarity for natural language queries
- **Weighted combination** - Blending both methods (30% BM25 + 70% Vector)

## Key Technologies

- **PostgreSQL 17.5** - Latest stable PostgreSQL
- **ParadeDB pg_search 0.20.2** - BM25 full-text search with custom operators
- **pgvector 0.8.0** - Vector similarity search with HNSW indexing
- **Rust** - Type-safe, data-oriented API layer
- **Leptos 0.7+** - Reactive web framework (in progress)
- **CloudNativePG** - Kubernetes operator for high availability

## Quick Start

### Docker Compose (Local Development)

```bash
# Start PostgreSQL + pgAdmin
make compose-up

# Run validation tests
make validate-all

# Connect to database
psql -h localhost -U postgres -d database -p 5432
# Password: custom_secure_password_123
```

Access pgAdmin at <http://localhost:5433>

### Kubernetes (Production)

```bash
# Complete setup (local kind cluster)
make setup-all

# Run tests
make test-all

# Deploy to production cluster
cd k8s/
helm install irdb-postgres . \
  --namespace databases \
  --create-namespace \
  -f values-prod.yaml
```

## Documentation

Comprehensive documentation is available in the [docs/](./docs/) directory:

1. **[Architecture Overview](./docs/01-architecture.md)** - System design, technologies, and architectural decisions
2. **[Deployment Guide](./docs/02-deployment.md)** - Docker Compose and Kubernetes deployment
3. **[Hybrid Search Deep Dive](./docs/03-hybrid-search.md)** - Implementation details, algorithm, and examples
4. **[Web Application Development](./docs/04-web-app.md)** - Building the Leptos-based UI
5. **[References & Resources](./docs/05-references.md)** - Papers, documentation, and learning resources

## Project Status

- ✅ **Database Foundation** - PostgreSQL 17.5 with ParadeDB and pgvector
- ✅ **Hybrid Search Implementation** - 30/70 weighted combination with FULL OUTER JOIN
- ✅ **Rust API Layer** - Pure functional queries with compile-time checking
- ✅ **Comprehensive Tests** - 17/17 tests passing (unit + integration)
- ✅ **Docker Deployment** - Multi-stage build (~850MB image)
- ✅ **Kubernetes Deployment** - Production-ready Helm charts with CloudNativePG
- 🚧 **Leptos Web UI** - Components and pages (in progress)

## Testing

### Run All Tests

```bash
cd pg_search_tests

# Unit tests (fast, no database)
cargo test --lib --features web

# Integration tests (requires database)
export DATABASE_URL="postgresql://postgres:password@localhost:5432/database"
cargo test --test web_app_search_tests --features web

# Specific test with output
cargo test test_hybrid_search_basic -- --nocapture
```

### Test Coverage

**17/17 tests passing:**

- 9 unit tests (models, type conversions)
- 8 integration tests (BM25, Vector, Hybrid search with real database)

## Contributing

Contributions are welcome! Please see:

- [Architecture documentation](./docs/01-architecture.md) for design principles
- [Developer guide](./.claude/CLAUDE.md) for setup and conventions
- [Test suite](./pg_search_tests/README_WEB_APP.md) for testing guidelines

## License

Apache License 2.0 - See [LICENSE](./LICENSE) file for details.

This is a study project demonstrating hybrid search techniques. The code is provided as-is for learning and research purposes.

## Acknowledgments

Built with open-source technologies:

- **PostgreSQL Global Development Group** - Core database
- **ParadeDB Team** - BM25 search extension
- **pgvector Contributors** - Vector similarity search
- **Leptos Team** - Rust web framework
- **CloudNativePG Team** - Kubernetes operator
- **Rust Community** - Language and ecosystem

---

**Study Focus**: Hybrid Search in PostgreSQL & Rust
**Last Updated**: 2025-12-17

---
© 2025 sojoner
