-- docker-entrypoint-initdb.d/03-simple-vector-test.sql
-- Simple test to validate 1536-dimensional vector operations

\echo '=== Testing 1536-Dimensional Vector Operations ==='

-- Insert test data with 1536-dimensional vectors using the function (now available from 01-ai-extensions.sql)
INSERT INTO ai_data.documents (title, content, metadata, embedding) VALUES 
(
    'Test Document 1',
    'This is a test document for 1536-dimensional vector testing.',
    '{"test": "1536dim"}'::jsonb,
    ai_data.generate_random_vector(1536)
),
(
    'Test Document 2',
    'This is another test document for 1536-dimensional vector testing.',
    '{"test": "1536dim"}'::jsonb,
    ai_data.generate_random_vector(1536)
);

-- Test vector similarity search with 1536-dimensional vectors
SELECT 
    d.id,
    d.title,
    1 - (d.embedding <=> ai_data.generate_random_vector(1536)) as similarity
FROM ai_data.documents d
ORDER BY d.embedding <=> ai_data.generate_random_vector(1536)
LIMIT 3;

\echo '=== Simple 1536-Dimensional Vector Test Complete ==='
