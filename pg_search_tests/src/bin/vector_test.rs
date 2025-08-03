use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing vector similarity search...");

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected successfully!");

    // Check if pgvector extension is available
    let pgvector_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')"
    )
    .fetch_one(&pool)
    .await?;

    if !pgvector_check.0 {
        println!("✗ pgvector extension is NOT installed - skipping vector tests");
        return Ok(());
    }

    println!("✓ pgvector extension is installed");

    // TODO: Add vector similarity search tests here

    Ok(())
}
