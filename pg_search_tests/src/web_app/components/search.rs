// web_app/components/search.rs - Search-related UI components
//
// These components handle the search interface including:
// - SearchBar: Input field with search mode toggle
// - SearchModeToggle: Radio buttons for BM25/Vector/Hybrid
// - FilterPanel: Category facets, price range, rating filter
// - SortDropdown: Sort options selector

use leptos::prelude::*;
use crate::web_app::model::{SearchMode, SortOption, FacetCount};

/// Search bar component with input and mode toggle
///
/// Handles user input and search mode selection.
#[component]
pub fn SearchBar(
    /// Current search query
    query: RwSignal<String>,
    /// Current search mode
    mode: RwSignal<SearchMode>,
    /// Callback when search is triggered
    on_search: Callback<()>,
) -> impl IntoView {
    // Local state for the input (allows typing without triggering search on every keystroke)
    let local_query = RwSignal::new(query.get_untracked());

    // Sync local with external when external changes
    Effect::new(move || {
        local_query.set(query.get());
    });

    let on_submit = move |ev: leptos::web_sys::SubmitEvent| {
        ev.prevent_default();
        query.set(local_query.get());
        on_search.run(());
    };

    view! {
        <form on:submit=on_submit class="w-full">
            <div class="flex gap-4 mb-4">
                <div class="relative flex-1">
                    <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                        <span class="text-gray-400">"🔍"</span>
                    </div>
                    <input
                        type="text"
                        placeholder="Search products..."
                        class="w-full pl-10 pr-4 py-3 border-2 border-gray-200 rounded-xl \
                               focus:ring-4 focus:ring-blue-100 focus:border-blue-500 \
                               outline-none text-lg transition-all shadow-sm"
                        prop:value=move || local_query.get()
                        on:input=move |ev| local_query.set(event_target_value(&ev))
                    />
                </div>
                <button
                    type="submit"
                    class="px-8 py-3 bg-blue-600 text-white rounded-xl \
                           hover:bg-blue-700 active:bg-blue-800 transition-all \
                           font-semibold shadow-md hover:shadow-lg transform hover:-translate-y-0.5"
                >
                    "Search"
                </button>
            </div>

            <SearchModeToggle mode=mode />
        </form>
    }
}

/// Search mode toggle (BM25/Vector/Hybrid)
///
/// Radio buttons for selecting the search algorithm.
#[component]
pub fn SearchModeToggle(
    /// Current search mode
    mode: RwSignal<SearchMode>,
) -> impl IntoView {
    let modes = [
        (SearchMode::Bm25, "BM25", "Keyword matching"),
        (SearchMode::Vector, "Vector", "Semantic similarity"),
        (SearchMode::Hybrid, "Hybrid", "Combined (recommended)"),
    ];

    view! {
        <div class="bg-gray-50 p-4 rounded-xl border border-gray-100">
            <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3 block">
                "Search Algorithm"
            </span>
            <div class="flex flex-wrap gap-4">
                {modes.into_iter().map(|(mode_value, label, description)| {
                    let is_selected = move || mode.get() == mode_value;
                    view! {
                        <label class="flex items-center gap-3 cursor-pointer group relative">
                            <input
                                type="radio"
                                name="search_mode"
                                checked=is_selected
                                on:change=move |_| mode.set(mode_value)
                                class="peer sr-only"
                            />
                            <div class="w-5 h-5 border-2 border-gray-300 rounded-full peer-checked:border-blue-600 \
                                        peer-checked:border-[6px] transition-all bg-white"></div>
                            <div class="flex flex-col">
                                <span class=move || {
                                    if is_selected() {
                                        "text-blue-700 font-bold transition-colors"
                                    } else {
                                        "text-gray-700 font-medium group-hover:text-gray-900 transition-colors"
                                    }
                                }>
                                    {label}
                                </span>
                                <span class="text-xs text-gray-500">{description}</span>
                            </div>
                        </label>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Sort dropdown component
///
/// Allows users to sort results by different criteria.
#[component]
pub fn SortDropdown(
    /// Current sort option
    sort: RwSignal<SortOption>,
) -> impl IntoView {
    let options = [
        (SortOption::Relevance, "Relevance"),
        (SortOption::PriceAsc, "Price: Low to High"),
        (SortOption::PriceDesc, "Price: High to Low"),
        (SortOption::RatingDesc, "Rating: High to Low"),
        (SortOption::Newest, "Newest First"),
    ];

    view! {
        <div class="flex items-center gap-3 bg-white px-4 py-2 rounded-lg border border-gray-200 shadow-sm">
            <label class="text-sm font-medium text-gray-600">"Sort by:"</label>
            <select
                class="text-sm font-semibold text-gray-800 bg-transparent border-none \
                       focus:ring-0 cursor-pointer pr-8"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    let new_sort = match value.as_str() {
                        "relevance" => SortOption::Relevance,
                        "price_asc" => SortOption::PriceAsc,
                        "price_desc" => SortOption::PriceDesc,
                        "rating_desc" => SortOption::RatingDesc,
                        "newest" => SortOption::Newest,
                        _ => SortOption::Relevance,
                    };
                    sort.set(new_sort);
                }
            >
                {options.into_iter().map(|(opt_value, label)| {
                    let value_str = match opt_value {
                        SortOption::Relevance => "relevance",
                        SortOption::PriceAsc => "price_asc",
                        SortOption::PriceDesc => "price_desc",
                        SortOption::RatingDesc => "rating_desc",
                        SortOption::Newest => "newest",
                    };
                    view! {
                        <option
                            value=value_str
                            selected=move || sort.get() == opt_value
                        >
                            {label}
                        </option>
                    }
                }).collect_view()}
            </select>
        </div>
    }
}

/// Category facet list component
///
/// Displays category checkboxes with counts.
#[component]
pub fn CategoryFacets(
    /// Available category facets with counts
    facets: Signal<Vec<FacetCount>>,
    /// Currently selected categories
    selected: RwSignal<Vec<String>>,
) -> impl IntoView {
    let toggle_category = move |category: String| {
        selected.update(|cats| {
            if cats.contains(&category) {
                cats.retain(|c| c != &category);
            } else {
                cats.push(category);
            }
        });
    };

    view! {
        <div class="space-y-3">
            <h3 class="font-bold text-gray-900 flex items-center gap-2 text-sm uppercase tracking-wide">
                <span class="text-blue-500">"▼"</span>
                "Categories"
            </h3>
            <div class="space-y-1 max-h-60 overflow-y-auto pr-2 custom-scrollbar">
                <For
                    each=move || facets.get()
                    key=|f| f.value.clone()
                    children=move |facet| {
                        let category = facet.value.clone();
                        let cat_for_check = category.clone();
                        let cat_for_toggle = category.clone();
                        let is_checked = move || selected.get().contains(&cat_for_check);

                        view! {
                            <label class="flex items-center gap-3 cursor-pointer \
                                          hover:bg-white p-2 rounded-lg transition-colors group">
                                <div class="relative flex items-center">
                                    <input
                                        type="checkbox"
                                        checked=is_checked
                                        on:change=move |_| toggle_category(cat_for_toggle.clone())
                                        class="peer h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                                    />
                                </div>
                                <span class="flex-1 text-sm text-gray-700 group-hover:text-gray-900 font-medium">
                                    {category}
                                </span>
                                <span class="text-xs bg-gray-200 text-gray-600 px-2 py-0.5 rounded-full">
                                    {facet.count}
                                </span>
                            </label>
                        }
                    }
                />
            </div>
        </div>
    }
}

/// Price range filter component
///
/// Two input fields for min/max price.
#[component]
pub fn PriceRangeFilter(
    /// Minimum price
    price_min: RwSignal<Option<f64>>,
    /// Maximum price
    price_max: RwSignal<Option<f64>>,
) -> impl IntoView {
    // Local state for inputs (allows typing without immediate updates)
    let local_min = RwSignal::new(price_min.get_untracked().map(|v| v.to_string()).unwrap_or_default());
    let local_max = RwSignal::new(price_max.get_untracked().map(|v| v.to_string()).unwrap_or_default());

    let apply_min = move || {
        let value = local_min.get();
        if value.is_empty() {
            price_min.set(None);
        } else if let Ok(num) = value.parse::<f64>() {
            price_min.set(Some(num));
        }
    };

    let apply_max = move || {
        let value = local_max.get();
        if value.is_empty() {
            price_max.set(None);
        } else if let Ok(num) = value.parse::<f64>() {
            price_max.set(Some(num));
        }
    };

    view! {
        <div class="space-y-3">
            <h3 class="font-bold text-gray-900 flex items-center gap-2 text-sm uppercase tracking-wide">
                <span class="text-blue-500">"▼"</span>
                "Price Range"
            </h3>
            <div class="flex items-center gap-2 bg-white p-1 rounded-lg border border-gray-200">
                <div class="relative flex-1">
                    <span class="absolute left-2 top-1/2 -translate-y-1/2 text-gray-400 text-xs">$</span>
                    <input
                        type="number"
                        placeholder="Min"
                        class="w-full pl-5 pr-2 py-1.5 border-none rounded text-sm focus:ring-0"
                        prop:value=move || local_min.get()
                        on:input=move |ev| local_min.set(event_target_value(&ev))
                        on:blur=move |_| apply_min()
                    />
                </div>
                <span class="text-gray-300">"–"</span>
                <div class="relative flex-1">
                    <span class="absolute left-2 top-1/2 -translate-y-1/2 text-gray-400 text-xs">$</span>
                    <input
                        type="number"
                        placeholder="Max"
                        class="w-full pl-5 pr-2 py-1.5 border-none rounded text-sm focus:ring-0"
                        prop:value=move || local_max.get()
                        on:input=move |ev| local_max.set(event_target_value(&ev))
                        on:blur=move |_| apply_max()
                    />
                </div>
            </div>
        </div>
    }
}

/// Rating filter component
///
/// Buttons for minimum rating (4+, 3+, etc.)
#[component]
pub fn RatingFilter(
    /// Minimum rating (None = any)
    min_rating: RwSignal<Option<f64>>,
) -> impl IntoView {
    let options = [
        (Some(4.0), "4+ ★"),
        (Some(3.0), "3+ ★"),
        (Some(2.0), "2+ ★"),
        (None, "Any"),
    ];

    view! {
        <div class="space-y-3">
            <h3 class="font-bold text-gray-900 flex items-center gap-2 text-sm uppercase tracking-wide">
                <span class="text-blue-500">"▼"</span>
                "Rating"
            </h3>
            <div class="flex flex-wrap gap-2">
                {options.into_iter().map(|(value, label)| {
                    let is_selected = move || min_rating.get() == value;
                    view! {
                        <button
                            type="button"
                            class=move || {
                                if is_selected() {
                                    "px-3 py-1.5 rounded-lg text-sm font-medium bg-blue-600 text-white shadow-sm transition-all"
                                } else {
                                    "px-3 py-1.5 rounded-lg text-sm font-medium bg-white border border-gray-200 \
                                     text-gray-700 hover:bg-gray-50 hover:border-gray-300 transition-all"
                                }
                            }
                            on:click=move |_| min_rating.set(value)
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// In-stock toggle component
#[component]
pub fn InStockToggle(
    /// Whether to show only in-stock items
    in_stock_only: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <label class="flex items-center gap-3 cursor-pointer group p-2 hover:bg-white rounded-lg transition-colors">
            <div class="relative flex items-center">
                <input
                    type="checkbox"
                    checked=move || in_stock_only.get()
                    on:change=move |_| in_stock_only.update(|v| *v = !*v)
                    class="peer h-5 w-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                />
            </div>
            <span class="text-sm font-medium text-gray-700 group-hover:text-gray-900">
                "In Stock Only"
            </span>
        </label>
    }
}

/// Complete filter panel component
///
/// Combines all filter components into a sidebar panel.
#[component]
pub fn FilterPanel(
    /// Category facets
    category_facets: Signal<Vec<FacetCount>>,
    /// Selected categories
    selected_categories: RwSignal<Vec<String>>,
    /// Price range
    price_min: RwSignal<Option<f64>>,
    price_max: RwSignal<Option<f64>>,
    /// Rating filter
    min_rating: RwSignal<Option<f64>>,
    /// In-stock filter
    in_stock_only: RwSignal<bool>,
    /// Clear filters callback
    on_clear: Callback<()>,
) -> impl IntoView {
    view! {
        <aside class="w-72 bg-gray-50/50 p-6 rounded-2xl border border-gray-100 space-y-8 h-fit sticky top-6">
            <div class="flex justify-between items-center pb-4 border-b border-gray-200">
                <h2 class="font-bold text-lg text-gray-900">"Filters"</h2>
                <button
                    type="button"
                    class="text-xs font-semibold text-blue-600 hover:text-blue-800 hover:underline uppercase tracking-wide"
                    on:click=move |_| on_clear.run(())
                >
                    "Clear All"
                </button>
            </div>

            <CategoryFacets
                facets=category_facets
                selected=selected_categories
            />

            <PriceRangeFilter
                price_min=price_min
                price_max=price_max
            />

            <RatingFilter min_rating=min_rating />

            <div class="pt-4 border-t border-gray-200">
                <InStockToggle in_stock_only=in_stock_only />
            </div>
        </aside>
    }
}

/// Pagination component
#[component]
pub fn Pagination(
    /// Current page (0-indexed)
    current_page: RwSignal<u32>,
    /// Total number of items
    total_items: Signal<i64>,
    /// Items per page
    page_size: u32,
) -> impl IntoView {
    let total_pages = move || {
        let total = total_items.get() as f64;
        (total / page_size as f64).ceil() as u32
    };

    let can_go_prev = move || current_page.get() > 0;
    let can_go_next = move || current_page.get() < total_pages().saturating_sub(1);

    let go_prev = move |_| {
        if can_go_prev() {
            current_page.update(|p| *p = p.saturating_sub(1));
        }
    };

    let go_next = move |_| {
        if can_go_next() {
            current_page.update(|p| *p += 1);
        }
    };

    view! {
        <div class="flex items-center justify-center gap-4 mt-12 mb-8">
            <button
                type="button"
                class="px-4 py-2 bg-white border border-gray-200 rounded-lg shadow-sm \
                       disabled:opacity-50 disabled:cursor-not-allowed \
                       hover:bg-gray-50 hover:border-gray-300 transition-all font-medium text-gray-700"
                disabled=move || !can_go_prev()
                on:click=go_prev
            >
                "← Previous"
            </button>

            <span class="text-sm font-medium text-gray-600 bg-gray-100 px-4 py-2 rounded-lg">
                "Page " {move || current_page.get() + 1} " of " {total_pages}
            </span>

            <button
                type="button"
                class="px-4 py-2 bg-white border border-gray-200 rounded-lg shadow-sm \
                       disabled:opacity-50 disabled:cursor-not-allowed \
                       hover:bg-gray-50 hover:border-gray-300 transition-all font-medium text-gray-700"
                disabled=move || !can_go_next()
                on:click=go_next
            >
                "Next →"
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_option_string_mapping() {
        // Test that our string mapping is correct
        let options = [
            (SortOption::Relevance, "relevance"),
            (SortOption::PriceAsc, "price_asc"),
            (SortOption::PriceDesc, "price_desc"),
            (SortOption::RatingDesc, "rating_desc"),
            (SortOption::Newest, "newest"),
        ];

        for (opt, expected_str) in options {
            let actual = match opt {
                SortOption::Relevance => "relevance",
                SortOption::PriceAsc => "price_asc",
                SortOption::PriceDesc => "price_desc",
                SortOption::RatingDesc => "rating_desc",
                SortOption::Newest => "newest",
            };
            assert_eq!(actual, expected_str);
        }
    }

    #[test]
    fn test_search_mode_variants() {
        // Verify all search modes exist
        let modes = [SearchMode::Bm25, SearchMode::Vector, SearchMode::Hybrid];
        assert_eq!(modes.len(), 3);
        assert_eq!(SearchMode::default(), SearchMode::Hybrid);
    }

    #[test]
    fn test_pagination_logic_pure() {
        // Test the math behind pagination
        let total_items = 100i64;
        let page_size = 10u32;
        
        let total_pages = (total_items as f64 / page_size as f64).ceil() as u32;
        assert_eq!(total_pages, 10);

        let total_items_2 = 101i64;
        let total_pages_2 = (total_items_2 as f64 / page_size as f64).ceil() as u32;
        assert_eq!(total_pages_2, 11);
    }
}
