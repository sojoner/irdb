// tests/integration_tests.rs
// Integration tests for BM25 search

use sqlx::postgres::{PgPool, PgPoolOptions};

use pg_search_tests::fixtures::{TestTable, tables::products::ProductsTable};

async fn setup_db() -> Result<PgPool, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    Ok(pool)
}

#[tokio::test]
async fn test_bm25_search() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let mut conn = pool.acquire().await?;

    // 1. Clean up
    sqlx::query("DROP TABLE IF EXISTS products CASCADE")
        .execute(&mut *conn)
        .await?;

    // 2. Setup (Execute each command in order)
    for sql in ProductsTable::setup_sql() {
        sqlx::query(sql)
            .execute(&mut *conn)
            .await?;
    }

    // 3. Test BM25 Search
    let results: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, name FROM products WHERE products @@@ 'description:headphones' ORDER BY id"
    )
    .fetch_all(&mut *conn)
    .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "Wireless Headphones");
    println!("✓ Found 'Wireless Headphones'");

    Ok(())
}
