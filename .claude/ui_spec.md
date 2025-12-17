# Product Search UI Specification

## Full-Stack Rust Web Application with Leptos

This specification defines a full-stack Rust web application for product search and analytics, built on top of PostgreSQL with ParadeDB pg_search (BM25) and pgvector extensions.

---

## 1. Technology Stack

### 1.1 Frontend
| Technology | Version | Purpose |
|------------|---------|---------|
| **Leptos** | 0.7+ | Reactive web framework (compiles to WebAssembly) |
| **Tailwind CSS** | 4.0 | Utility-first CSS framework |
| **cargo-leptos** | 0.3+ | Build tool with hot-reloading |

### 1.2 Backend
| Technology | Version | Purpose |
|------------|---------|---------|
| **Leptos Server Functions** | - | RPC-style server calls via `#[server]` macro |
| **Actix-web** | 4.x | HTTP server (Leptos integration) |
| **sqlx** | 0.8+ | Async PostgreSQL driver with compile-time checks |

### 1.3 Database
| Technology | Version | Purpose |
|------------|---------|---------|
| **PostgreSQL** | 17.5 | Core database |
| **ParadeDB pg_search** | 0.20.2 | BM25 full-text search |
| **pgvector** | 0.8.0 | Vector similarity search (1536 dimensions) |

---

## 2. Component Architecture

### 2.1 Component Tree

```
App
├── Header
│   └── Logo, Navigation
├── MainLayout
│   ├── Sidebar
│   │   └── Navigation (Search, Import, Analytics)
│   └── Content
│       ├── SearchPage
│       │   ├── SearchBar
│       │   │   ├── SearchInput
│       │   │   └── SearchModeToggle (BM25 | Vector | Hybrid)
│       │   ├── FilterPanel
│       │   │   ├── CategoryFacets
│       │   │   ├── PriceRangeSlider
│       │   │   ├── RatingFilter
│       │   │   └── InStockToggle
│       │   ├── ResultsGrid
│       │   │   ├── SortControls
│       │   │   ├── ProductCard[] (iterates via <For>)
│       │   │   └── Pagination
│       │   └── ProductDetailModal
│       ├── ImportPage
│       │   ├── FileUploader
│       │   ├── ImportProgress
│       │   └── ImportStatus
│       └── AnalyticsPage
│           ├── StatsOverview
│           ├── CategoryPriceChart
│           ├── RatingDistribution
│           └── TopBrandsList
└── Footer
```

### 2.2 State Management Strategy

Leptos uses **fine-grained reactivity** via signals. State flows through:

1. **Local Signals**: Component-scoped reactive state
2. **Context API**: Shared state across component tree
3. **Resources**: Async data fetching with automatic suspense
4. **Actions**: Server function invocations with optimistic updates

```rust
// Global app state via Context
#[derive(Clone)]
pub struct AppState {
    pub search_query: RwSignal<String>,
    pub search_mode: RwSignal<SearchMode>,
    pub filters: RwSignal<SearchFilters>,
    pub results: Resource<SearchParams, Result<SearchResults, ServerFnError>>,
}

#[component]
pub fn App() -> impl IntoView {
    // Provide global state
    let search_query = RwSignal::new(String::new());
    let search_mode = RwSignal::new(SearchMode::Hybrid);
    let filters = RwSignal::new(SearchFilters::default());

    // Create resource that re-fetches when dependencies change
    let search_params = Signal::derive(move || SearchParams {
        query: search_query.get(),
        mode: search_mode.get(),
        filters: filters.get(),
    });

    let results = Resource::new(
        move || search_params.get(),
        |params| search_products(params)
    );

    provide_context(AppState {
        search_query,
        search_mode,
        filters,
        results,
    });

    view! {
        <Router>
            <MainLayout />
        </Router>
    }
}
```

---

## 3. Shared Model Structs

### 3.1 Core Types (`src/model/mod.rs`)

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Search mode enumeration
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    Bm25,       // Keyword matching only
    Vector,     // Semantic similarity only
    #[default]
    Hybrid,     // 70% vector + 30% BM25
}

/// Product from database
#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub brand: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub tags: Vec<String>,
    pub price: rust_decimal::Decimal,
    pub rating: rust_decimal::Decimal,
    pub review_count: i32,
    pub stock_quantity: i32,
    pub in_stock: bool,
    pub featured: bool,
    pub attributes: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Search result with scores
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub product: Product,
    pub bm25_score: Option<f64>,
    pub vector_score: Option<f64>,
    pub combined_score: f64,
    pub snippet: Option<String>,  // Highlighted text
}

/// Search filters
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOption {
    #[default]
    Relevance,
    PriceAsc,
    PriceDesc,
    RatingDesc,
    Newest,
}

/// Facet count for filters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: String,
    pub count: i64,
}

/// Price histogram bucket
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceBucket {
    pub min: f64,
    pub max: f64,
    pub count: i64,
}

/// Search response with results and facets
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub category_facets: Vec<FacetCount>,
    pub brand_facets: Vec<FacetCount>,
    pub price_histogram: Vec<PriceBucket>,
    pub avg_price: f64,
    pub avg_rating: f64,
}

/// Analytics data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub total_products: i64,
    pub category_stats: Vec<CategoryStat>,
    pub rating_distribution: Vec<RatingBucket>,
    pub price_histogram: Vec<PriceBucket>,
    pub top_brands: Vec<BrandStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryStat {
    pub category: String,
    pub count: i64,
    pub avg_price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatingBucket {
    pub rating: f64,
    pub count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrandStat {
    pub brand: String,
    pub count: i64,
}

/// Import status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportStatus {
    pub total: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub complete: bool,
}

/// Product from JSON import
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductImport {
    pub name: String,
    pub description: String,
    pub brand: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub tags: Option<Vec<String>>,
    pub price: f64,
    pub rating: Option<f64>,
    pub review_count: Option<i32>,
    pub stock_quantity: Option<i32>,
    pub in_stock: Option<bool>,
    pub featured: Option<bool>,
    pub attributes: Option<serde_json::Value>,
}
```

---

## 4. Server Functions (API Design)

### 4.1 Server Function Definitions (`src/api.rs`)

```rust
use leptos::prelude::*;
use crate::model::*;

/// Search products with specified mode and filters
#[server(SearchProducts, "/api")]
pub async fn search_products(
    query: String,
    mode: SearchMode,
    filters: SearchFilters,
) -> Result<SearchResults, ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not available"))?;

    let results = match mode {
        SearchMode::Bm25 => search_bm25(&pool, &query, &filters).await?,
        SearchMode::Vector => search_vector(&pool, &query, &filters).await?,
        SearchMode::Hybrid => search_hybrid(&pool, &query, &filters).await?,
    };

    Ok(results)
}

/// Get facet counts for current search
#[server(GetFacets, "/api")]
pub async fn get_facets(
    query: String,
    mode: SearchMode,
) -> Result<(Vec<FacetCount>, Vec<PriceBucket>), ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not available"))?;

    // Get category facets
    let category_facets = sqlx::query_as!(
        FacetCount,
        r#"
        SELECT category as value, COUNT(*) as "count!"
        FROM products.items
        WHERE description ||| $1 OR $1 = ''
        GROUP BY category
        ORDER BY count DESC
        "#,
        query
    )
    .fetch_all(&pool)
    .await?;

    // Get price histogram
    let price_histogram = sqlx::query_as!(
        PriceBucket,
        r#"
        SELECT
            FLOOR(price / 50) * 50 as "min!",
            FLOOR(price / 50) * 50 + 50 as "max!",
            COUNT(*) as "count!"
        FROM products.items
        WHERE description ||| $1 OR $1 = ''
        GROUP BY FLOOR(price / 50)
        ORDER BY min
        "#,
        query
    )
    .fetch_all(&pool)
    .await?;

    Ok((category_facets, price_histogram))
}

/// Import products from JSON
#[server(ImportProducts, "/api")]
pub async fn import_products(
    products_json: String,
) -> Result<ImportStatus, ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not available"))?;

    let products: Vec<ProductImport> = serde_json::from_str(&products_json)
        .map_err(|e| ServerFnError::new(format!("JSON parse error: {}", e)))?;

    let mut status = ImportStatus {
        total: products.len(),
        processed: 0,
        succeeded: 0,
        failed: 0,
        errors: Vec::new(),
        complete: false,
    };

    for product in products {
        status.processed += 1;

        // Generate random embedding for MVP (placeholder)
        let embedding = generate_random_embedding();

        let result = sqlx::query!(
            r#"
            INSERT INTO products.items (
                name, description, brand, category, subcategory, tags,
                price, rating, review_count, stock_quantity, in_stock,
                featured, attributes, description_embedding
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14::vector(1536)
            )
            ON CONFLICT (name, brand) DO UPDATE SET
                description = EXCLUDED.description,
                price = EXCLUDED.price,
                updated_at = NOW()
            "#,
            product.name,
            product.description,
            product.brand,
            product.category,
            product.subcategory,
            &product.tags.unwrap_or_default(),
            product.price,
            product.rating.unwrap_or(0.0),
            product.review_count.unwrap_or(0),
            product.stock_quantity.unwrap_or(0),
            product.in_stock.unwrap_or(true),
            product.featured.unwrap_or(false),
            product.attributes,
            &embedding,
        )
        .execute(&pool)
        .await;

        match result {
            Ok(_) => status.succeeded += 1,
            Err(e) => {
                status.failed += 1;
                status.errors.push(format!("{}: {}", product.name, e));
            }
        }
    }

    status.complete = true;
    Ok(status)
}

/// Get single product by ID
#[server(GetProduct, "/api")]
pub async fn get_product(id: i32) -> Result<Product, ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not available"))?;

    let product = sqlx::query_as!(
        Product,
        r#"
        SELECT
            id, name, description, brand, category, subcategory,
            tags, price, rating, review_count, stock_quantity,
            in_stock, featured, attributes, created_at, updated_at
        FROM products.items
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Product not found: {}", e)))?;

    Ok(product)
}

/// Get analytics data
#[server(GetAnalytics, "/api")]
pub async fn get_analytics() -> Result<AnalyticsData, ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not available"))?;

    // Total products
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products.items")
        .fetch_one(&pool)
        .await?;

    // Category stats
    let category_stats = sqlx::query_as!(
        CategoryStat,
        r#"
        SELECT
            category,
            COUNT(*) as "count!",
            AVG(price::float8) as "avg_price!"
        FROM products.items
        GROUP BY category
        ORDER BY count DESC
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Rating distribution
    let rating_distribution = sqlx::query_as!(
        RatingBucket,
        r#"
        SELECT
            FLOOR(rating) as "rating!",
            COUNT(*) as "count!"
        FROM products.items
        GROUP BY FLOOR(rating)
        ORDER BY rating
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Price histogram
    let price_histogram = sqlx::query_as!(
        PriceBucket,
        r#"
        SELECT
            FLOOR(price::float8 / 100) * 100 as "min!",
            FLOOR(price::float8 / 100) * 100 + 100 as "max!",
            COUNT(*) as "count!"
        FROM products.items
        GROUP BY FLOOR(price::float8 / 100)
        ORDER BY min
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Top brands
    let top_brands = sqlx::query_as!(
        BrandStat,
        r#"
        SELECT
            brand,
            COUNT(*) as "count!"
        FROM products.items
        GROUP BY brand
        ORDER BY count DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(AnalyticsData {
        total_products: total.0,
        category_stats,
        rating_distribution,
        price_histogram,
        top_brands,
    })
}

// Helper function for MVP - generates random 1536-dim vector
fn generate_random_embedding() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();
    format!("[{}]", vec.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}
```

### 4.2 Database Query Functions (`src/api/queries.rs`)

```rust
use sqlx::PgPool;
use crate::model::*;

/// BM25 full-text search using ParadeDB operators
pub async fn search_bm25(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error> {
    let offset = (filters.page * filters.page_size) as i64;
    let limit = filters.page_size as i64;

    // Build WHERE clause for filters
    let category_filter = if filters.categories.is_empty() {
        "TRUE".to_string()
    } else {
        format!("category = ANY($4)")
    };

    let results = sqlx::query_as!(
        SearchResultRow,
        r#"
        SELECT
            p.id, p.name, p.description, p.brand, p.category,
            p.subcategory, p.tags, p.price, p.rating, p.review_count,
            p.stock_quantity, p.in_stock, p.featured, p.attributes,
            p.created_at, p.updated_at,
            pdb.score(p.id) as "bm25_score!",
            NULL::float8 as vector_score,
            pdb.score(p.id) as "combined_score!",
            pdb.snippet(p.description) as snippet
        FROM products.items p
        WHERE p.description ||| $1
          AND ($2::float8 IS NULL OR p.price >= $2)
          AND ($3::float8 IS NULL OR p.price <= $3)
          AND ($4::text[] IS NULL OR p.category = ANY($4))
          AND ($5::float8 IS NULL OR p.rating >= $5)
          AND ($6::bool IS FALSE OR p.in_stock = TRUE)
        ORDER BY
            CASE WHEN $7 = 'relevance' THEN pdb.score(p.id) END DESC,
            CASE WHEN $7 = 'price_asc' THEN p.price END ASC,
            CASE WHEN $7 = 'price_desc' THEN p.price END DESC,
            CASE WHEN $7 = 'rating_desc' THEN p.rating END DESC,
            CASE WHEN $7 = 'newest' THEN p.created_at END DESC
        LIMIT $8 OFFSET $9
        "#,
        query,
        filters.price_min,
        filters.price_max,
        &filters.categories,
        filters.min_rating,
        filters.in_stock_only,
        sort_to_string(&filters.sort_by),
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    // Get total count
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM products.items
        WHERE description ||| $1
        "#
    )
    .bind(query)
    .fetch_one(pool)
    .await?;

    // Get facets
    let category_facets = get_category_facets(pool, query).await?;
    let brand_facets = get_brand_facets(pool, query).await?;
    let price_histogram = get_price_histogram(pool, query).await?;

    Ok(SearchResults {
        results: results.into_iter().map(|r| r.into()).collect(),
        total_count: total.0,
        category_facets,
        brand_facets,
        price_histogram,
        avg_price: 0.0,  // Calculate if needed
        avg_rating: 0.0,
    })
}

/// Vector similarity search using pgvector
pub async fn search_vector(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error> {
    // For MVP, use a placeholder embedding
    // In production, call an embedding API
    let query_embedding = generate_query_embedding(query);
    let offset = (filters.page * filters.page_size) as i64;
    let limit = filters.page_size as i64;

    let results = sqlx::query_as!(
        SearchResultRow,
        r#"
        SELECT
            p.id, p.name, p.description, p.brand, p.category,
            p.subcategory, p.tags, p.price, p.rating, p.review_count,
            p.stock_quantity, p.in_stock, p.featured, p.attributes,
            p.created_at, p.updated_at,
            NULL::float8 as bm25_score,
            (1 - (p.description_embedding <=> $1::vector(1536))) as "vector_score!",
            (1 - (p.description_embedding <=> $1::vector(1536))) as "combined_score!",
            NULL::text as snippet
        FROM products.items p
        WHERE ($2::float8 IS NULL OR p.price >= $2)
          AND ($3::float8 IS NULL OR p.price <= $3)
          AND ($4::text[] IS NULL OR p.category = ANY($4))
          AND ($5::float8 IS NULL OR p.rating >= $5)
          AND ($6::bool IS FALSE OR p.in_stock = TRUE)
        ORDER BY p.description_embedding <=> $1::vector(1536)
        LIMIT $7 OFFSET $8
        "#,
        query_embedding,
        filters.price_min,
        filters.price_max,
        &filters.categories,
        filters.min_rating,
        filters.in_stock_only,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    // Get total count (approximate for vector search)
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products.items")
        .fetch_one(pool)
        .await?;

    Ok(SearchResults {
        results: results.into_iter().map(|r| r.into()).collect(),
        total_count: total.0,
        category_facets: vec![],
        brand_facets: vec![],
        price_histogram: vec![],
        avg_price: 0.0,
        avg_rating: 0.0,
    })
}

/// Hybrid search combining BM25 (30%) and Vector (70%)
pub async fn search_hybrid(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error> {
    let query_embedding = generate_query_embedding(query);
    let offset = (filters.page * filters.page_size) as i64;
    let limit = filters.page_size as i64;

    let results = sqlx::query_as!(
        SearchResultRow,
        r#"
        WITH bm25_results AS (
            SELECT id, pdb.score(id) AS bm25_score
            FROM products.items
            WHERE description ||| $1
            ORDER BY pdb.score(id) DESC
            LIMIT 100
        ),
        vector_results AS (
            SELECT id, 1 - (description_embedding <=> $2::vector(1536)) AS vector_score
            FROM products.items
            ORDER BY description_embedding <=> $2::vector(1536)
            LIMIT 100
        ),
        combined AS (
            SELECT
                COALESCE(b.id, v.id) AS id,
                COALESCE(b.bm25_score, 0) AS bm25_score,
                COALESCE(v.vector_score, 0) AS vector_score,
                (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
        )
        SELECT
            p.id, p.name, p.description, p.brand, p.category,
            p.subcategory, p.tags, p.price, p.rating, p.review_count,
            p.stock_quantity, p.in_stock, p.featured, p.attributes,
            p.created_at, p.updated_at,
            c.bm25_score as "bm25_score!",
            c.vector_score as "vector_score!",
            c.combined_score as "combined_score!",
            NULL::text as snippet
        FROM combined c
        JOIN products.items p ON p.id = c.id
        WHERE ($3::float8 IS NULL OR p.price >= $3)
          AND ($4::float8 IS NULL OR p.price <= $4)
          AND ($5::text[] IS NULL OR p.category = ANY($5))
          AND ($6::float8 IS NULL OR p.rating >= $6)
          AND ($7::bool IS FALSE OR p.in_stock = TRUE)
        ORDER BY c.combined_score DESC
        LIMIT $8 OFFSET $9
        "#,
        query,
        query_embedding,
        filters.price_min,
        filters.price_max,
        &filters.categories,
        filters.min_rating,
        filters.in_stock_only,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    // Get facets
    let category_facets = get_category_facets(pool, query).await?;
    let brand_facets = get_brand_facets(pool, query).await?;
    let price_histogram = get_price_histogram(pool, query).await?;

    Ok(SearchResults {
        results: results.into_iter().map(|r| r.into()).collect(),
        total_count: results.len() as i64,  // Approximate
        category_facets,
        brand_facets,
        price_histogram,
        avg_price: 0.0,
        avg_rating: 0.0,
    })
}

// Helper functions
async fn get_category_facets(pool: &PgPool, query: &str) -> Result<Vec<FacetCount>, sqlx::Error> {
    sqlx::query_as!(
        FacetCount,
        r#"
        SELECT category as "value!", COUNT(*) as "count!"
        FROM products.items
        WHERE description ||| $1 OR $1 = ''
        GROUP BY category
        ORDER BY count DESC
        "#,
        query
    )
    .fetch_all(pool)
    .await
}

async fn get_brand_facets(pool: &PgPool, query: &str) -> Result<Vec<FacetCount>, sqlx::Error> {
    sqlx::query_as!(
        FacetCount,
        r#"
        SELECT brand as "value!", COUNT(*) as "count!"
        FROM products.items
        WHERE description ||| $1 OR $1 = ''
        GROUP BY brand
        ORDER BY count DESC
        LIMIT 20
        "#,
        query
    )
    .fetch_all(pool)
    .await
}

async fn get_price_histogram(pool: &PgPool, query: &str) -> Result<Vec<PriceBucket>, sqlx::Error> {
    sqlx::query_as!(
        PriceBucket,
        r#"
        SELECT
            FLOOR(price::float8 / 50) * 50 as "min!",
            FLOOR(price::float8 / 50) * 50 + 50 as "max!",
            COUNT(*) as "count!"
        FROM products.items
        WHERE description ||| $1 OR $1 = ''
        GROUP BY FLOOR(price::float8 / 50)
        ORDER BY min
        "#,
        query
    )
    .fetch_all(pool)
    .await
}

fn generate_query_embedding(_query: &str) -> String {
    // MVP: Return random embedding
    // Production: Call OpenAI/local embedding model
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();
    format!("[{}]", vec.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

fn sort_to_string(sort: &SortOption) -> String {
    match sort {
        SortOption::Relevance => "relevance",
        SortOption::PriceAsc => "price_asc",
        SortOption::PriceDesc => "price_desc",
        SortOption::RatingDesc => "rating_desc",
        SortOption::Newest => "newest",
    }.to_string()
}
```

---

## 5. UI Wireframes (Text-Based)

### 5.1 Search Page Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  🔍 IRDB Product Search                                    [Import] [Analytics]│
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ [Search products...                                        ] [Search]│  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Search Mode:  ○ BM25 (Keyword)   ○ Vector (Semantic)   ● Hybrid (Both)    │
│                                                                             │
├────────────────────────┬────────────────────────────────────────────────────┤
│   FILTERS              │  RESULTS                              123 products │
│                        │  ───────────────────────────────────────────────── │
│  ▼ Categories          │  Sort by: [Relevance ▼]                            │
│   ☑ Electronics (45)   │                                                    │
│   ☐ Clothing (32)      │  ┌─────────────────────────────────────────────┐   │
│   ☐ Home & Garden (28) │  │ ★★★★☆ 4.8  $349.99                          │   │
│   ☐ Sports (15)        │  │ Sony WH-1000XM5 Wireless Headphones          │   │
│                        │  │ Industry-leading noise cancellation with...  │   │
│  ▼ Price Range         │  │ Brand: Sony  |  Category: Electronics        │   │
│   $0 ────●────── $500  │  │ [View Details]                               │   │
│   Min: $50  Max: $350  │  └─────────────────────────────────────────────┘   │
│                        │                                                    │
│  ▼ Rating              │  ┌─────────────────────────────────────────────┐   │
│   ★★★★☆ & up          │  │ ★★★★☆ 4.5  $129.99                          │   │
│                        │  │ Logitech MX Keys Wireless Keyboard           │   │
│  ☐ In Stock Only       │  │ Advanced wireless illuminated keyboard...    │   │
│                        │  │ Brand: Logitech  |  Category: Electronics    │   │
│  [Clear Filters]       │  │ [View Details]                               │   │
│                        │  └─────────────────────────────────────────────┘   │
│                        │                                                    │
│                        │  ┌─────────────────────────────────────────────┐   │
│                        │  │ ★★★★★ 4.9  $79.99                           │   │
│                        │  │ Anker PowerCore 26800mAh                     │   │
│                        │  │ Ultra-high capacity portable charger...      │   │
│                        │  │ Brand: Anker  |  Category: Electronics       │   │
│                        │  │ [View Details]                               │   │
│                        │  └─────────────────────────────────────────────┘   │
│                        │                                                    │
│                        │  ◄ 1 2 3 4 5 ... 13 ►                             │
└────────────────────────┴────────────────────────────────────────────────────┘
```

### 5.2 Product Detail Modal

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                    [X]  │
│  Sony WH-1000XM5 Wireless Headphones                                    │
│  ═══════════════════════════════════                                    │
│                                                                         │
│  ★★★★☆ 4.8 (2,847 reviews)                              $349.99        │
│                                                                         │
│  Brand: Sony                                                            │
│  Category: Electronics > Headphones                                     │
│  Tags: wireless, bluetooth, noise-cancellation, premium                 │
│                                                                         │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Industry-leading noise cancellation with Auto NC Optimizer. Crystal    │
│  clear hands-free calling with 4 beamforming microphones. Up to         │
│  30-hour battery life with quick charging (3 min charge for 3 hours     │
│  playback). Multipoint connection allows pairing with two Bluetooth     │
│  devices simultaneously. Speak-to-Chat automatically pauses music       │
│  when you start talking.                                                │
│                                                                         │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Specifications:                                                        │
│    • Color: Black                                                       │
│    • Connectivity: Bluetooth 5.2                                        │
│    • Battery Life: 30 hours                                             │
│    • Weight: 250g                                                       │
│    • Driver Size: 30mm                                                  │
│                                                                         │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Stock: ✓ In Stock (156 available)                                      │
│                                                                         │
│  Search Scores:                                                         │
│    BM25 Score:    0.85                                                  │
│    Vector Score:  0.92                                                  │
│    Combined:      0.90                                                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Import Page Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│  📦 Import Products                                [Search] [Analytics] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Upload products.json file to import products into the database.        │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                                                                   │  │
│  │                    📁 Drop JSON file here                         │  │
│  │                                                                   │  │
│  │                    or click to browse                             │  │
│  │                                                                   │  │
│  │                    Accepts: .json files                           │  │
│  │                                                                   │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Import Progress:                                                       │
│                                                                         │
│  ████████████████████████████░░░░░░░░░░  75%                           │
│                                                                         │
│  Total: 45   |   Processed: 34   |   ✓ Success: 32   |   ✗ Failed: 2   │
│                                                                         │
│  ─────────────────────────────────────────────────────────────────────  │
│                                                                         │
│  Recent Errors:                                                         │
│  • "Defective Widget": Duplicate name/brand combination                 │
│  • "Test Product": Missing required field 'description'                 │
│                                                                         │
│                                                           [Cancel]      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.4 Analytics Page Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│  📊 Analytics Dashboard                            [Search] [Import]    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   TOTAL     │  │  AVG PRICE  │  │ AVG RATING  │  │   BRANDS    │    │
│  │    45       │  │   $156.42   │  │    4.2★     │  │     12      │    │
│  │  products   │  │             │  │             │  │             │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│                                                                         │
├─────────────────────────────────┬───────────────────────────────────────┤
│  Average Price by Category      │  Rating Distribution                  │
│  ────────────────────────────   │  ──────────────────────               │
│                                 │                                       │
│  Electronics    ████████ $245   │     ★★★★★  ████████████  32%         │
│  Sports         ██████   $180   │     ★★★★☆  ██████████    28%         │
│  Home & Garden  █████    $145   │     ★★★☆☆  ██████        18%         │
│  Clothing       ████     $89    │     ★★☆☆☆  ████          12%         │
│  Books          ██       $35    │     ★☆☆☆☆  ██            10%         │
│                                 │                                       │
├─────────────────────────────────┼───────────────────────────────────────┤
│  Price Distribution             │  Top Brands by Product Count          │
│  ────────────────────────────   │  ──────────────────────               │
│                                 │                                       │
│      ▐▐                         │  1. Sony           ████████  8        │
│      ▐▐▐▐                       │  2. Logitech       ██████    6        │
│      ▐▐▐▐▐▐                     │  3. Apple          █████     5        │
│    ▐▐▐▐▐▐▐▐▐▐                   │  4. Samsung        ████      4        │
│  ▐▐▐▐▐▐▐▐▐▐▐▐▐▐                 │  5. Nike           ████      4        │
│  ─────────────────              │  6. Anker          ███       3        │
│  $0  $100 $200 $300 $400 $500+  │  7. Bose           ███       3        │
│                                 │  8. Dell           ██        2        │
│                                 │                                       │
└─────────────────────────────────┴───────────────────────────────────────┘
```

---

## 6. File Structure

```
product-search-ui/
├── Cargo.toml                      # Workspace root
├── Cargo.lock
├── Trunk.toml                      # WASM build config (if using Trunk)
├── rust-toolchain.toml             # Specify nightly Rust
│
├── src/
│   ├── main.rs                     # Server entry point (Actix-web)
│   ├── lib.rs                      # Shared library (client + server)
│   ├── app.rs                      # Root Leptos App component
│   │
│   ├── model/
│   │   ├── mod.rs                  # Model exports
│   │   ├── product.rs              # Product, SearchResult structs
│   │   ├── search.rs               # SearchFilters, SearchResults
│   │   ├── analytics.rs            # AnalyticsData, CategoryStat
│   │   └── import.rs               # ImportStatus, ProductImport
│   │
│   ├── api/
│   │   ├── mod.rs                  # API exports
│   │   ├── server_fns.rs           # #[server] function definitions
│   │   ├── queries.rs              # SQL query implementations
│   │   └── db.rs                   # Database pool setup
│   │
│   ├── components/
│   │   ├── mod.rs                  # Component exports
│   │   ├── layout/
│   │   │   ├── mod.rs
│   │   │   ├── header.rs           # App header with navigation
│   │   │   ├── sidebar.rs          # Navigation sidebar
│   │   │   └── footer.rs           # App footer
│   │   │
│   │   ├── search/
│   │   │   ├── mod.rs
│   │   │   ├── search_bar.rs       # Search input + mode toggle
│   │   │   ├── filter_panel.rs     # Category, price, rating filters
│   │   │   ├── results_grid.rs     # Product card grid
│   │   │   ├── product_card.rs     # Individual product card
│   │   │   ├── product_modal.rs    # Product detail overlay
│   │   │   ├── pagination.rs       # Page navigation
│   │   │   └── facets.rs           # Facet checkboxes
│   │   │
│   │   ├── import/
│   │   │   ├── mod.rs
│   │   │   ├── file_uploader.rs    # Drag-drop file upload
│   │   │   └── import_progress.rs  # Progress bar + status
│   │   │
│   │   ├── analytics/
│   │   │   ├── mod.rs
│   │   │   ├── stats_cards.rs      # Summary stat cards
│   │   │   ├── category_chart.rs   # Bar chart by category
│   │   │   ├── rating_chart.rs     # Rating histogram
│   │   │   ├── price_chart.rs      # Price distribution
│   │   │   └── brand_list.rs       # Top brands table
│   │   │
│   │   └── common/
│   │       ├── mod.rs
│   │       ├── loading.rs          # Loading spinner
│   │       ├── error.rs            # Error display
│   │       ├── modal.rs            # Modal wrapper
│   │       └── button.rs           # Styled button
│   │
│   └── pages/
│       ├── mod.rs
│       ├── search_page.rs          # /search route
│       ├── import_page.rs          # /import route
│       └── analytics_page.rs       # /analytics route
│
├── style/
│   ├── main.css                    # Tailwind imports + custom styles
│   └── tailwind.config.js          # Tailwind configuration
│
├── public/
│   └── favicon.ico
│
├── tests/
│   ├── integration/
│   │   ├── search_tests.rs         # Search API tests
│   │   ├── import_tests.rs         # Import API tests
│   │   └── analytics_tests.rs      # Analytics API tests
│   └── e2e/
│       └── playwright.config.ts    # E2E test config (optional)
│
└── .cargo/
    └── config.toml                 # Cargo config for WASM target
```

### 6.1 Cargo.toml

```toml
[package]
name = "product-search-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# Leptos framework
leptos = { version = "0.7", features = ["csr", "ssr", "nightly"] }
leptos_meta = { version = "0.7" }
leptos_router = { version = "0.7" }
leptos_actix = { version = "0.7" }

# Server
actix-web = "4"
actix-files = "0.6"
tokio = { version = "1", features = ["full"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "chrono", "rust_decimal", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Types
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1.33", features = ["serde", "serde-with-str"] }
uuid = { version = "1", features = ["v4", "serde"] }

# Utilities
rand = "0.8"
thiserror = "1.0"
cfg-if = "1"
console_error_panic_hook = "0.1"
wasm-bindgen = "0.2"

[features]
default = ["ssr"]
csr = ["leptos/csr"]
ssr = ["leptos/ssr", "leptos_actix"]
hydrate = ["leptos/hydrate"]

[profile.release]
lto = true
opt-level = 'z'
codegen-units = 1
```

### 6.2 Leptos.toml (for cargo-leptos)

```toml
[package]
name = "product-search-ui"
bin-package = "product-search-ui"
lib-package = "product-search-ui"

output-name = "product-search-ui"
site-root = "target/site"
site-pkg-dir = "pkg"

# Tailwind CSS
tailwind-input-file = "style/tailwind.css"
tailwind-config-file = "style/tailwind.config.js"

# Style output
style-file = "style/main.css"

# Assets
assets-dir = "public"

# Environment
env = "DEV"

[watch]
reload-port = 3001

[end2end]
dir = "tests/e2e"
```

---

## 7. Implementation Phases

### Phase 1: Project Setup & Basic Search (BM25 Only)
**Goal:** Working search page with BM25 keyword search

**Tasks:**
1. Initialize Leptos project with cargo-leptos
2. Set up Tailwind CSS integration
3. Configure sqlx with PostgreSQL connection pool
4. Create model structs for Product, SearchFilters, SearchResults
5. Implement `search_bm25()` server function
6. Build SearchBar component with input field
7. Build ResultsGrid with ProductCard components
8. Implement basic pagination
9. Add loading states with Suspense

**Deliverables:**
- `/search` route with working BM25 search
- Product cards displaying search results
- Basic error handling

### Phase 2: Filters and Facets
**Goal:** Dynamic filtering with facet counts

**Tasks:**
1. Implement FilterPanel component structure
2. Add CategoryFacets with checkboxes
3. Create PriceRangeSlider component
4. Add RatingFilter (stars)
5. Implement InStockToggle
6. Connect filters to search query
7. Implement `get_facets()` server function
8. Update facet counts reactively on search
9. Add "Clear Filters" functionality
10. Implement sort options dropdown

**Deliverables:**
- Fully functional filter panel
- Real-time facet count updates
- Multiple sort options

### Phase 3: Vector Search Integration
**Goal:** Add semantic search capability

**Tasks:**
1. Implement `search_vector()` server function
2. Add placeholder embedding generation (random vectors)
3. Create SearchModeToggle component (BM25/Vector/Hybrid)
4. Wire mode selection to API calls
5. Display vector similarity scores in results
6. Implement `search_hybrid()` combining both modes
7. Add score breakdown in ProductDetailModal
8. Test hybrid search with various queries

**Deliverables:**
- Three search modes working
- Hybrid search with weighted scoring
- Score visibility in UI

### Phase 4: Import & Analytics
**Goal:** Complete application with data management and insights

**Tasks:**
1. Build FileUploader component with drag-drop
2. Implement `import_products()` server function
3. Add ImportProgress component with progress bar
4. Handle import errors gracefully
5. Create AnalyticsPage layout
6. Implement `get_analytics()` server function
7. Build StatsCards (total, avg price, avg rating)
8. Create CategoryPriceChart (horizontal bars)
9. Build RatingDistribution histogram
10. Implement TopBrandsList table
11. Add navigation between pages

**Deliverables:**
- Working JSON import with progress feedback
- Analytics dashboard with charts
- Complete navigation flow

### Phase 5: Polish & Production Readiness
**Goal:** Production-ready application

**Tasks:**
1. Add comprehensive error boundaries
2. Implement SSR hydration properly
3. Add meta tags for SEO (via leptos_meta)
4. Optimize bundle size (code splitting)
5. Add keyboard navigation (accessibility)
6. Implement responsive design (mobile-friendly)
7. Add unit tests for server functions
8. Write integration tests
9. Set up environment variable configuration
10. Document deployment process

**Deliverables:**
- Production-ready application
- Test coverage
- Deployment documentation

---

## 8. Database Schema Reference

The application uses the `products.items` table defined in [spec.md](spec.md):

```sql
CREATE TABLE products.items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    brand TEXT NOT NULL,
    category TEXT NOT NULL,
    subcategory TEXT,
    tags TEXT[],
    price DECIMAL(10, 2) NOT NULL,
    rating DECIMAL(2, 1) DEFAULT 0.0,
    review_count INTEGER DEFAULT 0,
    stock_quantity INTEGER DEFAULT 0,
    in_stock BOOLEAN DEFAULT true,
    featured BOOLEAN DEFAULT false,
    attributes JSONB,
    description_embedding vector(1536),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- BM25 index
CREATE INDEX products_bm25_idx ON products.items
USING bm25 (id, name, description, brand, (category::pdb.literal), price, rating)
WITH (key_field = 'id');

-- Vector index
CREATE INDEX products_vector_idx ON products.items
USING hnsw (description_embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);
```

---

## 9. API Endpoints Summary

| Server Function | Method | Path | Description |
|-----------------|--------|------|-------------|
| `search_products` | POST | `/api/search_products` | Main search with mode and filters |
| `get_facets` | POST | `/api/get_facets` | Get category/price facets |
| `get_product` | POST | `/api/get_product` | Get single product by ID |
| `import_products` | POST | `/api/import_products` | Bulk import from JSON |
| `get_analytics` | POST | `/api/get_analytics` | Dashboard statistics |

All endpoints use Leptos server function conventions (POST with URL-encoded or JSON body).

---

## 10. Error Handling Strategy

### 10.1 Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Import error: {0}")]
    Import(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}
```

### 10.2 Error Display Component

```rust
#[component]
pub fn ErrorDisplay(error: AppError) -> impl IntoView {
    view! {
        <div class="bg-red-50 border border-red-200 rounded-lg p-4">
            <div class="flex items-center">
                <span class="text-red-500 mr-2">"⚠"</span>
                <span class="text-red-700">{error.to_string()}</span>
            </div>
        </div>
    }
}
```

---

## 11. Constraints & Assumptions

### 11.1 Technical Constraints
- **No JavaScript dependencies**: Pure Rust + WASM
- **No external embedding API** in MVP: Use random vectors or pre-computed embeddings
- **Must work with existing products schema** from spec.md
- **Server functions use `#[server]` macro** pattern

### 11.2 Assumptions
- PostgreSQL 17.5 with pg_search 0.20.2 and pgvector 0.8.0 installed
- Products table pre-created with BM25 and HNSW indexes
- Default 45 products from spec.md mock data
- Development on Rust nightly (for Leptos features)

### 11.3 Future Enhancements (Post-MVP)
- Real embedding generation via OpenAI API
- Real-time search suggestions (typeahead)
- Search history and saved searches
- User authentication and personalized results
- A/B testing for search ranking algorithms
- Export functionality (CSV, JSON)
- Batch operations (bulk update, delete)

---

## Appendix A: Component Code Examples

### A.1 SearchBar Component

```rust
use leptos::prelude::*;
use crate::model::SearchMode;

#[component]
pub fn SearchBar() -> impl IntoView {
    let app_state = use_context::<AppState>().expect("AppState not provided");
    let (local_query, set_local_query) = signal(String::new());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        app_state.search_query.set(local_query.get());
    };

    view! {
        <form on:submit=on_submit class="w-full">
            <div class="flex gap-4 mb-4">
                <input
                    type="text"
                    placeholder="Search products..."
                    class="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    prop:value=local_query
                    on:input=move |ev| set_local_query.set(event_target_value(&ev))
                />
                <button
                    type="submit"
                    class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                >
                    "Search"
                </button>
            </div>

            <SearchModeToggle />
        </form>
    }
}

#[component]
pub fn SearchModeToggle() -> impl IntoView {
    let app_state = use_context::<AppState>().expect("AppState not provided");

    let modes = [
        (SearchMode::Bm25, "BM25 (Keyword)"),
        (SearchMode::Vector, "Vector (Semantic)"),
        (SearchMode::Hybrid, "Hybrid (Both)"),
    ];

    view! {
        <div class="flex gap-4 text-sm">
            <span class="text-gray-600">"Search Mode:"</span>
            <For
                each=move || modes.clone()
                key=|(mode, _)| *mode
                children=move |(mode, label)| {
                    let is_selected = move || app_state.search_mode.get() == mode;
                    view! {
                        <label class="flex items-center gap-1 cursor-pointer">
                            <input
                                type="radio"
                                name="search_mode"
                                checked=is_selected
                                on:change=move |_| app_state.search_mode.set(mode)
                                class="text-blue-600"
                            />
                            <span class=move || if is_selected() { "text-blue-600 font-medium" } else { "text-gray-600" }>
                                {label}
                            </span>
                        </label>
                    }
                }
            />
        </div>
    }
}
```

### A.2 ProductCard Component

```rust
use leptos::prelude::*;
use crate::model::SearchResult;

#[component]
pub fn ProductCard(
    result: SearchResult,
    #[prop(into)] on_click: Callback<i32>,
) -> impl IntoView {
    let product = result.product;

    view! {
        <div
            class="bg-white rounded-lg shadow-md p-4 hover:shadow-lg transition-shadow cursor-pointer"
            on:click=move |_| on_click.run(product.id)
        >
            <div class="flex justify-between items-start mb-2">
                <div class="flex items-center gap-1">
                    <span class="text-yellow-500">"★"</span>
                    <span class="font-medium">{format!("{:.1}", product.rating)}</span>
                </div>
                <span class="text-lg font-bold text-green-600">
                    {format!("${:.2}", product.price)}
                </span>
            </div>

            <h3 class="font-semibold text-gray-900 mb-2 line-clamp-2">
                {product.name.clone()}
            </h3>

            <p class="text-gray-600 text-sm mb-3 line-clamp-3">
                {result.snippet.unwrap_or_else(|| product.description[..100.min(product.description.len())].to_string())}
            </p>

            <div class="flex justify-between items-center text-xs text-gray-500">
                <span>"Brand: " {product.brand}</span>
                <span>{product.category}</span>
            </div>

            <Show when=move || result.combined_score > 0.0>
                <div class="mt-2 pt-2 border-t text-xs text-gray-400">
                    "Score: " {format!("{:.2}", result.combined_score)}
                </div>
            </Show>
        </div>
    }
}
```

### A.3 FilterPanel Component

```rust
use leptos::prelude::*;
use crate::model::{FacetCount, SearchFilters};

#[component]
pub fn FilterPanel(
    category_facets: Signal<Vec<FacetCount>>,
) -> impl IntoView {
    let app_state = use_context::<AppState>().expect("AppState not provided");

    let toggle_category = move |category: String| {
        app_state.filters.update(|f| {
            if f.categories.contains(&category) {
                f.categories.retain(|c| c != &category);
            } else {
                f.categories.push(category);
            }
        });
    };

    let clear_filters = move |_| {
        app_state.filters.set(SearchFilters::default());
    };

    view! {
        <aside class="w-64 bg-gray-50 p-4 rounded-lg">
            <div class="flex justify-between items-center mb-4">
                <h2 class="font-bold text-lg">"Filters"</h2>
                <button
                    on:click=clear_filters
                    class="text-sm text-blue-600 hover:underline"
                >
                    "Clear All"
                </button>
            </div>

            // Categories
            <div class="mb-6">
                <h3 class="font-semibold mb-2 flex items-center gap-1">
                    <span>"▼"</span>
                    "Categories"
                </h3>
                <div class="space-y-2">
                    <For
                        each=category_facets
                        key=|f| f.value.clone()
                        children=move |facet| {
                            let category = facet.value.clone();
                            let is_checked = move || {
                                app_state.filters.get().categories.contains(&category)
                            };
                            view! {
                                <label class="flex items-center gap-2 cursor-pointer">
                                    <input
                                        type="checkbox"
                                        checked=is_checked
                                        on:change=move |_| toggle_category(category.clone())
                                        class="rounded text-blue-600"
                                    />
                                    <span class="flex-1">{facet.value.clone()}</span>
                                    <span class="text-gray-400 text-sm">
                                        "(" {facet.count} ")"
                                    </span>
                                </label>
                            }
                        }
                    />
                </div>
            </div>

            // Price Range
            <div class="mb-6">
                <h3 class="font-semibold mb-2">"▼ Price Range"</h3>
                <PriceRangeSlider />
            </div>

            // Rating Filter
            <div class="mb-6">
                <h3 class="font-semibold mb-2">"▼ Rating"</h3>
                <RatingFilter />
            </div>

            // In Stock Toggle
            <div class="mb-6">
                <label class="flex items-center gap-2 cursor-pointer">
                    <input
                        type="checkbox"
                        checked=move || app_state.filters.get().in_stock_only
                        on:change=move |_| {
                            app_state.filters.update(|f| f.in_stock_only = !f.in_stock_only)
                        }
                        class="rounded text-blue-600"
                    />
                    <span>"In Stock Only"</span>
                </label>
            </div>
        </aside>
    }
}
```

---

## Appendix B: Tailwind CSS Configuration

### B.1 tailwind.config.js

```javascript
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          100: '#dbeafe',
          500: '#3b82f6',
          600: '#2563eb',
          700: '#1d4ed8',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
    require('@tailwindcss/line-clamp'),
  ],
}
```

### B.2 style/main.css

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer components {
  .btn-primary {
    @apply px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors;
  }

  .input-field {
    @apply w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent;
  }

  .card {
    @apply bg-white rounded-lg shadow-md p-4;
  }

  .card-hover {
    @apply card hover:shadow-lg transition-shadow;
  }
}

/* Custom scrollbar */
::-webkit-scrollbar {
  width: 8px;
}

::-webkit-scrollbar-track {
  background: #f1f1f1;
}

::-webkit-scrollbar-thumb {
  background: #c1c1c1;
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: #a1a1a1;
}
```

---

## Appendix C: Development Commands

```bash
# Install dependencies
rustup toolchain install nightly
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos

# Create new project
cargo leptos new product-search-ui

# Development server with hot reload
cargo leptos watch

# Build for production
cargo leptos build --release

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

---

*Last Updated: December 2025*
*Compatible with: Leptos 0.7+, PostgreSQL 17.5, pg_search 0.20.2, pgvector 0.8.0*
