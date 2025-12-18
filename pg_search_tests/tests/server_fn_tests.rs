// tests/server_fn_tests.rs
// Test suite for Leptos server functions

mod common;

use pg_search_tests::web_app::server_fns::*;
use pg_search_tests::web_app::model::*;
use pg_search_tests::web_app::api::db;
use common::{create_test_pool, setup_test_db};

#[tokio::test]
async fn test_server_functions_comprehensive() -> anyhow::Result<()> {
    let pool = create_test_pool().await?;
    setup_test_db(&pool, None).await?;
    db::set_test_pool(pool.clone());

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

    // 1. Test search_products
    println!("Testing search_products...");
    let results = search_products("Sony".to_string(), SearchMode::Bm25, filters.clone()).await
        .map_err(|e| anyhow::anyhow!("search_products failed: {}", e))?;
    assert!(results.total_count > 0);
    let product_id = results.results[0].product.id;

    // 2. Test get_product
    println!("Testing get_product for id={}...", product_id);
    let product = get_product(product_id).await
        .map_err(|e| anyhow::anyhow!("get_product failed: {}", e))?;
    assert_eq!(product.id, product_id);

    // 3. Test get_analytics
    println!("Testing get_analytics...");
    let analytics = get_analytics().await
        .map_err(|e| anyhow::anyhow!("get_analytics failed: {}", e))?;
    assert!(analytics.total_products > 0);

    Ok(())
}
