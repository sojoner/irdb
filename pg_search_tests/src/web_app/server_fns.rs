// web_app/server_fns.rs - Leptos server function declarations
//
// These are the server function declarations that are accessible from both
// client (WASM) and server (native Rust). The #[server] macro automatically
// generates:
// - On server: The actual function implementation
// - On client: A stub that makes HTTP POST requests to the server
//
// IMPORTANT: This file must be compiled for BOTH ssr and hydrate features!

use leptos::prelude::*;
use crate::web_app::model::*;

#[cfg(feature = "ssr")]
async fn pool() -> Result<sqlx::PgPool, ServerFnError> {
    use actix_web::{web::Data, HttpRequest};
    use sqlx::PgPool;
    use leptos_actix::extract;
    use crate::web_app::api::db;
    
    // First try to get from context (for testing or if manually set)
    if let Some(pool) = use_context::<PgPool>() {
        return Ok(pool);
    }

    // Try global pool (most reliable fallback)
    if let Some(pool) = db::get_db() {
        return Ok(pool);
    }

    let req_result = extract().await;

    match req_result {
        Ok(req) => {
            let req: HttpRequest = req;
            if let Some(pool_data) = req.app_data::<Data<PgPool>>() {
                return Ok(pool_data.as_ref().clone());
            }

            if let Some(pool) = req.app_data::<PgPool>() {
                return Ok(pool.clone());
            }
        },
        Err(e) => {
            tracing::error!("Failed to extract HttpRequest: {}", e);
        }
    }

    Err(ServerFnError::new("Database pool not available"))
}

/// Search products with specified mode and filters
#[server(SearchProducts, "/api")]
pub async fn search_products(
    query: String,
    mode: SearchMode,
    filters: SearchFilters,
) -> Result<SearchResults, ServerFnError> {
    use crate::web_app::api::queries;

    tracing::info!("Search request: query='{}', mode={:?}, filters={:?}", query, mode, filters);

    // Extract the database pool
    let pool = pool().await?;

    // Dispatch to the appropriate search function
    let results = match mode {
        SearchMode::Bm25 => queries::search_bm25(&pool, &query, &filters).await,
        SearchMode::Vector => queries::search_vector(&pool, &query, &filters).await,
        SearchMode::Hybrid => queries::search_hybrid(&pool, &query, &filters).await,
    };

    match &results {
        Ok(res) => tracing::info!("Search successful: found {} results", res.results.len()),
        Err(e) => tracing::error!("Search failed: {}", e),
    }

    results.map_err(|e| ServerFnError::new(format!("Search failed: {}", e)))
}

/// Get a single product by ID
#[server(GetProduct, "/api")]
pub async fn get_product(id: i32) -> Result<Product, ServerFnError> {
    use sqlx::Row;

    let pool = pool().await?;

    let sql = r#"
        SELECT
            id, name, description, brand, category, subcategory,
            tags, price::numeric as price, rating::numeric as rating,
            review_count, stock_quantity, in_stock, featured,
            attributes, created_at, updated_at
        FROM products.items
        WHERE id = $1
    "#;

    let row = sqlx::query(sql)
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Product not found: {}", e)))?;

    Ok(Product {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        brand: row.get("brand"),
        category: row.get("category"),
        subcategory: row.get("subcategory"),
        tags: row.get("tags"),
        price: row.get("price"),
        rating: row.get("rating"),
        review_count: row.get("review_count"),
        stock_quantity: row.get("stock_quantity"),
        in_stock: row.get("in_stock"),
        featured: row.get("featured"),
        attributes: row.get("attributes"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// Get analytics data for the dashboard
#[server(GetAnalytics, "/api")]
pub async fn get_analytics() -> Result<AnalyticsData, ServerFnError> {
    use sqlx::Row;

    let pool = pool().await?;

    // Total products
    let total_row = sqlx::query("SELECT COUNT(*) as count FROM products.items")
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get total: {}", e)))?;
    let total_products: i64 = total_row.get("count");

    // Category stats
    let category_rows = sqlx::query(
        r#"
        SELECT
            category,
            COUNT(*) as count,
            AVG(price::float8) as avg_price
        FROM products.items
        GROUP BY category
        ORDER BY count DESC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to get category stats: {}", e)))?;

    let category_stats: Vec<CategoryStat> = category_rows
        .into_iter()
        .map(|row| CategoryStat {
            category: row.get("category"),
            count: row.get("count"),
            avg_price: row.get("avg_price"),
        })
        .collect();

    // Rating distribution
    let rating_rows = sqlx::query(
        r#"
        SELECT
            FLOOR(rating::float8) as rating,
            COUNT(*) as count
        FROM products.items
        GROUP BY FLOOR(rating::float8)
        ORDER BY rating
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to get rating distribution: {}", e)))?;

    let rating_distribution: Vec<RatingBucket> = rating_rows
        .into_iter()
        .map(|row| RatingBucket {
            rating: row.get("rating"),
            count: row.get("count"),
        })
        .collect();

    // Price histogram
    let price_rows = sqlx::query(
        r#"
        SELECT
            FLOOR(price::float8 / 100) * 100 as min,
            FLOOR(price::float8 / 100) * 100 + 100 as max,
            COUNT(*) as count
        FROM products.items
        GROUP BY FLOOR(price::float8 / 100)
        ORDER BY min
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to get price histogram: {}", e)))?;

    let price_histogram: Vec<PriceBucket> = price_rows
        .into_iter()
        .map(|row| PriceBucket {
            min: row.get("min"),
            max: row.get("max"),
            count: row.get("count"),
        })
        .collect();

    // Top brands
    let brand_rows = sqlx::query(
        r#"
        SELECT brand, COUNT(*) as count
        FROM products.items
        GROUP BY brand
        ORDER BY count DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to get top brands: {}", e)))?;

    let top_brands: Vec<BrandStat> = brand_rows
        .into_iter()
        .map(|row| BrandStat {
            brand: row.get("brand"),
            count: row.get("count"),
        })
        .collect();

    Ok(AnalyticsData {
        total_products,
        category_stats,
        rating_distribution,
        price_histogram,
        top_brands,
    })
}
