-- 12_hybrid_search_tests.sql
-- Hybrid search combining BM25 (lexical) and vector (semantic) search
-- Two approaches: Weighted Score Combination and Reciprocal Rank Fusion (RRF)

\echo '=== Hybrid Search Tests ==='

-- Ensure test embeddings exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'pg_temp' AND tablename LIKE 'test_embeddings%') THEN
        CREATE TEMP TABLE test_embeddings (
            query_name TEXT PRIMARY KEY,
            embedding vector(1536)
        );
        INSERT INTO test_embeddings (query_name, embedding) VALUES
        ('wireless_headphones', products.generate_random_embedding(1536)),
        ('gaming_setup', products.generate_random_embedding(1536)),
        ('professional_photography', products.generate_random_embedding(1536)),
        ('home_office', products.generate_random_embedding(1536)),
        ('fitness_gear', products.generate_random_embedding(1536));
    END IF;
END $$;

-- Test 1: Weighted Score Combination (70% Vector + 30% BM25)
\echo 'Test 1: Hybrid Weighted - "wireless headphones" (70% vector, 30% BM25)'
WITH bm25_results AS (
    SELECT
        id,
        pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| 'wireless headphones'
    ORDER BY pdb.score(id) DESC
    LIMIT 50
),
vector_results AS (
    SELECT
        id,
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')) AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.price,
    p.rating,
    COALESCE(b.bm25_score, 0) AS bm25_score,
    COALESCE(v.vector_score, 0) AS vector_score,
    (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 10;

-- Test 2: Reciprocal Rank Fusion (RRF)
\echo 'Test 2: Hybrid RRF - "gaming peripherals" (k=60)'
WITH bm25_ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
    FROM products.items
    WHERE description ||| 'gaming peripherals mouse keyboard'
    LIMIT 50
),
vector_ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_setup')) AS rank
    FROM products.items
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.category,
    p.price,
    b.rank AS bm25_rank,
    v.rank AS vector_rank,
    -- RRF formula: 1/(k + rank), k=60 is standard
    (1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000))) AS rrf_score
FROM bm25_ranked b
FULL OUTER JOIN vector_ranked v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY rrf_score DESC
LIMIT 10;

-- Test 3: Hybrid Search with Price Filter
\echo 'Test 3: Hybrid Weighted + Price Filter - "professional camera" under $1000'
WITH bm25_results AS (
    SELECT
        id,
        pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| 'professional camera photography'
      AND price < 1000
    ORDER BY pdb.score(id) DESC
    LIMIT 30
),
vector_results AS (
    SELECT
        id,
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_photography')) AS vector_score
    FROM products.items
    WHERE price < 1000
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'professional_photography')
    LIMIT 30
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.price,
    p.rating,
    COALESCE(b.bm25_score, 0) AS bm25_score,
    COALESCE(v.vector_score, 0) AS vector_score,
    (COALESCE(b.bm25_score, 0) * 0.4 + COALESCE(v.vector_score, 0) * 0.6) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 5;

-- Test 4: Hybrid Search with Category and Rating Filters
\echo 'Test 4: Hybrid RRF + Filters - "office ergonomic" in Home & Garden, rating >= 4.5'
WITH bm25_ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
    FROM products.items
    WHERE description ||| 'office ergonomic comfortable'
      AND category = 'Home & Garden'
      AND rating >= 4.5
    LIMIT 30
),
vector_ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'home_office')) AS rank
    FROM products.items
    WHERE category = 'Home & Garden'
      AND rating >= 4.5
    LIMIT 30
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.price,
    p.rating,
    b.rank AS bm25_rank,
    v.rank AS vector_rank,
    (1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000))) AS rrf_score
FROM bm25_ranked b
FULL OUTER JOIN vector_ranked v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY rrf_score DESC
LIMIT 5;

-- Test 5: Adjustable Weight Hybrid Search (50-50 split)
\echo 'Test 5: Hybrid Balanced - "fitness training" (50% vector, 50% BM25)'
WITH bm25_results AS (
    SELECT
        id,
        pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| 'fitness training workout exercise'
    ORDER BY pdb.score(id) DESC
    LIMIT 40
),
vector_results AS (
    SELECT
        id,
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'fitness_gear')) AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'fitness_gear')
    LIMIT 40
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.category,
    p.price,
    COALESCE(b.bm25_score, 0) AS bm25_score,
    COALESCE(v.vector_score, 0) AS vector_score,
    (COALESCE(b.bm25_score, 0) * 0.5 + COALESCE(v.vector_score, 0) * 0.5) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 10;

-- Test 6: Hybrid Search with Stock Filter
\echo 'Test 6: Hybrid Weighted - "wireless bluetooth" in stock only'
WITH bm25_results AS (
    SELECT
        id,
        pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| 'wireless bluetooth'
      AND in_stock = true
      AND stock_quantity > 0
    ORDER BY pdb.score(id) DESC
    LIMIT 50
),
vector_results AS (
    SELECT
        id,
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')) AS vector_score
    FROM products.items
    WHERE in_stock = true
      AND stock_quantity > 0
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.stock_quantity,
    p.price,
    COALESCE(b.bm25_score, 0) AS bm25_score,
    COALESCE(v.vector_score, 0) AS vector_score,
    (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 10;

-- Test 7: Hybrid with Multi-Field BM25
\echo 'Test 7: Hybrid Multi-Field - Search name and description'
WITH bm25_results AS (
    SELECT
        id,
        pdb.score(id) AS bm25_score
    FROM products.items
    WHERE (name ||| 'wireless headphones' OR description ||| 'wireless headphones')
    ORDER BY pdb.score(id) DESC
    LIMIT 50
),
vector_results AS (
    SELECT
        id,
        1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')) AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.brand,
    p.price,
    p.rating,
    COALESCE(b.bm25_score, 0) AS bm25_score,
    COALESCE(v.vector_score, 0) AS vector_score,
    (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 10;

-- Test 8: RRF with Different K Values
\echo 'Test 8: Hybrid RRF Comparison - k=30 vs k=60 for "gaming"'
WITH bm25_ranked AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
    FROM products.items
    WHERE description ||| 'gaming professional esports'
    LIMIT 30
),
vector_ranked AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'gaming_setup')) AS rank
    FROM products.items
    LIMIT 30
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.price,
    (1.0 / (30 + COALESCE(b.rank, 1000)) + 1.0 / (30 + COALESCE(v.rank, 1000))) AS rrf_k30,
    (1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000))) AS rrf_k60
FROM bm25_ranked b
FULL OUTER JOIN vector_ranked v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY rrf_k60 DESC
LIMIT 10;

-- Test 9: Score Distribution Analysis
\echo 'Test 9: Hybrid Score Distribution - Analyze BM25 vs Vector contribution'
WITH bm25_results AS (
    SELECT id, pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| 'wireless bluetooth'
    ORDER BY pdb.score(id) DESC
    LIMIT 50
),
vector_results AS (
    SELECT id, 1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')) AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')
    LIMIT 50
),
combined AS (
    SELECT
        COALESCE(b.id, v.id) AS id,
        COALESCE(b.bm25_score, 0) AS bm25_score,
        COALESCE(v.vector_score, 0) AS vector_score,
        (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
    FROM bm25_results b
    FULL OUTER JOIN vector_results v ON b.id = v.id
)
SELECT
    COUNT(*) AS total_results,
    AVG(bm25_score) AS avg_bm25,
    AVG(vector_score) AS avg_vector,
    AVG(combined_score) AS avg_combined,
    MAX(bm25_score) AS max_bm25,
    MAX(vector_score) AS max_vector,
    MAX(combined_score) AS max_combined
FROM combined;

-- Test 10: Performance Comparison
\echo 'Test 10: EXPLAIN ANALYZE - Hybrid search performance'
EXPLAIN ANALYZE
WITH bm25_results AS (
    SELECT id, pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| 'wireless headphones'
    ORDER BY pdb.score(id) DESC
    LIMIT 50
),
vector_results AS (
    SELECT id, 1 - (description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')) AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> (SELECT embedding FROM test_embeddings WHERE query_name = 'wireless_headphones')
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 10;

\echo '=== Hybrid Search Tests Complete ==='
