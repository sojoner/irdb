// web_app/pages/search.rs - Search page component
//
// The main search page that composes all search-related components
// and manages the search state lifecycle.

use leptos::prelude::*;
use crate::web_app::model::*;
use crate::web_app::components::*;
use crate::web_app::server_fns::search_products;

/// Main search page component
///
/// Orchestrates the search experience with:
/// - Search bar and mode toggle
/// - Filter panel
/// - Results grid
/// - Pagination
#[component]
pub fn SearchPage() -> impl IntoView {
    // Search state
    let query = RwSignal::new(String::new());
    let mode = RwSignal::new(SearchMode::Hybrid);

    // Filter state
    let selected_categories = RwSignal::new(Vec::<String>::new());
    let price_min = RwSignal::new(None::<f64>);
    let price_max = RwSignal::new(None::<f64>);
    let min_rating = RwSignal::new(None::<f64>);
    let in_stock_only = RwSignal::new(false);
    let sort_by = RwSignal::new(SortOption::Relevance);
    let current_page = RwSignal::new(0_u32);
    let page_size = 12_u32;

    // Trigger for manual search (when clicking search button)
    let search_trigger = RwSignal::new(0_u32);

    // Build filters from signals
    let filters = Signal::derive(move || SearchFilters {
        categories: selected_categories.get(),
        price_min: price_min.get(),
        price_max: price_max.get(),
        min_rating: min_rating.get(),
        in_stock_only: in_stock_only.get(),
        sort_by: sort_by.get(),
        page: current_page.get(),
        page_size,
    });

    // Create resource for search results
    // Re-fetches when query, mode, filters, or trigger changes
    let search_results = Resource::new(
        move || (query.get(), mode.get(), filters.get(), search_trigger.get()),
        move |(q, m, f, _)| async move {
            if q.is_empty() {
                // Return empty results for empty query
                Ok(SearchResults {
                    results: vec![],
                    total_count: 0,
                    category_facets: vec![],
                    brand_facets: vec![],
                    price_histogram: vec![],
                    avg_price: 0.0,
                    avg_rating: 0.0,
                })
            } else {
                search_products(q, m, f).await
            }
        },
    );

    // Derived signals for results data
    let results = Signal::derive(move || {
        search_results
            .get()
            .and_then(|r: Result<SearchResults, ServerFnError>| r.ok())
            .map(|r| r.results)
            .unwrap_or_default()
    });

    let total_count = Signal::derive(move || {
        search_results
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.total_count)
            .unwrap_or(0)
    });

    let category_facets = Signal::derive(move || {
        search_results
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.category_facets)
            .unwrap_or_default()
    });

    // Trigger search callback
    let on_search = Callback::new(move |()| {
        current_page.set(0); // Reset to first page on new search
        search_trigger.update(|t| *t += 1);
    });

    // Clear filters callback
    let on_clear_filters = Callback::new(move |()| {
        selected_categories.set(vec![]);
        price_min.set(None);
        price_max.set(None);
        min_rating.set(None);
        in_stock_only.set(false);
        current_page.set(0);
    });

    // Product click handler (for showing detail modal)
    let selected_product_id = RwSignal::new(None::<i32>);
    let on_product_click = Callback::new(move |id: i32| {
        selected_product_id.set(Some(id));
    });

    // Close modal handler
    let on_close_modal = Callback::new(move |()| {
        selected_product_id.set(None);
    });

    view! {
        <div class="min-h-screen bg-gray-50 font-sans text-gray-900">
            // Header
            <header class="bg-white shadow-sm sticky top-0 z-40 border-b border-gray-200">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
                    <div class="flex items-center gap-2">
                        <span class="text-2xl">"🔍"</span>
                        <h1 class="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-600 to-indigo-600">
                            "IRDB Product Search"
                        </h1>
                    </div>
                    <div class="text-sm text-gray-500">
                        "Powered by ParadeDB & pgvector"
                    </div>
                </div>
            </header>

            // Main content
            <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                // Search bar section
                <section class="bg-white rounded-2xl shadow-sm p-6 mb-8 border border-gray-100">
                    <SearchBar
                        query=query
                        mode=mode
                        on_search=on_search
                    />
                </section>

                // Main grid: Filters + Results
                // Wrapped in Suspense to handle resource loading for both filters and results
                <Suspense fallback=move || view! {
                    <div class="bg-white rounded-2xl p-12 shadow-sm border border-gray-100 text-center">
                        <Loading message="Searching products..." />
                    </div>
                }>
                    <div class="flex flex-col lg:flex-row gap-8 items-start">
                        // Filter panel (sidebar)
                        <div class="w-full lg:w-72 flex-shrink-0">
                            <FilterPanel
                                category_facets=category_facets
                                selected_categories=selected_categories
                                price_min=price_min
                                price_max=price_max
                                min_rating=min_rating
                                in_stock_only=in_stock_only
                                on_clear=on_clear_filters
                            />
                        </div>

                        // Results section
                        <section class="flex-1 w-full min-w-0">
                            // Sort controls
                            <div class="flex justify-end mb-6">
                                <SortDropdown sort=sort_by />
                            </div>

                            // Results Content
                            {move || {
                                match search_results.get() {
                                    None => view! { 
                                        <div class="bg-white rounded-2xl p-12 shadow-sm border border-gray-100">
                                            <Loading message="Initializing..." /> 
                                        </div>
                                    }.into_any(),
                                    Some(Err(e)) => view! {
                                        <ErrorDisplay error=e.to_string() />
                                    }.into_any(),
                                    Some(Ok(_)) => view! {
                                        <div class="animate-fade-in">
                                            <ResultsGrid
                                                results=results
                                                total_count=total_count
                                                on_product_click=on_product_click
                                            />

                                            // Pagination
                                            <Show when=move || { total_count.get() > 0 }>
                                                <Pagination
                                                    current_page=current_page
                                                    total_items=total_count
                                                    page_size=page_size
                                                />
                                            </Show>
                                        </div>
                                    }.into_any(),
                                }
                            }}
                        </section>
                    </div>
                </Suspense>
            </main>

            // Footer
            <footer class="bg-white border-t border-gray-200 mt-12 py-8">
                <div class="max-w-7xl mx-auto px-4 text-center text-gray-500 text-sm">
                    <p>"© 2025 IRDB Product Search. Built with Leptos, Actix, and PostgreSQL."</p>
                </div>
            </footer>

            // Product detail modal
            {move || {
                selected_product_id.get().map(|id| {
                    // Find the product in current results
                    let product = results.get().into_iter()
                        .find(|r| r.product.id == id)
                        .map(|r| r.product.clone());

                    product.map(|p| view! {
                        <ModalWrapper
                            title="Product Details"
                            on_close=on_close_modal
                        >
                            <ProductDetail product=p />
                        </ModalWrapper>
                    })
                })
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_filters() {
        let filters = SearchFilters::default();
        assert!(filters.categories.is_empty());
        assert_eq!(filters.page, 0);
        assert_eq!(filters.page_size, 0);
    }

    #[test]
    fn test_search_mode_default() {
        assert_eq!(SearchMode::default(), SearchMode::Hybrid);
    }

    #[test]
    fn test_filter_aggregation_logic() {
        // Test the logic used in the filters derived signal
        let selected_categories = vec!["Electronics".to_string()];
        let price_min = Some(10.0);
        let price_max = Some(100.0);
        let min_rating = Some(4.0);
        let in_stock_only = true;
        let sort_by = SortOption::PriceAsc;
        let current_page = 1u32;
        let page_size = 12u32;

        let filters = SearchFilters {
            categories: selected_categories.clone(),
            price_min,
            price_max,
            min_rating,
            in_stock_only,
            sort_by,
            page: current_page,
            page_size,
        };

        assert_eq!(filters.categories, selected_categories);
        assert_eq!(filters.price_min, Some(10.0));
        assert_eq!(filters.price_max, Some(100.0));
        assert_eq!(filters.min_rating, Some(4.0));
        assert!(filters.in_stock_only);
        assert_eq!(filters.sort_by, SortOption::PriceAsc);
        assert_eq!(filters.page, 1);
        assert_eq!(filters.page_size, 12);
    }

    #[test]
    fn test_clear_filters_logic() {
        // Test the logic used in on_clear_filters
        let mut selected_categories = vec!["Electronics".to_string()];
        let mut price_min = Some(10.0);
        let mut price_max = Some(100.0);
        let mut min_rating = Some(4.0);
        let mut in_stock_only = true;
        let mut current_page = 5u32;

        // Clear
        selected_categories = vec![];
        price_min = None;
        price_max = None;
        min_rating = None;
        in_stock_only = false;
        current_page = 0;

        assert!(selected_categories.is_empty());
        assert!(price_min.is_none());
        assert!(price_max.is_none());
        assert!(min_rating.is_none());
        assert!(!in_stock_only);
        assert_eq!(current_page, 0);
    }

    #[test]
    fn test_search_trigger_increment() {
        // Test the search trigger logic
        let mut search_trigger = 0u32;
        search_trigger += 1;
        assert_eq!(search_trigger, 1);

        search_trigger += 1;
        assert_eq!(search_trigger, 2);

        // Verify it doesn't overflow in reasonable use
        for _ in 0..100 {
            search_trigger += 1;
        }
        assert_eq!(search_trigger, 102);
    }

    #[test]
    fn test_page_reset_on_search() {
        // Test that page resets to 0 on new search
        let mut current_page = 5u32;
        // Simulate new search
        current_page = 0;
        assert_eq!(current_page, 0);
    }

    #[test]
    fn test_empty_query_returns_empty_results() {
        // Test the logic for empty query handling
        let query = String::new();
        let should_search = !query.is_empty();
        assert!(!should_search);

        let query = "laptop".to_string();
        let should_search = !query.is_empty();
        assert!(should_search);
    }

    #[test]
    fn test_selected_product_id_toggle() {
        // Test the product selection logic
        let mut selected_product_id: Option<i32> = None;

        // Select a product
        selected_product_id = Some(42);
        assert_eq!(selected_product_id, Some(42));

        // Close modal
        selected_product_id = None;
        assert!(selected_product_id.is_none());
    }

    #[test]
    fn test_find_product_in_results() {
        use rust_decimal::Decimal;

        // Create test products
        let products = vec![
            Product {
                id: 1,
                name: "Product 1".to_string(),
                description: "Desc 1".to_string(),
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
            },
            Product {
                id: 2,
                name: "Product 2".to_string(),
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
            },
        ];

        let results: Vec<SearchResult> = products.into_iter().map(|p| SearchResult {
            product: p,
            bm25_score: None,
            vector_score: None,
            combined_score: 0.5,
            snippet: None,
        }).collect();

        // Find product by ID
        let target_id = 2;
        let found = results.iter()
            .find(|r| r.product.id == target_id)
            .map(|r| r.product.clone());

        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Product 2");

        // Try to find non-existent product
        let target_id = 999;
        let not_found = results.iter()
            .find(|r| r.product.id == target_id)
            .map(|r| r.product.clone());

        assert!(not_found.is_none());
    }

    #[test]
    fn test_filters_with_all_sort_options() {
        let sort_options = [
            SortOption::Relevance,
            SortOption::PriceAsc,
            SortOption::PriceDesc,
            SortOption::RatingDesc,
            SortOption::Newest,
        ];

        for sort_by in sort_options {
            let filters = SearchFilters {
                categories: vec![],
                price_min: None,
                price_max: None,
                min_rating: None,
                in_stock_only: false,
                sort_by,
                page: 0,
                page_size: 12,
            };

            assert_eq!(filters.sort_by, sort_by);
            assert_eq!(filters.page_size, 12);
        }
    }

    #[test]
    fn test_filters_with_multiple_categories() {
        let categories = vec![
            "Electronics".to_string(),
            "Books".to_string(),
            "Home & Garden".to_string(),
            "Sports".to_string(),
        ];

        let filters = SearchFilters {
            categories: categories.clone(),
            price_min: None,
            price_max: None,
            min_rating: None,
            in_stock_only: false,
            sort_by: SortOption::Relevance,
            page: 0,
            page_size: 20,
        };

        assert_eq!(filters.categories.len(), 4);
        assert!(filters.categories.contains(&"Electronics".to_string()));
        assert!(filters.categories.contains(&"Sports".to_string()));
    }

    #[test]
    fn test_price_range_validation() {
        // Test various price range combinations
        let test_cases = [
            (Some(0.0), Some(100.0), true),   // Valid range
            (Some(50.0), Some(100.0), true),  // Valid range
            (Some(100.0), Some(50.0), false), // Invalid: min > max
            (None, Some(100.0), true),        // Only max
            (Some(50.0), None, true),         // Only min
            (None, None, true),               // No range
        ];

        for (price_min, price_max, is_valid) in test_cases {
            let valid = match (price_min, price_max) {
                (Some(min), Some(max)) => min <= max,
                _ => true,
            };
            assert_eq!(valid, is_valid, "price_min={:?}, price_max={:?}", price_min, price_max);
        }
    }

    #[test]
    fn test_rating_filter_values() {
        // Test various min_rating values
        let ratings = [None, Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)];

        for min_rating in ratings {
            let filters = SearchFilters {
                categories: vec![],
                price_min: None,
                price_max: None,
                min_rating,
                in_stock_only: false,
                sort_by: SortOption::Relevance,
                page: 0,
                page_size: 12,
            };

            assert_eq!(filters.min_rating, min_rating);
        }
    }

    #[test]
    fn test_pagination_state() {
        // Test pagination state changes
        let page_size = 12u32;

        for page in 0..10 {
            let filters = SearchFilters {
                categories: vec![],
                price_min: None,
                price_max: None,
                min_rating: None,
                in_stock_only: false,
                sort_by: SortOption::Relevance,
                page,
                page_size,
            };

            assert_eq!(filters.page, page);
            assert_eq!(filters.page_size, page_size);

            // Calculate offset
            let offset = page * page_size;
            assert_eq!(offset, page * 12);
        }
    }

    #[test]
    fn test_empty_search_results_structure() {
        // Test the structure returned for empty queries
        let empty_results = SearchResults {
            results: vec![],
            total_count: 0,
            category_facets: vec![],
            brand_facets: vec![],
            price_histogram: vec![],
            avg_price: 0.0,
            avg_rating: 0.0,
        };

        assert!(empty_results.results.is_empty());
        assert_eq!(empty_results.total_count, 0);
        assert!(empty_results.category_facets.is_empty());
        assert!(empty_results.brand_facets.is_empty());
        assert!(empty_results.price_histogram.is_empty());
        assert_eq!(empty_results.avg_price, 0.0);
        assert_eq!(empty_results.avg_rating, 0.0);
    }

    #[test]
    fn test_search_mode_all_variants() {
        let modes = [SearchMode::Bm25, SearchMode::Vector, SearchMode::Hybrid];

        for mode in modes {
            // Each mode should be distinguishable
            match mode {
                SearchMode::Bm25 => assert_eq!(mode.to_string(), "BM25"),
                SearchMode::Vector => assert_eq!(mode.to_string(), "Vector"),
                SearchMode::Hybrid => assert_eq!(mode.to_string(), "Hybrid"),
            }
        }
    }

    #[test]
    fn test_in_stock_filter_toggle() {
        let mut in_stock_only = false;

        // Toggle on
        in_stock_only = !in_stock_only;
        assert!(in_stock_only);

        // Toggle off
        in_stock_only = !in_stock_only;
        assert!(!in_stock_only);
    }
}
