-- 13_facet_aggregation_tests.sql
-- Faceted search and aggregation tests using ParadeDB pdb.agg()
-- Enables filtering and analytics on search results
--
-- This test file is IDEMPOTENT and SELF-CONTAINED
-- It sets up its own test data and cleans up after itself
--
-- Usage:
--   psql -h localhost -U postgres -d database -f 13_facet_aggregation_tests.sql

\echo '=============================================='
\echo '=== Facet Aggregation Tests (Self-Contained) ==='
\echo '=============================================='

--------------------------------------------------------------------------------
-- SETUP: Initialize test environment
--------------------------------------------------------------------------------
\echo ''
\echo '--- SETUP: Loading test utilities and data ---'

-- Load the test utilities (creates functions if not exist)
\i test_utils.sql

-- Run setup to create schema and load data
SELECT * FROM test_utils.setup();

\echo ''
\echo '--- SETUP COMPLETE ---'
\echo ''

--------------------------------------------------------------------------------
-- TEST SUITE: Faceted Search and Aggregations
--------------------------------------------------------------------------------

-- Test 1: Value Count - Total Results
\echo 'Test 1: Value Count - Total results for "wireless" search'
SELECT
    id,
    name,
    category,
    price,
    pdb.score(id) AS score,
    pdb.agg('{"value_count": {"field": "id"}}') OVER () AS total_count
FROM test_products.items
WHERE description ||| 'wireless'
ORDER BY score DESC
LIMIT 10;

-- Test 2: Category Facets (Terms Aggregation)
\echo 'Test 2: Category Facets - Count by category for "wireless"'
SELECT
    category,
    COUNT(*) AS count,
    AVG(price) AS avg_price,
    AVG(rating) AS avg_rating
FROM test_products.items
WHERE description ||| 'wireless'
GROUP BY category
ORDER BY count DESC;

-- Test 3: Price Histogram
\echo 'Test 3: Price Histogram - $50 intervals for Electronics'
SELECT
    id,
    name,
    price,
    pdb.agg('{"histogram": {"field": "price", "interval": 50}}') OVER () AS price_histogram
FROM test_products.items
WHERE category = 'Electronics'
ORDER BY pdb.score(id) DESC
LIMIT 10;

-- Test 4: Multiple Aggregations
\echo 'Test 4: Multiple Aggregations - Stats for Electronics category'
SELECT
    pdb.agg('{"avg": {"field": "price"}}') AS avg_price,
    pdb.agg('{"avg": {"field": "rating"}}') AS avg_rating,
    pdb.agg('{"value_count": {"field": "id"}}') AS total_products,
    pdb.agg('{"sum": {"field": "review_count"}}') AS total_reviews
FROM test_products.items
WHERE category = 'Electronics';

-- Test 5: Price Range Facets
\echo 'Test 5: Price Range Facets - Budget, Mid, Premium, Luxury'
SELECT
    CASE
        WHEN price < 25 THEN 'Budget ($0-25)'
        WHEN price < 100 THEN 'Mid-range ($25-100)'
        WHEN price < 500 THEN 'Premium ($100-500)'
        ELSE 'Luxury ($500+)'
    END AS price_range,
    COUNT(*) AS count,
    AVG(rating) AS avg_rating,
    MIN(price) AS min_price,
    MAX(price) AS max_price
FROM test_products.items
WHERE description ||| 'wireless bluetooth'
GROUP BY price_range
ORDER BY MIN(price);

-- Test 6: Rating Distribution
\echo 'Test 6: Rating Distribution - Group by rating buckets'
SELECT
    CASE
        WHEN rating >= 4.8 THEN 'Excellent (4.8-5.0)'
        WHEN rating >= 4.5 THEN 'Very Good (4.5-4.7)'
        WHEN rating >= 4.0 THEN 'Good (4.0-4.4)'
        ELSE 'Average (< 4.0)'
    END AS rating_category,
    COUNT(*) AS count,
    AVG(price) AS avg_price,
    AVG(review_count) AS avg_reviews
FROM test_products.items
WHERE in_stock = true
GROUP BY rating_category
ORDER BY MIN(rating) DESC;

-- Test 7: Brand Facets with Stats
\echo 'Test 7: Brand Facets - Top brands by product count'
SELECT
    brand,
    COUNT(*) AS product_count,
    AVG(price)::DECIMAL(10,2) AS avg_price,
    AVG(rating)::DECIMAL(3,1) AS avg_rating,
    SUM(review_count) AS total_reviews
FROM test_products.items
WHERE description ||| 'wireless'
GROUP BY brand
HAVING COUNT(*) >= 1
ORDER BY product_count DESC
LIMIT 10;

-- Test 8: Subcategory Facets within Category
\echo 'Test 8: Subcategory Facets - Electronics subcategories'
SELECT
    category,
    subcategory,
    COUNT(*) AS count,
    AVG(price)::DECIMAL(10,2) AS avg_price,
    MIN(price) AS min_price,
    MAX(price) AS max_price
FROM test_products.items
WHERE category = 'Electronics'
GROUP BY category, subcategory
ORDER BY count DESC;

-- Test 9: Stock Availability Facets
\echo 'Test 9: Stock Availability - In stock vs Out of stock'
SELECT
    in_stock,
    COUNT(*) AS count,
    AVG(price)::DECIMAL(10,2) AS avg_price,
    SUM(stock_quantity) AS total_stock
FROM test_products.items
WHERE description ||| 'headphones keyboard mouse'
GROUP BY in_stock
ORDER BY in_stock DESC;

-- Test 10: Featured Products Facet
\echo 'Test 10: Featured Status - Featured vs Regular products'
SELECT
    featured,
    COUNT(*) AS count,
    AVG(price)::DECIMAL(10,2) AS avg_price,
    AVG(rating)::DECIMAL(3,1) AS avg_rating
FROM test_products.items
WHERE category = 'Electronics'
GROUP BY featured
ORDER BY featured DESC;

-- Test 11: Review Count Ranges
\echo 'Test 11: Review Count Ranges - Product popularity'
SELECT
    CASE
        WHEN review_count >= 50000 THEN 'Viral (50k+)'
        WHEN review_count >= 10000 THEN 'Very Popular (10k-50k)'
        WHEN review_count >= 1000 THEN 'Popular (1k-10k)'
        ELSE 'New/Niche (< 1k)'
    END AS popularity,
    COUNT(*) AS count,
    AVG(rating)::DECIMAL(3,1) AS avg_rating,
    AVG(price)::DECIMAL(10,2) AS avg_price
FROM test_products.items
GROUP BY popularity
ORDER BY MIN(review_count) DESC;

-- Test 12: Combined Facets - Category + Price Range
\echo 'Test 12: Combined Facets - Category and price range breakdown'
SELECT
    category,
    CASE
        WHEN price < 100 THEN 'Under $100'
        WHEN price < 300 THEN '$100-$300'
        WHEN price < 600 THEN '$300-$600'
        ELSE 'Over $600'
    END AS price_range,
    COUNT(*) AS count,
    AVG(rating)::DECIMAL(3,1) AS avg_rating
FROM test_products.items
WHERE in_stock = true
GROUP BY category, price_range
ORDER BY category, MIN(price);

-- Test 13: Price Statistics by Category
\echo 'Test 13: Price Statistics - Min, Max, Avg, Median by category'
SELECT
    category,
    COUNT(*) AS product_count,
    MIN(price) AS min_price,
    AVG(price)::DECIMAL(10,2) AS avg_price,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY price) AS median_price,
    MAX(price) AS max_price,
    STDDEV(price)::DECIMAL(10,2) AS price_stddev
FROM test_products.items
GROUP BY category
ORDER BY avg_price DESC;

-- Test 14: Attributes Facet (JSONB)
\echo 'Test 14: Color Attribute Facet - Available colors'
SELECT
    attributes->>'color' AS color,
    COUNT(*) AS count,
    AVG(price)::DECIMAL(10,2) AS avg_price
FROM test_products.items
WHERE attributes ? 'color'
  AND category = 'Electronics'
GROUP BY attributes->>'color'
ORDER BY count DESC
LIMIT 10;

-- Test 15: Multi-Dimensional Facet - Category, Brand, Price
\echo 'Test 15: Multi-Dimensional Facet - Category + Brand + Price for wireless'
WITH wireless_products AS (
    SELECT
        id,
        category,
        brand,
        price,
        rating,
        in_stock
    FROM test_products.items
    WHERE description ||| 'wireless'
)
SELECT
    category,
    brand,
    CASE
        WHEN price < 50 THEN 'Budget'
        WHEN price < 150 THEN 'Mid-range'
        ELSE 'Premium'
    END AS price_tier,
    COUNT(*) AS count,
    AVG(rating)::DECIMAL(3,1) AS avg_rating
FROM wireless_products
WHERE in_stock = true
GROUP BY category, brand, price_tier
HAVING COUNT(*) >= 1
ORDER BY category, brand, price_tier;

-- Test 16: Time-Based Aggregation (using created_at)
\echo 'Test 16: Recently Added Products - Count by time period'
SELECT
    CASE
        WHEN created_at > NOW() - INTERVAL '7 days' THEN 'Last Week'
        WHEN created_at > NOW() - INTERVAL '30 days' THEN 'Last Month'
        WHEN created_at > NOW() - INTERVAL '90 days' THEN 'Last Quarter'
        ELSE 'Older'
    END AS time_period,
    COUNT(*) AS count,
    AVG(rating)::DECIMAL(3,1) AS avg_rating
FROM test_products.items
GROUP BY time_period
ORDER BY MIN(created_at) DESC;

-- Test 17: Stock Quantity Ranges
\echo 'Test 17: Stock Quantity Distribution'
SELECT
    CASE
        WHEN stock_quantity = 0 THEN 'Out of Stock'
        WHEN stock_quantity < 50 THEN 'Low Stock (1-49)'
        WHEN stock_quantity < 200 THEN 'Medium Stock (50-199)'
        WHEN stock_quantity < 500 THEN 'High Stock (200-499)'
        ELSE 'Very High Stock (500+)'
    END AS stock_level,
    COUNT(*) AS count,
    SUM(stock_quantity) AS total_units
FROM test_products.items
GROUP BY stock_level
ORDER BY MIN(stock_quantity);

-- Test 18: Faceted Search with Pagination Simulation
\echo 'Test 18: Faceted Search Results - Page 1 with category counts'
WITH search_results AS (
    SELECT
        id,
        name,
        category,
        price,
        rating,
        pdb.score(id) AS score
    FROM test_products.items
    WHERE description ||| 'wireless bluetooth'
    ORDER BY pdb.score(id) DESC
),
category_facets AS (
    SELECT category, COUNT(*) AS count
    FROM search_results
    GROUP BY category
)
SELECT
    sr.id,
    sr.name,
    sr.category,
    sr.price,
    sr.rating,
    sr.score,
    cf.count AS category_total
FROM search_results sr
LEFT JOIN category_facets cf ON sr.category = cf.category
LIMIT 10;

-- Test 19: Advanced Aggregation - Nested Groups
\echo 'Test 19: Nested Aggregation - Category > Subcategory > Price'
SELECT
    category,
    subcategory,
    CASE
        WHEN price < 100 THEN '<$100'
        WHEN price < 300 THEN '$100-$300'
        ELSE '>$300'
    END AS price_bucket,
    COUNT(*) AS count,
    AVG(rating)::DECIMAL(3,1) AS avg_rating,
    SUM(stock_quantity) AS total_stock
FROM test_products.items
WHERE in_stock = true
GROUP BY ROLLUP(category, subcategory, price_bucket)
ORDER BY category NULLS LAST, subcategory NULLS LAST, price_bucket NULLS LAST;

-- Test 20: Aggregation Performance Test
\echo 'Test 20: EXPLAIN ANALYZE - Faceted aggregation performance'
EXPLAIN ANALYZE
SELECT
    category,
    COUNT(*) AS count,
    AVG(price) AS avg_price,
    AVG(rating) AS avg_rating
FROM test_products.items
WHERE description ||| 'wireless'
GROUP BY category
ORDER BY count DESC;

--------------------------------------------------------------------------------
-- TEARDOWN: Clean up test environment
--------------------------------------------------------------------------------
\echo ''
\echo '--- TEARDOWN: Cleaning up test data ---'

SELECT * FROM test_utils.teardown();

\echo ''
\echo '=============================================='
\echo '=== Facet Aggregation Tests Complete ==='
\echo '=============================================='
