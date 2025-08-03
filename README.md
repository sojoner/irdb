# IR DB - AI-Enhanced PostgreSQL Platform

PostgreSQL 17.5 with pgvector and ParadeDB extensions, optimized for RAG (Retrieval Augmented Generation) applications.

## Features

- **PostgreSQL 17.5** - Latest stable release
- **pgvector v0.8.0** - Vector similarity search (1536 dimensions for OpenAI embeddings)
- **ParadeDB pg_search v0.20.x** - Full-text search with BM25 ranking
- **Hybrid Search** - Combines vector similarity (70%) and text search (30%)
- **Pre-configured RAG Schema** - Ready-to-use tables, indexes, and functions
- **Multi-stage Docker Build** - Optimized ~850MB final image
- **Production-Ready Helm Chart** - CloudNativePG for Kubernetes

## Quick Start

Choose your deployment method:

## Documentation

### Deployment Guides

- **[README_DOCKER.md](README_DOCKER.md)** - Docker Compose deployment for local development
  - pgAdmin setup, backup/restore, performance tuning, Docker-specific troubleshooting

- **[README_K8s.md](README_K8s.md)** - Kubernetes deployment with CloudNativePG operator
  - High availability, scaling, monitoring, Kubernetes-specific troubleshooting

- **[k8s/README.md](k8s/README.md)** - Helm chart reference and CloudNativePG best practices
  - Chart configuration, resource management, backup strategies, production checklist

### Developer Resources

- **[.claude/CLAUDE.md](.claude/CLAUDE.md)** - Development guide for contributors using Claude Code
- **[pg_search_tests/README.md](pg_search_tests/README.md)** - Test suite documentation
  - BM25 full-text search tests
  - Vector search validation
  - Hybrid search functionality
  - Database configuration tests

## Testing

Run the test suite with:

```bash
cd pg_search_tests
cargo test
```

**Test categories:**

- **advanced_search_tests.rs** - ParadeDB pg_search 0.20.x BM25 syntax and fuzzy matching
- **bm25_detailed_tests.rs** - Comprehensive BM25 search scenarios (12 tests)
- **dbtuning_test.rs** - PostgreSQL configuration validation (17 tests)
- **integration_tests.rs** - Basic integration testing

All tests use isolated temporary tables to ensure they can run independently.

## Resources

- [PostgreSQL 17 Documentation](https://www.postgresql.org/docs/17/)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [ParadeDB Documentation](https://docs.paradedb.com/)
- [CloudNativePG Documentation](https://cloudnative-pg.io/documentation/)

## Structure

```bash
.
├── .claude
│   ├── 2025-12-11-documentation.md
│   ├── 2025-12-11-pg_search-rust-test-examples.md
│   ├── CLAUDE.md
│   └── settings.local.json
├── .git
│   ├── hooks
│   ├── info
│   ├── logs
│   ├── objects
│   ├── refs
│   ├── COMMIT_EDITMSG
│   ├── config
│   ├── description
│   ├── FETCH_HEAD
│   ├── HEAD
│   ├── index
│   └── ORIG_HEAD
├── docker-entrypoint-initdb.d
│   ├── 00-extensions.sql
│   ├── 01-ai-extensions.sql
│   ├── 02-validating-bm25.sql
│   ├── 03-simple-vector-test.sql
│   └── 05-comprehensive-test.sql
├── k8s
│   ├── charts
│   ├── templates
│   ├── .helmignore
│   ├── Chart.lock
│   ├── Chart.yaml
│   ├── README.md
│   ├── setup.sh
│   ├── values-dev.yaml
│   ├── values-prod.yaml
│   ├── values.yaml
│   └── verify-extensions.sh
├── pg_search_tests
│   ├── sql_examples
│   ├── src
│   ├── target
│   ├── tests
│   ├── .gitignore
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── README.md
├── .DS_Store
├── .gitignore
├── docker-compose.yml
├── Dockerfile
├── kind-config.yaml
├── Makefile
├── postgresql.conf
├── README_DOCKER.md
├── README_K8s.md
└── README.md
```

## License

[Add your license here]

## Contributing

[Add contributing guidelines here]

---
© 2025 Sojoner
