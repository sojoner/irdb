-- docker-entrypoint-initdb.d/01-ai-extensions.sql
-- Create extensions for AI/ML workloads
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create schema for AI applications
CREATE SCHEMA IF NOT EXISTS ai_data;

-- Grant permissions on the schema to postgres user (default superuser)
GRANT ALL ON SCHEMA ai_data TO postgres;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA ai_data TO postgres;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ai_data TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA ai_data GRANT ALL ON TABLES TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA ai_data GRANT ALL ON SEQUENCES TO postgres;

-- Create tables for RAG applications
CREATE TABLE ai_data.documents (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    embedding vector(1536), -- OpenAI ada-002 dimension
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE ai_data.chunks (
    id SERIAL PRIMARY KEY,
    document_id INTEGER REFERENCES ai_data.documents(id),
    chunk_text TEXT NOT NULL,
    chunk_index INTEGER,
    embedding vector(1536),
    token_count INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create indexes for vector similarity search
CREATE INDEX ON ai_data.documents USING hnsw (embedding vector_cosine_ops);
CREATE INDEX ON ai_data.chunks USING hnsw (embedding vector_cosine_ops);

-- Create simple text search index (fallback if ParadeDB is not available)
CREATE INDEX idx_documents_title_content ON ai_data.documents USING gin(to_tsvector('english', title || ' ' || content));

-- Create a function to initialize text search indexes
CREATE OR REPLACE FUNCTION ai_data.setup_text_indexes()
RETURNS void AS $$
BEGIN
    -- Create standard PostgreSQL text search indexes
    RAISE NOTICE 'Setting up standard PostgreSQL text search indexes';
END;
$$ LANGUAGE plpgsql;

-- Call the function to setup indexes (this will run during initialization)
SELECT ai_data.setup_text_indexes();

-- Create functions for hybrid search (vector + text)
CREATE OR REPLACE FUNCTION ai_data.hybrid_search(
    query_text TEXT,
    query_embedding vector(1536),
    similarity_threshold FLOAT DEFAULT 0.8,
    limit_count INTEGER DEFAULT 10
)
RETURNS TABLE (
    id INTEGER,
    title TEXT,
    content TEXT,
    vector_similarity FLOAT,
    text_score DOUBLE PRECISION,
    combined_score FLOAT
) AS $$
BEGIN
    RETURN QUERY
    WITH vector_results AS (
        SELECT 
            d.id,
            d.title,
            d.content,
            1 - (d.embedding <=> query_embedding) as vector_similarity
        FROM ai_data.documents d
        WHERE 1 - (d.embedding <=> query_embedding) > similarity_threshold
        ORDER BY d.embedding <=> query_embedding
        LIMIT limit_count * 2
    ),
    text_results AS (
        SELECT 
            d.id,
            d.title, 
            d.content,
            ts_rank(to_tsvector('english', d.title || ' ' || d.content), plainto_tsquery('english', query_text))::DOUBLE PRECISION as text_score
        FROM ai_data.documents d
        WHERE to_tsvector('english', d.title || ' ' || d.content) @@ plainto_tsquery('english', query_text)
        ORDER BY ts_rank(to_tsvector('english', d.title || ' ' || d.content), plainto_tsquery('english', query_text)) DESC
        LIMIT limit_count * 2
    )
    SELECT 
        COALESCE(vr.id, tr.id) as id,
        COALESCE(vr.title, tr.title) as title,
        COALESCE(vr.content, tr.content) as content,
        COALESCE(vr.vector_similarity, 0.0) as vector_similarity,
        COALESCE(tr.text_score, 0.0) as text_score,
        (COALESCE(vr.vector_similarity, 0.0) * 0.7 + COALESCE(tr.text_score, 0.0) * 0.3) as combined_score
    FROM vector_results vr
    FULL OUTER JOIN text_results tr ON vr.id = tr.id
    ORDER BY combined_score DESC
    LIMIT limit_count;
END;
$$ LANGUAGE plpgsql;

-- Create function to generate random vectors for testing
CREATE OR REPLACE FUNCTION ai_data.generate_random_vector(dimensions INTEGER)
RETURNS vector AS $$
DECLARE
    result vector;
BEGIN
    -- Generate a random vector with specified dimensions
    SELECT INTO result
        (SELECT array_agg(random()) FROM generate_series(1, dimensions))::vector;
    RETURN result;
END;
$$ LANGUAGE plpgsql;

-- Test the function to ensure it works correctly
DO $$
BEGIN
    -- Test that we can generate a 1536-dimensional vector
    PERFORM ai_data.generate_random_vector(1536);
    RAISE NOTICE 'Vector generator function successfully tested with 1536 dimensions';
END $$;
