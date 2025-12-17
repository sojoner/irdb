# Web Application Development

A full-stack Rust web application for product search, built with **Leptos 0.8**, **ParadeDB pg_search (BM25)**, and **pgvector** for hybrid search capabilities.

## Architecture

The web application follows a functional, data-oriented design:

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser (WASM)                          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Leptos Components (Reactive UI)                       │ │
│  │  - SearchBar, FilterPanel, ResultsGrid                 │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────────────────┘
                       │ HTTP (Leptos Server Functions)
┌──────────────────────▼──────────────────────────────────────┐
│                   Actix-web Server                           │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  API Layer (Pure Functions)                            │ │
│  │  - search_bm25(), search_vector(), search_hybrid()     │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────────────────┘
                       │ sqlx (Async PostgreSQL)
┌──────────────────────▼──────────────────────────────────────┐
│              PostgreSQL with Extensions                      │
│  - ParadeDB (BM25)  - pgvector (Embeddings)                 │
└─────────────────────────────────────────────────────────────┘
```

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

### 📊 UI Features
- Real-time facet updates
- Pagination with no duplicates
- Product detail modal
- Responsive Tailwind CSS design
- Loading states and error handling

## Project Structure

```
pg_search_tests/
├── src/
│   ├── lib.rs                    # Library entry point with hydrate function
│   ├── bin/
│   │   └── main.rs               # Actix-web server with SSR
│   └── web_app/
│       ├── mod.rs                # Module definitions
│       ├── model/                # Shared data models (Client + Server)
│       │   └── mod.rs            # Product, SearchFilters, SearchResults, etc.
│       ├── server_fns.rs         # Leptos #[server] function declarations
│       ├── app.rs                # Root App component with routing
│       ├── api/                  # Server-side logic (SSR only)
│       │   ├── db.rs             # Connection pool setup
│       │   └── queries.rs        # Search functions (BM25, Vector, Hybrid)
│       ├── components/           # Leptos UI components
│       │   ├── mod.rs
│       │   ├── common.rs         # Loading, Button, Modal, StarRating, etc.
│       │   ├── search.rs         # SearchBar, FilterPanel, Pagination
│       │   └── product.rs        # ProductCard, ProductDetail, ResultsGrid
│       └── pages/                # Page components
│           ├── mod.rs
│           └── search.rs         # SearchPage (main interface)
├── tests/
│   ├── web_app_search_tests.rs   # Integration tests
│   └── backend_search_tests.rs   # Backend search tests
├── Cargo.toml                    # With cargo-leptos metadata
└── Leptos.toml                   # Leptos configuration
```

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-leptos
cargo install cargo-leptos

# Install PostgreSQL 17.5 with extensions
# See main README for setup instructions

# Set environment variable
export DATABASE_URL="postgresql://postgres:password@localhost:5432/database"
```

### Running the Web App

```bash
# Navigate to the project
cd pg_search_tests

# Run development server (includes hot-reload)
cargo leptos watch
```

The app will be available at <http://127.0.0.1:3000>

### Running Tests

```bash
# Unit tests (fast, no database required)
cargo test --lib --features ssr

# Integration tests (requires database with sample data)
cargo test --test web_app_search_tests --features ssr

# Backend search tests
cargo test --test backend_search_tests --features ssr

# Specific test with output
cargo test --test web_app_search_tests --features ssr test_hybrid_search_basic -- --nocapture
```

## Configuration

### Feature Flags

```toml
[features]
default = []
# Database tools for tests and CLI binaries
db-tools = ["postgres", "tokio", "sqlx", "dotenv", "pgvector", "tracing-subscriber"]
# Server-side rendering with database access
ssr = ["leptos/ssr", "leptos_meta/ssr", "leptos_router/ssr", "leptos_actix", ...]
# Client-side hydration (WASM) - NO database dependencies
hydrate = ["leptos/hydrate", "leptos_meta", "leptos_router", ...]
```

### Leptos Configuration (Cargo.toml)

```toml
[package.metadata.leptos]
bin-target = "pg_search_tests"
output-name = "pg_search_tests"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "src/web_app/style/main.css"
assets-dir = "public"
reload-port = 3001
env = "DEV"
bin-features = ["ssr"]
lib-features = ["hydrate"]
watch = true
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

### Server Functions

Server functions are defined in `server_fns.rs` and automatically generate HTTP endpoints:

```rust
#[server(SearchProducts, "/api")]
pub async fn search_products(
    query: String,
    mode: SearchMode,
    filters: SearchFilters,
) -> Result<SearchResults, ServerFnError>

#[server(GetProduct, "/api")]
pub async fn get_product(id: i32) -> Result<Product, ServerFnError>

#[server(GetAnalytics, "/api")]
pub async fn get_analytics() -> Result<AnalyticsData, ServerFnError>
```

### Database Query Functions

```rust
use pg_search_tests::web_app::api::queries;

// BM25 keyword search
let results = queries::search_bm25(&pool, "wireless headphones", &filters).await?;

// Vector semantic search
let results = queries::search_vector(&pool, "gaming peripherals", &filters).await?;

// Hybrid search (30% BM25 + 70% Vector)
let results = queries::search_hybrid(&pool, "professional camera", &filters).await?;
```

## Component Reference

### Common Components (`components/common.rs`)

- `Loading` - Spinner with optional message
- `ErrorDisplay` - Error message display
- `Button` / `SecondaryButton` - Styled buttons
- `ModalWrapper` - Modal dialog with backdrop
- `StarRating` - 5-star rating display
- `Badge` - Small label/tag
- `TextInput` / `SelectString` / `Checkbox` - Form inputs
- `PriceDisplay` - Formatted price

### Search Components (`components/search.rs`)

- `SearchBar` - Input with mode toggle
- `SearchModeToggle` - BM25/Vector/Hybrid selector
- `SortDropdown` - Sort option selector
- `CategoryFacets` - Category checkboxes with counts
- `PriceRangeFilter` - Min/max price inputs
- `RatingFilter` - Minimum rating buttons
- `InStockToggle` - Stock availability checkbox
- `FilterPanel` - Complete filter sidebar
- `Pagination` - Page navigation

### Product Components (`components/product.rs`)

- `ProductCard` - Grid card for search results
- `ProductDetail` - Full product information
- `ResultsGrid` - Grid layout with empty state
- `ScoreBreakdown` - Debug score display

## Hybrid Search Algorithm

1. Get top 100 results from BM25 (keyword matching)
2. Get top 100 results from Vector (semantic similarity)
3. FULL OUTER JOIN on product ID
4. Calculate combined score: `0.3 * BM25 + 0.7 * Vector`
5. Sort by combined score DESC
6. Apply filters (price, category, rating, stock)
7. Return paginated results with facets

### Query Performance

| Search Mode | Index Used | Performance | Best For |
|-------------|-----------|-------------|----------|
| BM25 | Inverted index | Fast | Keyword queries, exact matches |
| Vector | HNSW (ANN) | Fast | Semantic queries, concepts |
| Hybrid | Both | Moderate | General-purpose search |

## Development

### Design Principles

1. **Pure Functions**: All query functions are stateless and side-effect-free
2. **Type Safety**: Enums for modes, `rust_decimal::Decimal` for prices
3. **Data-Oriented**: Focus on data transformations
4. **Test-Driven**: Comprehensive unit and integration tests

### Building for Production

```bash
# Build optimized WASM + server binary
cargo leptos build --release

# Run server
./target/release/pg_search_tests
```

### Styling with Tailwind CSS

The project uses Tailwind CSS. Configuration is in `tailwind.config.js`:

```javascript
module.exports = {
  content: [
    "./src/**/*.rs",
    "./index.html",
  ],
  theme: { extend: {} },
  plugins: [],
}
```

## Troubleshooting

### "Database pool not available"
- Ensure `DATABASE_URL` is set
- Check PostgreSQL is running
- Verify network connectivity

### "Extension not found"
- Install ParadeDB: `CREATE EXTENSION pg_search;`
- Install pgvector: `CREATE EXTENSION vector;`
- Check extension versions

### "Tests failing"
- Run `make setup-all` to initialize database
- Load sample data
- Check test output with `--nocapture` flag

## References

- [Leptos Documentation](https://leptos.dev/)
- [Leptos Server Functions](https://leptos.dev/guide/server_functions.html)
- [Actix-web Documentation](https://actix.rs/)
- [ParadeDB Documentation](https://docs.paradedb.com/)
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [sqlx Documentation](https://docs.rs/sqlx/)

---

**Status:** UI Complete ✅ | Server Functions Complete ✅
**Last Updated:** 2025-12-17
