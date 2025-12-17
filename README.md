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

## Documentation

Comprehensive documentation is available in the [docs/](./docs/) directory:

1. **[Architecture Overview](./docs/01-architecture.md)** - System design, technologies, and architectural decisions
2. **[Deployment Guide](./docs/02-deployment.md)** - Docker Compose and Kubernetes deployment
3. **[Hybrid Search Deep Dive](./docs/03-hybrid-search.md)** - Implementation details, algorithm, and examples
4. **[Web Application Development](./docs/04-web-app.md)** - Building the Leptos-based UI
5. **[References & Resources](./docs/05-references.md)** - Papers, documentation, and learning resources
6. **[Testing Guide](./docs/06-testing.md)** - Testing and coverage notes

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

© 2025 sojoner
