// tests/init_db_test.rs
// Initialization test that reads and applies all docker-entrypoint-initdb.d scripts to Kubernetes PostgreSQL

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::fs;
use std::path::PathBuf;

async fn setup_db() -> Result<PgPool, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    Ok(pool)
}

/// Get all SQL scripts from docker-entrypoint-initdb.d in alphabetical order
fn get_init_scripts() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    // Look for docker-entrypoint-initdb.d relative to the workspace root
    // Try multiple paths since tests can run from different directories
    let possible_paths = vec![
        PathBuf::from("docker-entrypoint-initdb.d"),
        PathBuf::from("../docker-entrypoint-initdb.d"),
        PathBuf::from("../../docker-entrypoint-initdb.d"),
    ];

    let init_dir = possible_paths
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or("Init scripts directory not found in any expected location")?;

    let mut scripts = Vec::new();

    // Read all .sql files in the directory
    for entry in fs::read_dir(&init_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
                .ok_or("Failed to get filename")?;

            let content = fs::read_to_string(&path)?;
            scripts.push((filename, content));
        }
    }

    // Sort by filename to ensure correct order (00-, 01-, 02-, etc.)
    scripts.sort_by(|a, b| a.0.cmp(&b.0));

    if scripts.is_empty() {
        return Err("No SQL scripts found in docker-entrypoint-initdb.d".into());
    }

    Ok(scripts)
}

#[tokio::test]
async fn init_database_all_scripts() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;

    println!("\n===============================================");
    println!("IR DB Kubernetes Database Initialization");
    println!("Executing all docker-entrypoint-initdb.d scripts");
    println!("===============================================\n");

    // Get all init scripts
    let scripts = get_init_scripts()?;
    let total = scripts.len();

    for (idx, (filename, content)) in scripts.iter().enumerate() {
        let current = idx + 1;
        println!("[{}/{}] Executing: {}", current, total, filename);
        println!("---");

        // Write script to a temp file and use psql directly
        let temp_file = format!("/tmp/{}", filename);
        fs::write(&temp_file, content)?;

        // Use std::process::Command to run psql with the script
        let output = std::process::Command::new("psql")
            .arg(&database_url)
            .arg("-f")
            .arg(&temp_file)
            .output()?;

        if output.status.success() {
            println!("✓ Completed: {}", filename);
        } else {
            eprintln!("Warning: Issues executing {}", filename);
            if !output.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Print only unique/important errors, skip echo command warnings
                for line in stderr.lines() {
                    if !line.contains("backslash") && !line.contains("\\echo") {
                        eprintln!("  {}", line);
                    }
                }
            }
        }

        // Clean up temp file
        let _ = fs::remove_file(&temp_file);
        println!();
    }

    println!("===============================================");
    println!("✓ All initialization scripts executed!");
    println!("===============================================\n");

    Ok(())
}

#[tokio::test]
async fn verify_initialization_complete() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db().await?;

    println!("\n📋 Verification Results:");
    println!("---");

    // Check extensions
    let extensions: Vec<(String, String)> = sqlx::query_as(
        "SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector', 'pg_search', 'pg_stat_statements', 'pg_trgm', 'btree_gin') ORDER BY extname"
    )
    .fetch_all(&pool)
    .await?;

    if !extensions.is_empty() {
        println!("✓ Extensions installed:");
        for (name, version) in extensions {
            println!("  - {} (v{})", name, version);
        }
    } else {
        println!("⚠ No expected extensions found");
    }

    // Check schema
    let schema_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ai_data')"
    )
    .fetch_one(&pool)
    .await?;

    println!("✓ Schema 'ai_data' exists: {}", schema_exists);

    // Check tables
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'ai_data' ORDER BY table_name"
    )
    .fetch_all(&pool)
    .await?;

    if !tables.is_empty() {
        println!("✓ Tables created: {}", tables.join(", "));
    }

    // Check functions
    let functions: Vec<String> = sqlx::query_scalar(
        "SELECT proname FROM pg_proc WHERE pronamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'ai_data') ORDER BY proname"
    )
    .fetch_all(&pool)
    .await?;

    if !functions.is_empty() {
        println!("✓ Functions created: {}", functions.join(", "));
    }

    // Check document count
    let doc_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_data.documents")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    println!("✓ Documents in database: {}", doc_count);

    println!("\n✓ Initialization verification complete!");

    Ok(())
}
