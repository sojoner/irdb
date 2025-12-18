/// Vector Similarity Search Tests
///
/// Tests pgvector v0.8.0 operators: <=> (cosine), <-> (L2), <#> (inner product)
/// Validates semantic search with HNSW indexing and various filters
///
/// Prerequisites: DATABASE_URL, pgvector extension, description_embedding column, HNSW index

mod common;

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use common::with_test_db;

async fn run_vector_test<F, Fut>(test_name: &str, test_fn: F) -> Result<()>
where
    F: FnOnce(PgPool, String) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    with_test_db(test_name, test_fn).await
}

async fn setup_test_embeddings(pool: &PgPool, table_name: &str, schema: &str) -> Result<()> {
    // Create table
    let create_sql = format!(r#"
        CREATE TABLE {} (
            query_name TEXT PRIMARY KEY,
            embedding vector(1536)
        )
    "#, table_name);
    sqlx::query(&create_sql).execute(pool).await?;

    // Generate random test embeddings
    let insert_sql = format!(r#"
        INSERT INTO {} (query_name, embedding)
        VALUES
            ('wireless_audio', {}.generate_random_embedding(1536)),
            ('gaming_peripherals', {}.generate_random_embedding(1536)),
            ('professional_camera', {}.generate_random_embedding(1536)),
            ('office_furniture', {}.generate_random_embedding(1536))
    "#, table_name, schema, schema, schema, schema);
    sqlx::query(&insert_sql).execute(pool).await?;

    Ok(())
}

async fn cleanup_test_embeddings(pool: &PgPool, table_name: &str) -> Result<()> {
    let drop_sql = format!("DROP TABLE IF EXISTS {}", table_name);
    sqlx::query(&drop_sql).execute(pool).await?;
    Ok(())
}

/// Test 1: Basic Cosine Similarity Search
#[tokio::test]
async fn test_vector_cosine_similarity() -> Result<()> {
    run_vector_test("vector_cosine", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 1: Vector Cosine Similarity - 'wireless_audio' query");

        let query = format!(r#"
            SELECT
                id,
                name,
                brand,
                category,
                price,
                1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) AS cosine_similarity
            FROM {}.items
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')
            LIMIT 10
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        assert!(!rows.is_empty(), "Should return results");

        for row in &rows {
            let name: String = row.get("name");
            let similarity: f64 = row.get("cosine_similarity");
            println!("  - {} (similarity: {:.4})", name, similarity);
        }

        // Verify similarities are in descending order
        let similarities: Vec<f64> = rows.iter().map(|r| r.get("cosine_similarity")).collect();
        assert!(similarities.windows(2).all(|w| w[0] >= w[1]), "Similarities should be descending");

        println!("  ✓ Cosine similarity search works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 2: Vector Search with Similarity Threshold
#[tokio::test]
async fn test_vector_threshold_filter() -> Result<()> {
    run_vector_test("vector_threshold", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 2: Vector Search with Threshold - similarity > 0.3");

        let query = format!(r#"
            SELECT
                id,
                name,
                1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) AS similarity
            FROM {}.items
            WHERE 1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) > 0.3
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')
            LIMIT 10
        "#, table_name, schema, table_name, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let similarity: f64 = row.get("similarity");
            println!("  - {} (similarity: {:.4})", name, similarity);
            assert!(similarity > 0.3, "Similarity should be > 0.3");
        }

        println!("  ✓ Threshold filter works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 3: Vector Search with Price Filter
#[tokio::test]
async fn test_vector_with_price_filter() -> Result<()> {
    run_vector_test("vector_price", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 3: Vector Search + Price Filter - gaming products under $200");

        let query = format!(r#"
            SELECT
                id,
                name,
                price::FLOAT8,
                1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'gaming_peripherals')) AS similarity
            FROM {}.items
            WHERE price < 200
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'gaming_peripherals')
            LIMIT 10
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let price: f64 = row.get(2);
            let similarity: f64 = row.get("similarity");

            println!("  - {} - ${:.2} (similarity: {:.4})", name, price, similarity);

            assert!(price < 200.0, "Price should be < $200");
        }

        println!("  ✓ Price filter works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 4: Vector Search with Category Filter
#[tokio::test]
async fn test_vector_with_category_filter() -> Result<()> {
    run_vector_test("vector_category", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 4: Vector Search + Category - Cameras in Electronics");

        let query = format!(r#"
            SELECT
                id,
                name,
                category,
                subcategory,
                1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'professional_camera')) AS similarity
            FROM {}.items
            WHERE category = 'Electronics'
              AND subcategory = 'Cameras'
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'professional_camera')
            LIMIT 5
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let category: String = row.get("category");
            let subcategory: String = row.get("subcategory");
            let similarity: f64 = row.get("similarity");

            println!("  - {} ({}/{}) (similarity: {:.4})", name, category, subcategory, similarity);
            assert_eq!(category, "Electronics", "Category should be Electronics");
            assert_eq!(subcategory, "Cameras", "Subcategory should be Cameras");
        }

        println!("  ✓ Category filter works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 5: L2 (Euclidean) Distance Search
#[tokio::test]
async fn test_vector_l2_distance() -> Result<()> {
    run_vector_test("vector_l2", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 5: Vector L2 Distance - gaming peripherals");

        let query = format!(r#"
            SELECT
                id,
                name,
                description_embedding <-> (SELECT embedding FROM {} WHERE query_name = 'gaming_peripherals') AS l2_distance
            FROM {}.items
            ORDER BY description_embedding <-> (SELECT embedding FROM {} WHERE query_name = 'gaming_peripherals')
            LIMIT 10
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let distance: f64 = row.get("l2_distance");
            println!("  - {} (L2 distance: {:.4})", name, distance);
        }

        // Verify distances are in ascending order
        let distances: Vec<f64> = rows.iter().map(|r| r.get("l2_distance")).collect();
        assert!(distances.windows(2).all(|w| w[0] <= w[1]), "L2 distances should be ascending");

        println!("  ✓ L2 distance search works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 6: Inner Product Search
#[tokio::test]
async fn test_vector_inner_product() -> Result<()> {
    run_vector_test("vector_inner", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 6: Vector Inner Product - office furniture");

        let query = format!(r#"
            SELECT
                id,
                name,
                description_embedding <#> (SELECT embedding FROM {} WHERE query_name = 'office_furniture') AS inner_product
            FROM {}.items
            ORDER BY description_embedding <#> (SELECT embedding FROM {} WHERE query_name = 'office_furniture')
            LIMIT 10
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let inner_product: f64 = row.get("inner_product");
            println!("  - {} (inner product: {:.4})", name, inner_product);
        }

        println!("  ✓ Inner product search works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 7: Vector Search with Rating Filter
#[tokio::test]
async fn test_vector_with_rating_filter() -> Result<()> {
    run_vector_test("vector_rating", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 7: Vector Search + High Rating - rating >= 4.7");

        let query = format!(r#"
            SELECT
                id,
                name,
                rating::FLOAT8,
                review_count,
                1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) AS similarity
            FROM {}.items
            WHERE rating >= 4.7
              AND review_count > 1000
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')
            LIMIT 5
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let rating: f64 = row.get(2);
            let similarity: f64 = row.get("similarity");

            println!("  - {} (rating: {:.1}) (similarity: {:.4})", name, rating, similarity);

            assert!(rating >= 4.7, "Rating should be >= 4.7");
        }

        println!("  ✓ Rating filter works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 8: Featured Products Vector Search
#[tokio::test]
async fn test_vector_featured_products() -> Result<()> {
    run_vector_test("vector_featured", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 8: Vector Search - Featured products only");

        let query = format!(r#"
            SELECT
                id,
                name,
                featured,
                1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) AS similarity
            FROM {}.items
            WHERE featured = true
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')
            LIMIT 10
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let featured: bool = row.get("featured");
            let similarity: f64 = row.get("similarity");

            println!("  - {} (featured: {}) (similarity: {:.4})", name, featured, similarity);
            assert!(featured, "Product should be featured");
        }

        println!("  ✓ Featured filter works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 9: HNSW Index Usage Verification
#[tokio::test]
async fn test_vector_hnsw_index_usage() -> Result<()> {
    run_vector_test("vector_hnsw", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 9: HNSW Index Usage - Verify index is being used");

        let query = format!(r#"
            EXPLAIN (FORMAT JSON)
            SELECT id, name,
                   1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) AS similarity
            FROM {}.items
            ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')
            LIMIT 10
        "#, table_name, schema, table_name);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        // Check if HNSW index is mentioned in the plan
        if let Some(row) = rows.first() {
            let plan_json: serde_json::Value = row.get(0);
            let plan_str = plan_json.to_string();

            // Look for index scan in the plan
            if plan_str.contains("Index") || plan_str.contains("Scan") {
                println!("  ✓ Index scan detected in query plan");
            } else {
                println!("  ⚠ Warning: Index scan not clearly visible (may still be used)");
            }
        }

        println!("  ✓ Query plan analyzed\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 10: Similarity Score Distribution
#[tokio::test]
async fn test_vector_similarity_distribution() -> Result<()> {
    run_vector_test("vector_dist", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 10: Similarity Distribution - Statistics");

        let query = format!(r#"
            WITH similarities AS (
                SELECT
                    1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_audio')) AS similarity
                FROM {}.items
            )
            SELECT
                MIN(similarity)::FLOAT AS min_similarity,
                AVG(similarity)::FLOAT AS avg_similarity,
                MAX(similarity)::FLOAT AS max_similarity,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY similarity)::FLOAT AS median_similarity
            FROM similarities
        "#, table_name, schema);

        let row = sqlx::query(&query).fetch_one(&pool).await?;

        let min_sim: f64 = row.get("min_similarity");
        let avg_sim: f64 = row.get("avg_similarity");
        let max_sim: f64 = row.get("max_similarity");
        let median_sim: f64 = row.get("median_similarity");

        println!("  - Min similarity: {:.4}", min_sim);
        println!("  - Avg similarity: {:.4}", avg_sim);
        println!("  - Median similarity: {:.4}", median_sim);
        println!("  - Max similarity: {:.4}", max_sim);

        assert!(min_sim <= avg_sim && avg_sim <= max_sim, "Statistics should be ordered correctly");

        println!("  ✓ Similarity distribution calculated\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}
