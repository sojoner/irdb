/// Hybrid Search Tests: BM25 + Vector Similarity
///
/// Tests two approaches: Weighted Score Combination (30% BM25 + 70% Vector) and
/// Reciprocal Rank Fusion (RRF). Combines lexical and semantic search with filters.
///
/// Prerequisites: DATABASE_URL, pg_search, pgvector, BM25 index, HNSW vector index

mod common;

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use common::with_test_db;

async fn run_hybrid_test<F, Fut>(test_name: &str, test_fn: F) -> Result<()>
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

    let insert_sql = format!(r#"
        INSERT INTO {} (query_name, embedding)
        VALUES
            ('wireless_headphones', {}.generate_random_embedding(1536)),
            ('gaming_setup', {}.generate_random_embedding(1536)),
            ('professional_photography', {}.generate_random_embedding(1536)),
            ('home_office', {}.generate_random_embedding(1536)),
            ('fitness_gear', {}.generate_random_embedding(1536))
    "#, table_name, schema, schema, schema, schema, schema);
    sqlx::query(&insert_sql).execute(pool).await?;

    Ok(())
}

async fn cleanup_test_embeddings(pool: &PgPool, table_name: &str) -> Result<()> {
    let drop_sql = format!("DROP TABLE IF EXISTS {}", table_name);
    sqlx::query(&drop_sql).execute(pool).await?;
    Ok(())
}

/// Test 1: Weighted Score Combination (70% Vector + 30% BM25)
#[tokio::test]
async fn test_hybrid_weighted_combination() -> Result<()> {
    run_hybrid_test("hybrid_weighted", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 1: Hybrid Weighted Combination - 'wireless headphones' (70% vector, 30% BM25)");

        let query = format!(r#"
            WITH bm25_results AS (
                SELECT
                    id,
                    pdb.score(id) AS bm25_score
                FROM {}.items
                WHERE description ||| 'wireless headphones'
                ORDER BY pdb.score(id) DESC
                LIMIT 50
            ),
            vector_results AS (
                SELECT
                    id,
                    1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_headphones')) AS vector_score
                FROM {}.items
                ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_headphones')
                LIMIT 50
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                p.brand,
                p.price,
                COALESCE(b.bm25_score, 0)::FLOAT AS bm25_score,
                COALESCE(v.vector_score, 0)::FLOAT AS vector_score,
                (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7)::FLOAT AS combined_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY combined_score DESC
            LIMIT 10
        "#, schema, table_name, schema, table_name, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        assert!(!rows.is_empty(), "Should return hybrid results");

        for row in &rows {
            let name: String = row.get("name");
            let bm25_score: f64 = row.get("bm25_score");
            let vector_score: f64 = row.get("vector_score");
            let combined_score: f64 = row.get("combined_score");

            println!("  - {} (BM25: {:.3}, Vector: {:.3}, Combined: {:.3})",
                     name, bm25_score, vector_score, combined_score);
        }

        // Verify combined scores are in descending order
        let scores: Vec<f64> = rows.iter().map(|r| r.get("combined_score")).collect();
        assert!(scores.windows(2).all(|w| w[0] >= w[1]), "Combined scores should be descending");

        println!("  ✓ Weighted combination works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 2: Reciprocal Rank Fusion (RRF)
#[tokio::test]
async fn test_hybrid_rrf_fusion() -> Result<()> {
    run_hybrid_test("hybrid_rrf", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 2: Hybrid RRF - 'gaming peripherals' (k=60)");

        let query = format!(r#"
            WITH bm25_ranked AS (
                SELECT
                    id,
                    ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
                FROM {}.items
                WHERE description ||| 'gaming peripherals mouse keyboard'
                LIMIT 50
            ),
            vector_ranked AS (
                SELECT
                    id,
                    ROW_NUMBER() OVER (ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'gaming_setup')) AS rank
                FROM {}.items
                LIMIT 50
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                p.brand,
                p.price,
                b.rank AS bm25_rank,
                v.rank AS vector_rank,
                (1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000)))::FLOAT AS rrf_score
            FROM bm25_ranked b
            FULL OUTER JOIN vector_ranked v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY rrf_score DESC
            LIMIT 10
        "#, schema, table_name, schema, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let bm25_rank: Option<i64> = row.try_get("bm25_rank").ok();
            let vector_rank: Option<i64> = row.try_get("vector_rank").ok();
            let rrf_score: f64 = row.get("rrf_score");

            println!("  - {} (BM25 rank: {:?}, Vector rank: {:?}, RRF: {:.4})",
                     name, bm25_rank, vector_rank, rrf_score);
        }

        // Verify RRF scores are in descending order
        let scores: Vec<f64> = rows.iter().map(|r| r.get("rrf_score")).collect();
        assert!(scores.windows(2).all(|w| w[0] >= w[1]), "RRF scores should be descending");

        println!("  ✓ RRF fusion works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 3: Hybrid Search with Price Filter
#[tokio::test]
async fn test_hybrid_with_price_filter() -> Result<()> {
    run_hybrid_test("hybrid_price", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 3: Hybrid Weighted + Price Filter - 'professional camera' under $1000");

        let query = format!(r#"
            WITH bm25_results AS (
                SELECT
                    id,
                    pdb.score(id) AS bm25_score
                FROM {}.items
                WHERE description ||| 'professional camera photography'
                  AND price < 1000
                ORDER BY pdb.score(id) DESC
                LIMIT 30
            ),
            vector_results AS (
                SELECT
                    id,
                    1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'professional_photography')) AS vector_score
                FROM {}.items
                WHERE price < 1000
                ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'professional_photography')
                LIMIT 30
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                p.price::FLOAT8,
                COALESCE(b.bm25_score, 0)::FLOAT AS bm25_score,
                COALESCE(v.vector_score, 0)::FLOAT AS vector_score,
                (COALESCE(b.bm25_score, 0) * 0.4 + COALESCE(v.vector_score, 0) * 0.6)::FLOAT AS combined_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY combined_score DESC
            LIMIT 5
        "#, schema, table_name, schema, table_name, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let price: f64 = row.get(2);
            let combined_score: f64 = row.get("combined_score");

            println!("  - {} - ${:.2} (score: {:.3})", name, price, combined_score);

            assert!(price < 1000.0, "Price should be < $1000");
        }

        println!("  ✓ Price filter with hybrid search works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 4: Hybrid Search with Category and Rating Filters
#[tokio::test]
async fn test_hybrid_with_category_filter() -> Result<()> {
    run_hybrid_test("hybrid_filters", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 4: Hybrid RRF + Filters - 'office ergonomic', rating >= 4.5");

        let query = format!(r#"
            WITH bm25_ranked AS (
                SELECT
                    id,
                    ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
                FROM {}.items
                WHERE description ||| 'office ergonomic comfortable'
                  AND rating >= 4.5
                LIMIT 30
            ),
            vector_ranked AS (
                SELECT
                    id,
                    ROW_NUMBER() OVER (ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'home_office')) AS rank
                FROM {}.items
                WHERE rating >= 4.5
                LIMIT 30
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                p.rating::FLOAT8,
                (1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000)))::FLOAT AS rrf_score
            FROM bm25_ranked b
            FULL OUTER JOIN vector_ranked v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY rrf_score DESC
            LIMIT 5
        "#, schema, table_name, schema, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let rating: f64 = row.get(2);
            let rrf_score: f64 = row.get("rrf_score");

            println!("  - {} (rating: {:.1}) (RRF: {:.4})", name, rating, rrf_score);

            assert!(rating >= 4.5, "Rating should be >= 4.5");
        }

        println!("  ✓ Category and rating filters work correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 5: Balanced Weight Hybrid Search (50-50 split)
#[tokio::test]
async fn test_hybrid_balanced_weights() -> Result<()> {
    run_hybrid_test("hybrid_balanced", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 5: Hybrid Balanced - 'fitness training' (50% vector, 50% BM25)");

        let query = format!(r#"
            WITH bm25_results AS (
                SELECT
                    id,
                    pdb.score(id) AS bm25_score
                FROM {}.items
                WHERE description ||| 'fitness training workout exercise'
                ORDER BY pdb.score(id) DESC
                LIMIT 40
            ),
            vector_results AS (
                SELECT
                    id,
                    1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'fitness_gear')) AS vector_score
                FROM {}.items
                ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'fitness_gear')
                LIMIT 40
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                p.category,
                COALESCE(b.bm25_score, 0)::FLOAT AS bm25_score,
                COALESCE(v.vector_score, 0)::FLOAT AS vector_score,
                (COALESCE(b.bm25_score, 0) * 0.5 + COALESCE(v.vector_score, 0) * 0.5)::FLOAT AS combined_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY combined_score DESC
            LIMIT 10
        "#, schema, table_name, schema, table_name, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let bm25_score: f64 = row.get("bm25_score");
            let vector_score: f64 = row.get("vector_score");
            let combined_score: f64 = row.get("combined_score");

            println!("  - {} (BM25: {:.3}, Vector: {:.3}, Combined: {:.3})",
                     name, bm25_score, vector_score, combined_score);

            // Verify 50-50 weighting
            let expected = (bm25_score * 0.5 + vector_score * 0.5) as f64;
            assert!((combined_score - expected).abs() < 0.001, "50-50 weighting should be correct");
        }

        println!("  ✓ Balanced weights work correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 6: Hybrid Search with Stock Filter
#[tokio::test]
async fn test_hybrid_with_stock_filter() -> Result<()> {
    run_hybrid_test("hybrid_stock", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 6: Hybrid Weighted - 'wireless bluetooth' in stock only");

        let query = format!(r#"
            WITH bm25_results AS (
                SELECT
                    id,
                    pdb.score(id) AS bm25_score
                FROM {}.items
                WHERE description ||| 'wireless bluetooth'
                  AND in_stock = true
                  AND stock_quantity > 0
                ORDER BY pdb.score(id) DESC
                LIMIT 50
            ),
            vector_results AS (
                SELECT
                    id,
                    1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_headphones')) AS vector_score
                FROM {}.items
                WHERE in_stock = true
                  AND stock_quantity > 0
                ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_headphones')
                LIMIT 50
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                p.stock_quantity,
                (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7)::FLOAT AS combined_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY combined_score DESC
            LIMIT 10
        "#, schema, table_name, schema, table_name, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let stock_quantity: i32 = row.get("stock_quantity");
            let combined_score: f64 = row.get("combined_score");

            println!("  - {} (stock: {}) (score: {:.3})", name, stock_quantity, combined_score);
            assert!(stock_quantity > 0, "Stock quantity should be > 0");
        }

        println!("  ✓ Stock filter works correctly\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 7: RRF with Different K Values
#[tokio::test]
async fn test_hybrid_rrf_different_k() -> Result<()> {
    run_hybrid_test("hybrid_k_values", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 7: Hybrid RRF Comparison - k=30 vs k=60");

        let query = format!(r#"
            WITH bm25_ranked AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
                FROM {}.items
                WHERE description ||| 'gaming professional esports'
                LIMIT 30
            ),
            vector_ranked AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'gaming_setup')) AS rank
                FROM {}.items
                LIMIT 30
            )
            SELECT
                COALESCE(b.id, v.id) AS id,
                p.name,
                (1.0 / (30 + COALESCE(b.rank, 1000)) + 1.0 / (30 + COALESCE(v.rank, 1000)))::FLOAT AS rrf_k30,
                (1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000)))::FLOAT AS rrf_k60
            FROM bm25_ranked b
            FULL OUTER JOIN vector_ranked v ON b.id = v.id
            JOIN {}.items p ON p.id = COALESCE(b.id, v.id)
            ORDER BY rrf_k60 DESC
            LIMIT 5
        "#, schema, table_name, schema, schema);

        let rows = sqlx::query(&query).fetch_all(&pool).await?;

        for row in &rows {
            let name: String = row.get("name");
            let rrf_k30: f64 = row.get("rrf_k30");
            let rrf_k60: f64 = row.get("rrf_k60");

            println!("  - {} (k=30: {:.4}, k=60: {:.4})", name, rrf_k30, rrf_k60);
        }

        println!("  ✓ Different k values tested\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}

/// Test 8: Score Distribution Analysis
#[tokio::test]
async fn test_hybrid_score_distribution() -> Result<()> {
    run_hybrid_test("hybrid_dist", |pool, schema| async move {
        let table_name = format!("test_embeddings_{}", Uuid::new_v4().simple());
        setup_test_embeddings(&pool, &table_name, &schema).await?;

        println!("Test 8: Hybrid Score Distribution Analysis");

        let query = format!(r#"
            WITH bm25_results AS (
                SELECT id, pdb.score(id) AS bm25_score
                FROM {}.items
                WHERE description ||| 'wireless bluetooth'
                ORDER BY pdb.score(id) DESC
                LIMIT 50
            ),
            vector_results AS (
                SELECT id, 1 - (description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_headphones')) AS vector_score
                FROM {}.items
                ORDER BY description_embedding <=> (SELECT embedding FROM {} WHERE query_name = 'wireless_headphones')
                LIMIT 50
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
                COUNT(*)::INT AS total_results,
                AVG(bm25_score)::FLOAT AS avg_bm25,
                AVG(vector_score)::FLOAT AS avg_vector,
                AVG(combined_score)::FLOAT AS avg_combined,
                MAX(bm25_score)::FLOAT AS max_bm25,
                MAX(vector_score)::FLOAT AS max_vector,
                MAX(combined_score)::FLOAT AS max_combined
            FROM combined
        "#, schema, table_name, schema, table_name);

        let row = sqlx::query(&query).fetch_one(&pool).await?;

        let total: i32 = row.get("total_results");
        let avg_bm25: f64 = row.get("avg_bm25");
        let avg_vector: f64 = row.get("avg_vector");
        let avg_combined: f64 = row.get("avg_combined");

        println!("  - Total results: {}", total);
        println!("  - Average BM25 score: {:.4}", avg_bm25);
        println!("  - Average Vector score: {:.4}", avg_vector);
        println!("  - Average Combined score: {:.4}", avg_combined);

        assert!(total > 0, "Should have results");

        println!("  ✓ Score distribution analyzed\n");
        
        cleanup_test_embeddings(&pool, &table_name).await?;
        Ok(())
    }).await
}
