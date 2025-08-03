-- Snippet and Highlighting Examples for ParadeDB pg_search v0.17.2
-- This demonstrates using paradedb.snippet() for search result highlighting

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
CREATE INDEX test_snippets_idx ON test_snippets
USING bm25 (id, title, content)
WITH (
    key_field='id',
    text_fields='{"title": {}, "content": {}}'
);

-- Example 1: Basic snippet extraction
-- Default highlighting uses <b> tags
SELECT
    id,
    title,
    paradedb.snippet(content) as snippet
FROM test_snippets
WHERE test_snippets @@@ 'content:database'
ORDER BY id;
-- Expected: Returns snippets with "database" highlighted as <b>database</b>

-- Example 2: Snippet for title field
SELECT
    id,
    paradedb.snippet(title) as title_snippet,
    paradedb.snippet(content) as content_snippet
FROM test_snippets
WHERE test_snippets @@@ 'PostgreSQL'
ORDER BY id;
-- Expected: Shows highlighted matches in both title and content

-- Example 3: Search with multiple terms
SELECT
    id,
    title,
    paradedb.snippet(content) as snippet
FROM test_snippets
WHERE test_snippets @@@ 'content:search OR content:performance'
ORDER BY id;
-- Expected: Multiple words highlighted in results

-- Example 4: Phrase search with snippet
SELECT
    id,
    title,
    paradedb.snippet(content) as snippet
FROM test_snippets
WHERE test_snippets @@@ paradedb.phrase('content', ARRAY['full', 'text', 'search'])
ORDER BY id;
-- Expected: Entire phrase highlighted

-- Example 5: Fuzzy search with snippet
SELECT
    id,
    title,
    paradedb.snippet(content) as snippet
FROM test_snippets
WHERE test_snippets @@@ paradedb.fuzzy_term('content', 'databse')
ORDER BY id;
-- Expected: Shows matched term even with typo (if fuzzy matching works)

-- Example 6: Combining snippet with score/ranking
-- Note: BM25 provides relevance scoring
SELECT
    id,
    title,
    paradedb.snippet(content) as snippet
FROM test_snippets
WHERE test_snippets @@@ 'content:database OR content:search'
ORDER BY id
LIMIT 5;

-- Example 7: Multiple field search with snippets
SELECT
    id,
    paradedb.snippet(title) as title_snippet,
    paradedb.snippet(content) as content_snippet
FROM test_snippets
WHERE test_snippets @@@ 'title:database OR content:database'
ORDER BY id;

-- Note: The default snippet highlighting format is:
-- <b>matched_term</b>
--
-- In a real application, you might want to:
-- 1. Replace <b> tags with your preferred HTML/styling
-- 2. Truncate long snippets to show context around matches
-- 3. Handle multiple matches in the same field

-- Cleanup
DROP TABLE test_snippets CASCADE;
