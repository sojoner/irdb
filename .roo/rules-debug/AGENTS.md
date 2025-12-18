# Project Debug Rules (Non-Obvious Only)

- Database tests require DATABASE_URL environment variable (export DATABASE_URL=postgres://...)
- Test feature flags: cargo test --features db-tools (enables postgres/sqlx dependencies)
- Init scripts only run on first database creation - use docker-compose down -v to reset
- Kubernetes DB access requires port-forward: make port-forward (separate terminal)
- pgAdmin runs on localhost:5433 with pre-configured connection (no manual setup needed)
- ParadeDB extensions may show warnings in logs but still function (expected behavior)
- Hybrid search weights: 30% BM25 + 70% Vector (debug scoring issues here)
- Query embedding generation uses random vectors (MVP) - check generate_query_embedding()