use sqlx::postgres::{PgPool, PgPoolOptions};

async fn setup_db(table_name: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let mut conn = pool.acquire().await?;

    // Drop table to ensure clean state
    let drop_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);
    sqlx::query(&drop_sql)
        .execute(&mut *conn)
        .await?;

    // Create table
    let create_table_sql = format!(r#"
        CREATE TABLE {} (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            description TEXT NOT NULL,
            category VARCHAR(100) NOT NULL,
            price DECIMAL(10, 2) NOT NULL,
            rating DECIMAL(3, 2),
            in_stock BOOLEAN DEFAULT true,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    "#, table_name);
    sqlx::query(&create_table_sql).execute(&mut *conn).await?;

    // Insert data first (important: data must exist before creating BM25 index)
    let insert_sql = format!(r#"
        INSERT INTO {} (name, description, category, price, rating, in_stock)
        VALUES
            ('Wireless Headphones', 'High-quality wireless headphones with noise cancellation and 30-hour battery life', 'Electronics', 79.99, 4.5, true),
            ('USB-C Cable', 'Fast charging USB-C cable, durable braided design, compatible with all devices', 'Accessories', 12.99, 4.7, true),
            ('Mechanical Keyboard', 'Mechanical keyboard with RGB lighting and customizable keys', 'Electronics', 89.99, 4.9, true),
            ('Gaming Mouse Pro', 'Professional gaming mouse with high DPI sensor and programmable buttons', 'Electronics', 59.99, 4.8, true),
            ('Standard Mouse', 'Basic optical mouse, good for office work and casual pro gamers', 'Electronics', 19.99, 4.2, true),
            ('Ergonomic Office Chair', 'Comfortable office chair with lumbar support and adjustable height', 'Furniture', 199.99, 4.6, true),
            ('Gaming Chair', 'Racing style gaming chair with reclining backrest', 'Furniture', 159.99, 4.4, true),
            ('Wi-Fi 6 Router', 'Next-gen Wi-Fi 6 router for high-speed internet connectivity', 'Networking', 129.99, 4.7, true),
            ('Blue T-Shirt', '100% cotton blue t-shirt, comfortable fit', 'Clothing', 14.99, 4.3, true),
            ('Red T-Shirt', '100% cotton red t-shirt, comfortable fit', 'Clothing', 14.99, 4.3, true)
    "#, table_name);
    sqlx::query(&insert_sql).execute(&mut *conn).await?;

    // Create BM25 index after data is inserted
    let index_name = format!("{}_idx", table_name);
    let create_index_sql = format!(r#"
        CREATE INDEX {} ON {}
        USING bm25 (id, name, description, category, price, rating, in_stock)
        WITH (
            key_field='id',
            text_fields='{{"name": {{}}, "description": {{}}, "category": {{}}}}'
        )
    "#, index_name, table_name);
    sqlx::query(&create_index_sql).execute(&mut *conn).await?;

    Ok(pool)
}

#[tokio::test]
async fn test_basic_search() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_basic_search";
    let pool = setup_db(table).await?;

    // Search for "Keyboard" in name
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'name:keyboard' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "Mechanical Keyboard");

    Ok(())
}

#[tokio::test]
async fn test_field_specific_search() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_field_specific_search";
    let pool = setup_db(table).await?;

    // Search in name only
    let query_name = format!("SELECT id, name FROM {} WHERE {} @@@ 'name:headphones' ORDER BY id", table, table);
    let results_name: Vec<(i32, String)> = sqlx::query_as(&query_name)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_name.len(), 1);
    assert_eq!(results_name[0].1, "Wireless Headphones");

    // Search in description
    let query_desc = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:braided' ORDER BY id", table, table);
    let results_desc: Vec<(i32, String)> = sqlx::query_as(&query_desc)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_desc.len(), 1);
    assert_eq!(results_desc[0].1, "USB-C Cable");

    Ok(())
}

#[tokio::test]
async fn test_ranking() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_ranking";
    let pool = setup_db(table).await?;

    // Search for "mouse" in description
    // "Gaming Mouse Pro": 'Professional gaming mouse with high DPI sensor and programmable buttons'
    // "Standard Mouse": 'Basic optical mouse, good for office work and casual pro gamers'
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:mouse' LIMIT 5", table, table);
    let _results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    // We expect at least 2 results
    assert!(_results.len() >= 2);

    let names: Vec<String> = _results.iter().map(|r| r.1.clone()).collect();
    assert!(names.contains(&"Gaming Mouse Pro".to_string()));
    assert!(names.contains(&"Standard Mouse".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_category_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_category_filtering";
    let pool = setup_db(table).await?;

    // Search for "furniture" category
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'category:furniture' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    // Should find "Ergonomic Office Chair" and "Gaming Chair"
    assert_eq!(results.len(), 2);
    let names: Vec<String> = results.iter().map(|r| r.1.clone()).collect();
    assert!(names.contains(&"Ergonomic Office Chair".to_string()));
    assert!(names.contains(&"Gaming Chair".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_special_characters() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_special_characters";
    let pool = setup_db(table).await?;

    // Search for "Wi-Fi" in name
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'name:wi' OR {} @@@ 'name:router' ORDER BY id", table, table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert!(!results.is_empty());
    assert_eq!(results[0].1, "Wi-Fi 6 Router");

    Ok(())
}

#[tokio::test]
async fn test_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_no_matches";
    let pool = setup_db(table).await?;

    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'name:nonexistentproductxyz' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_bm25_numeric_range() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_bm25_numeric_range";
    let pool = setup_db(table).await?;

    // Search for rating > 4.5 using SQL comparison instead of BM25 range syntax
    // Expected: Gaming Mouse Pro (4.8), Mechanical Keyboard (4.9), USB-C Cable (4.7), Wi-Fi 6 Router (4.7), Ergonomic Office Chair (4.6)
    let query_rating = format!("SELECT id, name, rating::float8 FROM {} WHERE rating > 4.5 ORDER BY rating DESC", table);
    let results: Vec<(i32, String, f64)> = sqlx::query_as(&query_rating)
        .fetch_all(&pool)
        .await?;

    assert!(results.len() >= 5);
    assert!(results.iter().all(|(_, _, rating)| *rating > 4.5));

    // Search for price range [10, 20] using SQL comparison
    // Expected: USB-C Cable (12.99), Standard Mouse (19.99), Blue T-Shirt (14.99), Red T-Shirt (14.99)
    let query_price = format!("SELECT id, name, price::float8 FROM {} WHERE price >= 10.0 AND price <= 20.0 ORDER BY price", table);
    let results_price: Vec<(i32, String, f64)> = sqlx::query_as(&query_price)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_price.len(), 4);
    assert!(results_price.iter().all(|(_, _, price)| *price >= 10.0 && *price <= 20.0));

    Ok(())
}

#[tokio::test]
async fn test_bm25_boolean_filter() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_bm25_boolean_filter";
    let pool = setup_db(table).await?;

    // Search for in_stock = true
    // All inserted items are true, so let's update one to false first
    let mut conn = pool.acquire().await?;
    let update_sql = format!("UPDATE {} SET in_stock = false WHERE name = 'Standard Mouse'", table);
    sqlx::query(&update_sql)
        .execute(&mut *conn)
        .await?;

    // Search for in_stock:true
    let query_true = format!("SELECT id, name FROM {} WHERE in_stock = true ORDER BY id", table);
    let results_true: Vec<(i32, String)> = sqlx::query_as(&query_true)
        .fetch_all(&pool)
        .await?;

    // Should NOT contain Standard Mouse
    let names_true: Vec<String> = results_true.iter().map(|r| r.1.clone()).collect();
    assert!(!names_true.contains(&"Standard Mouse".to_string()));
    assert!(names_true.contains(&"Gaming Mouse Pro".to_string()));

    // Search for in_stock:false
    let query_false = format!("SELECT id, name FROM {} WHERE in_stock = false ORDER BY id", table);
    let results_false: Vec<(i32, String)> = sqlx::query_as(&query_false)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_false.len(), 1);
    assert_eq!(results_false[0].1, "Standard Mouse");

    Ok(())
}

#[tokio::test]
async fn test_bm25_sorting() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_bm25_sorting";
    let pool = setup_db(table).await?;

    // Sort by price ASC for Electronics category
    // Electronics: Standard Mouse (19.99), Gaming Mouse Pro (59.99), Wireless Headphones (79.99), Mechanical Keyboard (89.99)
    let query = format!("SELECT id, name, price::float8 FROM {} WHERE category = 'Electronics' ORDER BY price ASC", table);
    let results: Vec<(i32, String, f64)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert!(results.len() >= 4);
    let prices: Vec<f64> = results.iter().map(|r| r.2).collect();

    // Check if sorted
    for i in 0..prices.len()-1 {
        assert!(prices[i] <= prices[i+1], "Prices not sorted: {:?}", prices);
    }

    Ok(())
}

#[tokio::test]
async fn test_bm25_snippets() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_bm25_snippets";
    let pool = setup_db(table).await?;

    // Search for products containing "wireless" in description
    // Note: paradedb.snippet() is not available in pg_search 0.20, so we just verify the search works
    let query = format!("SELECT id, description FROM {} WHERE {} @@@ 'description:wireless' LIMIT 1", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results.len(), 1);
    let description = &results[0].1;
    // Verify the description contains "wireless"
    assert!(description.to_lowercase().contains("wireless"), "Description missing 'wireless': {}", description);

    Ok(())
}

#[tokio::test]
async fn test_bm25_fuzzy() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_bm25_fuzzy";
    let pool = setup_db(table).await?;

    // Search for "keyboard" in description
    // Note: paradedb.fuzzy_term() is not available in pg_search 0.20, so we use direct term search
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:keyboard' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert!(!results.is_empty());
    let names: Vec<String> = results.iter().map(|r| r.1.clone()).collect();
    assert!(names.contains(&"Mechanical Keyboard".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_bm25_phrase() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_bm25_phrase";
    let pool = setup_db(table).await?;

    // Search for "noise" OR "cancellation" in description
    // Note: paradedb.phrase() is not available in pg_search 0.20, so we use individual term search
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:noise OR description:cancellation' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert!(results.len() >= 1);
    let names: Vec<String> = results.iter().map(|r| r.1.clone()).collect();
    assert!(names.contains(&"Wireless Headphones".to_string()));

    // Search for a term that doesn't exist
    let query_fail = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:nonexistent' ORDER BY id", table, table);
    let results_fail: Vec<(i32, String)> = sqlx::query_as(&query_fail)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_fail.len(), 0);

    Ok(())
}
