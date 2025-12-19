// web_app/model/mod.rs - Shared data models for client and server
//
// These structs are used throughout the application for type-safe
// communication between frontend and backend.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use sqlx::FromRow;

/// Search mode enumeration
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    Bm25,       // Keyword matching only (ParadeDB BM25)
    Vector,     // Semantic similarity only (pgvector cosine)
    #[default]
    Hybrid,     // 70% vector + 30% BM25 weighted combination
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchMode::Bm25 => write!(f, "BM25"),
            SearchMode::Vector => write!(f, "Vector"),
            SearchMode::Hybrid => write!(f, "Hybrid"),
        }
    }
}

/// Product from database (matches products.items schema)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(FromRow))]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub brand: String,
    pub category: String,
    pub subcategory: Option<String>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub tags: Vec<String>,
    pub price: rust_decimal::Decimal,
    pub rating: rust_decimal::Decimal,
    pub review_count: i32,
    pub stock_quantity: i32,
    pub in_stock: bool,
    pub featured: bool,
    pub attributes: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Search result with scores
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub product: Product,
    pub bm25_score: Option<f64>,
    pub vector_score: Option<f64>,
    pub combined_score: f64,
    pub snippet: Option<String>,  // Highlighted text from BM25
}

/// Search filters applied by user
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SearchFilters {
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub price_min: Option<f64>,
    #[serde(default)]
    pub price_max: Option<f64>,
    #[serde(default)]
    pub min_rating: Option<f64>,
    #[serde(default)]
    pub in_stock_only: bool,
    #[serde(default)]
    pub sort_by: SortOption,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub page_size: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOption {
    #[default]
    Relevance,
    PriceAsc,
    PriceDesc,
    RatingDesc,
    Newest,
}

impl std::fmt::Display for SortOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOption::Relevance => write!(f, "Relevance"),
            SortOption::PriceAsc => write!(f, "Price: Low to High"),
            SortOption::PriceDesc => write!(f, "Price: High to Low"),
            SortOption::RatingDesc => write!(f, "Rating: High to Low"),
            SortOption::Newest => write!(f, "Newest First"),
        }
    }
}

/// Facet count for filters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: String,
    pub count: i64,
}

/// Price histogram bucket
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceBucket {
    pub min: f64,
    pub max: f64,
    pub count: i64,
}

/// Search response with results and facets
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub category_facets: Vec<FacetCount>,
    pub brand_facets: Vec<FacetCount>,
    pub price_histogram: Vec<PriceBucket>,
    pub avg_price: f64,
    pub avg_rating: f64,
}

/// Analytics data for dashboard
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub total_products: i64,
    pub category_stats: Vec<CategoryStat>,
    pub rating_distribution: Vec<RatingBucket>,
    pub price_histogram: Vec<PriceBucket>,
    pub top_brands: Vec<BrandStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryStat {
    pub category: String,
    pub count: i64,
    pub avg_price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatingBucket {
    pub rating: f64,
    pub count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrandStat {
    pub brand: String,
    pub count: i64,
}

/// Import status tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportStatus {
    pub total: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub complete: bool,
}

/// Product from JSON import (flexible schema)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductImport {
    pub name: String,
    pub description: String,
    pub brand: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub tags: Option<Vec<String>>,
    pub price: f64,
    pub rating: Option<f64>,
    pub review_count: Option<i32>,
    pub stock_quantity: Option<i32>,
    pub in_stock: Option<bool>,
    pub featured: Option<bool>,
    pub attributes: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_mode_default() {
        let mode = SearchMode::default();
        assert_eq!(mode, SearchMode::Hybrid);
    }

    #[test]
    fn test_search_mode_display() {
        assert_eq!(SearchMode::Bm25.to_string(), "BM25");
        assert_eq!(SearchMode::Vector.to_string(), "Vector");
        assert_eq!(SearchMode::Hybrid.to_string(), "Hybrid");
    }

    #[test]
    fn test_search_mode_equality() {
        assert_eq!(SearchMode::Bm25, SearchMode::Bm25);
        assert_eq!(SearchMode::Vector, SearchMode::Vector);
        assert_eq!(SearchMode::Hybrid, SearchMode::Hybrid);
        assert_ne!(SearchMode::Bm25, SearchMode::Vector);
        assert_ne!(SearchMode::Vector, SearchMode::Hybrid);
    }

    #[test]
    fn test_search_mode_clone() {
        let mode = SearchMode::Hybrid;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_search_mode_serialization() {
        let modes = [SearchMode::Bm25, SearchMode::Vector, SearchMode::Hybrid];
        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: SearchMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn test_sort_option_default() {
        let sort = SortOption::default();
        assert_eq!(sort, SortOption::Relevance);
    }

    #[test]
    fn test_sort_option_display() {
        assert_eq!(SortOption::Relevance.to_string(), "Relevance");
        assert_eq!(SortOption::PriceAsc.to_string(), "Price: Low to High");
        assert_eq!(SortOption::PriceDesc.to_string(), "Price: High to Low");
        assert_eq!(SortOption::RatingDesc.to_string(), "Rating: High to Low");
        assert_eq!(SortOption::Newest.to_string(), "Newest First");
    }

    #[test]
    fn test_sort_option_equality() {
        assert_eq!(SortOption::Relevance, SortOption::Relevance);
        assert_eq!(SortOption::PriceAsc, SortOption::PriceAsc);
        assert_ne!(SortOption::PriceAsc, SortOption::PriceDesc);
    }

    #[test]
    fn test_sort_option_serialization() {
        let options = [
            SortOption::Relevance,
            SortOption::PriceAsc,
            SortOption::PriceDesc,
            SortOption::RatingDesc,
            SortOption::Newest,
        ];
        for opt in options {
            let json = serde_json::to_string(&opt).unwrap();
            let deserialized: SortOption = serde_json::from_str(&json).unwrap();
            assert_eq!(opt, deserialized);
        }
    }

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.categories.is_empty());
        assert_eq!(filters.page, 0);
        assert_eq!(filters.page_size, 0);
        assert_eq!(filters.sort_by, SortOption::Relevance);
        assert!(!filters.in_stock_only);
        assert!(filters.price_min.is_none());
        assert!(filters.price_max.is_none());
        assert!(filters.min_rating.is_none());
    }

    #[test]
    fn test_search_filters_with_values() {
        let filters = SearchFilters {
            categories: vec!["Electronics".to_string(), "Books".to_string()],
            price_min: Some(10.0),
            price_max: Some(1000.0),
            min_rating: Some(4.0),
            in_stock_only: true,
            sort_by: SortOption::PriceAsc,
            page: 2,
            page_size: 20,
        };

        assert_eq!(filters.categories.len(), 2);
        assert_eq!(filters.price_min, Some(10.0));
        assert_eq!(filters.price_max, Some(1000.0));
        assert_eq!(filters.min_rating, Some(4.0));
        assert!(filters.in_stock_only);
        assert_eq!(filters.sort_by, SortOption::PriceAsc);
        assert_eq!(filters.page, 2);
        assert_eq!(filters.page_size, 20);
    }

    #[test]
    fn test_search_filters_serialization() {
        let filters = SearchFilters {
            categories: vec!["Electronics".to_string()],
            price_min: Some(50.0),
            price_max: Some(500.0),
            min_rating: Some(3.5),
            in_stock_only: true,
            sort_by: SortOption::RatingDesc,
            page: 1,
            page_size: 12,
        };

        let json = serde_json::to_string(&filters).unwrap();
        let deserialized: SearchFilters = serde_json::from_str(&json).unwrap();

        assert_eq!(filters, deserialized);
    }

    #[test]
    fn test_facet_count_creation() {
        let facet = FacetCount {
            value: "Electronics".to_string(),
            count: 42,
        };

        assert_eq!(facet.value, "Electronics");
        assert_eq!(facet.count, 42);
    }

    #[test]
    fn test_facet_count_serialization() {
        let facet = FacetCount {
            value: "Books".to_string(),
            count: 100,
        };

        let json = serde_json::to_string(&facet).unwrap();
        let deserialized: FacetCount = serde_json::from_str(&json).unwrap();

        assert_eq!(facet.value, deserialized.value);
        assert_eq!(facet.count, deserialized.count);
    }

    #[test]
    fn test_price_bucket_creation() {
        let bucket = PriceBucket {
            min: 0.0,
            max: 100.0,
            count: 25,
        };

        assert_eq!(bucket.min, 0.0);
        assert_eq!(bucket.max, 100.0);
        assert_eq!(bucket.count, 25);
    }

    #[test]
    fn test_price_bucket_serialization() {
        let bucket = PriceBucket {
            min: 100.0,
            max: 200.0,
            count: 50,
        };

        let json = serde_json::to_string(&bucket).unwrap();
        let deserialized: PriceBucket = serde_json::from_str(&json).unwrap();

        assert!((bucket.min - deserialized.min).abs() < 0.001);
        assert!((bucket.max - deserialized.max).abs() < 0.001);
        assert_eq!(bucket.count, deserialized.count);
    }

    #[test]
    fn test_search_results_creation() {
        let results = SearchResults {
            results: vec![],
            total_count: 0,
            category_facets: vec![],
            brand_facets: vec![],
            price_histogram: vec![],
            avg_price: 0.0,
            avg_rating: 0.0,
        };

        assert!(results.results.is_empty());
        assert_eq!(results.total_count, 0);
        assert!(results.category_facets.is_empty());
        assert!(results.brand_facets.is_empty());
        assert!(results.price_histogram.is_empty());
    }

    #[test]
    fn test_search_results_with_data() {
        let results = SearchResults {
            results: vec![],
            total_count: 100,
            category_facets: vec![
                FacetCount { value: "Electronics".to_string(), count: 50 },
                FacetCount { value: "Books".to_string(), count: 30 },
            ],
            brand_facets: vec![
                FacetCount { value: "Apple".to_string(), count: 20 },
            ],
            price_histogram: vec![
                PriceBucket { min: 0.0, max: 100.0, count: 25 },
            ],
            avg_price: 150.50,
            avg_rating: 4.2,
        };

        assert_eq!(results.total_count, 100);
        assert_eq!(results.category_facets.len(), 2);
        assert_eq!(results.brand_facets.len(), 1);
        assert_eq!(results.price_histogram.len(), 1);
        assert!((results.avg_price - 150.50).abs() < 0.001);
        assert!((results.avg_rating - 4.2).abs() < 0.001);
    }

    #[test]
    fn test_analytics_data_creation() {
        let analytics = AnalyticsData {
            total_products: 1000,
            category_stats: vec![],
            rating_distribution: vec![],
            price_histogram: vec![],
            top_brands: vec![],
        };

        assert_eq!(analytics.total_products, 1000);
        assert!(analytics.category_stats.is_empty());
    }

    #[test]
    fn test_category_stat_creation() {
        let stat = CategoryStat {
            category: "Electronics".to_string(),
            count: 500,
            avg_price: 299.99,
        };

        assert_eq!(stat.category, "Electronics");
        assert_eq!(stat.count, 500);
        assert!((stat.avg_price - 299.99).abs() < 0.001);
    }

    #[test]
    fn test_rating_bucket_creation() {
        let bucket = RatingBucket {
            rating: 4.0,
            count: 250,
        };

        assert!((bucket.rating - 4.0).abs() < 0.001);
        assert_eq!(bucket.count, 250);
    }

    #[test]
    fn test_brand_stat_creation() {
        let stat = BrandStat {
            brand: "Apple".to_string(),
            count: 100,
        };

        assert_eq!(stat.brand, "Apple");
        assert_eq!(stat.count, 100);
    }

    #[test]
    fn test_product_import_serialization() {
        let product = ProductImport {
            name: "Test Product".to_string(),
            description: "A test product".to_string(),
            brand: "TestBrand".to_string(),
            category: "Electronics".to_string(),
            subcategory: None,
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
            price: 99.99,
            rating: Some(4.5),
            review_count: Some(100),
            stock_quantity: Some(50),
            in_stock: Some(true),
            featured: Some(false),
            attributes: None,
        };

        let json = serde_json::to_string(&product).unwrap();
        let deserialized: ProductImport = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Test Product");
        assert_eq!(deserialized.price, 99.99);
        assert_eq!(deserialized.rating, Some(4.5));
    }

    #[test]
    fn test_product_import_minimal() {
        // Test with only required fields
        let product = ProductImport {
            name: "Minimal Product".to_string(),
            description: "Minimal".to_string(),
            brand: "Brand".to_string(),
            category: "Category".to_string(),
            subcategory: None,
            tags: None,
            price: 10.0,
            rating: None,
            review_count: None,
            stock_quantity: None,
            in_stock: None,
            featured: None,
            attributes: None,
        };

        let json = serde_json::to_string(&product).unwrap();
        let deserialized: ProductImport = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Minimal Product");
        assert!(deserialized.rating.is_none());
        assert!(deserialized.tags.is_none());
    }

    #[test]
    fn test_product_import_with_attributes() {
        use serde_json::json;

        let product = ProductImport {
            name: "Product with Attrs".to_string(),
            description: "Has attributes".to_string(),
            brand: "Brand".to_string(),
            category: "Electronics".to_string(),
            subcategory: Some("Phones".to_string()),
            tags: Some(vec!["smartphone".to_string()]),
            price: 999.99,
            rating: Some(4.8),
            review_count: Some(1000),
            stock_quantity: Some(100),
            in_stock: Some(true),
            featured: Some(true),
            attributes: Some(json!({
                "screen_size": "6.1 inches",
                "storage": "256GB",
                "color": "Black"
            })),
        };

        let json = serde_json::to_string(&product).unwrap();
        let deserialized: ProductImport = serde_json::from_str(&json).unwrap();

        assert!(deserialized.attributes.is_some());
        let attrs = deserialized.attributes.unwrap();
        assert!(attrs.get("screen_size").is_some());
    }

    #[test]
    fn test_import_status_tracking() {
        let mut status = ImportStatus {
            total: 10,
            processed: 0,
            succeeded: 0,
            failed: 0,
            errors: Vec::new(),
            complete: false,
        };

        // Simulate processing
        status.processed += 1;
        status.succeeded += 1;
        assert_eq!(status.processed, 1);
        assert_eq!(status.succeeded, 1);

        // Simulate an error
        status.processed += 1;
        status.failed += 1;
        status.errors.push("Test error".to_string());
        assert_eq!(status.failed, 1);
        assert_eq!(status.errors.len(), 1);
    }

    #[test]
    fn test_import_status_completion() {
        let mut status = ImportStatus {
            total: 5,
            processed: 0,
            succeeded: 0,
            failed: 0,
            errors: Vec::new(),
            complete: false,
        };

        // Process all items
        for i in 0..5 {
            status.processed += 1;
            if i % 2 == 0 {
                status.succeeded += 1;
            } else {
                status.failed += 1;
                status.errors.push(format!("Error on item {}", i));
            }
        }

        status.complete = true;

        assert_eq!(status.total, 5);
        assert_eq!(status.processed, 5);
        assert_eq!(status.succeeded, 3);
        assert_eq!(status.failed, 2);
        assert_eq!(status.errors.len(), 2);
        assert!(status.complete);
    }

    #[test]
    fn test_import_status_serialization() {
        let status = ImportStatus {
            total: 100,
            processed: 50,
            succeeded: 45,
            failed: 5,
            errors: vec!["Error 1".to_string(), "Error 2".to_string()],
            complete: false,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ImportStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status.total, deserialized.total);
        assert_eq!(status.processed, deserialized.processed);
        assert_eq!(status.succeeded, deserialized.succeeded);
        assert_eq!(status.failed, deserialized.failed);
        assert_eq!(status.errors, deserialized.errors);
        assert_eq!(status.complete, deserialized.complete);
    }

    #[test]
    fn test_search_result_creation() {
        use rust_decimal::Decimal;

        let product = Product {
            id: 1,
            name: "Test".to_string(),
            description: "Desc".to_string(),
            brand: "Brand".to_string(),
            category: "Cat".to_string(),
            subcategory: None,
            tags: vec![],
            price: Decimal::new(100, 0),
            rating: Decimal::new(40, 1),
            review_count: 10,
            stock_quantity: 5,
            in_stock: true,
            featured: false,
            attributes: None,
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
        };

        let result = SearchResult {
            product,
            bm25_score: Some(0.8),
            vector_score: Some(0.9),
            combined_score: 0.85,
            snippet: Some("Highlighted text".to_string()),
        };

        assert_eq!(result.product.id, 1);
        assert_eq!(result.bm25_score, Some(0.8));
        assert_eq!(result.vector_score, Some(0.9));
        assert!((result.combined_score - 0.85).abs() < 0.001);
        assert!(result.snippet.is_some());
    }

    #[test]
    fn test_search_result_without_scores() {
        use rust_decimal::Decimal;

        let product = Product {
            id: 2,
            name: "Test 2".to_string(),
            description: "Desc 2".to_string(),
            brand: "Brand".to_string(),
            category: "Cat".to_string(),
            subcategory: None,
            tags: vec![],
            price: Decimal::new(200, 0),
            rating: Decimal::new(45, 1),
            review_count: 20,
            stock_quantity: 10,
            in_stock: true,
            featured: true,
            attributes: None,
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
        };

        let result = SearchResult {
            product,
            bm25_score: None,
            vector_score: None,
            combined_score: 0.0,
            snippet: None,
        };

        assert!(result.bm25_score.is_none());
        assert!(result.vector_score.is_none());
        assert!(result.snippet.is_none());
    }
}
