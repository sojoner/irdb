// web_app/api/db.rs - Database connection pool setup
//
// This module provides database pool initialization.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use std::sync::OnceLock;

static POOL: OnceLock<PgPool> = OnceLock::new();

/// Initialize the global database pool
pub fn init_db(pool: PgPool) {
    tracing::info!("Initializing global database pool");
    if POOL.set(pool).is_err() {
        tracing::warn!("Database pool already initialized");
    } else {
        tracing::info!("Global database pool initialized successfully");
    }
}

/// Get the global database pool
pub fn get_db() -> Option<PgPool> {
    let pool = POOL.get().cloned();
    if pool.is_some() {
        tracing::debug!("Global pool retrieved successfully");
    } else {
        tracing::warn!("Global pool is empty!");
    }
    pool
}

/// Create a PostgreSQL connection pool
///
/// Reads DATABASE_URL from environment and creates a connection pool
/// with sensible defaults for a web application.
pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_create_pool() {
        dotenv::dotenv().ok();

        let pool_result = create_pool().await;
        assert!(pool_result.is_ok(), "Should create pool successfully");

        let pool = pool_result.unwrap();

        // Test basic query
        let result: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(result.0, 1);
    }
}
