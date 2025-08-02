-- docker-entrypoint-initdb.d/01-ai-extensions.sql
-- Create extensions for AI/ML workloads
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;

-- Create schema for AI applications
CREATE SCHEMA IF NOT EXISTS ai_data;

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

-- Create BM25 search indexes with ParadeDB
CALL paradedb.create_bm25_index(
    index_name => 'documents_search_idx',
    schema_name => 'ai_data', 
    table_name => 'documents',
    key_field => 'id',
    text_fields => '{
        "title": {"weight": 3.0},
        "content": {"weight": 1.0}
    }'
);

CALL paradedb.create_bm25_index(
    index_name => 'chunks_search_idx',
    schema_name => 'ai_data',
    table_name => 'chunks', 
    key_field => 'id',
    text_fields => '{
        "chunk_text": {"weight": 1.0}
    }'
);

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
    text_score FLOAT,
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
            paradedb.score(d.id) as text_score
        FROM ai_data.documents d
        WHERE d.id @@@ query_text
        ORDER BY paradedb.score(d.id) DESC
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
