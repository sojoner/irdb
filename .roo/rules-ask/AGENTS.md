# Project Documentation Rules (Non-Obvious Only)

- pg_search_tests/ is the main Rust application (not just tests) with Leptos web framework
- Hybrid search combines ParadeDB BM25 (30%) with pgvector similarity (70%)
- Multi-stage Docker build compiles PostgreSQL extensions from source for size optimization
- Feature flags control compilation: ssr (server), hydrate (WASM), db-tools (CLI)
- Database schema has two parts: ai_data (RAG docs) and products (e-commerce search)
- Query embeddings use random vectors (MVP implementation) - production needs embedding API
- Makefile provides complete orchestration for Docker Compose and Kubernetes workflows