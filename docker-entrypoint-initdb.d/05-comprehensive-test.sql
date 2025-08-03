-- docker-entrypoint-initdb.d/05-comprehensive-test.sql
-- Comprehensive test to demonstrate both BM25 and vector search functionality

\echo '=== Comprehensive Test of BM25 and Vector Search ==='

\echo 'Testing Extension Installation...'
-- Verify all extensions are installed
SELECT 
    extname,
    extversion
FROM pg_extension 
WHERE extname IN ('vector', 'pg_search', 'pg_trgm');

\echo 'Creating test data for both search types...'

-- Insert sample documents with embeddings for vector search
INSERT INTO ai_data.documents (title, content, metadata, embedding) VALUES 
(
    'PostgreSQL Vector Search Guide',
    'PostgreSQL vector search with pgvector allows you to perform similarity searches on vector embeddings. This is particularly useful for AI applications like recommendation systems, semantic search, and clustering.',
    '{"category": "tutorial", "tags": ["postgres", "vector", "ai"]}'::jsonb,
    ai_data.generate_random_vector(1536)
),
(
    'BM25 Full-Text Search with ParadeDB',
    'ParadeDB provides BM25 (Best Matching 25) full-text search capabilities for PostgreSQL. This algorithm is widely used in modern search engines and provides high relevance scoring for text searches.',
    '{"category": "documentation", "tags": ["pg_search", "bm25", "search"]}'::jsonb,
    ai_data.generate_random_vector(1536)
),
(
    'Hybrid Search Implementation',
    'Combining BM25 text search with vector similarity search creates powerful hybrid search capabilities that leverage the strengths of both approaches for better results.',
    '{"category": "implementation", "tags": ["hybrid", "search", "ai"]}'::jsonb,
    ai_data.generate_random_vector(1536)
);

\echo 'Testing Vector Search Functionality...'

-- Test 1: Simple vector similarity search using the correct 1536-dimensional vector
SELECT 
    d.id,
    d.title,
    1 - (d.embedding <=> ai_data.generate_random_vector(1536)) as similarity
FROM ai_data.documents d
ORDER BY d.embedding <=> ai_data.generate_random_vector(1536)
LIMIT 3;

\echo 'Testing BM25 Search Functionality...'

-- Test 2: Simple BM25 text search using pg_trgm
SELECT 
    id,
    title,
    ts_rank(to_tsvector('english', title || ' ' || content), to_tsquery('english', 'search & postgres')) as score
FROM ai_data.documents 
WHERE to_tsvector('english', title || ' ' || content) @@ to_tsquery('english', 'search & postgres')
ORDER BY score DESC;

\echo 'Testing Hybrid Search Functionality...'

-- Test 3: Hybrid search function (both text and vector)
SELECT 
    id,
    title,
    vector_similarity,
    text_score,
    combined_score
FROM ai_data.hybrid_search(
    query_text => 'PostgreSQL search',
    query_embedding => ai_data.generate_random_vector(1536),
    similarity_threshold => 0.5,
    limit_count => 5
)
ORDER BY combined_score DESC;

\echo '=== Comprehensive Test Complete ==='
