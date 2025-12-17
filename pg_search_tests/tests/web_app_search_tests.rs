/// Integration tests for web_app search functionality
///
/// These tests verify that BM25, Vector, and Hybrid search queries
/// work correctly against a real PostgreSQL database with ParadeDB and pgvector.
///
/// Prerequisites:
/// - DATABASE_URL environment variable set
/// - PostgreSQL 17.5 with pg_search 0.20+ and pgvector 0.8+
/// - products.items table created with sample data
///
/// Run with: cargo test --test web_app_search_tests --features web

use anyhow::Result;
use pg_search_tests::web_app::api::db;
use pg_search_tests::web_app::api::queries::{search_bm25, search_vector, search_hybrid};
use pg_search_tests::web_app::model::{SearchFilters, SortOption};

/// Helper to setup dotenv and create database pool
async fn setup() -> Result<sqlx::PgPool> {
    dotenv::dotenv().ok();
    let pool = db::create_pool().await?;
    Ok(pool)
}

#[tokio::test]
async fn test_bm25_search_basic() -> Result<()> {
    let pool = setup().await?;

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

    let results = search_bm25(&pool, "wireless headphones", &filters).await?;

    println!("BM25 Search: Found {} results", results.results.len());
    for result in &results.results {
        println!(
            "  - {} (BM25: {:.3})",
            result.product.name,
            result.bm25_score.unwrap_or(0.0)
        );
    }

    assert!(
        !results.results.is_empty(),
        "Should return at least one result for 'wireless headphones'"
    );

    // Verify scores are in descending order
    let scores: Vec<f64> = results
        .results
        .iter()
        .map(|r| r.bm25_score.unwrap_or(0.0))
        .collect();

    for i in 0..scores.len() - 1 {
        assert!(
            scores[i] >= scores[i + 1],
            "BM25 scores should be in descending order"
        );
    }

    println!("✓ BM25 basic search works correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_bm25_search_with_price_filter() -> Result<()> {
    let pool = setup().await?;

    let filters = SearchFilters {
        categories: vec![],
        price_min: Some(50.0),
        price_max: Some(150.0),
        min_rating: None,
        in_stock_only: false,
        sort_by: SortOption::Relevance,
        page: 0,
        page_size: 10,
    };

    let results = search_bm25(&pool, "mouse keyboard", &filters).await?;

    println!("BM25 Search with Price Filter: Found {} results", results.results.len());

    for result in &results.results {
        let price = result.product.price.to_string().parse::<f64>().unwrap();
        println!("  - {} (${:.2})", result.product.name, price);

        assert!(
            price >= 50.0 && price <= 150.0,
            "Price should be between $50 and $150"
        );
    }

    println!("✓ BM25 search with price filter works correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_bm25_search_with_category_filter() -> Result<()> {
    let pool = setup().await?;

    let filters = SearchFilters {
        categories: vec!["Electronics".to_string()],
        price_min: None,
        price_max: None,
        min_rating: None,
        in_stock_only: false,
        sort_by: SortOption::Relevance,
        page: 0,
        page_size: 10,
    };

    let results = search_bm25(&pool, "", &filters).await?;

    println!("BM25 Search with Category Filter: Found {} results", results.results.len());

    for result in &results.results {
        println!("  - {} (Category: {})", result.product.name, result.product.category);

        assert_eq!(
            result.product.category, "Electronics",
            "All results should be in Electronics category"
        );
    }

    println!("✓ BM25 search with category filter works correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_vector_search_basic() -> Result<()> {
    let pool = setup().await?;

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

    let results = search_vector(&pool, "gaming peripherals", &filters).await?;

    println!("Vector Search: Found {} results", results.results.len());
    for result in &results.results {
        println!(
            "  - {} (Vector: {:.3})",
            result.product.name,
            result.vector_score.unwrap_or(0.0)
        );
    }

    assert!(
        !results.results.is_empty(),
        "Should return results for vector search"
    );

    // Verify vector scores are in descending order
    let scores: Vec<f64> = results
        .results
        .iter()
        .map(|r| r.vector_score.unwrap_or(0.0))
        .collect();

    for i in 0..scores.len() - 1 {
        assert!(
            scores[i] >= scores[i + 1],
            "Vector scores should be in descending order"
        );
    }

    println!("✓ Vector search works correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_hybrid_search_basic() -> Result<()> {
    let pool = setup().await?;

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

    let results = search_hybrid(&pool, "professional camera", &filters).await?;

    println!("Hybrid Search: Found {} results", results.results.len());
    for result in &results.results {
        println!(
            "  - {} (BM25: {:.3}, Vector: {:.3}, Combined: {:.3})",
            result.product.name,
            result.bm25_score.unwrap_or(0.0),
            result.vector_score.unwrap_or(0.0),
            result.combined_score
        );
    }

    assert!(
        !results.results.is_empty(),
        "Should return results for hybrid search"
    );

    // Verify combined scores are in descending order
    let scores: Vec<f64> = results
        .results
        .iter()
        .map(|r| r.combined_score)
        .collect();

    for i in 0..scores.len() - 1 {
        assert!(
            scores[i] >= scores[i + 1],
            "Combined scores should be in descending order"
        );
    }

    // Verify hybrid combines both scores
    for result in &results.results {
        if result.bm25_score.is_some() && result.vector_score.is_some() {
            let expected = result.bm25_score.unwrap() * 0.3 + result.vector_score.unwrap() * 0.7;
            let diff = (result.combined_score - expected).abs();
            assert!(
                diff < 0.01,
                "Combined score should be 30% BM25 + 70% Vector"
            );
        }
    }

    println!("✓ Hybrid search works correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_search_facets() -> Result<()> {
    let pool = setup().await?;

    let filters = SearchFilters::default();
    let results = search_bm25(&pool, "", &filters).await?;

    println!("Search Facets Test:");
    println!("  Category Facets: {}", results.category_facets.len());
    for facet in &results.category_facets {
        println!("    - {} ({})", facet.value, facet.count);
    }

    println!("  Brand Facets: {}", results.brand_facets.len());
    for facet in &results.brand_facets {
        println!("    - {} ({})", facet.value, facet.count);
    }

    println!("  Price Histogram: {}", results.price_histogram.len());
    for bucket in &results.price_histogram {
        println!("    - ${:.0}-${:.0} ({})", bucket.min, bucket.max, bucket.count);
    }

    assert!(
        !results.category_facets.is_empty(),
        "Should return category facets"
    );

    println!("✓ Facets work correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_pagination() -> Result<()> {
    let pool = setup().await?;

    let page_size = 5;
    let filters_page0 = SearchFilters {
        categories: vec![],
        price_min: None,
        price_max: None,
        min_rating: None,
        in_stock_only: false,
        sort_by: SortOption::PriceAsc,
        page: 0,
        page_size,
    };

    let filters_page1 = SearchFilters {
        page: 1,
        ..filters_page0.clone()
    };

    let results_page0 = search_bm25(&pool, "", &filters_page0).await?;
    let results_page1 = search_bm25(&pool, "", &filters_page1).await?;

    println!("Pagination Test:");
    println!("  Page 0: {} results", results_page0.results.len());
    println!("  Page 1: {} results", results_page1.results.len());

    // Verify pages don't overlap
    let page0_ids: Vec<i32> = results_page0.results.iter().map(|r| r.product.id).collect();
    let page1_ids: Vec<i32> = results_page1.results.iter().map(|r| r.product.id).collect();

    for id in &page1_ids {
        assert!(
            !page0_ids.contains(id),
            "Pages should not contain duplicate IDs"
        );
    }

    println!("✓ Pagination works correctly\n");
    Ok(())
}

#[tokio::test]
async fn test_sort_options() -> Result<()> {
    let pool = setup().await?;

    // Test price ascending sort
    let filters = SearchFilters {
        categories: vec![],
        price_min: None,
        price_max: None,
        min_rating: None,
        in_stock_only: false,
        sort_by: SortOption::PriceAsc,
        page: 0,
        page_size: 5,
    };

    let results = search_bm25(&pool, "", &filters).await?;

    println!("Sort Test (Price Ascending):");
    let prices: Vec<f64> = results
        .results
        .iter()
        .map(|r| r.product.price.to_string().parse::<f64>().unwrap())
        .collect();

    for (i, price) in prices.iter().enumerate() {
        println!("  - Result {}: ${:.2}", i + 1, price);
    }

    // Verify ascending order
    for i in 0..prices.len() - 1 {
        assert!(
            prices[i] <= prices[i + 1],
            "Prices should be in ascending order"
        );
    }

    println!("✓ Sort options work correctly\n");
    Ok(())
}
