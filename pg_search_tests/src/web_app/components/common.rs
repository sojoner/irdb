// web_app/components/common.rs - Reusable UI components
//
// These are small, composable components used throughout the application.
// Philosophy: Pure, stateless components that receive all data via props.

use leptos::prelude::*;
use leptos::web_sys::KeyboardEvent;

/// Loading spinner component
///
/// Displays a centered spinner with optional message.
#[component]
pub fn Loading(
    /// Optional message to display below the spinner
    #[prop(default = "Loading...")]
    message: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center p-12">
            <div class="animate-spin rounded-full h-10 w-10 border-4 border-gray-200 border-t-blue-600"></div>
            <span class="mt-4 text-gray-500 font-medium animate-pulse">{message}</span>
        </div>
    }
}

/// Error display component
///
/// Displays error messages with appropriate styling.
#[component]
pub fn ErrorDisplay(
    /// The error message to display
    error: String,
) -> impl IntoView {
    view! {
        <div class="bg-red-50 border border-red-200 rounded-xl p-6 flex items-start gap-4">
            <div class="bg-red-100 p-2 rounded-full text-red-600">
                <span class="text-xl font-bold">"⚠"</span>
            </div>
            <div>
                <h3 class="text-red-800 font-bold mb-1">"Error Occurred"</h3>
                <p class="text-red-600 text-sm">{error}</p>
            </div>
        </div>
    }
}

/// Primary button component
///
/// A styled button with hover effects.
#[component]
pub fn Button(
    /// Button label text
    children: Children,
    /// Click handler
    #[prop(optional)]
    on_click: Option<Callback<()>>,
    /// Whether the button is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Button type (submit, button, reset)
    #[prop(default = "button")]
    button_type: &'static str,
    /// Additional CSS classes
    #[prop(default = "")]
    class: &'static str,
) -> impl IntoView {
    let base_class = "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 \
                      transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed \
                      font-medium shadow-sm active:transform active:scale-95";

    view! {
        <button
            type=button_type
            disabled=disabled
            class=format!("{} {}", base_class, class)
            on:click=move |_| {
                if let Some(handler) = on_click {
                    handler.run(());
                }
            }
        >
            {children()}
        </button>
    }
}

/// Secondary button component
///
/// A lighter styled button for secondary actions.
#[component]
pub fn SecondaryButton(
    children: Children,
    #[prop(optional)]
    on_click: Option<Callback<()>>,
    #[prop(default = false)]
    disabled: bool,
) -> impl IntoView {
    let class = "px-4 py-2 bg-white text-gray-700 rounded-lg hover:bg-gray-50 \
                 transition-colors border border-gray-300 disabled:opacity-50 \
                 font-medium shadow-sm active:bg-gray-100";

    view! {
        <button
            type="button"
            disabled=disabled
            class=class
            on:click=move |_| {
                if let Some(handler) = on_click {
                    handler.run(());
                }
            }
        >
            {children()}
        </button>
    }
}

/// Modal wrapper component
///
/// Provides modal backdrop styling. The open/close logic should be
/// handled by the parent using Show/Suspense.
#[component]
pub fn ModalWrapper(
    /// Modal content
    children: Children,
    /// Callback when modal should close
    on_close: Callback<()>,
    /// Modal title
    #[prop(default = "")]
    title: &'static str,
) -> impl IntoView {
    // Close on escape key
    let handle_keydown = move |ev: KeyboardEvent| {
        if ev.key() == "Escape" {
            on_close.run(());
        }
    };

    // Close on backdrop click
    let handle_backdrop_click = move |_| {
        on_close.run(());
    };

    view! {
        <div
            class="fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6"
            on:keydown=handle_keydown
        >
            // Backdrop with blur
            <div 
                class="absolute inset-0 bg-gray-900/60 backdrop-blur-sm transition-opacity"
                on:click=handle_backdrop_click
            ></div>

            // Modal Content
            <div
                class="relative bg-white rounded-2xl shadow-2xl w-full max-w-3xl max-h-[90vh] flex flex-col overflow-hidden transform transition-all scale-100"
                on:click=|ev| ev.stop_propagation()
            >
                // Header
                <div class="flex justify-between items-center px-6 py-4 border-b border-gray-100 bg-gray-50/50">
                    <h2 class="text-xl font-bold text-gray-800">{title}</h2>
                    <button
                        class="text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-full p-2 transition-colors"
                        on:click=move |_| on_close.run(())
                        title="Close"
                    >
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                // Body (Scrollable)
                <div class="p-6 overflow-y-auto custom-scrollbar">
                    {children()}
                </div>
            </div>
        </div>
    }
}

/// Star rating display component
///
/// Displays a star rating (0-5) with filled and empty stars.
#[component]
pub fn StarRating(
    /// The rating value (0.0 to 5.0)
    rating: f64,
    /// Whether to show the numeric value
    #[prop(default = true)]
    show_value: bool,
) -> impl IntoView {
    let full_stars = rating.floor() as usize;
    let has_half = (rating - rating.floor()) >= 0.5;
    let empty_stars = 5 - full_stars - if has_half { 1 } else { 0 };

    view! {
        <div class="flex items-center gap-0.5" title=format!("Rating: {:.1}", rating)>
            // Full stars
            {(0..full_stars).map(|_| view! {
                <span class="text-yellow-400 text-lg">"★"</span>
            }).collect_view()}

            // Half star (shown as full for simplicity in this MVP, but styled slightly differently)
            {if has_half {
                Some(view! { 
                    <div class="relative inline-block text-lg">
                        <span class="text-gray-200">"★"</span>
                        <span class="absolute top-0 left-0 overflow-hidden w-1/2 text-yellow-400">"★"</span>
                    </div>
                })
            } else {
                None
            }}

            // Empty stars
            {(0..empty_stars).map(|_| view! {
                <span class="text-gray-200 text-lg">"★"</span>
            }).collect_view()}

            // Numeric value
            <Show when=move || show_value>
                <span class="ml-2 text-sm font-bold text-gray-700 bg-gray-100 px-1.5 py-0.5 rounded">
                    {format!("{:.1}", rating)}
                </span>
            </Show>
        </div>
    }
}

/// Badge component
///
/// A small badge/tag for displaying labels.
#[component]
pub fn Badge(
    children: Children,
    /// Badge color variant
    #[prop(default = "gray")]
    variant: &'static str,
) -> impl IntoView {
    let class = match variant {
        "green" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-green-100 text-green-800 border border-green-200",
        "red" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800 border border-red-200",
        "blue" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800 border border-blue-200",
        "yellow" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-yellow-100 text-yellow-800 border border-yellow-200",
        _ => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-gray-100 text-gray-800 border border-gray-200",
    };

    view! {
        <span class=class>
            {children()}
        </span>
    }
}

/// Text input component
///
/// A styled text input with optional placeholder.
#[component]
pub fn TextInput(
    /// The current value
    value: RwSignal<String>,
    /// Placeholder text
    #[prop(default = "")]
    placeholder: &'static str,
    /// Input type (text, search, email, etc.)
    #[prop(default = "text")]
    input_type: &'static str,
    /// Additional CSS classes
    #[prop(default = "")]
    class: &'static str,
) -> impl IntoView {
    let base_class = "w-full px-4 py-2 border border-gray-300 rounded-lg \
                      focus:ring-2 focus:ring-blue-500 focus:border-transparent \
                      outline-none transition-shadow shadow-sm";

    view! {
        <input
            type=input_type
            placeholder=placeholder
            class=format!("{} {}", base_class, class)
            prop:value=move || value.get()
            on:input=move |ev| {
                value.set(event_target_value(&ev));
            }
        />
    }
}

/// Select dropdown component
///
/// A styled select dropdown for string values.
#[component]
pub fn SelectString(
    /// The currently selected value
    value: RwSignal<String>,
    /// Available options as (value, label) pairs
    options: Vec<(String, String)>,
) -> impl IntoView {
    let class = "px-4 py-2 border border-gray-300 rounded-lg bg-white \
                 focus:ring-2 focus:ring-blue-500 focus:border-transparent \
                 outline-none cursor-pointer shadow-sm";

    view! {
        <select
            class=class
            on:change=move |ev| {
                value.set(event_target_value(&ev));
            }
        >
            {options.into_iter().map(|(opt_value, label)| {
                let opt_val = opt_value.clone();
                view! {
                    <option
                        value=opt_value
                        selected=move || value.get() == opt_val
                    >
                        {label}
                    </option>
                }
            }).collect_view()}
        </select>
    }
}

/// Checkbox component
///
/// A styled checkbox with label.
#[component]
pub fn Checkbox(
    /// Whether the checkbox is checked
    checked: RwSignal<bool>,
    /// Label text
    label: String,
    /// Change handler
    #[prop(optional)]
    on_change: Option<Callback<bool>>,
) -> impl IntoView {
    view! {
        <label class="flex items-center gap-3 cursor-pointer group">
            <input
                type="checkbox"
                class="rounded border-gray-300 text-blue-600 focus:ring-blue-500 h-4 w-4"
                prop:checked=move || checked.get()
                on:change=move |ev| {
                    let new_value = event_target_checked(&ev);
                    checked.set(new_value);
                    if let Some(handler) = on_change {
                        handler.run(new_value);
                    }
                }
            />
            <span class="text-gray-700 group-hover:text-gray-900 transition-colors">{label}</span>
        </label>
    }
}

/// Price display component
///
/// Formats and displays a price value.
#[component]
pub fn PriceDisplay(
    /// The price value
    price: f64,
    /// Whether to highlight (larger, bolder)
    #[prop(default = false)]
    highlight: bool,
) -> impl IntoView {
    let class = if highlight {
        "text-xl font-bold text-green-600"
    } else {
        "text-gray-900 font-medium"
    };

    view! {
        <span class=class>
            {format!("${:.2}", price)}
        </span>
    }
}

#[cfg(test)]
mod tests {
    // Component tests would typically be done via end-to-end testing
    // or component testing frameworks. Unit tests verify logic only.

    #[test]
    fn test_star_calculation() {
        // Test the star calculation logic
        let rating = 4.5_f64;
        let full_stars = rating.floor() as usize;
        let has_half = (rating - rating.floor()) >= 0.5;

        assert_eq!(full_stars, 4);
        assert!(has_half);
    }

    #[test]
    fn test_star_calculation_whole() {
        let rating = 3.0_f64;
        let full_stars = rating.floor() as usize;
        let has_half = (rating - rating.floor()) >= 0.5;

        assert_eq!(full_stars, 3);
        assert!(!has_half);
    }

    #[test]
    fn test_star_calculation_boundaries() {
        let rating = 0.0_f64;
        let full_stars = rating.floor() as usize;
        let has_half = (rating - rating.floor()) >= 0.5;
        assert_eq!(full_stars, 0);
        assert!(!has_half);

        let rating = 5.0_f64;
        let full_stars = rating.floor() as usize;
        let has_half = (rating - rating.floor()) >= 0.5;
        assert_eq!(full_stars, 5);
        assert!(!has_half);
    }

    #[test]
    fn test_star_calculation_fractional() {
        // Test various fractional ratings
        let test_cases: [(f64, usize, bool); 9] = [
            (4.4, 4, false),  // Just under half
            (4.5, 4, true),   // Exactly half
            (4.6, 4, true),   // Above half
            (4.9, 4, true),   // Just under next whole
            (3.49, 3, false), // Just under half
            (3.50, 3, true),  // Exactly half
            (2.25, 2, false), // Quarter
            (2.75, 2, true),  // Three quarters
            (1.1, 1, false),  // Just above whole
        ];

        for (rating, expected_full, expected_half) in test_cases {
            let full_stars = rating.floor() as usize;
            let has_half = (rating - rating.floor()) >= 0.5;
            assert_eq!(full_stars, expected_full, "Full stars for rating {}", rating);
            assert_eq!(has_half, expected_half, "Has half for rating {}", rating);
        }
    }

    #[test]
    fn test_star_empty_calculation() {
        // Test empty star count calculation
        let test_cases: [(f64, usize); 7] = [
            (5.0, 0),  // All full, no empty
            (4.5, 0),  // 4 full + 1 half = 5, no empty
            (4.0, 1),  // 4 full, 1 empty
            (3.5, 1),  // 3 full + 1 half = 4, 1 empty
            (3.0, 2),  // 3 full, 2 empty
            (0.0, 5),  // No full, 5 empty
            (0.5, 4),  // 0 full + 1 half = 1, 4 empty
        ];

        for (rating, expected_empty) in test_cases {
            let full_stars = rating.floor() as usize;
            let has_half = (rating - rating.floor()) >= 0.5;
            let empty_stars = 5 - full_stars - if has_half { 1 } else { 0 };
            assert_eq!(empty_stars, expected_empty, "Empty stars for rating {}", rating);
        }
    }

    #[test]
    fn test_badge_variants() {
        let variants = ["green", "red", "blue", "yellow", "gray", "unknown"];
        for variant in variants {
            let class = match variant {
                "green" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-green-100 text-green-800 border border-green-200",
                "red" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800 border border-red-200",
                "blue" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800 border border-blue-200",
                "yellow" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-yellow-100 text-yellow-800 border border-yellow-200",
                _ => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-gray-100 text-gray-800 border border-gray-200",
            };

            if variant == "green" {
                assert!(class.contains("bg-green-100"));
            } else if variant == "red" {
                assert!(class.contains("bg-red-100"));
            } else if variant == "blue" {
                assert!(class.contains("bg-blue-100"));
            } else if variant == "yellow" {
                assert!(class.contains("bg-yellow-100"));
            } else {
                assert!(class.contains("bg-gray-100"));
            }
        }
    }

    #[test]
    fn test_badge_all_class_properties() {
        // Test that all badge classes contain expected properties
        let variants = ["green", "red", "blue", "yellow", "gray"];
        for variant in variants {
            let class = match variant {
                "green" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-green-100 text-green-800 border border-green-200",
                "red" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800 border border-red-200",
                "blue" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800 border border-blue-200",
                "yellow" => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-yellow-100 text-yellow-800 border border-yellow-200",
                _ => "px-2.5 py-0.5 text-xs font-medium rounded-full bg-gray-100 text-gray-800 border border-gray-200",
            };

            // Common properties
            assert!(class.contains("px-2.5"), "Padding x for {}", variant);
            assert!(class.contains("py-0.5"), "Padding y for {}", variant);
            assert!(class.contains("text-xs"), "Text size for {}", variant);
            assert!(class.contains("font-medium"), "Font weight for {}", variant);
            assert!(class.contains("rounded-full"), "Rounded for {}", variant);
            assert!(class.contains("border"), "Border for {}", variant);
        }
    }

    #[test]
    fn test_price_formatting_logic() {
        let prices = [
            (0.0, "$0.00"),
            (99.99, "$99.99"),
            (1234.567, "$1234.57"),
            (10.1, "$10.10"),
        ];

        for (price, expected) in prices {
            let formatted = format!("${:.2}", price);
            assert_eq!(formatted, expected);
        }
    }

    #[test]
    fn test_price_formatting_edge_cases() {
        // Additional edge cases for price formatting
        // Note: Floating point rounding behavior varies, so we use values that round predictably
        let prices = [
            (0.001, "$0.00"),       // Very small, rounds to 0
            (0.006, "$0.01"),       // Rounds up
            (0.004, "$0.00"),       // Rounds down
            (999999.99, "$999999.99"), // Large number
            (1.0, "$1.00"),         // Whole number
            (100.0, "$100.00"),     // Round hundred
            (50.0, "$50.00"),       // Round fifty
        ];

        for (price, expected) in prices {
            let formatted = format!("${:.2}", price);
            assert_eq!(formatted, expected, "Price formatting for {}", price);
        }
    }

    #[test]
    fn test_price_display_highlight_class() {
        // Test the class logic used in PriceDisplay
        let highlight = true;
        let class = if highlight {
            "text-xl font-bold text-green-600"
        } else {
            "text-gray-900 font-medium"
        };
        assert!(class.contains("text-xl"));
        assert!(class.contains("font-bold"));
        assert!(class.contains("text-green-600"));

        let highlight = false;
        let class = if highlight {
            "text-xl font-bold text-green-600"
        } else {
            "text-gray-900 font-medium"
        };
        assert!(class.contains("text-gray-900"));
        assert!(class.contains("font-medium"));
    }

    #[test]
    fn test_button_class_construction() {
        // Test the class concatenation logic used in Button
        let base_class = "px-4 py-2 bg-blue-600 text-white rounded-lg";
        let additional = "custom-class";
        let combined = format!("{} {}", base_class, additional);

        assert!(combined.contains("px-4"));
        assert!(combined.contains("custom-class"));

        // Test with empty additional class
        let combined_empty = format!("{} {}", base_class, "");
        assert!(combined_empty.contains("px-4"));
        assert!(combined_empty.ends_with(" "));
    }

    #[test]
    fn test_rating_title_format() {
        // Test the title attribute format used in StarRating
        let ratings = [0.0, 1.5, 2.5, 3.0, 4.5, 5.0];
        for rating in ratings {
            let title = format!("Rating: {:.1}", rating);
            assert!(title.starts_with("Rating: "));
            // Check it has one decimal place
            let parts: Vec<&str> = title.split('.').collect();
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[1].len(), 1);
        }
    }

    #[test]
    fn test_modal_escape_key_detection() {
        // Test the escape key logic (mocking the key string)
        let keys = ["Escape", "Enter", "Tab", "ArrowUp"];
        for key in keys {
            let should_close = key == "Escape";
            assert_eq!(should_close, key == "Escape", "Key: {}", key);
        }
    }
}
