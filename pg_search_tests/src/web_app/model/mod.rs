// web_app/model/mod.rs - Shared data models for client and server
//
// These structs are used throughout the application for type-safe
// communication between frontend and backend.

use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
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
#[cfg_attr(feature = "web", derive(FromRow))]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub brand: String,
    pub category: String,
    pub subcategory: Option<String>,
    #[cfg_attr(feature = "web", sqlx(default))]
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
    pub categories: Vec<String>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
    pub min_rating: Option<f64>,
    pub in_stock_only: bool,
    pub sort_by: SortOption,
    pub page: u32,
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
    fn test_sort_option_default() {
        let sort = SortOption::default();
        assert_eq!(sort, SortOption::Relevance);
    }

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.categories.is_empty());
        assert_eq!(filters.page, 0);
        assert_eq!(filters.page_size, 0);
        assert_eq!(filters.sort_by, SortOption::Relevance);
        assert!(!filters.in_stock_only);
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
}
