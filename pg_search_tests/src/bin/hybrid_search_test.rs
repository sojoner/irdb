use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing hybrid search (vector + BM25)...");

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected successfully!");

    // Check if both extensions are available
    let pgvector_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')"
    )
    .fetch_one(&pool)
    .await?;

    let pg_search_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'pg_search')"
    )
    .fetch_one(&pool)
    .await?;

    if !pgvector_check.0 || !pg_search_check.0 {
        println!("✗ Required extensions not installed - skipping hybrid search tests");
        return Ok(());
    }

    println!("✓ Both pgvector and pg_search extensions are installed");

    // TODO: Add hybrid search tests here

    Ok(())
}
