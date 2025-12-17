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
}
