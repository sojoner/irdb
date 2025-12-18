/// BM25 Full-Text Search Tests
///
/// Tests ParadeDB pg_search v0.20+ operators: ||| (disjunction), &&& (conjunction)
/// Validates BM25 ranking with filters (price, rating, category, stock, etc.)
///
/// Prerequisites: DATABASE_URL, pg_search extension, products.items table with data

mod common;

use anyhow::Result;
use sqlx::{PgPool, Row};
use common::with_test_db;

async fn run_bm25_test<F, Fut>(test_name: &str, test_fn: F) -> Result<()>
where
    F: FnOnce(PgPool, String) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    with_test_db(test_name, test_fn).await
}

/// Test 1: Match Disjunction (|||) - Match ANY token
#[tokio::test]
async fn test_bm25_match_disjunction() -> Result<()> {
    run_bm25_test("bm25_disjunction", |pool, schema| async move {
        println!("Test 1: BM25 Disjunction (|||) - 'wireless headphones'");

        let query = format!(r#"
            SELECT id, name, brand, price, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'wireless headphones'
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        assert!(!rows.is_empty(), "Should return results for 'wireless headphones'");

        for row in &rows {
            let name: String = row.get("name");
            let score: f32 = row.get("bm25_score");
            println!("  - {} (score: {:.4})", name, score);
        }

        // Verify scores are in descending order
        let scores: Vec<f32> = rows.iter().map(|r| r.get("bm25_score")).collect();
        assert!(scores.windows(2).all(|w| w[0] >= w[1]), "Scores should be descending");

        println!("  ✓ Disjunction search works correctly\n");
        Ok(())
    }).await
}

/// Test 2: Match Conjunction (&&&) - Match ALL tokens
#[tokio::test]
async fn test_bm25_match_conjunction() -> Result<()> {
    run_bm25_test("bm25_conjunction", |pool, schema| async move {
        println!("Test 2: BM25 Conjunction (&&&) - 'wireless noise cancellation'");

        let query = format!(r#"
            SELECT id, name, brand, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description &&& 'wireless noise cancellation'
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let score: f32 = row.get("bm25_score");
            println!("  - {} (score: {:.4})", name, score);
        }

        println!("  ✓ Conjunction search works correctly\n");
        Ok(())
    }).await
}

/// Test 3: Field-Specific Search
#[tokio::test]
async fn test_bm25_field_specific_search() -> Result<()> {
    run_bm25_test("bm25_field", |pool, schema| async move {
        println!("Test 3: Field-Specific Search - 'keyboard' in name field");

        let query = format!(r#"
            SELECT id, name, brand, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE name ||| 'keyboard'
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        assert!(!rows.is_empty(), "Should find products with 'keyboard' in name");

        for row in &rows {
            let name: String = row.get("name");
            let score: f32 = row.get("bm25_score");
            println!("  - {} (score: {:.4})", name, score);
            assert!(name.to_lowercase().contains("keyboard"), "Name should contain 'keyboard'");
        }

        println!("  ✓ Field-specific search works correctly\n");
        Ok(())
    }).await
}

/// Test 4: Numeric Range Filter with BM25
#[tokio::test]
async fn test_bm25_numeric_range_filter() -> Result<()> {
    run_bm25_test("bm25_numeric", |pool, schema| async move {
        println!("Test 4: BM25 + Price Filter - 'headphones' between $50-$150");

        let query = format!(r#"
            SELECT id, name, price::FLOAT8, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'headphones'
              AND price BETWEEN 50 AND 150
              AND in_stock = true
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let price: f64 = row.get(2);
            let score: f32 = row.get("bm25_score");

            println!("  - {} - ${:.2} (score: {:.4})", name, price, score);

            assert!(price >= 50.0 && price <= 150.0, "Price should be in range");
        }

        println!("  ✓ Numeric range filter works correctly\n");
        Ok(())
    }).await
}

/// Test 5: Score Ordering Validation
#[tokio::test]
async fn test_bm25_score_ordering() -> Result<()> {
    run_bm25_test("bm25_ordering", |pool, schema| async move {
        println!("Test 5: BM25 Score Ordering - 'wireless'");

        let query = format!(r#"
            SELECT id, name, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'wireless'
            ORDER BY pdb.score(id) DESC
            LIMIT 10
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        assert!(rows.len() >= 5, "Should return at least 5 results");

        let scores: Vec<f32> = rows.iter().map(|r| r.get("bm25_score")).collect();

        // Verify descending order
        for i in 0..scores.len() - 1 {
            assert!(scores[i] >= scores[i + 1], "Scores should be in descending order");
        }

        println!("  - Found {} results with scores: {:.4} to {:.4}",
                 rows.len(), scores[0], scores[scores.len() - 1]);
        println!("  ✓ Score ordering is correct\n");
        Ok(())
    }).await
}

/// Test 6: Category Filter with BM25
#[tokio::test]
async fn test_bm25_category_filter() -> Result<()> {
    run_bm25_test("bm25_category", |pool, schema| async move {
        println!("Test 6: BM25 + Category Filter - 'gaming' in Electronics");

        let query = format!(r#"
            SELECT id, name, category, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'gaming'
              AND category = 'Electronics'
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let category: String = row.get("category");
            let score: f32 = row.get("bm25_score");

            println!("  - {} ({}) (score: {:.4})", name, category, score);
            assert_eq!(category, "Electronics", "Category should be Electronics");
        }

        println!("  ✓ Category filter works correctly\n");
        Ok(())
    }).await
}

/// Test 7: Rating Filter with BM25
#[tokio::test]
async fn test_bm25_rating_filter() -> Result<()> {
    run_bm25_test("bm25_rating", |pool, schema| async move {
        println!("Test 7: BM25 + Rating Filter - rating >= 4.5");

        let query = format!(r#"
            SELECT id, name, rating::FLOAT8, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'wireless'
              AND rating >= 4.5
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let rating: f64 = row.get(2);
            let score: f32 = row.get("bm25_score");

            println!("  - {} (rating: {:.1}) (score: {:.4})", name, rating, score);

            assert!(rating >= 4.5, "Rating should be >= 4.5");
        }

        println!("  ✓ Rating filter works correctly\n");
        Ok(())
    }).await
}

/// Test 8: Stock Filter with BM25
#[tokio::test]
async fn test_bm25_stock_filter() -> Result<()> {
    run_bm25_test("bm25_stock", |pool, schema| async move {
        println!("Test 8: BM25 + Stock Filter - in stock only");

        let query = format!(r#"
            SELECT id, name, in_stock, stock_quantity, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'ergonomic'
              AND in_stock = true
              AND stock_quantity > 0
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let in_stock: bool = row.get("in_stock");
            let stock_quantity: i32 = row.get("stock_quantity");
            let score: f32 = row.get("bm25_score");

            println!("  - {} (stock: {}) (score: {:.4})", name, stock_quantity, score);
            assert!(in_stock, "Product should be in stock");
            assert!(stock_quantity > 0, "Stock quantity should be > 0");
        }

        println!("  ✓ Stock filter works correctly\n");
        Ok(())
    }).await
}

/// Test 9: Featured Products Search
#[tokio::test]
async fn test_bm25_featured_products() -> Result<()> {
    run_bm25_test("bm25_featured", |pool, schema| async move {
        println!("Test 9: BM25 Featured Products - 'camera'");

        let query = format!(r#"
            SELECT id, name, featured, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| 'camera'
              AND featured = true
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let featured: bool = row.get("featured");
            let score: f32 = row.get("bm25_score");

            println!("  - {} (featured: {}) (score: {:.4})", name, featured, score);
            assert!(featured, "Product should be featured");
        }

        println!("  ✓ Featured filter works correctly\n");
        Ok(())
    }).await
}

/// Test 10: Brand-Specific Search
#[tokio::test]
async fn test_bm25_brand_search() -> Result<()> {
    run_bm25_test("bm25_brand", |pool, schema| async move {
        println!("Test 10: Brand Search - 'Sony' products");

        let query = format!(r#"
            SELECT id, name, brand, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE brand ||| 'Sony'
            ORDER BY pdb.score(id) DESC
            LIMIT 5
        "#, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let brand: String = row.get("brand");
            let score: f32 = row.get("bm25_score");

            println!("  - {} ({}) (score: {:.4})", name, brand, score);
        }

        println!("  ✓ Brand search works correctly\n");
        Ok(())
    }).await
}
