// web_app/components/product.rs - Product display components
//
// Components for displaying products including:
// - ProductCard: Grid card for search results
// - ProductDetail: Full product detail view
// - ProductGrid: Grid layout for multiple products

use leptos::prelude::*;
use crate::web_app::model::{Product, SearchResult};
use super::common::StarRating;

/// Product card for search results grid
///
/// Displays a product summary with click handler for details.
#[component]
pub fn ProductCard(
    /// The search result to display
    result: SearchResult,
    /// Click handler for viewing details
    on_click: Callback<i32>,
) -> impl IntoView {
    let product = result.product;
    let product_id = product.id;

    // Format price
    let price_display = format!("${:.2}", product.price);

    // Truncate description
    let description_preview = if product.description.len() > 120 {
        format!("{}...", &product.description[..120])
    } else {
        product.description.clone()
    };

    // Rating as f64
    let rating_f64: f64 = product.rating.try_into().unwrap_or(0.0);

    view! {
        <div
            class="group bg-white rounded-xl shadow-sm hover:shadow-xl \
                   transition-all duration-300 cursor-pointer border border-gray-100 \
                   flex flex-col h-full overflow-hidden transform hover:-translate-y-1"
            on:click=move |_| on_click.run(product_id)
        >
            // Image placeholder (since we don't have real images)
            <div class="h-48 bg-gray-100 flex items-center justify-center text-gray-300 group-hover:bg-gray-50 transition-colors">
                <span class="text-4xl">"📦"</span>
            </div>

            <div class="p-5 flex flex-col flex-1">
                // Header: Rating and Price
                <div class="flex justify-between items-start mb-3">
                    <StarRating rating=rating_f64 />
                    <span class="text-lg font-bold text-blue-600 bg-blue-50 px-2 py-1 rounded-lg">
                        {price_display}
                    </span>
                </div>

                // Title
                <h3 class="font-bold text-gray-900 mb-2 line-clamp-2 text-lg group-hover:text-blue-600 transition-colors">
                    {product.name.clone()}
                </h3>

                // Description snippet
                <p class="text-gray-600 text-sm mb-4 line-clamp-3 flex-1">
                    {result.snippet.clone().unwrap_or(description_preview)}
                </p>

                // Metadata: Brand and Category
                <div class="flex justify-between items-center text-xs text-gray-500 mb-3 pt-3 border-t border-gray-100">
                    <span class="font-medium bg-gray-100 px-2 py-1 rounded text-gray-600">
                        {product.brand.clone()}
                    </span>
                    <span class="text-gray-400">{product.category.clone()}</span>
                </div>

                // Stock status & Badges
                <div class="flex items-center gap-2 flex-wrap">
                    {if product.in_stock {
                        view! {
                            <span class="text-xs px-2 py-1 bg-green-100 text-green-700 rounded-full font-medium flex items-center gap-1">
                                <span class="w-1.5 h-1.5 bg-green-500 rounded-full"></span>
                                "In Stock"
                            </span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="text-xs px-2 py-1 bg-red-100 text-red-700 rounded-full font-medium flex items-center gap-1">
                                <span class="w-1.5 h-1.5 bg-red-500 rounded-full"></span>
                                "Out of Stock"
                            </span>
                        }.into_any()
                    }}

                    {if product.featured {
                        Some(view! {
                            <span class="text-xs px-2 py-1 bg-yellow-100 text-yellow-800 rounded-full font-medium flex items-center gap-1">
                                "★ Featured"
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>

                // Score display (for debugging/transparency)
                {
                    let score = result.combined_score;
                    let bm25 = result.bm25_score;
                    let vector = result.vector_score;
                    (score > 0.0).then(|| view! {
                        <div class="mt-3 pt-2 border-t border-gray-100 text-[10px] text-gray-400 flex gap-2 font-mono">
                            <span>"S:" {format!("{:.2}", score)}</span>
                            {bm25.map(|s| view! {
                                <span>"B:" {format!("{:.2}", s)}</span>
                            })}
                            {vector.map(|s| view! {
                                <span>"V:" {format!("{:.2}", s)}</span>
                            })}
                        </div>
                    })
                }
            </div>
        </div>
    }
}

/// Product detail view
///
/// Full product information display, typically shown in a modal.
#[component]
pub fn ProductDetail(
    /// The product to display
    product: Product,
) -> impl IntoView {
    let rating_f64: f64 = product.rating.try_into().unwrap_or(0.0);
    let price_display = format!("${:.2}", product.price);

    view! {
        <div class="space-y-8">
            // Header Section
            <div class="flex flex-col md:flex-row gap-6">
                // Image Placeholder
                <div class="w-full md:w-1/3 aspect-square bg-gray-100 rounded-xl flex items-center justify-center text-gray-300">
                    <span class="text-6xl">"📦"</span>
                </div>

                // Main Info
                <div class="flex-1 space-y-4">
                    <div class="flex justify-between items-start">
                        <div>
                            <h2 class="text-3xl font-bold text-gray-900 leading-tight mb-2">
                                {product.name.clone()}
                            </h2>
                            <div class="flex items-center gap-3 text-sm">
                                <span class="font-semibold text-gray-900 bg-gray-100 px-3 py-1 rounded-full">
                                    {product.brand.clone()}
                                </span>
                                <span class="text-gray-500">
                                    {product.category.clone()}
                                    {product.subcategory.clone().map(|sub| view! {
                                        <span class="mx-1">"›"</span> {sub}
                                    })}
                                </span>
                            </div>
                        </div>
                        <span class="text-3xl font-bold text-blue-600 bg-blue-50 px-4 py-2 rounded-xl">
                            {price_display}
                        </span>
                    </div>

                    <div class="flex items-center gap-4 py-2">
                        <div class="flex items-center gap-2">
                            <StarRating rating=rating_f64 />
                            <span class="text-gray-600 font-medium">
                                {format!("{:.1}", rating_f64)}
                            </span>
                        </div>
                        <span class="text-gray-300">"|"</span>
                        <span class="text-gray-600">
                            {product.review_count} " reviews"
                        </span>
                    </div>

                    // Stock status
                    <div class="flex items-center gap-4">
                        {if product.in_stock {
                            view! {
                                <div class="flex items-center gap-2 bg-green-50 text-green-700 px-3 py-1.5 rounded-lg border border-green-100">
                                    <span class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
                                    <span class="font-medium">"In Stock"</span>
                                    <span class="text-green-600 text-sm">
                                        "(" {product.stock_quantity} " available)"
                                    </span>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex items-center gap-2 bg-red-50 text-red-700 px-3 py-1.5 rounded-lg border border-red-100">
                                    <span class="w-2 h-2 bg-red-500 rounded-full"></span>
                                    <span class="font-medium">"Out of Stock"</span>
                                </div>
                            }.into_any()
                        }}
                    </div>

                    // Tags
                    {
                        let tags = product.tags.clone();
                        (!tags.is_empty()).then(|| view! {
                            <div class="flex flex-wrap gap-2 pt-2">
                                {tags.into_iter().map(|tag| view! {
                                    <span class="px-3 py-1 text-xs font-medium bg-gray-100 text-gray-600 rounded-full border border-gray-200">
                                        "#" {tag}
                                    </span>
                                }).collect_view()}
                            </div>
                        })
                    }
                </div>
            </div>

            <hr class="border-gray-100" />

            // Description
            <div class="prose prose-blue max-w-none">
                <h3 class="text-lg font-bold text-gray-900 mb-3">"Description"</h3>
                <p class="text-gray-600 leading-relaxed text-lg">
                    {product.description.clone()}
                </p>
            </div>

            // Attributes (if any)
            {product.attributes.clone().map(|attrs| {
                view! {
                    <div class="bg-gray-50 rounded-xl p-6 border border-gray-100">
                        <h3 class="text-lg font-bold text-gray-900 mb-4">"Specifications"</h3>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                            {
                                if let serde_json::Value::Object(map) = attrs {
                                    view! {
                                        {map.into_iter().map(|(k, v)| {
                                            let val_str = match v {
                                                serde_json::Value::String(s) => s,
                                                serde_json::Value::Number(n) => n.to_string(),
                                                serde_json::Value::Bool(b) => b.to_string(),
                                                _ => v.to_string(),
                                            };
                                            // Format key: snake_case to Title Case
                                            let key_display = k.replace('_', " ")
                                                .split_whitespace()
                                                .map(|word| {
                                                    let mut c = word.chars();
                                                    match c.next() {
                                                        None => String::new(),
                                                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                                                    }
                                                })
                                                .collect::<Vec<String>>()
                                                .join(" ");

                                            view! {
                                                <div class="flex justify-between border-b border-gray-200 pb-2 last:border-0">
                                                    <span class="text-gray-500">{key_display}</span>
                                                    <span class="font-medium text-gray-900">{val_str}</span>
                                                </div>
                                            }
                                        }).collect_view()}
                                    }.into_any()
                                } else {
                                    view! { <pre>{serde_json::to_string_pretty(&attrs).unwrap_or_default()}</pre> }.into_any()
                                }
                            }
                        </div>
                    </div>
                }
            })}
        </div>
    }
}

/// Results grid component
///
/// Displays a grid of ProductCards with optional empty state.
#[component]
pub fn ResultsGrid(
    /// Search results to display
    results: Signal<Vec<SearchResult>>,
    /// Total count for display
    total_count: Signal<i64>,
    /// Click handler for product details
    on_product_click: Callback<i32>,
) -> impl IntoView {
    view! {
        <div class="w-full">
            // Results header
            <div class="flex justify-between items-center mb-6">
                <span class="text-gray-500 font-medium">
                    {move || {
                        let count = total_count.get();
                        if count == 1 {
                            "1 product found".to_string()
                        } else {
                            format!("{} products found", count)
                        }
                    }}
                </span>
            </div>

            // Grid or empty state
            <Show
                when=move || !results.get().is_empty()
                fallback=|| view! {
                    <div class="text-center py-16 bg-white rounded-2xl border border-dashed border-gray-300">
                        <div class="text-gray-300 text-6xl mb-4">"🔍"</div>
                        <h3 class="text-xl font-bold text-gray-900 mb-2">"No products found"</h3>
                        <p class="text-gray-500 max-w-md mx-auto">
                            "We couldn't find any products matching your search. Try adjusting your filters or search terms."
                        </p>
                    </div>
                }
            >
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    <For
                        each=move || results.get()
                        key=|r| r.product.id
                        children=move |result| {
                            let handler = on_product_click;
                            view! {
                                <ProductCard
                                    result=result
                                    on_click=handler
                                />
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

/// Search result with detail score breakdown (for debugging)
#[component]
pub fn ScoreBreakdown(
    /// BM25 score
    bm25_score: Option<f64>,
    /// Vector similarity score
    vector_score: Option<f64>,
    /// Combined/final score
    combined_score: f64,
) -> impl IntoView {
    view! {
        <div class="bg-gray-50 rounded-lg p-4 text-sm border border-gray-200">
            <h4 class="font-medium text-gray-700 mb-2">"Search Scores"</h4>
            <div class="grid grid-cols-3 gap-4">
                <div>
                    <span class="text-gray-500 block text-xs uppercase">"BM25"</span>
                    <span class="font-mono font-medium">
                        {bm25_score.map(|s| format!("{:.3}", s)).unwrap_or_else(|| "N/A".to_string())}
                    </span>
                </div>
                <div>
                    <span class="text-gray-500 block text-xs uppercase">"Vector"</span>
                    <span class="font-mono font-medium">
                        {vector_score.map(|s| format!("{:.3}", s)).unwrap_or_else(|| "N/A".to_string())}
                    </span>
                </div>
                <div>
                    <span class="text-gray-500 block text-xs uppercase">"Combined"</span>
                    <span class="font-mono font-bold text-blue-600">
                        {format!("{:.3}", combined_score)}
                    </span>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use crate::web_app::model::Product;

    fn create_test_product() -> Product {
        Product {
            id: 1,
            name: "Test Product".to_string(),
            description: "A test product description that is longer than 120 characters to test truncation behavior in the product card component display.".to_string(),
            brand: "TestBrand".to_string(),
            category: "Electronics".to_string(),
            subcategory: Some("Gadgets".to_string()),
            tags: vec!["test".to_string(), "sample".to_string()],
            price: Decimal::new(9999, 2), // 99.99
            rating: Decimal::new(45, 1),  // 4.5
            review_count: 100,
            stock_quantity: 50,
            in_stock: true,
            featured: true,
            attributes: None,
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        }
    }

    #[test]
    fn test_description_truncation() {
        let product = create_test_product();
        let truncated = if product.description.len() > 120 {
            format!("{}...", &product.description[..120])
        } else {
            product.description.clone()
        };

        assert!(truncated.len() <= 123); // 120 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_price_formatting() {
        let product = create_test_product();
        let price_display = format!("${:.2}", product.price);
        assert_eq!(price_display, "$99.99");
    }

    #[test]
    fn test_rating_conversion() {
        let product = create_test_product();
        let rating_f64: f64 = product.rating.try_into().unwrap_or(0.0);
        assert!((rating_f64 - 4.5).abs() < 0.01);
    }
}
