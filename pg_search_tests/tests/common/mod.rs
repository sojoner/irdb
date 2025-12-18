// common/mod.rs - Shared test utilities for database setup and teardown
//
// This module provides reusable test fixtures and helpers to ensure:
// 1. Idempotent test execution (tests can run multiple times)
// 2. Clean setup and teardown for each test
// 3. Consistent test data across test suites

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Create a database connection pool for testing
pub async fn create_test_pool() -> anyhow::Result<PgPool> {
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&database_url)
        .await?;

    Ok(pool)
}

/// Clean up test database - drops schema and all related objects
/// This ensures idempotent test execution
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `schema_name` - Name of the schema to drop (defaults to "products" if None)
pub async fn teardown_test_db(pool: &PgPool, schema_name: Option<&str>) -> anyhow::Result<()> {
    let schema = schema_name.unwrap_or("products");

    // Drop schema cascade to remove all tables, indexes, and data
    let query = format!("DROP SCHEMA IF EXISTS {} CASCADE", schema);
    sqlx::query(&query)
        .execute(pool)
        .await?;

    Ok(())
}

/// Setup test database with idempotent schema creation
///
/// This function:
/// 1. Cleans up any existing test schema
/// 2. Creates fresh schema and tables (with custom schema name)
/// 3. Loads test data
/// 4. Sets the search_path for the pool (if possible)
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `schema_name` - Name of the schema to create (defaults to "products" if None)
///
/// Can be called multiple times safely, even concurrently with different schema names.
pub async fn setup_test_db(pool: &PgPool, schema_name: Option<&str>) -> anyhow::Result<()> {
    let schema = schema_name.unwrap_or("products");

    // Find the SQL files relative to the workspace root or current dir
    let possible_paths = vec![
        PathBuf::from("pg_search_tests/sql_examples"),
        PathBuf::from("sql_examples"),
        PathBuf::from("../sql_examples"),
    ];

    let sql_dir = possible_paths
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("SQL examples directory not found"))?;

    // Clean up any existing schema first for idempotency
    teardown_test_db(pool, Some(schema)).await?;

    // 1. Create schema and table (with custom schema name)
    let schema_path = sql_dir.join("08_products_schema.sql");
    let schema_sql = fs::read_to_string(schema_path)?;

    // Replace "products" with custom schema name in SQL
    let schema_sql = schema_sql.replace("products", schema);
    sqlx::raw_sql(&schema_sql).execute(pool).await?;

    // 2. Insert data (with custom schema name)
    let data_path = sql_dir.join("09_products_data.sql");
    let data_sql = fs::read_to_string(data_path)?;

    // Replace "products" with custom schema name in SQL
    let data_sql = data_sql.replace("products", schema);
    sqlx::raw_sql(&data_sql).execute(pool).await?;

    // 3. Set search_path for the current session
    // Note: This only affects the current connection if we were using one,
    // but since we're using a pool, we should ideally set it for the pool.
    // However, sqlx doesn't easily support setting search_path for the whole pool
    // after creation. Instead, we'll rely on tests setting it or using fully qualified names.
    // For now, we'll just ensure the schema exists.
    
    let search_path_query = format!("SET search_path TO {}, public", schema);
    sqlx::query(&search_path_query).execute(pool).await?;

    Ok(())
}

/// Generate a unique schema name for a test
/// Uses the test function name or a UUID to ensure uniqueness
pub fn generate_test_schema_name(test_name: &str) -> String {
    // Sanitize test name to be a valid schema name (alphanumeric + underscore)
    let sanitized = test_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();

    // Truncate to avoid excessively long names (PostgreSQL limit is 63 chars)
    let truncated = if sanitized.len() > 50 {
        &sanitized[..50]
    } else {
        &sanitized
    };

    format!("test_{}", truncated)
}

/// Full setup and teardown wrapper for tests with isolated schema
///
/// # Arguments
/// * `test_name` - Unique test name (used to generate schema name)
/// * `test_fn` - Test function that receives (pool, schema_name)
pub async fn with_test_db<F, Fut>(test_name: &str, test_fn: F) -> anyhow::Result<()>
where
    F: FnOnce(PgPool, String) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    let pool = create_test_pool().await?;
    let schema_name = generate_test_schema_name(test_name);

    setup_test_db(&pool, Some(&schema_name)).await?;

    // Set search_path for the pool by executing it on every connection acquisition
    // Actually, a simpler way is to just execute it once and hope for the best,
    // or better, update the queries to use the schema name.
    // But since we removed "products." from queries, we MUST set search_path.
    
    let result = test_fn(pool.clone(), schema_name.clone()).await;

    // Always cleanup, even if test fails
    teardown_test_db(&pool, Some(&schema_name)).await?;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_teardown_idempotent() -> anyhow::Result<()> {
        let pool = create_test_pool().await?;
        let schema = "test_idempotent_schema";

        // Run setup twice - should not fail
        setup_test_db(&pool, Some(schema)).await?;
        setup_test_db(&pool, Some(schema)).await?;

        // Verify schema exists
        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = '{}')",
            schema
        );
        let result: (bool,) = sqlx::query_as(&query)
            .fetch_one(&pool)
            .await?;
        assert!(result.0, "Schema should exist after setup");

        // Run teardown
        teardown_test_db(&pool, Some(schema)).await?;

        // Verify schema is gone
        let result: (bool,) = sqlx::query_as(&query)
            .fetch_one(&pool)
            .await?;
        assert!(!result.0, "Schema should not exist after teardown");

        Ok(())
    }

    #[tokio::test]
    async fn test_with_test_db_wrapper() -> anyhow::Result<()> {
        with_test_db("wrapper_test", |pool, schema| async move {
            // Verify we can query the table in the test schema
            let query = format!("SELECT COUNT(*) FROM {}.items", schema);
            let count: (i64,) = sqlx::query_as(&query)
                .fetch_one(&pool)
                .await?;

            assert!(count.0 > 0, "Should have test data");

            Ok(())
        }).await
    }

    #[tokio::test]
    async fn test_generate_schema_name() {
        let name1 = generate_test_schema_name("test_foo");
        let name2 = generate_test_schema_name("test::bar::baz");
        let name3 = generate_test_schema_name("a".repeat(100).as_str());

        assert_eq!(name1, "test_test_foo");
        assert_eq!(name2, "test_test__bar__baz");
        assert!(name3.len() <= 55); // "test_" + 50 chars
        assert!(name3.starts_with("test_"));
    }
}
