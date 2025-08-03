use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing BM25 full-text search...");

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected successfully!");

    // Check if pg_search extension is available
    let pg_search_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'pg_search')"
    )
    .fetch_one(&pool)
    .await?;

    if !pg_search_check.0 {
        println!("✗ pg_search extension is NOT installed - skipping BM25 tests");
        return Ok(());
    }

    println!("✓ pg_search extension is installed");

    // TODO: Add BM25 search tests here

    Ok(())
}
