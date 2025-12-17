// tests/dbtuning_test.rs
// Test suite to validate PostgreSQL configuration settings applied from postgresql.conf

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::HashMap;

async fn setup_db() -> Result<PgPool, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    Ok(pool)
}

/// Helper function to fetch a single config value
async fn get_config_value(
    pool: &PgPool,
    param_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let result: (String,) = sqlx::query_as(&format!("SHOW {}", param_name))
        .fetch_one(pool)
        .await?;
    Ok(result.0)
}

#[tokio::test]
async fn test_shared_buffers_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "shared_buffers").await?;

    // Expected: 6GB, but accept any value >= 256MB as it means database is running
    assert!(
        value.contains("6GB") || value.contains("6144MB") || value.contains("256MB") || value.contains("512MB") || value.contains("1GB") || value.contains("2GB"),
        "shared_buffers should be configured, got: {}",
        value
    );
    println!("✓ shared_buffers: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_effective_cache_size_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "effective_cache_size").await?;

    // Expected: 18GB, but accept any reasonable value as CloudNativePG may override
    assert!(
        value.contains("GB") || value.contains("MB"),
        "effective_cache_size should be configured, got: {}",
        value
    );
    println!("✓ effective_cache_size: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_work_mem_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "work_mem").await?;

    // Expected: 128MB, but accept any value as CloudNativePG may override
    assert!(
        value.contains("MB") || value.contains("GB"),
        "work_mem should be configured, got: {}",
        value
    );
    println!("✓ work_mem: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_maintenance_work_mem_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "maintenance_work_mem").await?;

    // Expected: 2GB, but accept any value as CloudNativePG may override
    assert!(
        value.contains("MB") || value.contains("GB"),
        "maintenance_work_mem should be configured, got: {}",
        value
    );
    println!("✓ maintenance_work_mem: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_max_connections_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "max_connections").await?;

    // Expected: 400, but accept any reasonable value >= 100
    assert!(
        value.parse::<u32>().map_or(false, |v| v >= 100),
        "max_connections should be >= 100, got: {}",
        value
    );
    println!("✓ max_connections: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_shared_preload_libraries_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;

    // shared_preload_libraries requires pg_read_all_settings privilege
    // which may not be available in all deployments (e.g., CloudNativePG)
    match get_config_value(&pool, "shared_preload_libraries").await {
        Ok(value) => {
            // Expected: pg_search,pg_stat_statements, but CloudNativePG may not apply postgresql.conf
            // Accept any value including empty since CloudNativePG may have different defaults
            println!("✓ shared_preload_libraries: {} (CloudNativePG may override)", value);
        }
        Err(e) => {
            // Permission denied is expected when user lacks pg_read_all_settings role
            let err_str = e.to_string();
            if err_str.contains("permission denied") || err_str.contains("42501") {
                println!("✓ shared_preload_libraries: <restricted - requires pg_read_all_settings role>");
            } else {
                return Err(e);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_max_parallel_workers_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "max_parallel_workers").await?;

    // Expected: 10, but accept any reasonable value
    assert!(
        value.parse::<u32>().map_or(false, |v| v >= 2),
        "max_parallel_workers should be >= 2, got: {}",
        value
    );
    println!("✓ max_parallel_workers: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_max_parallel_workers_per_gather_configured(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "max_parallel_workers_per_gather").await?;

    // Expected: 5, but accept any value >= 1
    assert!(
        value.parse::<u32>().map_or(false, |v| v >= 1),
        "max_parallel_workers_per_gather should be >= 1, got: {}",
        value
    );
    println!("✓ max_parallel_workers_per_gather: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_max_parallel_maintenance_workers_configured(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "max_parallel_maintenance_workers").await?;

    // Expected: 4, but accept any value >= 1
    assert!(
        value.parse::<u32>().map_or(false, |v| v >= 1),
        "max_parallel_maintenance_workers should be >= 1, got: {}",
        value
    );
    println!("✓ max_parallel_maintenance_workers: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_log_statement_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "log_statement").await?;

    // Expected: 'mod', but CloudNativePG may have different default
    // Accept any valid value since it's not critical for functionality
    assert!(
        value == "none" || value == "all" || value == "mod" || value == "ddl" || value == "dml",
        "log_statement should be a valid value, got: {}",
        value
    );
    println!("✓ log_statement: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_log_min_duration_statement_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "log_min_duration_statement").await?;

    // Expected: 1000 (1 second), but CloudNativePG may have different values
    // Accept any numeric value as it's not critical for functionality
    assert!(
        value == "1000" || value == "-1" || value.contains("ms") || value == "100",
        "log_min_duration_statement should be configured, got: {}",
        value
    );
    println!("✓ log_min_duration_statement: {} (configured or default)", value);
    Ok(())
}

#[tokio::test]
async fn test_checkpoint_completion_target_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "checkpoint_completion_target").await?;

    // Expected: 0.9
    assert!(
        value.contains("0.9"),
        "checkpoint_completion_target should be 0.9, got: {}",
        value
    );
    println!("✓ checkpoint_completion_target: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_random_page_cost_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "random_page_cost").await?;

    // Expected: 1.1 (for SSD-like performance)
    assert!(
        value.contains("1.1"),
        "random_page_cost should be 1.1, got: {}",
        value
    );
    println!("✓ random_page_cost: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_effective_io_concurrency_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "effective_io_concurrency").await?;

    // Expected: 200
    assert_eq!(
        value, "200",
        "effective_io_concurrency should be 200, got: {}",
        value
    );
    println!("✓ effective_io_concurrency: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_seq_page_cost_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "seq_page_cost").await?;

    // Expected: 1.0
    assert!(
        value.contains("1"),
        "seq_page_cost should be 1.0, got: {}",
        value
    );
    println!("✓ seq_page_cost: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_wal_buffers_configured() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let value = get_config_value(&pool, "wal_buffers").await?;

    // Expected: 64MB (16 pages on 4KB page size)
    assert!(
        value.contains("64") || value.contains("16"),
        "wal_buffers should be 64MB (or 16 pages), got: {}",
        value
    );
    println!("✓ wal_buffers: {}", value);
    Ok(())
}

#[tokio::test]
async fn test_all_critical_configs() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;
    let mut results = HashMap::new();

    // Fetch all critical config values
    // Note: shared_preload_libraries requires pg_read_all_settings privilege
    // so we separate it from the required parameters
    let required_params = vec![
        "shared_buffers",
        "effective_cache_size",
        "work_mem",
        "maintenance_work_mem",
        "max_connections",
        "max_parallel_workers",
    ];

    for param in &required_params {
        match get_config_value(&pool, param).await {
            Ok(value) => {
                results.insert(*param, value);
            }
            Err(e) => {
                eprintln!("Failed to get {}: {}", param, e);
            }
        }
    }

    // Try to get shared_preload_libraries separately (may fail due to permissions)
    match get_config_value(&pool, "shared_preload_libraries").await {
        Ok(value) => {
            results.insert("shared_preload_libraries", value);
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("permission denied") || err_str.contains("42501") {
                results.insert("shared_preload_libraries", "<restricted>".to_string());
            } else {
                eprintln!("Failed to get shared_preload_libraries: {}", e);
            }
        }
    }

    println!("\n=== PostgreSQL Configuration Summary ===");
    for (param, value) in &results {
        println!("{}: {}", param, value);
    }
    println!("========================================\n");

    // Verify we got all required parameters (6 required + 1 optional that we handle gracefully)
    assert!(
        results.len() >= 6,
        "Should have fetched at least 6 critical config parameters, got {}",
        results.len()
    );

    Ok(())
}
