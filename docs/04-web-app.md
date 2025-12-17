# Web Application Development

This guide covers the Leptos-based web application for IRDB's hybrid search interface.

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

## Project Structure

```
pg_search_tests/
├── src/
│   └── web_app/
│       ├── model/              # Shared data models (Client + Server)
│       │   └── mod.rs          # Product, SearchFilters, SearchResults, etc.
│       ├── api/                # Server-side logic
│       │   ├── db.rs           # Connection pool
│       │   └── queries.rs      # Search functions (BM25, Vector, Hybrid)
│       ├── components/         # Leptos UI components (TODO)
│       │   ├── search_bar.rs
│       │   ├── filter_panel.rs
│       │   └── results_grid.rs
│       └── pages/              # Page components (TODO)
│           ├── search_page.rs
│           └── analytics_page.rs
├── tests/
│   └── web_app_search_tests.rs  # Integration tests
└── Cargo.toml
```

## Phase 1: Foundation (Complete ✅)

### Data Models

All core types with full serde support:

```rust
// Search modes
pub enum SearchMode {
    Bm25,       // Keyword matching
    Vector,     // Semantic similarity
    Hybrid,     // Combined (default)
}

// Product representation
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub brand: String,
    pub category: String,
    pub price: rust_decimal::Decimal,
    pub rating: rust_decimal::Decimal,
    pub review_count: i32,
    pub stock_quantity: i32,
    pub in_stock: bool,
    // ... more fields
}

// Search parameters
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

// Search response
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

### Database Query Functions

Three search implementations:

```rust
// BM25 keyword search
pub async fn search_bm25(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error>

// Vector semantic search
pub async fn search_vector(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error>

// Hybrid search (30% BM25 + 70% Vector)
pub async fn search_hybrid(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error>
```

**Design principles:**
- Pure functions (no side effects)
- Explicit dependencies (connection pool passed in)
- Type-safe (compile-time query checking)
- Easy to test (no mocking needed)

### Testing

**17/17 tests passing:**

**Unit Tests (9):**
- Model defaults and serialization
- Display trait implementations
- Type conversions
- Embedding generation

**Integration Tests (8):**
- BM25 basic search
- BM25 with price/category filters
- Vector search
- Hybrid search with score verification
- Facet aggregations
- Pagination
- Sort options

```bash
# Run all tests
cargo test --features web

# Run specific test
cargo test --test web_app_search_tests test_hybrid_search_basic -- --nocapture
```

## Phase 2: Server Functions (TODO)

Leptos server functions bridge client and server:

```rust
#[server(SearchProducts, "/api")]
pub async fn search_products(
    query: String,
    mode: SearchMode,
    filters: SearchFilters,
) -> Result<SearchResults, ServerFnError> {
    let pool = use_context::<PgPool>()
        .expect("Database pool not provided");

    let results = match mode {
        SearchMode::Bm25 => search_bm25(&pool, &query, &filters).await?,
        SearchMode::Vector => search_vector(&pool, &query, &filters).await?,
        SearchMode::Hybrid => search_hybrid(&pool, &query, &filters).await?,
    };

    Ok(results)
}

#[server(GetProduct, "/api")]
pub async fn get_product(id: i32) -> Result<Product, ServerFnError> {
    let pool = use_context::<PgPool>()
        .expect("Database pool not provided");

    let product = sqlx::query_as::<_, Product>(
        "SELECT * FROM products.items WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&pool)
    .await?;

    Ok(product)
}
```

**Key features:**
- Automatic serialization/deserialization
- Type-safe RPC between client and server
- SSR and CSR support
- Error handling

Documentation: https://leptos.dev/guide/server_functions.html

## Phase 3: UI Components (TODO)

### SearchBar Component

```rust
#[component]
pub fn SearchBar(
    query: RwSignal<String>,
    mode: RwSignal<SearchMode>,
    on_search: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <div class="search-bar">
            <input
                type="text"
                placeholder="Search products..."
                prop:value=move || query.get()
                on:input=move |ev| query.set(event_target_value(&ev))
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        on_search();
                    }
                }
            />

            <SearchModeToggle mode=mode />

            <button on:click=move |_| on_search()>
                "Search"
            </button>
        </div>
    }
}
```

### FilterPanel Component

```rust
#[component]
pub fn FilterPanel(
    filters: RwSignal<SearchFilters>,
    facets: Signal<Vec<FacetCount>>,
) -> impl IntoView {
    view! {
        <div class="filter-panel">
            <CategoryFacets
                categories=facets
                selected=move || filters.get().categories
                on_change=move |cats| {
                    filters.update(|f| f.categories = cats);
                }
            />

            <PriceRangeSlider
                min=move || filters.get().price_min
                max=move || filters.get().price_max
                on_change=move |(min, max)| {
                    filters.update(|f| {
                        f.price_min = min;
                        f.price_max = max;
                    });
                }
            />

            <RatingFilter
                min_rating=move || filters.get().min_rating
                on_change=move |rating| {
                    filters.update(|f| f.min_rating = rating);
                }
            />

            <InStockToggle
                checked=move || filters.get().in_stock_only
                on_change=move |checked| {
                    filters.update(|f| f.in_stock_only = checked);
                }
            />
        </div>
    }
}
```

### ResultsGrid Component

```rust
#[component]
pub fn ResultsGrid(
    results: Signal<Vec<SearchResult>>,
    on_product_click: impl Fn(i32) + 'static,
) -> impl IntoView {
    view! {
        <div class="results-grid">
            <For
                each=move || results.get()
                key=|r| r.product.id
                children=move |result| {
                    view! {
                        <ProductCard
                            product=result.product.clone()
                            bm25_score=result.bm25_score
                            vector_score=result.vector_score
                            combined_score=result.combined_score
                            on_click=move |_| on_product_click(result.product.id)
                        />
                    }
                }
            />
        </div>
    }
}
```

Leptos components documentation: https://leptos.dev/guide/components.html

## Phase 4: Pages (TODO)

### SearchPage

Main interface combining all components:

```rust
#[component]
pub fn SearchPage() -> impl IntoView {
    let query = create_rw_signal(String::new());
    let mode = create_rw_signal(SearchMode::Hybrid);
    let filters = create_rw_signal(SearchFilters::default());

    let search_action = create_action(|input: &(String, SearchMode, SearchFilters)| {
        let (q, m, f) = input.clone();
        async move { search_products(q, m, f).await }
    });

    let results = move || {
        search_action.value().get()
            .and_then(|res| res.ok())
    };

    let on_search = move || {
        search_action.dispatch((
            query.get(),
            mode.get(),
            filters.get(),
        ));
    };

    view! {
        <div class="search-page">
            <SearchBar query=query mode=mode on_search=on_search />

            <div class="search-content">
                <FilterPanel
                    filters=filters
                    facets=Signal::derive(move || {
                        results()
                            .map(|r| r.category_facets.clone())
                            .unwrap_or_default()
                    })
                />

                <div class="results-container">
                    <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                        {move || results().map(|r| view! {
                            <ResultsGrid
                                results=Signal::derive(move || r.results.clone())
                                on_product_click=move |id| {
                                    // Navigate to product detail
                                }
                            />

                            <Pagination
                                current_page=move || filters.get().page
                                total_count=r.total_count
                                page_size=move || filters.get().page_size
                                on_page_change=move |page| {
                                    filters.update(|f| f.page = page);
                                    on_search();
                                }
                            />
                        })}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}
```

Leptos routing: https://leptos.dev/guide/routing.html

## Running the Application

### Development Mode

```bash
# Set database URL
export DATABASE_URL="postgresql://postgres:password@localhost:5432/database"

# Run with hot reload
cargo leptos watch --features web
```

Access at http://localhost:3000

### Production Build

```bash
# Build optimized WASM + server binary
cargo leptos build --release --features web

# Run server
./target/release/pg_search_tests
```

## Configuration

### Cargo.toml Features

```toml
[features]
default = []
web = ["leptos", "leptos_meta", "leptos_router", "leptos_actix", "actix-web"]
hydrate = ["leptos/hydrate", "wasm-bindgen"]
```

### Leptos Configuration

```toml
[package.metadata.leptos]
output-name = "irdb-search"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "style/main.scss"
assets-dir = "public"
site-addr = "127.0.0.1:3000"
reload-port = 3001
browserquery = "defaults"
watch = false
env = "DEV"
bin-features = ["web"]
lib-features = ["hydrate"]
```

## Styling with Tailwind CSS

Install Tailwind:

```bash
npm install -D tailwindcss
npx tailwindcss init
```

Configure `tailwind.config.js`:

```javascript
module.exports = {
  content: [
    "./src/**/*.rs",
    "./index.html",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

Build CSS:

```bash
npx tailwindcss -i ./style/input.css -o ./style/output.css --watch
```

## Testing Strategy

### Unit Tests

Test pure functions and components:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert_eq!(filters.page, 0);
        assert_eq!(filters.page_size, 10);
        assert_eq!(filters.sort_by, SortOption::Relevance);
    }

    #[test]
    fn test_search_mode_display() {
        assert_eq!(SearchMode::Hybrid.to_string(), "Hybrid");
    }
}
```

### Integration Tests

Test against real database:

```rust
#[tokio::test]
async fn test_search_bm25() -> Result<()> {
    let pool = setup().await?;
    let filters = SearchFilters::default();
    let results = search_bm25(&pool, "wireless", &filters).await?;
    assert!(!results.results.is_empty());
    Ok(())
}
```

### E2E Tests

Use browser testing frameworks:

```bash
# Install Playwright
cargo add --dev playwright

# Run E2E tests
cargo test --test e2e --features web
```

## Performance Optimization

### Server-Side Rendering (SSR)

Leptos supports SSR for faster initial load:

```rust
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <Routes>
                <Route path="/" view=SearchPage />
                <Route path="/product/:id" view=ProductPage />
            </Routes>
        </Router>
    }
}
```

### Code Splitting

Split large components into separate chunks:

```rust
// Lazy load analytics page
let analytics = || async {
    leptos::lazy_load_component!(AnalyticsPage)
};
```

### Caching

Cache search results on the server:

```rust
use std::sync::Arc;
use moka::future::Cache;

#[derive(Clone)]
pub struct SearchCache {
    cache: Arc<Cache<String, SearchResults>>,
}

impl SearchCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300))
                .build()),
        }
    }

    pub async fn get_or_fetch<F, Fut>(
        &self,
        key: String,
        fetch: F,
    ) -> Result<SearchResults, sqlx::Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<SearchResults, sqlx::Error>>,
    {
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let results = fetch().await?;
        self.cache.insert(key, results.clone()).await;
        Ok(results)
    }
}
```

## Next Steps

1. Implement server functions for search operations
2. Build UI components (SearchBar, FilterPanel, ResultsGrid)
3. Create SearchPage with all components
4. Add Tailwind CSS styling
5. Implement SSR and hydration
6. Add E2E tests
7. Optimize bundle size and performance

## References

- [Leptos Documentation](https://leptos.dev/)
- [Leptos Server Functions](https://leptos.dev/guide/server_functions.html)
- [Actix-web Documentation](https://actix.rs/)
- [WASM Bindgen Book](https://rustwasm.github.io/wasm-bindgen/)

For detailed progress tracking, see [pg_search_tests/WEB_APP_PROGRESS.md](../pg_search_tests/WEB_APP_PROGRESS.md)
