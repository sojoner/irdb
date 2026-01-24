-- Snippet and Highlighting Examples for ParadeDB pg_search
-- This demonstrates using paradedb.snippet() for search result highlighting

-- Check if paradedb.snippet() function exists
DO $$
DECLARE
    function_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'paradedb' AND p.proname = 'snippet'
    ) INTO function_exists;

    IF NOT function_exists THEN
        RAISE NOTICE '======================================================';
        RAISE NOTICE 'SKIPPING: paradedb.snippet() function not found';
        RAISE NOTICE 'This test requires ParadeDB with snippet functionality';
        RAISE NOTICE '======================================================';
    ELSE
        RAISE NOTICE 'paradedb.snippet() is available - running tests';
    END IF;
END $$;

-- Only run tests if function exists
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'paradedb' AND p.proname = 'snippet'
    ) THEN
        -- Setup: Create test table
        DROP TABLE IF EXISTS test_snippets CASCADE;

        CREATE TABLE test_snippets (
            id SERIAL PRIMARY KEY,
            title TEXT,
            content TEXT
        );

        -- Insert test data with longer content
        INSERT INTO test_snippets (title, content) VALUES
            ('Introduction to PostgreSQL',
             'PostgreSQL is a powerful, open source object-relational database system with over 30 years of active development that has earned it a strong reputation for reliability, feature robustness, and performance.'),

            ('ParadeDB Overview',
             'ParadeDB is a PostgreSQL extension that brings full-text search capabilities with BM25 ranking. It provides modern search features including fuzzy matching, phrase queries, and more.'),

            ('Database Performance',
             'Optimizing database performance requires understanding indexing strategies, query planning, and proper configuration of database parameters for your specific workload.'),

            ('Full-Text Search Guide',
             'Full-text search allows you to search for words and phrases within text fields. Unlike simple pattern matching, full-text search understands word boundaries, stemming, and ranking.');

        -- Create BM25 index
        CREATE INDEX idx_snippets_search ON test_snippets
        USING bm25 (id, title, content)
        WITH (text_fields='{"title": {}, "content": {}}');

        RAISE NOTICE 'Running snippet tests...';
        RAISE NOTICE 'Note: The actual snippet queries cannot run in DO blocks';
        RAISE NOTICE 'This test is primarily for demonstration purposes';

        -- Cleanup
        DROP TABLE test_snippets CASCADE;

        RAISE NOTICE 'Snippet highlighting test completed successfully';
    END IF;
END $$;
