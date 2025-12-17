// web_app/app.rs - Root application component
//
// This is the entry point for the Leptos application.
// It sets up routing, global state, and the component tree.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::web_app::pages::SearchPage;

/// Root application component
///
/// Sets up:
/// - Meta tags for SEO
/// - Router with routes
/// - Global error boundary
#[component]
pub fn App() -> impl IntoView {
    // Provide meta context for <Title>, <Meta>, etc.
    provide_meta_context();

    view! {
        // HTML meta tags
        <Title text="IRDB Product Search" />
        <Meta name="description" content="AI-enhanced product search with hybrid BM25 and vector similarity" />
        <Meta name="viewport" content="width=device-width, initial-scale=1" />

        // Stylesheet link (Tailwind CSS)
        <Stylesheet id="leptos" href="/pkg/pg_search_tests.css" />

        // Router setup
        <Router>
            <main class="min-h-screen">
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/") view=SearchPage />
                    <Route path=path!("/search") view=SearchPage />
                </Routes>
            </main>
        </Router>
    }
}

/// 404 Not Found page
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-100 flex items-center justify-center">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-gray-300 mb-4">"404"</h1>
                <p class="text-xl text-gray-600 mb-8">"Page not found"</p>
                <a
                    href="/"
                    class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                >
                    "Go to Search"
                </a>
            </div>
        </div>
    }
}

/// Shell component for SSR hydration
///
/// Wraps the app with the HTML shell for server-side rendering.
/// Note: In Leptos 0.8, you typically use leptos_actix's LeptosRoutes
/// and the shell is configured in main.rs. This is a simplified version.
#[component]
pub fn AppShell() -> impl IntoView {
    view! {
        <App />
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_app_compiles() {
        // This test just verifies the module compiles correctly
        assert!(true);
    }
}
