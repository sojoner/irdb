# 🧪 IRDB Test Suite

Comprehensive Rust test suite for **hybrid search** with BM25 + Vector similarity.

## 🚀 Quick Start

```bash
# Set database connection
export DATABASE_URL="postgresql://postgres:password@localhost:5432/database"

# Run all tests
cargo test --features ssr

# Run specific test with output
cargo test test_hybrid_search_basic --features ssr -- --nocapture
```

## 📊 Test Coverage

**✅ 17/17 tests passing**

### Unit Tests (9)
- Data models and serialization
- Type conversions and defaults
- Display traits

### Integration Tests (8)
- 🔍 **BM25 search** - Keyword matching with filters
- 🎯 **Vector search** - Semantic similarity
- ⚡ **Hybrid search** - 30% BM25 + 70% Vector
- 📈 **Facets** - Category and brand aggregations
- 📄 **Pagination** - No duplicates across pages
- 🔢 **Sorting** - Price, rating, relevance

## 🏗️ Project Structure

```
pg_search_tests/
├── src/web_app/
│   ├── model/              # 📦 Data models (Product, SearchFilters, etc.)
│   ├── api/                # 🔧 Pure functional queries (SSR only)
│   │   ├── queries.rs      # search_bm25(), search_vector(), search_hybrid()
│   │   └── db.rs           # Connection pool
│   ├── server_fns.rs       # 🌐 Leptos server functions
│   ├── app.rs              # 🏠 Root App component
│   ├── components/         # 🎨 Leptos UI components
│   └── pages/              # 📱 Page components
└── tests/
    └── web_app_search_tests.rs  # Integration tests
```

## 🎯 Key Features

- **Pure functions** - No side effects, easy to test
- **Type-safe queries** - Compile-time checking with sqlx
- **Real database tests** - Integration tests against PostgreSQL
- **Data-oriented** - Focus on data transformations

## 📚 Documentation

- **[Web App Guide](../docs/04-web-app.md)** - Leptos application development
- **[Architecture](../docs/01-architecture.md)** - System design
- **[Hybrid Search](../docs/03-hybrid-search.md)** - Algorithm deep dive

## 🔬 Running Specific Tests

```bash
# All unit tests
cargo test --lib --features ssr

# All integration tests
cargo test --test web_app_search_tests --features ssr

# BM25 search only
cargo test test_bm25 --features ssr

# Hybrid search with debug output
cargo test test_hybrid_search_basic --features ssr -- --nocapture

# Run the web app
cargo leptos watch
```

## 🎨 Test Philosophy

> "Test data transformations, not implementations"

Every test validates **what** the code does, not **how** it does it. This makes tests resilient to refactoring while catching real bugs.

---

**Status**: ✅ Foundation complete | ✅ UI complete
