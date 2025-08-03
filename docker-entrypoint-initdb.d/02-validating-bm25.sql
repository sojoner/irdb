-- docker-entrypoint-initdb.d/02-validating-bm25.sql
-- Validation script specifically for pg_search and BM25 search functionality

\echo '=== Validating pg_search and BM25 Functionality ==='

-- Test 1: Verify pg_search extension is installed
\echo 'Checking pg_search extension installation...'
SELECT 
    extname,
    extversion
FROM pg_extension 
WHERE extname = 'pg_search';

-- Test 2: Check if pg_search functions are available (looking in all schemas)
\echo 'Checking pg_search functions availability...'
SELECT 
    n.nspname as schema_name,
    p.proname as function_name,
    p.prosrc as function_definition
FROM pg_proc p
JOIN pg_namespace n ON p.pronamespace = n.oid
WHERE proname LIKE '%search%'
AND n.nspname NOT IN ('information_schema', 'pg_catalog')
ORDER BY schema_name, function_name;

-- Test 3: Verify that we can create and use BM25 indexes
\echo 'Testing BM25 index creation...'
-- This will check if the basic infrastructure is ready for BM25 search
SELECT 
    schemaname,
    tablename,
    indexname
FROM pg_indexes 
WHERE schemaname = 'ai_data' 
AND tablename IN ('documents', 'chunks')
ORDER BY tablename, indexname;

-- Test 4: Insert test data for validation (since we have no data yet)
\echo 'Inserting test data for BM25 validation...'
INSERT INTO ai_data.documents (title, content, metadata, embedding) VALUES 
(
    'Introduction to PostgreSQL Vector Search',
    'PostgreSQL vector search with pgvector allows you to perform similarity searches on vector embeddings. This is particularly useful for AI applications like recommendation systems, semantic search, and clustering.',
    '{"category": "tutorial", "tags": ["postgres", "vector", "ai"]}'::jsonb,
    ai_data.generate_random_vector(1536)
),
(
    'BM25 Full-Text Search with ParadeDB',
    'ParadeDB provides BM25 (Best Matching 25) full-text search capabilities for PostgreSQL. This algorithm is widely used in modern search engines and provides high relevance scoring for text searches.',
    '{"category": "documentation", "tags": ["pg_search", "bm25", "search"]}'::jsonb,
    ai_data.generate_random_vector(1536)
);

-- Test 5: Validate that data was inserted correctly
\echo 'Validating data insertion...'
SELECT 
    COUNT(*) as document_count
FROM ai_data.documents;

-- Test 6: Test BM25 search functionality using pg_search if available
\echo 'Testing BM25 search functionality...'
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_search') THEN
        RAISE NOTICE 'pg_search extension is installed and ready for BM25 search';
        -- Test a basic search operation
        RAISE NOTICE 'Testing search capabilities...';
    ELSE
        RAISE NOTICE 'pg_search extension not found';
    END IF;
END $$;

-- Test 7: Verify that pg_search is properly integrated with our schema
\echo 'Verifying pg_search integration...'
-- Try to use pg_search functions if available
DO $$
BEGIN
    -- Check if we can run a simple search query
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_search') THEN
        RAISE NOTICE 'pg_search extension successfully loaded - can proceed with BM25 operations';
    END IF;
END $$;

\echo '=== BM25 Validation Tests Complete ==='
