use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing PostgreSQL connection...");

    // Connection string using the NodePort service
    // Host: k0s worker node IP, Port: NodePort 30432
    // Note: This connects to the database that was initialized with postInitSQL
    // The database password should be retrieved from Kubernetes secret
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    // Create connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected successfully!");

    // Get PostgreSQL version
    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&pool)
        .await?;

    println!("PostgreSQL version: {}", version.0);

    // Check if pgvector extension is available
    let pgvector_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')"
    )
    .fetch_one(&pool)
    .await?;

    if pgvector_check.0 {
        println!("✓ pgvector extension is installed");
    } else {
        println!("✗ pgvector extension is NOT installed");
    }

    // Check if pg_search extension is available
    let pg_search_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'pg_search')"
    )
    .fetch_one(&pool)
    .await?;

    if pg_search_check.0 {
        println!("✓ pg_search extension is installed");
    } else {
        println!("✗ pg_search extension is NOT installed");
    }

    // Check if ai_data schema exists
    let schema_check: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ai_data')"
    )
    .fetch_one(&pool)
    .await?;

    if schema_check.0 {
        println!("✓ ai_data schema exists");
    } else {
        println!("✗ ai_data schema does NOT exist");
    }

    // List all tables in ai_data schema
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'ai_data' ORDER BY table_name"
    )
    .fetch_all(&pool)
    .await?;

    if !tables.is_empty() {
        println!("\nTables in ai_data schema:");
        for (table_name,) in tables {
            println!("  - {}", table_name);
        }
    } else {
        println!("No tables found in ai_data schema");
    }

    Ok(())
}
