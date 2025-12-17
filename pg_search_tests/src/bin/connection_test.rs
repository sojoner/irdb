use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;

fn main() -> Result<()> {
    // Create a Tokio runtime explicitly to avoid macro issues if features aren't perfect
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    println!("Testing PostgreSQL connection...");

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected successfully!");

    // Check for indexes on products.items
    println!("\nChecking indexes on products.items:");
    let indexes: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT indexname, indexdef
        FROM pg_indexes
        WHERE schemaname = 'products' AND tablename = 'items'
        "#
    )
    .fetch_all(&pool)
    .await?;

    if !indexes.is_empty() {
        for (name, def) in indexes {
            println!("  - {}: {}", name, def);
        }
    } else {
        println!("  No indexes found on products.items");
    }

    // Test Search Query (BM25)
    println!("\nTesting BM25 Search with empty query:");
    let query = "";
    let sql = r#"
        SELECT id, name, description, pdb.score(id) as score
        FROM products.items
        WHERE ($1 = '' OR description ||| $1)
        LIMIT 5
    "#;

    let rows = sqlx::query(sql)
        .bind(query)
        .fetch_all(&pool)
        .await;

    match rows {
        Ok(rows) => {
            println!("✓ Search query executed successfully. Found {} rows.", rows.len());
            for row in rows {
                let id: i32 = row.get("id");
                let name: String = row.get("name");
                let score: Option<f64> = row.get("score");
                println!("  - ID: {}, Name: {}, Score: {:?}", id, name, score);
            }
        }
        Err(e) => {
            println!("✗ Search query failed: {}", e);
        }
    }

    // Test Count Query
    println!("\nTesting Count Query with empty query:");
    let count_sql = r#"
        SELECT COUNT(*)
        FROM products.items
        WHERE ($1 = '' OR description ||| $1)
    "#;
    
    let count_res = sqlx::query(count_sql)
        .bind(query)
        .fetch_one(&pool)
        .await;

    match count_res {
        Ok(row) => {
            let count: i64 = row.get(0);
            println!("✓ Count query executed successfully. Count: {}", count);
        }
        Err(e) => {
            println!("✗ Count query failed: {}", e);
        }
    }

    // Test Search Query (Vector)
    println!("\nTesting Vector Search (random vector):");
    // Generate a random vector string for testing
    let vec_dim = 1536;
    let vec_values: Vec<String> = (0..vec_dim).map(|_| "0.0".to_string()).collect();
    let vec_str = format!("[{}]", vec_values.join(","));

    let vector_sql = r#"
        SELECT id, name, (1 - (description_embedding <=> $1::vector(1536))) as score
        FROM products.items
        ORDER BY description_embedding <=> $1::vector(1536)
        LIMIT 5
    "#;

    let vector_rows = sqlx::query(vector_sql)
        .bind(&vec_str)
        .fetch_all(&pool)
        .await;

    match vector_rows {
        Ok(rows) => {
            println!("✓ Vector search executed successfully. Found {} rows.", rows.len());
            for row in rows {
                let id: i32 = row.get("id");
                let name: String = row.get("name");
                let score: Option<f64> = row.get("score");
                println!("  - ID: {}, Name: {}, Score: {:?}", id, name, score);
            }
        }
        Err(e) => {
            println!("✗ Vector search failed: {}", e);
        }
    }

    Ok(())
}
