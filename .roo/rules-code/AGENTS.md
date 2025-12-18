# Project Coding Rules (Non-Obvious Only)

- Database queries must be pure functions taking explicit PgPool parameter (no global state)
- Use ParadeDB `|||` operator for BM25 disjunction, not standard PostgreSQL operators
- Vector similarity requires `<=>` operator with pgvector, not `<->` or other distance functions
- Hybrid search combines results with FULL OUTER JOIN and weighted scoring (30% BM25 + 70% Vector)
- Query embeddings are currently random vectors (MVP) - production needs real embedding API
- Feature flags (`ssr`, `hydrate`, `db-tools`) control compilation targets - don't mix dependencies
- Use rust_decimal for prices, not f64 (precision issues)
- Enums for SearchMode/SortOption instead of strings for type safety
- Database pool initialized globally with OnceLock - call init_db() before queries