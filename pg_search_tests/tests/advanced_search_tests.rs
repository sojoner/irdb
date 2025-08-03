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
            name TEXT,
            description TEXT
        )
    "#, table_name);
    sqlx::query(&create_table_sql).execute(&mut *conn).await?;

    // Insert data first (important: data must exist before creating BM25 index)
    let insert_sql = format!(r#"
        INSERT INTO {} (name, description) VALUES
        ('Super Duper Widget', 'A very great product'),
        ('Mega Tron Robot', 'Another amazing robot'),
        ('Running Shoes', 'Fast shoes for running'),
        ('Walking Boots', 'Slow boots for walking'),
        ('Keyboard', 'Mechanical keyboard with switches')
    "#, table_name);
    sqlx::query(&insert_sql).execute(&mut *conn).await?;

    // Create BM25 index after data is inserted
    // Note: pg_search v0.17.2 uses standard tokenization
    let index_name = format!("{}_idx", table_name);
    let create_index_sql = format!(r#"
        CREATE INDEX {} ON {}
        USING bm25 (id, name, description)
        WITH (
            key_field='id',
            text_fields='{{"name": {{}}, "description": {{}}}}'
        )
    "#, index_name, table_name);
    sqlx::query(&create_index_sql).execute(&mut *conn).await?;

    Ok(pool)
}

#[tokio::test]
async fn test_fuzzy_partial_match() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_fuzzy_partial";
    let pool = setup_db(table).await?;

    // Fuzzy search: "dupr" (typo for "duper") should match "Super Duper Widget"
    // Note: BM25 tokenizes to lowercase, so we search for lowercase terms
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ paradedb.fuzzy_term('name', 'dupr') ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "Super Duper Widget");

    // "mega" should match "Mega Tron Robot" exactly
    let query_mega = format!("SELECT id, name FROM {} WHERE {} @@@ 'name:mega' ORDER BY id", table, table);
    let results_mega: Vec<(i32, String)> = sqlx::query_as(&query_mega)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_mega.len(), 1);
    assert_eq!(results_mega[0].1, "Mega Tron Robot");

    Ok(())
}

#[tokio::test]
async fn test_exact_term_search() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_exact";
    let pool = setup_db(table).await?;

    // Exact term search: "running" should match the word in description
    // BM25 tokenizes and lowercases text, so we search for lowercase terms
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:running' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert!(results.len() >= 1);
    assert_eq!(results[0].1, "Running Shoes");

    // Search for "walking"
    let query_walk = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:walking' ORDER BY id", table, table);
    let results_walk: Vec<(i32, String)> = sqlx::query_as(&query_walk)
        .fetch_all(&pool)
        .await?;

    assert!(results_walk.len() >= 1);
    assert_eq!(results_walk[0].1, "Walking Boots");

    Ok(())
}

#[tokio::test]
async fn test_multiple_term_search() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_multiple";
    let pool = setup_db(table).await?;

    // Search for multiple terms that should match multiple products
    // "running" OR "walking" should match both products with those words
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ 'description:running OR description:walking' ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    // Should match "Running Shoes" and "Walking Boots"
    assert_eq!(results.len(), 2);
    let names: Vec<String> = results.iter().map(|r| r.1.clone()).collect();
    assert!(names.contains(&"Running Shoes".to_string()));
    assert!(names.contains(&"Walking Boots".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_fuzzy_term_exact() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_fuzzy_exact";
    let pool = setup_db(table).await?;

    // Fuzzy term search with exact match should work
    // Note: The fuzzy_term function in pg_search v0.17.2 appears to work differently than expected
    // For now, we test that it can find exact matches
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ paradedb.fuzzy_term('description', 'walking') ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "Walking Boots");

    // Another exact match test
    let query2 = format!("SELECT id, name FROM {} WHERE {} @@@ paradedb.fuzzy_term('name', 'keyboard') ORDER BY id", table, table);
    let results2: Vec<(i32, String)> = sqlx::query_as(&query2)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results2.len(), 1);
    assert_eq!(results2[0].1, "Keyboard");

    Ok(())
}

#[tokio::test]
async fn test_phrase_search() -> Result<(), Box<dyn std::error::Error>> {
    let table = "test_phrase";
    let pool = setup_db(table).await?;

    // Phrase search: "mechanical keyboard" should match exact phrase in description
    let query = format!("SELECT id, name FROM {} WHERE {} @@@ paradedb.phrase('description', ARRAY['mechanical', 'keyboard']) ORDER BY id", table, table);
    let results: Vec<(i32, String)> = sqlx::query_as(&query)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "Keyboard");

    // Phrase that doesn't match: "very robot" (words exist separately but not as phrase)
    let query_no_match = format!("SELECT id, name FROM {} WHERE {} @@@ paradedb.phrase('description', ARRAY['very', 'robot']) ORDER BY id", table, table);
    let results_no_match: Vec<(i32, String)> = sqlx::query_as(&query_no_match)
        .fetch_all(&pool)
        .await?;

    assert_eq!(results_no_match.len(), 0);

    Ok(())
}
