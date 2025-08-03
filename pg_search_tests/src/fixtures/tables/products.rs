// fixtures/tables/products.rs
//
// What is this file?
// This defines a ProductsTable fixture with realistic e-commerce product data.
//
// Why do we need this?
// - Tests often need consistent test data
// - Instead of creating different data in each test, we define it once here
// - We can reuse this across many tests
//
// The philosophy:
// "Setup your test data once, use it everywhere"

use crate::fixtures::TestTable;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A representation of a product in our test table
/// This struct helps Rust understand what data structure we're getting from SQL
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub category: String,
    pub price: f64,
    pub rating: f64,
    pub in_stock: bool,
}

pub struct ProductsTable;

impl TestTable for ProductsTable {
    fn setup_sql() -> &'static [&'static str] {
        &[
            // 1. Create Table
            r#"
            CREATE TABLE IF NOT EXISTS products (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                description TEXT NOT NULL,
                category VARCHAR(100) NOT NULL,
                price DECIMAL(10, 2) NOT NULL,
                rating DECIMAL(3, 2),
                in_stock BOOLEAN DEFAULT true,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            
            // 2. Create BM25 Index
            r#"
            CREATE INDEX IF NOT EXISTS products_search_idx ON products
            USING bm25 (id, name, description, category, price, rating, in_stock)
            WITH (
                key_field='id',
                text_fields='{"name": {}, "description": {}, "category": {}}',
                numeric_fields='{"price": {}, "rating": {}}',
                boolean_fields='{"in_stock": {}}'
            )
            "#,

            // 3. Insert Data
            r#"
            INSERT INTO products (name, description, category, price, rating, in_stock)
            VALUES
                -- Electronics & Gadgets
                (
                    'Wireless Headphones',
                    'High-quality wireless headphones with noise cancellation and 30-hour battery life',
                    'Electronics',
                    79.99,
                    4.5,
                    true
                ),
                (
                    'USB-C Cable',
                    'Fast charging USB-C cable, durable braided design, compatible with all devices',
                    'Accessories',
                    12.99,
                    4.7,
                    true
                ),
                (
                    'Mechanical Keyboard',
                    'Mechanical keyboard with RGB lighting and customizable keys',
                    'Electronics',
                    89.99,
                    4.9,
                    true
                ),
                
                -- Ranking Test Items (Pro Series)
                (
                    'Gaming Mouse Pro',
                    'Professional gaming mouse with high DPI sensor and programmable buttons',
                    'Electronics',
                    59.99,
                    4.8,
                    true
                ),
                (
                    'Standard Mouse',
                    'Basic optical mouse, good for office work and casual pro gamers',
                    'Electronics',
                    19.99,
                    4.2,
                    true
                ),
                
                -- Category Test Items (Furniture)
                (
                    'Ergonomic Office Chair',
                    'Comfortable office chair with lumbar support and adjustable height',
                    'Furniture',
                    199.99,
                    4.6,
                    true
                ),
                (
                    'Gaming Chair',
                    'Racing style gaming chair with reclining backrest',
                    'Furniture',
                    159.99,
                    4.4,
                    true
                ),
                
                -- Special Characters & Edge Cases
                (
                    'Wi-Fi 6 Router',
                    'Next-gen Wi-Fi 6 router for high-speed internet connectivity',
                    'Networking',
                    129.99,
                    4.7,
                    true
                ),
                (
                    'Blue T-Shirt',
                    '100% cotton blue t-shirt, comfortable fit',
                    'Clothing',
                    14.99,
                    4.3,
                    true
                ),
                (
                    'Red T-Shirt',
                    '100% cotton red t-shirt, comfortable fit',
                    'Clothing',
                    14.99,
                    4.3,
                    true
                )
            "#
        ]
    }
}

#[cfg(test)]
mod tests {

    // Note: To run these integration tests, you'll need a real PostgreSQL database
    // For now, we'll keep them commented out until we set up proper test infrastructure

    // #[tokio::test]
    // async fn test_products_table_setup() {
    //     // This test would verify that ProductsTable::setup() works correctly
    //     // It would:
    //     // 1. Connect to a test database
    //     // 2. Call ProductsTable::setup()
    //     // 3. Verify the table was created
    //     // 4. Verify the data was inserted
    // }
}
