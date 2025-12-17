# IRDB Documentation

Welcome to the IRDB (Information Retrieval Database) documentation. This project demonstrates hybrid search capabilities combining PostgreSQL full-text search with vector similarity search.

## What is IRDB?

IRDB is a study project exploring **Hybrid Search in PostgreSQL & Rust**. It combines:
- **BM25 full-text search** (ParadeDB) for keyword matching
- **Vector similarity search** (pgvector) for semantic understanding
- **Hybrid search** that intelligently combines both approaches (30% BM25 + 70% Vector)

## Documentation Structure

1. **[Architecture](./01-architecture.md)** - System design, technologies, and architectural decisions
2. **[Deployment Guide](./02-deployment.md)** - How to deploy using Docker Compose or Kubernetes
3. **[Hybrid Search Deep Dive](./03-hybrid-search.md)** - Implementation details, SQL queries, and examples
4. **[Web Application Development](./04-web-app.md)** - Building the Leptos-based web UI
5. **[References & Resources](./05-references.md)** - Upstream documentation, papers, and related projects

## Quick Start

```bash
# Start with Docker Compose
make compose-up

# Or deploy to Kubernetes
make setup-all

# Run tests
make test-all
```

## Key Features

- PostgreSQL 17.5 with ParadeDB pg_search (v0.20.2) and pgvector (v0.8.0)
- Hybrid search combining lexical and semantic understanding
- Production-ready Kubernetes deployment with CloudNativePG
- Leptos-based web application (in progress)
- Comprehensive test suite with real database integration tests

## Learning Goals

This project explores:
- How to implement hybrid search at the database level
- Weighted scoring algorithms for combining BM25 and vector results
- HNSW (Hierarchical Navigable Small World) index performance
- Test-driven development with Rust and PostgreSQL
- Pure functional design patterns for database queries

## Project Status

- ✅ Database foundation with ParadeDB and pgvector
- ✅ Hybrid search implementation with configurable weights
- ✅ Docker and Kubernetes deployment
- ✅ Rust API layer with pure functional queries
- ✅ Comprehensive test suite (17/17 passing)
- 🚧 Leptos web UI components (in progress)

## License

Apache License 2.0 - See [LICENSE](../LICENSE) file for details.
