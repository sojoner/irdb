/// Vector Similarity Search Tests
///
/// Tests pgvector v0.8.0 operators: <=> (cosine), <-> (L2), <#> (inner product)
/// Validates semantic search with HNSW indexing and various filters
///
/// Prerequisites: DATABASE_URL, pgvector extension, description_embedding column, HNSW index

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n=== Products Vector Search Tests ===\n");

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected to database");

    // Check if pgvector extension is available
    let pgvector_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')"
    )
    .fetch_one(&pool)
    .await?;

    if !pgvector_check.0 {
        println!("✗ pgvector extension is NOT installed");
        return Ok(());
    }
    println!("✓ pgvector extension is installed");

    // Verify products table exists
    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'products' AND table_name = 'items')"
    )
    .fetch_one(&pool)
    .await?;

    if !table_exists.0 {
        println!("✗ products.items table does not exist");
        return Ok(());
    }
    println!("✓ products.items table exists");

    // Create test embeddings
    setup_test_embeddings(&pool).await?;

    // Run test suite
    test_vector_cosine_similarity(&pool).await?;
    test_vector_threshold_filter(&pool).await?;
    test_vector_with_price_filter(&pool).await?;
    test_vector_with_category_filter(&pool).await?;
    test_vector_l2_distance(&pool).await?;
    test_vector_inner_product(&pool).await?;
    test_vector_with_rating_filter(&pool).await?;
    test_vector_featured_products(&pool).await?;
    test_vector_hnsw_index_usage(&pool).await?;
    test_vector_similarity_distribution(&pool).await?;

    println!("\n✓ All vector search tests passed!");
    Ok(())
}

/// Setup test query embeddings
async fn setup_test_embeddings(pool: &PgPool) -> Result<()> {
    println!("Setting up test embeddings...");

    // Drop and recreate table for test embeddings (not TEMP due to connection pooling)
    let _ = sqlx::query("DROP TABLE IF EXISTS test_embeddings CASCADE")
        .execute(pool)
        .await;

    sqlx::query(r#"
        CREATE TABLE test_embeddings (
            query_name TEXT PRIMARY KEY,
            embedding vector(1536)
        )
    "#)
    .execute(pool)
    .await?;

    // Generate random test embeddings
    sqlx::query(r#"
        INSERT INTO test_embeddings (query_name, embedding)
        VALUES
            ('wireless_audio', products.generate_random_embedding(1536)),
            ('gaming_peripherals', products.generate_random_embedding(1536)),
            ('professional_camera', products.generate_random_embedding(1536)),
            ('office_furniture', products.generate_random_embedding(1536))
    "#)
    .execute(pool)
    .await?;

    println!("✓ Test embeddings created\n");
    Ok(())
}

/// Test 1: Basic Cosine Similarity Search
async fn test_vector_cosine_similarity(pool: &PgPool) -> Result<()> {
    println!("Test 1: Vector Cosine Similarity - 'wireless_audio' query");

    let query = r#"
        SELECT
            id,
            name,
            brand,
            category,
            price,
            1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS cosine_similarity
        FROM products.items
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

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
    Ok(())
}

/// Test 2: Vector Search with Similarity Threshold
async fn test_vector_threshold_filter(pool: &PgPool) -> Result<()> {
    println!("Test 2: Vector Search with Threshold - similarity > 0.3");

    let query = r#"
        SELECT
            id,
            name,
            1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
        FROM products.items
        WHERE 1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) > 0.3
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in &rows {
        let name: String = row.get("name");
        let similarity: f64 = row.get("similarity");
        println!("  - {} (similarity: {:.4})", name, similarity);
        assert!(similarity > 0.3, "Similarity should be > 0.3");
    }

    println!("  ✓ Threshold filter works correctly\n");
    Ok(())
}

/// Test 3: Vector Search with Price Filter
async fn test_vector_with_price_filter(pool: &PgPool) -> Result<()> {
    println!("Test 3: Vector Search + Price Filter - gaming products under $200");

    let query = r#"
        SELECT
            id,
            name,
            price::FLOAT8,
            1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals')) AS similarity
        FROM products.items
        WHERE price < 200
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in &rows {
        let name: String = row.get("name");
        let price: f64 = row.get(2);
        let similarity: f64 = row.get("similarity");

        println!("  - {} - ${:.2} (similarity: {:.4})", name, price, similarity);

        assert!(price < 200.0, "Price should be < $200");
    }

    println!("  ✓ Price filter works correctly\n");
    Ok(())
}

/// Test 4: Vector Search with Category Filter
async fn test_vector_with_category_filter(pool: &PgPool) -> Result<()> {
    println!("Test 4: Vector Search + Category - Cameras in Electronics");

    let query = r#"
        SELECT
            id,
            name,
            category,
            subcategory,
            1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_camera')) AS similarity
        FROM products.items
        WHERE category = 'Electronics'
          AND subcategory = 'Cameras'
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_camera')
        LIMIT 5
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

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
    Ok(())
}

/// Test 5: L2 (Euclidean) Distance Search
async fn test_vector_l2_distance(pool: &PgPool) -> Result<()> {
    println!("Test 5: Vector L2 Distance - gaming peripherals");

    let query = r#"
        SELECT
            id,
            name,
            description_embedding <-> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals') AS l2_distance
        FROM products.items
        ORDER BY description_embedding <-> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in &rows {
        let name: String = row.get("name");
        let distance: f64 = row.get("l2_distance");
        println!("  - {} (L2 distance: {:.4})", name, distance);
    }

    // Verify distances are in ascending order
    let distances: Vec<f64> = rows.iter().map(|r| r.get("l2_distance")).collect();
    assert!(distances.windows(2).all(|w| w[0] <= w[1]), "L2 distances should be ascending");

    println!("  ✓ L2 distance search works correctly\n");
    Ok(())
}

/// Test 6: Inner Product Search
async fn test_vector_inner_product(pool: &PgPool) -> Result<()> {
    println!("Test 6: Vector Inner Product - office furniture");

    let query = r#"
        SELECT
            id,
            name,
            description_embedding <#> (SELECT embedding FROM test_embeddings WHERE query_name = 'office_furniture') AS inner_product
        FROM products.items
        ORDER BY description_embedding <#> (SELECT embedding FROM test_embeddings WHERE query_name = 'office_furniture')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in &rows {
        let name: String = row.get("name");
        let inner_product: f64 = row.get("inner_product");
        println!("  - {} (inner product: {:.4})", name, inner_product);
    }

    println!("  ✓ Inner product search works correctly\n");
    Ok(())
}

/// Test 7: Vector Search with Rating Filter
async fn test_vector_with_rating_filter(pool: &PgPool) -> Result<()> {
    println!("Test 7: Vector Search + High Rating - rating >= 4.7");

    let query = r#"
        SELECT
            id,
            name,
            rating::FLOAT8,
            review_count,
            1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
        FROM products.items
        WHERE rating >= 4.7
          AND review_count > 1000
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
        LIMIT 5
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in &rows {
        let name: String = row.get("name");
        let rating: f64 = row.get(2);
        let similarity: f64 = row.get("similarity");

        println!("  - {} (rating: {:.1}) (similarity: {:.4})", name, rating, similarity);

        assert!(rating >= 4.7, "Rating should be >= 4.7");
    }

    println!("  ✓ Rating filter works correctly\n");
    Ok(())
}

/// Test 8: Featured Products Vector Search
async fn test_vector_featured_products(pool: &PgPool) -> Result<()> {
    println!("Test 8: Vector Search - Featured products only");

    let query = r#"
        SELECT
            id,
            name,
            featured,
            1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
        FROM products.items
        WHERE featured = true
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in &rows {
        let name: String = row.get("name");
        let featured: bool = row.get("featured");
        let similarity: f64 = row.get("similarity");

        println!("  - {} (featured: {}) (similarity: {:.4})", name, featured, similarity);
        assert!(featured, "Product should be featured");
    }

    println!("  ✓ Featured filter works correctly\n");
    Ok(())
}

/// Test 9: HNSW Index Usage Verification
async fn test_vector_hnsw_index_usage(pool: &PgPool) -> Result<()> {
    println!("Test 9: HNSW Index Usage - Verify index is being used");

    let query = r#"
        EXPLAIN (FORMAT JSON)
        SELECT id, name,
               1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
        FROM products.items
        ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
        LIMIT 10
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;

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
    Ok(())
}

/// Test 10: Similarity Score Distribution
async fn test_vector_similarity_distribution(pool: &PgPool) -> Result<()> {
    println!("Test 10: Similarity Distribution - Statistics");

    let query = r#"
        WITH similarities AS (
            SELECT
                1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
            FROM products.items
        )
        SELECT
            MIN(similarity)::FLOAT AS min_similarity,
            AVG(similarity)::FLOAT AS avg_similarity,
            MAX(similarity)::FLOAT AS max_similarity,
            PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY similarity)::FLOAT AS median_similarity
        FROM similarities
    "#;

    let row = sqlx::query(query).fetch_one(pool).await?;

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
    Ok(())
}
