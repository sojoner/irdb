-- 10_bm25_search_tests.sql
-- BM25 full-text search validation tests
-- ParadeDB pg_search v0.20+ operators: ||| (disjunction), &&& (conjunction), === (exact)

\echo '=== BM25 Search Tests ==='

-- Test 1: Match Disjunction (|||) - Match ANY token
\echo 'Test 1: BM25 Disjunction - Search for "wireless headphones"'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'wireless headphones'
ORDER BY pdb.score(id) DESC
LIMIT 10;

-- Test 2: Match Conjunction (&&&) - Match ALL tokens
\echo 'Test 2: BM25 Conjunction - Search for "wireless noise cancellation"'
SELECT
    id,
    name,
    brand,
    price,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description &&& 'wireless noise cancellation'
ORDER BY pdb.score(id) DESC
LIMIT 10;

-- Test 3: Field-Specific Search
\echo 'Test 3: Field-Specific - Search "keyboard" in name field'
SELECT
    id,
    name,
    brand,
    category,
    price,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE name ||| 'keyboard'
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 4: Numeric Range Filter with BM25
\echo 'Test 4: BM25 + Price Filter - "headphones" between $50-$150'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'headphones'
  AND price BETWEEN 50 AND 150
  AND in_stock = true
ORDER BY pdb.score(id) DESC
LIMIT 10;

-- Test 5: Rating Filter with BM25
\echo 'Test 5: BM25 + Rating Filter - "wireless" with rating >= 4.5'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'wireless'
  AND rating >= 4.5
ORDER BY pdb.score(id) DESC
LIMIT 10;

-- Test 6: Category Filter with BM25
\echo 'Test 6: BM25 + Category Filter - "gaming" in Electronics'
SELECT
    id,
    name,
    brand,
    category,
    subcategory,
    price,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'gaming'
  AND category = 'Electronics'
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 7: Brand-Specific Search
\echo 'Test 7: Brand Filter - Search for "Sony" products'
SELECT
    id,
    name,
    brand,
    category,
    price,
    rating,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE brand ||| 'Sony'
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 8: Multi-Field Conjunction
\echo 'Test 8: Complex Query - "professional camera" with filters'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    review_count,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description &&& 'professional camera'
  AND price >= 500
  AND rating >= 4.5
  AND in_stock = true
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 9: Low-Price Budget Search
\echo 'Test 9: Budget Search - Products under $30'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'wireless bluetooth'
  AND price < 30
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 10: High Rating Premium Search
\echo 'Test 10: Premium Search - Rating > 4.8, Price > $200'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    review_count,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'premium professional'
  AND rating > 4.8
  AND price > 200
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 11: Review Count Sorting
\echo 'Test 11: Popular Products - "wireless" sorted by reviews'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    review_count,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'wireless'
  AND review_count > 10000
ORDER BY review_count DESC
LIMIT 5;

-- Test 12: Stock Availability Check
\echo 'Test 12: In-Stock Check - "ergonomic" products available'
SELECT
    id,
    name,
    brand,
    stock_quantity,
    in_stock,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'ergonomic'
  AND in_stock = true
  AND stock_quantity > 0
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 13: Featured Products
\echo 'Test 13: Featured Products - Search "camera"'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    featured,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'camera'
  AND featured = true
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 14: Subcategory Search
\echo 'Test 14: Subcategory Search - "fitness" equipment'
SELECT
    id,
    name,
    brand,
    category,
    subcategory,
    price,
    pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'fitness training'
  AND category = 'Sports'
ORDER BY pdb.score(id) DESC
LIMIT 5;

-- Test 15: Combined Score and Price Sorting
\echo 'Test 15: Score + Price Sort - "office chair"'
SELECT
    id,
    name,
    brand,
    price,
    rating,
    pdb.score(id) AS bm25_score,
    (pdb.score(id) * 0.7 + (5.0 - price / 200.0) * 0.3) AS combined_score
FROM products.items
WHERE description ||| 'office chair ergonomic'
ORDER BY combined_score DESC
LIMIT 5;

-- Performance check: Show query plan for BM25 search
\echo 'Test 16: EXPLAIN ANALYZE - BM25 index usage'
EXPLAIN ANALYZE
SELECT id, name, pdb.score(id) AS score
FROM products.items
WHERE description ||| 'wireless bluetooth'
ORDER BY pdb.score(id) DESC
LIMIT 10;

\echo '=== BM25 Tests Complete ==='
