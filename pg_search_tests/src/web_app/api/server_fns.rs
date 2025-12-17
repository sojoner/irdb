// web_app/api/server_fns.rs - Leptos server functions
//
// These are thin wrappers around our query functions, using the #[server] macro
// to enable RPC-style calls from the client (WASM) to the server (Actix).
//
// Philosophy:
// - Server functions are bridges, not business logic
// - All complex logic lives in queries.rs (pure functions)
// - Context extraction happens here (pool from Actix state)
// - Error handling converts sqlx errors to ServerFnError

use leptos::prelude::*;
use crate::web_app::model::*;

#[cfg(feature = "ssr")]
async fn pool() -> Result<sqlx::PgPool, ServerFnError> {
    use actix_web::{web::Data, HttpRequest};
    use sqlx::PgPool;
    use leptos_actix::extract;
    use crate::web_app::api::db;
    
    tracing::info!("pool() called - attempting to resolve database connection");

    // First try to get from context (for testing or if manually set)
    if let Some(pool) = use_context::<PgPool>() {
        tracing::info!("Found PgPool in Leptos context");
        return Ok(pool);
    } else {
        tracing::warn!("PgPool NOT found in Leptos context");
    }

    // Try global pool (most reliable fallback)
    if let Some(pool) = db::get_db() {
        tracing::info!("Using global PgPool from db::get_db()");
        return Ok(pool);
    } else {
        tracing::error!("Global PgPool is empty (db::get_db() returned None)");
    }

    tracing::info!("Extracting HttpRequest to find pool...");
    let req_result = extract().await;

    match req_result {
        Ok(req) => {
            let req: HttpRequest = req;
            tracing::info!("HttpRequest extracted successfully");

            if let Some(pool_data) = req.app_data::<Data<PgPool>>() {
                tracing::info!("Found Data<PgPool> in request app_data");
                return Ok(pool_data.as_ref().clone());
            } else {
                tracing::warn!("Data<PgPool> NOT found in request app_data");
            }

            if let Some(pool) = req.app_data::<PgPool>() {
                tracing::info!("Found PgPool in request app_data");
                return Ok(pool.clone());
            } else {
                tracing::warn!("PgPool NOT found in request app_data");
            }
        },
        Err(e) => {
            tracing::error!("Failed to extract HttpRequest: {}", e);
        }
    }

    tracing::error!("CRITICAL: Database pool could not be resolved from any source");
    ServerFnError::new("Database pool not available")
}

/// Search products with specified mode and filters
///
/// This is the main search endpoint. It dispatches to the appropriate
/// search function (BM25, Vector, or Hybrid) based on the mode parameter.
///
/// # Arguments
/// * `query` - The search query string
/// * `mode` - Which search algorithm to use
/// * `filters` - Filtering, sorting, and pagination options
///
/// # Returns
/// * `SearchResults` with products, facets, and metadata
#[server(SearchProducts, "/api")]
pub async fn search_products(
    query: String,
    mode: SearchMode,
    filters: SearchFilters,
) -> Result<SearchResults, ServerFnError> {
    use super::queries;

    // Add logging
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
///
/// Used for the product detail modal/page.
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

/// Import products from JSON
///
/// Accepts a JSON array of products and inserts them into the database.
/// Returns import status with success/failure counts.
#[server(ImportProducts, "/api")]
pub async fn import_products(products_json: String) -> Result<ImportStatus, ServerFnError> {
    let pool = pool().await?;

    // Parse the JSON input
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

        // Generate random embedding for MVP
        let embedding = generate_random_embedding();

        let result = sqlx::query(
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
        )
        .bind(&product.name)
        .bind(&product.description)
        .bind(&product.brand)
        .bind(&product.category)
        .bind(&product.subcategory)
        .bind(&product.tags.clone().unwrap_or_default())
        .bind(product.price)
        .bind(product.rating.unwrap_or(0.0))
        .bind(product.review_count.unwrap_or(0))
        .bind(product.stock_quantity.unwrap_or(0))
        .bind(product.in_stock.unwrap_or(true))
        .bind(product.featured.unwrap_or(false))
        .bind(&product.attributes)
        .bind(&embedding)
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

/// Get analytics data for the dashboard
///
/// Returns aggregate statistics about products in the database.
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

/// Helper function to generate random embedding vector
///
/// MVP implementation: Returns a random 1536-dimension vector formatted for PostgreSQL.
/// Production: Should call an embedding API (OpenAI, local model, etc.)
fn generate_random_embedding() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();
    format!(
        "[{}]",
        vec.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_embedding_format() {
        let embedding = generate_random_embedding();

        // Should be properly formatted as PostgreSQL vector
        assert!(embedding.starts_with('['));
        assert!(embedding.ends_with(']'));

        // Should have 1536 elements (1535 commas)
        let comma_count = embedding.matches(',').count();
        assert_eq!(comma_count, 1535);
    }

    #[test]
    fn test_search_mode_serialization() {
        // Verify SearchMode can be serialized for server function transport
        let modes = [SearchMode::Bm25, SearchMode::Vector, SearchMode::Hybrid];

        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: SearchMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn test_search_filters_serialization() {
        let filters = SearchFilters {
            categories: vec!["Electronics".to_string()],
            price_min: Some(10.0),
            price_max: Some(500.0),
            min_rating: Some(4.0),
            in_stock_only: true,
            sort_by: SortOption::PriceAsc,
            page: 0,
            page_size: 20,
        };

        let json = serde_json::to_string(&filters).unwrap();
        let deserialized: SearchFilters = serde_json::from_str(&json).unwrap();

        assert_eq!(filters.categories, deserialized.categories);
        assert_eq!(filters.price_min, deserialized.price_min);
        assert_eq!(filters.sort_by, deserialized.sort_by);
    }
}
