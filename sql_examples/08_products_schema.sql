-- 08_products_schema.sql
-- Products schema and indexes for hybrid search testing
-- Combines BM25 (ParadeDB pg_search v0.20.2) and vector similarity (pgvector v0.8.0)

-- Create schema
CREATE SCHEMA IF NOT EXISTS products;

-- Create products table
CREATE TABLE products.items (
    -- Primary key (BM25 key_field)
    id SERIAL PRIMARY KEY,

    -- Text fields (BM25 searchable)
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    brand TEXT NOT NULL,
    category TEXT NOT NULL,           -- e.g., "Electronics", "Clothing"
    subcategory TEXT,                 -- e.g., "Headphones", "T-Shirts"
    tags TEXT[],                      -- e.g., ARRAY['wireless', 'bluetooth']

    -- Numeric fields (filterable, facetable)
    price DECIMAL(10, 2) NOT NULL,
    rating DECIMAL(2, 1) DEFAULT 0.0, -- 0.0 to 5.0
    review_count INTEGER DEFAULT 0,
    stock_quantity INTEGER DEFAULT 0,

    -- Boolean fields
    in_stock BOOLEAN DEFAULT true,
    featured BOOLEAN DEFAULT false,

    -- JSON metadata (for flexible attributes)
    attributes JSONB,                 -- e.g., {"color": "black", "size": "M"}

    -- Vector embedding (semantic search)
    description_embedding vector(1536), -- OpenAI ada-002 compatible

    -- Timestamps
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Create BM25 index (ParadeDB pg_search)
-- Using v0.20+ syntax with operators ||| (disjunction) and &&& (conjunction)
CREATE INDEX products_bm25_idx ON products.items
USING bm25 (
    id,
    name,
    description,
    brand,
    category,
    subcategory,
    price,
    rating,
    review_count,
    in_stock
)
WITH (key_field = 'id');

-- Create vector index (pgvector HNSW)
-- Using cosine distance for normalized embeddings
CREATE INDEX products_vector_idx ON products.items
USING hnsw (description_embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Create supporting B-tree indexes for filtering
CREATE INDEX idx_products_category ON products.items (category);
CREATE INDEX idx_products_price ON products.items (price);
CREATE INDEX idx_products_rating ON products.items (rating);
CREATE INDEX idx_products_in_stock ON products.items (in_stock);

-- Create GIN indexes for array/JSONB
CREATE INDEX idx_products_tags ON products.items USING gin (tags);
CREATE INDEX idx_products_attributes ON products.items USING gin (attributes);

-- Grant permissions
GRANT USAGE ON SCHEMA products TO PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA products TO PUBLIC;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA products TO PUBLIC;

-- Log completion
DO $$
BEGIN
    RAISE NOTICE 'Products schema created successfully';
    RAISE NOTICE 'Indexes: BM25 (products_bm25_idx), Vector (products_vector_idx)';
END $$;
