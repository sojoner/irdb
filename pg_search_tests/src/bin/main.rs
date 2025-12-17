// Leptos web application server
//
// This binary starts the web server with:
// - Actix-web for HTTP serving
// - Leptos for SSR (server-side rendering)
// - PostgreSQL connection pool
// - Static file serving

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::{web, App, HttpServer};
    use leptos::prelude::*;
    use leptos_actix::{generate_route_list, LeptosRoutes, handle_server_fns};
    use leptos_meta::MetaTags;
    use pg_search_tests::web_app::app::App as WebApp;
    use pg_search_tests::web_app::api::db;
    use sqlx::postgres::PgPoolOptions;
    use std::env;
    use tracing_subscriber;

    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:custom_secure_password_123@localhost/database".to_string());

    // Create PostgreSQL connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create connection pool");

    tracing::info!("Connected to database: {}", database_url);

    // Initialize global pool for server functions
    db::init_db(pool.clone());

    // Seed database if empty
    if let Err(e) = seed_database(&pool).await {
        tracing::error!("Failed to seed database: {}", e);
    }

    // Leptos configuration
    let conf = leptos_config::get_configuration(None).expect("could not read configuration");
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let site_root = leptos_options.site_root.clone();

    tracing::info!("Starting server at http://{}", addr);

    HttpServer::new(move || {
        // Generate the list of routes in the Leptos App
        let routes = generate_route_list(WebApp);
        let leptos_options_inner = leptos_options.clone();
        let site_root_str = site_root.clone().to_string();
        let pool_data = web::Data::new(pool.clone());

        tracing::info!("Configuring App with database pool");

        App::new()
            // Share database pool across all handlers
            .app_data(pool_data.clone())
            // Also share raw pool for direct access if needed
            .app_data(pool.clone())
            // Explicitly handle server functions
            .route("/api/{tail:.*}", handle_server_fns())
            // Serve JS/WASM/CSS from pkg directory
            .service(Files::new("/pkg", format!("{site_root_str}/pkg")))
            // Leptos routes for SSR with proper shell
            .leptos_routes(routes, {
                let leptos_options = leptos_options_inner.clone();
                move || {
                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone() />
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                            </head>
                            <body>
                                <WebApp/>
                            </body>
                        </html>
                    }
                }
            })
            .app_data(web::Data::new(leptos_options_inner.clone()))
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(feature = "ssr")]
async fn seed_database(pool: &sqlx::PgPool) -> std::io::Result<()> {
    use pg_search_tests::web_app::model::ProductImport;
    use std::fs::File;
    use std::io::BufReader;

    // Check if database is empty
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products.items")
        .fetch_one(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if count.0 > 0 {
        tracing::info!("Database already contains {} products, skipping seed.", count.0);
        return Ok(());
    }

    tracing::info!("Seeding database from pg_search_tests/data/products.json...");

    // Read file
    let file = File::open("pg_search_tests/data/products.json")?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader)?;
    let products_json = json.get("products").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing 'products' key")
    })?;
    
    let products: Vec<ProductImport> = serde_json::from_value(products_json.clone())?;

    for product in products {
        // Generate random embedding (same logic as server fn)
        let embedding = generate_random_embedding();

        sqlx::query(
            r#"
            INSERT INTO products.items (
                name, description, brand, category, subcategory, tags,
                price, rating, review_count, stock_quantity, in_stock,
                featured, attributes, description_embedding
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14::vector(1536)
            )
            "#,
        )
        .bind(&product.name)
        .bind(&product.description)
        .bind(&product.brand)
        .bind(&product.category)
        .bind(&product.subcategory)
        .bind(&product.tags.unwrap_or_default())
        .bind(product.price)
        .bind(product.rating.unwrap_or(0.0))
        .bind(product.review_count.unwrap_or(0))
        .bind(product.stock_quantity.unwrap_or(0))
        .bind(product.in_stock.unwrap_or(true))
        .bind(product.featured.unwrap_or(false))
        .bind(&product.attributes)
        .bind(&embedding)
        .execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }

    tracing::info!("Database seeded successfully.");
    Ok(())
}

// Helper function for MVP - generates random 1536-dim vector
#[cfg(feature = "ssr")]
fn generate_random_embedding() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();
    format!("[{}]", vec.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

#[cfg(not(feature = "ssr"))]
fn main() {
    panic!("This binary requires the 'ssr' feature. Run with: cargo leptos watch");
}
