# Project Architecture Rules (Non-Obvious Only)

- Pure functions with explicit dependency injection (pool parameter) - no global database state
- Data-oriented design: Input → SQL transformation → Row → From trait → API types
- Feature flags control compilation targets: ssr/hydrate/db-tools prevent dependency mixing
- Multi-stage Docker build compiles PostgreSQL extensions from source for minimal image size
- Hybrid search uses FULL OUTER JOIN with weighted scoring (30% BM25 + 70% Vector)
- Database schema split: ai_data (RAG documents) and products (e-commerce) with different indexing
- ParadeDB BM25 index created via CALL paradedb.create_bm25() function, not standard CREATE INDEX
- pgvector HNSW indexes use vector_cosine_ops for optimized similarity search