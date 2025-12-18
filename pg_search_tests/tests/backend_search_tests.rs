mod common;

use pg_search_tests::web_app::api::queries;
use pg_search_tests::web_app::model::{SearchFilters, SortOption};

use common::{create_test_pool, setup_test_db, teardown_test_db};

#[tokio::test]
async fn test_backend_search() -> anyhow::Result<()> {
    // Setup with default "products" schema (what queries module expects)
    let pool = create_test_pool().await?;
    setup_test_db(&pool, None).await?; // None = use default "products" schema

    // Default filters
    let filters = SearchFilters {
        categories: vec![],
        price_min: None,
        price_max: None,
        min_rating: None,
        in_stock_only: false,
        sort_by: SortOption::Relevance,
        page: 0,
        page_size: 10,
    };

    // Test 1: Wildcard Search (BM25)
    // This verifies that "*" is treated as match-all
    println!("\nTesting Wildcard Search (BM25)...");
    let results = queries::search_bm25(&pool, "*", &filters).await?;
    println!("Found {} results for wildcard", results.total_count);
    assert!(results.total_count > 0, "Wildcard search should return results");
    
    // Test 2: Specific Search (BM25)
    // Search for a known brand from the screenshot (Sony)
    println!("\nTesting Specific Search (BM25) for 'Sony'...");
    let results = queries::search_bm25(&pool, "Sony", &filters).await?;
    println!("Found {} results for 'Sony'", results.total_count);
    assert!(results.total_count > 0, "Should find Sony products");
    
    // Verify the first result is relevant
    if let Some(first) = results.results.first() {
        println!("First result: {} ({})", first.product.name, first.product.brand);
        assert!(
            first.product.name.to_lowercase().contains("sony") || 
            first.product.brand.to_lowercase().contains("sony"),
            "First result should be relevant to Sony"
        );
    }

    // Test 3: Vector Search
    // Search for "headphones" (semantic match)
    println!("\nTesting Vector Search for 'headphones'...");
    let results = queries::search_vector(&pool, "headphones", &filters).await?;
    println!("Found {} results for 'headphones'", results.total_count);
    assert!(results.total_count > 0, "Vector search should return results");

    // Test 4: Hybrid Search
    // Search for "wireless" (common term)
    println!("\nTesting Hybrid Search for 'wireless'...");
    let results = queries::search_hybrid(&pool, "wireless", &filters).await?;
    println!("Found {} results for 'wireless'", results.total_count);
    assert!(results.total_count > 0, "Hybrid search should return results");

    // Test 5: Empty Query (should match all)
    println!("\nTesting Empty Query...");
    let results = queries::search_hybrid(&pool, "", &filters).await?;
    println!("Found {} results for empty query", results.total_count);
    assert!(results.total_count > 0, "Empty query should return results");

    // Cleanup
    teardown_test_db(&pool, None).await?;

    Ok(())
}
