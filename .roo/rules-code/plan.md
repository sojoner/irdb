# AGENTS.md Creation Plan

## Analysis Steps

1. **Check for existing AGENTS.md files**
   - AGENTS.md (in project root)
   - .roo/rules-code/AGENTS.md
   - .roo/rules-debug/AGENTS.md
   - .roo/rules-ask/AGENTS.md
   - .roo/rules-architect/AGENTS.md

2. **Identify stack and build system**
   - Rust with Cargo.toml, Leptos web framework
   - PostgreSQL with pg_search and pgvector extensions
   - Docker, Kubernetes, Makefile for orchestration
   - Node.js for Tailwind CSS build

3. **Extract essential commands**
   - Build: `make compose-build`, `cargo build`
   - Test: `cargo test --features db-tools`, `make test-all`
   - Run: `make compose-up`, `cargo run --features ssr`
   - Lint: No explicit linter configured

4. **Map core architecture**
   - Hybrid search combining BM25 (30%) and vector similarity (70%)
   - Pure functions with sqlx for database queries
   - Leptos web app with server-side rendering
   - Multi-stage Docker build for PostgreSQL with extensions

5. **Document critical patterns discovered**
   - Hybrid search uses FULL OUTER JOIN with weighted scoring
   - Query embedding generation is MVP (random vectors)
   - Database pool initialized globally with OnceLock
   - Feature flags control compilation (ssr, hydrate, db-tools)

6. **Extract code style conventions**
   - Strong typing with enums for options (SearchMode, SortOption)
   - Pure functions with explicit dependency injection
   - Data-oriented design with From trait conversions
   - Async/await throughout for database operations

7. **Testing specifics**
   - Tests require DATABASE_URL environment variable
   - Feature flags needed: `cargo test --features db-tools`
   - Integration tests in tests/ directory
   - SQL test scripts in sql_examples/ with psql

8. **Compile AGENTS.md files**
   - Include CLAUDE.md content as existing AI rules
   - Focus on non-obvious information only
   - Create mode-specific files in .roo/rules-*/ directories