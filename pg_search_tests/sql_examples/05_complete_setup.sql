-- Complete BM25 Index Setup for ParadeDB pg_search v0.17.2
-- This shows the complete workflow for creating a searchable table

-- Step 1: Drop existing table if it exists
DROP TABLE IF EXISTS products CASCADE;

-- Step 2: Create table structure
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    price DECIMAL(10, 2),
    rating DECIMAL(3, 2),
    in_stock BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Step 3: Insert data BEFORE creating the BM25 index
-- This is CRITICAL - data must exist before indexing
INSERT INTO products (name, description, category, price, rating, in_stock) VALUES
    -- Electronics
    ('Wireless Headphones', 'High-quality wireless headphones with noise cancellation and 30-hour battery life', 'Electronics', 79.99, 4.5, true),
    ('USB-C Cable', 'Fast charging USB-C cable, durable braided design, compatible with all devices', 'Accessories', 12.99, 4.7, true),
    ('Mechanical Keyboard', 'Mechanical keyboard with RGB lighting and customizable keys', 'Electronics', 89.99, 4.9, true),
    ('Gaming Mouse Pro', 'Professional gaming mouse with high DPI sensor and programmable buttons', 'Electronics', 59.99, 4.8, true),
    ('Standard Mouse', 'Basic optical mouse, good for office work and casual pro gamers', 'Electronics', 19.99, 4.2, true),

    -- Furniture
    ('Ergonomic Office Chair', 'Comfortable office chair with lumbar support and adjustable height', 'Furniture', 199.99, 4.6, true),
    ('Gaming Chair', 'Racing style gaming chair with reclining backrest', 'Furniture', 159.99, 4.4, true),

    -- Other
    ('Wi-Fi 6 Router', 'Next-gen Wi-Fi 6 router for high-speed internet connectivity', 'Networking', 129.99, 4.7, true),
    ('Blue T-Shirt', '100% cotton blue t-shirt, comfortable fit', 'Clothing', 14.99, 4.3, true),
    ('Red T-Shirt', '100% cotton red t-shirt, comfortable fit', 'Clothing', 14.99, 4.3, true);

-- Step 4: Create BM25 index with proper field configuration
CREATE INDEX products_search_idx ON products
USING bm25 (id, name, description, category, price, rating, in_stock)
WITH (
    key_field='id',
    text_fields='{"name": {}, "description": {}, "category": {}}',
    numeric_fields='{"price": {}, "rating": {}}',
    boolean_fields='{"in_stock": {}}'
);

-- Now you can search!

-- Text search examples
SELECT id, name, price FROM products
WHERE products @@@ 'description:wireless'
ORDER BY id;

SELECT id, name, price FROM products
WHERE products @@@ 'name:keyboard'
ORDER BY id;

SELECT id, name, category FROM products
WHERE products @@@ 'category:Electronics'
ORDER BY id;

-- Numeric range search examples
SELECT id, name, price::float8 FROM products
WHERE products @@@ 'price:[10 TO 20]'
ORDER BY price;

SELECT id, name, rating::float8 FROM products
WHERE products @@@ 'rating:>4.5'
ORDER BY rating DESC;

-- Boolean search examples
SELECT id, name, in_stock FROM products
WHERE products @@@ 'in_stock:true'
ORDER BY id;

-- Fuzzy search examples
SELECT id, name FROM products
WHERE products @@@ paradedb.fuzzy_term('description', 'wireles')
ORDER BY id;
-- Note: fuzzy_term in v0.17.2 works best with exact or near-exact matches

-- Phrase search examples
SELECT id, name FROM products
WHERE products @@@ paradedb.phrase('description', ARRAY['noise', 'cancellation'])
ORDER BY id;

-- Combined boolean search
SELECT id, name, price::float8 FROM products
WHERE products @@@ 'category:Electronics AND price:[50 TO 100]'
ORDER BY price;

-- Snippet extraction (for highlighting)
SELECT id, name, paradedb.snippet(description)
FROM products
WHERE products @@@ 'description:wireless'
LIMIT 5;

-- Don't forget to clean up when testing
-- DROP TABLE products CASCADE;
