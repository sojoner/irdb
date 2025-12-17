# Web Search Application - Leptos + PostgreSQL

A full-stack Rust web application for product search, built with **Leptos 0.7+**, **ParadeDB pg_search (BM25)**, and **pgvector** for hybrid search capabilities.

## Features

### 🔍 Three Search Modes
- **BM25**: Keyword-based full-text search using ParadeDB
- **Vector**: Semantic similarity search using pgvector (1536-dim embeddings)
- **Hybrid**: Combined search (30% BM25 + 70% Vector) for best results

### ⚡ Advanced Filtering
- Category facets with counts
- Price range filtering
- Minimum rating filter
- In-stock toggle
- Brand facets
- Multiple sort options (relevance, price, rating, newest)

### 📊 Features Ready for Implementation
- Real-time facet updates
- Pagination with no duplicates
- Price histogram visualization
- Analytics dashboard
- Bulk product import (JSON)

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PostgreSQL 17.5 with extensions
# See main README for setup instructions

# Set environment variable
export DATABASE_URL="postgresql://postgres:password@localhost:5432/database"
```

### Running Tests

```bash
# Unit tests (fast, no database required)
cargo test --lib --features web

# Integration tests (requires database with sample data)
cargo test --test web_app_search_tests --features web

# Specific test
cargo test --test web_app_search_tests --features web test_hybrid_search_basic -- --nocapture
```

### Project Structure

```
src/web_app/
├── model/              # Shared data models (Product, SearchResult, etc.)
├── api/
│   ├── queries.rs     # Database query functions (BM25, Vector, Hybrid)
│   └── db.rs          # Connection pool setup
├── components/         # Leptos UI components (TODO)
└── pages/              # Page components (TODO)

tests/
└── web_app_search_tests.rs  # Integration tests
```

## API Reference

### Data Models

#### SearchMode
```rust
pub enum SearchMode {
    Bm25,       // Keyword matching
    Vector,     // Semantic similarity
    Hybrid,     // Combined (default)
}
```

#### SearchFilters
```rust
pub struct SearchFilters {
    pub categories: Vec<String>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
    pub min_rating: Option<f64>,
    pub in_stock_only: bool,
    pub sort_by: SortOption,
    pub page: u32,
    pub page_size: u32,
}
```

#### SearchResults
```rust
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub category_facets: Vec<FacetCount>,
    pub brand_facets: Vec<FacetCount>,
    pub price_histogram: Vec<PriceBucket>,
    pub avg_price: f64,
    pub avg_rating: f64,
}
```

### Query Functions

#### BM25 Search
```rust
use pg_search_tests::web_app::api::queries::search_bm25;

let filters = SearchFilters {
    price_min: Some(50.0),
    price_max: Some(150.0),
    ..Default::default()
};

let results = search_bm25(&pool, "wireless headphones", &filters).await?;
```

#### Vector Search
```rust
use pg_search_tests::web_app::api::queries::search_vector;

let results = search_vector(&pool, "gaming peripherals", &filters).await?;
```

#### Hybrid Search
```rust
use pg_search_tests::web_app::api::queries::search_hybrid;

let results = search_hybrid(&pool, "professional camera", &filters).await?;
```

## Example Usage

### Basic Search
```rust
use pg_search_tests::web_app::api::{db, queries};
use pg_search_tests::web_app::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    dotenv::dotenv().ok();
    let pool = db::create_pool().await?;

    // Search
    let filters = SearchFilters {
        categories: vec!["Electronics".to_string()],
        price_max: Some(500.0),
        sort_by: SortOption::PriceAsc,
        page: 0,
        page_size: 10,
        ..Default::default()
    };

    let results = queries::search_hybrid(&pool, "wireless", &filters).await?;

    // Display results
    for result in results.results {
        println!("{} - ${}",
            result.product.name,
            result.product.price
        );
        println!("  BM25: {:.3}, Vector: {:.3}, Combined: {:.3}",
            result.bm25_score.unwrap_or(0.0),
            result.vector_score.unwrap_or(0.0),
            result.combined_score
        );
    }

    Ok(())
}
```

### With Facets
```rust
let results = queries::search_bm25(&pool, "laptop", &filters).await?;

// Category facets
for facet in results.category_facets {
    println!("{}: {} products", facet.value, facet.count);
}

// Price histogram
for bucket in results.price_histogram {
    println!("${}-${}: {} products", bucket.min, bucket.max, bucket.count);
}
```

## Architecture

### Design Principles

#### 1. **Pure Functions**
All query functions are stateless and side-effect-free:
```rust
pub async fn search_bm25(
    pool: &PgPool,       // Explicit dependency
    query: &str,         // Input
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error>  // Typed output
```

#### 2. **Type Safety**
Strong typing throughout:
- Enums for modes and options (not strings)
- `rust_decimal::Decimal` for prices (not f64)
- Compile-time query checking with sqlx

#### 3. **Data-Oriented**
Focuses on data transformations:
- Input → Query → Results → Transformation → Output
- Easy to test, compose, and parallelize

#### 4. **Test-Driven**
Every module has comprehensive tests:
- Unit tests for pure logic
- Integration tests for database operations
- All tests passing ✅

### Query Performance

| Search Mode | Index Used | Performance | Best For |
|-------------|-----------|-------------|----------|
| BM25 | Inverted index | Fast | Keyword queries, exact matches |
| Vector | HNSW (ANN) | Fast | Semantic queries, concepts |
| Hybrid | Both | Moderate | General-purpose search |

### Hybrid Search Algorithm
1. Get top 100 results from BM25 (keyword matching)
2. Get top 100 results from Vector (semantic similarity)
3. FULL OUTER JOIN on product ID
4. Calculate combined score: `0.3 * BM25 + 0.7 * Vector`
5. Sort by combined score DESC
6. Apply filters (price, category, rating, stock)
7. Return paginated results with facets

## Testing

### Test Coverage

#### Unit Tests (9 tests)
- Model serialization/deserialization
- Default values and display traits
- Embedding generation format
- Type conversions

#### Integration Tests (8 tests)
- BM25 basic search
- BM25 with price filter
- BM25 with category filter
- Vector search basic
- Hybrid search basic (with score verification)
- Facet aggregations
- Pagination (no duplicates)
- Sort options

### Running Specific Tests

```bash
# Just the model tests
cargo test --lib web_app::model::tests

# Just the query unit tests
cargo test --lib web_app::api::queries::tests

# Integration test with output
cargo test --test web_app_search_tests test_hybrid_search_basic -- --nocapture

# All tests with features
cargo test --features web
```

## Development Roadmap

See [WEB_APP_PROGRESS.md](./WEB_APP_PROGRESS.md) for detailed progress tracking.

### ✅ Phase 1: Foundation (COMPLETE)
- [x] Project structure with feature flags
- [x] Core data models with serde support
- [x] Database query functions (BM25, Vector, Hybrid)
- [x] Helper functions (facets, histograms)
- [x] Database pool setup
- [x] Comprehensive test suite

### 🚧 Phase 2: Server Functions (IN PROGRESS)
- [ ] Leptos `#[server]` function wrappers
- [ ] Error handling and validation
- [ ] Import/export functionality
- [ ] Analytics aggregation functions

### 📋 Phase 3: UI Components (TODO)
- [ ] SearchBar with mode toggle
- [ ] FilterPanel with live facets
- [ ] ResultsGrid with product cards
- [ ] ProductDetailModal
- [ ] Pagination controls

### 📋 Phase 4: Pages (TODO)
- [ ] SearchPage (main interface)
- [ ] ImportPage (file upload)
- [ ] AnalyticsPage (dashboard)

### 📋 Phase 5: Styling & Polish (TODO)
- [ ] Tailwind CSS integration
- [ ] Responsive design
- [ ] Loading states
- [ ] Error boundaries

## Configuration

### Environment Variables

```bash
# Required
DATABASE_URL=postgresql://user:password@host:port/database

# Optional
DATABASE_MAX_CONNECTIONS=10
```

### Feature Flags

```toml
[features]
default = []
web = ["leptos", "actix-web", "leptos_actix"]  # Enable web framework
hydrate = ["leptos/hydrate"]  # Enable client-side hydration
```

## Performance Tips

1. **Use Hybrid Search by Default**: Best balance of relevance and performance
2. **Limit Page Size**: 10-20 results per page for optimal UX
3. **Enable Connection Pooling**: Reuse database connections
4. **Index Maintenance**: Vacuum and analyze regularly
5. **Monitor Query Performance**: Use EXPLAIN ANALYZE on slow queries

## Troubleshooting

### Common Issues

#### "Database pool not available"
- Ensure `DATABASE_URL` is set
- Check PostgreSQL is running
- Verify network connectivity

#### "Extension not found"
- Install ParadeDB: `CREATE EXTENSION pg_search;`
- Install pgvector: `CREATE EXTENSION vector;`
- Check extension versions

#### "Tests failing"
- Run `make setup-all` to initialize database
- Load sample data: `psql -f sql_examples/09_insert_sample_data.sql`
- Check test output with `--nocapture` flag

## Contributing

### Code Style
```bash
# Format code
cargo fmt

# Run linter
cargo clippy --features web -- -D warnings

# Run all tests
cargo test --all-features
```

### Adding New Search Modes
1. Add variant to `SearchMode` enum
2. Implement query function in `queries.rs`
3. Add unit tests
4. Add integration tests
5. Update documentation

## License

See main project LICENSE file.

## Links

- [Leptos Documentation](https://leptos.dev/)
- [ParadeDB Documentation](https://docs.paradedb.com/)
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [sqlx Documentation](https://docs.rs/sqlx/)

---

**Status:** Foundation Complete ✅ | UI Components In Progress 🚧
**Test Coverage:** 17/17 tests passing
**Last Updated:** 2025-12-17
