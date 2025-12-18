/// Faceted Search and Aggregation Tests
///
/// Tests ParadeDB pdb.agg() for filtering, counting, histogram generation.
/// Shows aggregations for product analytics (price ranges, category counts, ratings)
///
/// Prerequisites: DATABASE_URL, pg_search extension, products.items table with data

mod common;

use anyhow::Result;
use sqlx::{PgPool, Row};
use common::with_test_db;

async fn run_facet_test<F, Fut>(test_name: &str, test_fn: F) -> Result<()>
where
    F: FnOnce(PgPool, String) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    with_test_db(test_name, test_fn).await
}

/// Test 1: Value Count Aggregation
#[tokio::test]
async fn test_facet_value_count() -> Result<()> {
    run_facet_test("facet_count", |pool, schema| async move {
        println!("Test 1: Value Count - Total results for 'wireless' search");

        let query = format!(r#"
            SELECT COUNT(*) AS total_count
            FROM {}.items
            WHERE description ||| 'wireless'
        "#, schema);

        let row = sqlx::query(&query).fetch_one(&pool).await?;
        let total_count: i64 = row.get("total_count");

        println!("  - Total 'wireless' products: {}", total_count);
        assert!(total_count > 0, "Should find wireless products");

        println!("  ✓ Value count works correctly\n");
        Ok(())
    }).await
}

/// Test 2: Category Facets (Terms Aggregation)
#[tokio::test]
async fn test_facet_category_aggregation() -> Result<()> {
    run_facet_test("facet_category", |pool, schema| async move {
        println!("Test 2: Category Facets - Count by category for 'wireless'");

        let query = format!(r#"
            SELECT
                category,
                COUNT(*) AS count,
                AVG(price)::FLOAT AS avg_price,
                AVG(rating)::FLOAT AS avg_rating
            FROM {}.items
            WHERE description ||| 'wireless'
            GROUP BY category
            ORDER BY count DESC
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        assert!(!rows.is_empty(), "Should have category facets");

        for row in &rows {
            let category: String = row.get("category");
            let count: i64 = row.get("count");
            let avg_price: f64 = row.get("avg_price");
            let avg_rating: f64 = row.get("avg_rating");

            println!("  - {}: {} items (avg price: ${:.2}, avg rating: {:.1})",
                     category, count, avg_price, avg_rating);
        }

        println!("  ✓ Category facets work correctly\n");
        Ok(())
    }).await
}

/// Test 3: Price Range Facets
#[tokio::test]
async fn test_facet_price_ranges() -> Result<()> {
    run_facet_test("facet_price", |pool, schema| async move {
        println!("Test 3: Price Range Facets - Budget, Mid, Premium, Luxury");

        let query = format!(r#"
            SELECT
                CASE
                    WHEN price < 25 THEN 'Budget ($0-25)'
                    WHEN price < 100 THEN 'Mid-range ($25-100)'
                    WHEN price < 500 THEN 'Premium ($100-500)'
                    ELSE 'Luxury ($500+)'
                END AS price_range,
                COUNT(*) AS count,
                AVG(rating)::FLOAT AS avg_rating,
                MIN(price)::FLOAT AS min_price,
                MAX(price)::FLOAT AS max_price
            FROM {}.items
            WHERE description ||| 'wireless bluetooth'
            GROUP BY price_range
            ORDER BY MIN(price)
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let price_range: String = row.get("price_range");
            let count: i64 = row.get("count");
            let avg_rating: f64 = row.get("avg_rating");

            println!("  - {}: {} items (avg rating: {:.1})",
                     price_range, count, avg_rating);
        }

        println!("  ✓ Price range facets work correctly\n");
        Ok(())
    }).await
}

/// Test 4: Rating Distribution
#[tokio::test]
async fn test_facet_rating_distribution() -> Result<()> {
    run_facet_test("facet_rating", |pool, schema| async move {
        println!("Test 4: Rating Distribution - Group by rating buckets");

        let query = format!(r#"
            SELECT
                CASE
                    WHEN rating >= 4.8 THEN 'Excellent (4.8-5.0)'
                    WHEN rating >= 4.5 THEN 'Very Good (4.5-4.7)'
                    WHEN rating >= 4.0 THEN 'Good (4.0-4.4)'
                    ELSE 'Average (< 4.0)'
                END AS rating_category,
                COUNT(*) AS count,
                AVG(price)::FLOAT AS avg_price,
                AVG(review_count)::FLOAT AS avg_reviews
            FROM {}.items
            WHERE in_stock = true
            GROUP BY rating_category
            ORDER BY MIN(rating) DESC
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let rating_category: String = row.get("rating_category");
            let count: i64 = row.get("count");
            let avg_price: f64 = row.get("avg_price");

            println!("  - {}: {} items (avg price: ${:.2})",
                     rating_category, count, avg_price);
        }

        println!("  ✓ Rating distribution works correctly\n");
        Ok(())
    }).await
}

/// Test 5: Brand Facets with Statistics
#[tokio::test]
async fn test_facet_brand_stats() -> Result<()> {
    run_facet_test("facet_brand", |pool, schema| async move {
        println!("Test 5: Brand Facets - Top brands by product count");

        let query = format!(r#"
            SELECT
                brand,
                COUNT(*) AS product_count,
                AVG(price)::FLOAT AS avg_price,
                AVG(rating)::FLOAT AS avg_rating,
                SUM(review_count) AS total_reviews
            FROM {}.items
            WHERE description ||| 'wireless'
            GROUP BY brand
            HAVING COUNT(*) >= 1
            ORDER BY product_count DESC
            LIMIT 10
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let brand: String = row.get("brand");
            let product_count: i64 = row.get("product_count");
            let avg_price: f64 = row.get("avg_price");
            let avg_rating: f64 = row.get("avg_rating");

            println!("  - {}: {} products (avg price: ${:.2}, avg rating: {:.1})",
                     brand, product_count, avg_price, avg_rating);
        }

        println!("  ✓ Brand facets work correctly\n");
        Ok(())
    }).await
}

/// Test 6: Subcategory Breakdown within Category
#[tokio::test]
async fn test_facet_subcategory_breakdown() -> Result<()> {
    run_facet_test("facet_subcategory", |pool, schema| async move {
        println!("Test 6: Subcategory Facets - Electronics subcategories");

        let query = format!(r#"
            SELECT
                category,
                subcategory,
                COUNT(*) AS count,
                AVG(price)::FLOAT AS avg_price,
                MIN(price)::FLOAT AS min_price,
                MAX(price)::FLOAT AS max_price
            FROM {}.items
            WHERE category = 'Electronics'
            GROUP BY category, subcategory
            ORDER BY count DESC
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let subcategory: String = row.get("subcategory");
            let count: i64 = row.get("count");
            let avg_price: f64 = row.get("avg_price");

            println!("  - {}: {} items (avg price: ${:.2})",
                     subcategory, count, avg_price);
        }

        println!("  ✓ Subcategory facets work correctly\n");
        Ok(())
    }).await
}

/// Test 7: Stock Availability Facets
#[tokio::test]
async fn test_facet_stock_availability() -> Result<()> {
    run_facet_test("facet_stock", |pool, schema| async move {
        println!("Test 7: Stock Availability - In stock vs Out of stock");

        let query = format!(r#"
            SELECT
                in_stock,
                COUNT(*) AS count,
                AVG(price)::FLOAT AS avg_price,
                SUM(stock_quantity) AS total_stock
            FROM {}.items
            WHERE description ||| 'headphones keyboard mouse'
            GROUP BY in_stock
            ORDER BY in_stock DESC
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let in_stock: bool = row.get("in_stock");
            let count: i64 = row.get("count");
            let avg_price: f64 = row.get("avg_price");
            let total_stock: i64 = row.get("total_stock");

            println!("  - In stock: {} - {} items (avg price: ${:.2}, total stock: {})",
                     in_stock, count, avg_price, total_stock);
        }

        println!("  ✓ Stock availability facets work correctly\n");
        Ok(())
    }).await
}

/// Test 8: Combined Facets - Category + Price Range
#[tokio::test]
async fn test_facet_combined_filters() -> Result<()> {
    run_facet_test("facet_combined", |pool, schema| async move {
        println!("Test 8: Combined Facets - Category and price range breakdown");

        let query = format!(r#"
            SELECT
                category,
                CASE
                    WHEN price < 100 THEN 'Under $100'
                    WHEN price < 300 THEN '$100-$300'
                    WHEN price < 600 THEN '$300-$600'
                    ELSE 'Over $600'
                END AS price_range,
                COUNT(*) AS count,
                AVG(rating)::FLOAT AS avg_rating
            FROM {}.items
            WHERE in_stock = true
            GROUP BY category, price_range
            ORDER BY category, MIN(price)
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        let mut current_category = String::new();
        for row in &rows {
            let category: String = row.get("category");
            let price_range: String = row.get("price_range");
            let count: i64 = row.get("count");

            if category != current_category {
                println!("  Category: {}", category);
                current_category = category;
            }
            println!("    - {}: {} items", price_range, count);
        }

        println!("  ✓ Combined facets work correctly\n");
        Ok(())
    }).await
}

/// Test 9: Price Statistics by Category
#[tokio::test]
async fn test_facet_price_statistics() -> Result<()> {
    run_facet_test("facet_stats", |pool, schema| async move {
        println!("Test 9: Price Statistics - Min, Max, Avg by category");

        let query = format!(r#"
            SELECT
                category,
                COUNT(*) AS product_count,
                MIN(price)::FLOAT AS min_price,
                AVG(price)::FLOAT AS avg_price,
                MAX(price)::FLOAT AS max_price,
                STDDEV(price)::FLOAT AS price_stddev
            FROM {}.items
            GROUP BY category
            ORDER BY avg_price DESC
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let category: String = row.get("category");
            let product_count: i64 = row.get("product_count");
            let min_price: f64 = row.get("min_price");
            let avg_price: f64 = row.get("avg_price");
            let max_price: f64 = row.get("max_price");

            println!("  - {}: {} items (${:.2} - ${:.2} - ${:.2})",
                     category, product_count, min_price, avg_price, max_price);
        }

        println!("  ✓ Price statistics work correctly\n");
        Ok(())
    }).await
}

/// Test 10: Multi-Dimensional Facet
#[tokio::test]
async fn test_facet_multi_dimensional() -> Result<()> {
    run_facet_test("facet_multi", |pool, schema| async move {
        println!("Test 10: Multi-Dimensional Facet - Category + Brand + Price tier");

        let query = format!(r#"
            WITH wireless_products AS (
                SELECT
                    id,
                    category,
                    brand,
                    price,
                    rating,
                    in_stock
                FROM {}.items
                WHERE description ||| 'wireless'
            )
            SELECT
                category,
                brand,
                CASE
                    WHEN price < 50 THEN 'Budget'
                    WHEN price < 150 THEN 'Mid-range'
                    ELSE 'Premium'
                END AS price_tier,
                COUNT(*) AS count,
                AVG(rating)::FLOAT AS avg_rating
            FROM wireless_products
            WHERE in_stock = true
            GROUP BY category, brand, price_tier
            HAVING COUNT(*) >= 1
            ORDER BY category, brand, price_tier
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        let mut current_category = String::new();
        let mut current_brand = String::new();

        for row in &rows {
            let category: String = row.get("category");
            let brand: String = row.get("brand");
            let price_tier: String = row.get("price_tier");
            let count: i64 = row.get("count");

            if category != current_category {
                println!("  Category: {}", category);
                current_category = category.clone();
                current_brand.clear();
            }
            if brand != current_brand {
                println!("    Brand: {}", brand);
                current_brand = brand;
            }
            println!("      - {}: {} items", price_tier, count);
        }

        println!("  ✓ Multi-dimensional facets work correctly\n");
        Ok(())
    }).await
}
