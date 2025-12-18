-- 11_vector_search_tests.sql
-- Vector similarity search validation tests
-- pgvector v0.8.0 operators: <=> (cosine), <-> (L2), <#> (inner product)
--
-- This test file is IDEMPOTENT and SELF-CONTAINED
-- It sets up its own test data and cleans up after itself
--
-- Usage:
--   psql -h localhost -U postgres -d database -f 11_vector_search_tests.sql

\echo '=============================================='
\echo '=== Vector Search Tests (Self-Contained) ==='
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

-- Create test query embeddings
SELECT * FROM test_utils.create_test_embeddings();

\echo ''
\echo '--- SETUP COMPLETE ---'
\echo ''

--------------------------------------------------------------------------------
-- TEST SUITE: Vector Similarity Search
--------------------------------------------------------------------------------

-- Test 1: Basic Vector Similarity Search (Cosine Distance)
\echo 'Test 1: Vector Search - Find similar to "wireless_audio" query'
SELECT
    id,
    name,
    brand,
    category,
    price,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS cosine_similarity
FROM test_products.items
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
LIMIT 10;

-- Test 2: Vector Search with Similarity Threshold
\echo 'Test 2: Vector Search with Threshold - Similarity > 0.5'
SELECT
    id,
    name,
    brand,
    category,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
FROM test_products.items
WHERE 1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) > 0.5
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
LIMIT 10;

-- Test 3: Vector Search with Price Filter
\echo 'Test 3: Vector Search + Price Filter - Gaming products under $200'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals')) AS similarity
FROM test_products.items
WHERE price < 200
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals')
LIMIT 10;

-- Test 4: Vector Search with Category Filter
\echo 'Test 4: Vector Search + Category - Professional cameras in Electronics'
SELECT
    id,
    name,
    brand,
    category,
    price,
    rating,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_camera')) AS similarity
FROM test_products.items
WHERE category = 'Electronics'
  AND subcategory = 'Cameras'
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_camera')
LIMIT 5;

-- Test 5: Vector Search with Multiple Filters
\echo 'Test 5: Vector Search + Multiple Filters - Office furniture, premium, in stock'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    in_stock,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'office_furniture')) AS similarity
FROM test_products.items
WHERE category = 'Home & Garden'
  AND subcategory = 'Furniture'
  AND in_stock = true
  AND rating >= 4.5
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'office_furniture')
LIMIT 5;

-- Test 6: L2 (Euclidean) Distance Search
\echo 'Test 6: Vector Search with L2 Distance - Outdoor equipment'
SELECT
    id,
    name,
    brand,
    category,
    description_embedding <-> (SELECT embedding FROM test_embeddings WHERE query_name = 'outdoor_equipment') AS l2_distance
FROM test_products.items
ORDER BY description_embedding <-> (SELECT embedding FROM test_embeddings WHERE query_name = 'outdoor_equipment')
LIMIT 10;

-- Test 7: Inner Product Search
\echo 'Test 7: Vector Search with Inner Product - Gaming peripherals'
SELECT
    id,
    name,
    brand,
    price,
    description_embedding <#> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals') AS inner_product
FROM test_products.items
ORDER BY description_embedding <#> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_peripherals')
LIMIT 10;

-- Test 8: Vector Search with Rating Filter
\echo 'Test 8: Vector Search + High Rating - Similarity search for highly-rated products'
SELECT
    id,
    name,
    brand,
    rating,
    review_count,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
FROM test_products.items
WHERE rating >= 4.7
  AND review_count > 1000
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
LIMIT 5;

-- Test 9: Vector Search by Price Range
\echo 'Test 9: Vector Search in Price Range - $100-$500'
SELECT
    id,
    name,
    brand,
    price,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_camera')) AS similarity
FROM test_products.items
WHERE price BETWEEN 100 AND 500
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_camera')
LIMIT 10;

-- Test 10: Vector Search for Featured Products
\echo 'Test 10: Vector Search - Featured products only'
SELECT
    id,
    name,
    brand,
    category,
    featured,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
FROM test_products.items
WHERE featured = true
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
LIMIT 10;

-- Test 11: Vector Search with Stock Quantity
\echo 'Test 11: Vector Search - High stock availability (> 100 units)'
SELECT
    id,
    name,
    brand,
    stock_quantity,
    1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
FROM test_products.items
WHERE stock_quantity > 100
  AND in_stock = true
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
LIMIT 5;

-- Test 12: Cross-Category Vector Search
\echo 'Test 12: Cross-Category Vector Search - Find similar across all categories'
WITH category_results AS (
    SELECT
        id,
        name,
        brand,
        category,
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
    FROM test_products.items
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
    LIMIT 20
)
SELECT category, COUNT(*) as count, AVG(similarity) as avg_similarity
FROM category_results
GROUP BY category
ORDER BY count DESC;

-- Test 13: Vector Search Statistics
\echo 'Test 13: Vector Search Statistics - Distribution of similarity scores'
WITH similarities AS (
    SELECT
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
    FROM test_products.items
)
SELECT
    MIN(similarity) as min_similarity,
    AVG(similarity) as avg_similarity,
    MAX(similarity) as max_similarity,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY similarity) as median_similarity,
    STDDEV(similarity) as stddev_similarity
FROM similarities;

-- Test 14: Performance check - Show query plan for vector search
\echo 'Test 14: EXPLAIN ANALYZE - HNSW index usage'
EXPLAIN ANALYZE
SELECT id, name,
       1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')) AS similarity
FROM test_products.items
ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_audio')
LIMIT 10;

-- Test 15: Vector Search Index Statistics
\echo 'Test 15: Vector Index Statistics'
SELECT
    schemaname,
    relname as tablename,
    indexrelname as indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
WHERE relname = 'items' AND schemaname = 'test_products'
  AND indexrelname = 'test_products_vector_idx';

--------------------------------------------------------------------------------
-- TEARDOWN: Clean up test environment
--------------------------------------------------------------------------------
\echo ''
\echo '--- TEARDOWN: Cleaning up test data ---'

SELECT * FROM test_utils.teardown();

\echo ''
\echo '=============================================='
\echo '=== Vector Search Tests Complete ==='
\echo '=============================================='
