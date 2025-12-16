# Hybrid Search Products Database Specification

## Overview

This specification defines a **products database** that demonstrates hybrid search combining:
- **BM25 full-text search** (ParadeDB pg_search v0.20.2) for lexical/keyword matching
- **Vector similarity search** (pgvector v0.8.0) for semantic matching
- **Faceted aggregations** for filtering and analytics

The goal is to create a hands-on validation suite using **pure SQL** and **Rust tests** that showcases real-world e-commerce search patterns.

---

## 1. Database Schema

### 1.1 Products Table

```sql
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
```

### 1.2 Schema Setup

```sql
CREATE SCHEMA IF NOT EXISTS products;
```

---

## 2. Index Strategy

### 2.1 BM25 Index (ParadeDB pg_search)

Using the new v0.20+ syntax with operators `|||` (disjunction) and `&&&` (conjunction):

```sql
CREATE INDEX products_bm25_idx ON products.items
USING bm25 (
    id,
    name,
    description,
    brand,
    (category::pdb.literal),      -- Literal tokenizer for exact category matches
    (subcategory::pdb.literal),
    price,
    rating,
    review_count,
    in_stock
)
WITH (key_field = 'id');
```

**Key points:**
- `category` and `subcategory` use `pdb.literal` tokenizer for exact filtering
- Numeric fields (`price`, `rating`) enable range queries and filter pushdown
- `key_field` must be the PRIMARY KEY with UNIQUE constraint

### 2.2 Vector Index (pgvector HNSW)

```sql
CREATE INDEX products_vector_idx ON products.items
USING hnsw (description_embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);
```

**Operators:**
- `<=>` - Cosine distance (best for normalized embeddings)
- `<->` - L2/Euclidean distance
- `<#>` - Inner product (negative, for max similarity)

### 2.3 Supporting Indexes

```sql
-- B-tree for filtering
CREATE INDEX idx_products_category ON products.items (category);
CREATE INDEX idx_products_price ON products.items (price);
CREATE INDEX idx_products_rating ON products.items (rating);

-- GIN for array/JSONB
CREATE INDEX idx_products_tags ON products.items USING gin (tags);
CREATE INDEX idx_products_attributes ON products.items USING gin (attributes);
```

---

## 3. Search Patterns

### 3.1 BM25 Full-Text Search

**Match Disjunction (`|||`)** - Returns documents containing ANY token:

```sql
SELECT id, name, description, pdb.score(id) AS bm25_score
FROM products.items
WHERE description ||| 'wireless bluetooth headphones'
ORDER BY pdb.score(id) DESC
LIMIT 10;
```

**Match Conjunction (`&&&`)** - Returns documents containing ALL tokens:

```sql
SELECT id, name, description, pdb.score(id) AS bm25_score
FROM products.items
WHERE description &&& 'wireless noise cancellation'
ORDER BY pdb.score(id) DESC
LIMIT 10;
```

**Field-Specific Search:**

```sql
SELECT id, name, pdb.score(id) AS score
FROM products.items
WHERE name ||| 'keyboard' AND category = 'Electronics'
ORDER BY score DESC;
```

### 3.2 Vector Similarity Search

```sql
-- Using cosine similarity (1 - distance)
SELECT
    id,
    name,
    description,
    1 - (description_embedding <=> $query_embedding) AS similarity
FROM products.items
WHERE 1 - (description_embedding <=> $query_embedding) > 0.7
ORDER BY description_embedding <=> $query_embedding
LIMIT 10;
```

**Important:** For index usage, ORDER BY must use the distance operator directly:
```sql
-- Uses index
ORDER BY embedding <=> '[...]' LIMIT 10;

-- Does NOT use index
ORDER BY 1 - (embedding <=> '[...]') DESC LIMIT 10;
```

### 3.3 Hybrid Search (BM25 + Vector)

**Approach 1: Weighted Score Combination**

```sql
WITH bm25_results AS (
    SELECT id, pdb.score(id) AS bm25_score
    FROM products.items
    WHERE description ||| $query_text
    ORDER BY pdb.score(id) DESC
    LIMIT 50
),
vector_results AS (
    SELECT id, 1 - (description_embedding <=> $query_embedding) AS vector_score
    FROM products.items
    ORDER BY description_embedding <=> $query_embedding
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    p.description,
    COALESCE(b.bm25_score, 0) AS bm25_score,
    COALESCE(v.vector_score, 0) AS vector_score,
    (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7) AS combined_score
FROM bm25_results b
FULL OUTER JOIN vector_results v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY combined_score DESC
LIMIT 10;
```

**Approach 2: Reciprocal Rank Fusion (RRF)**

```sql
WITH bm25_ranked AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY pdb.score(id) DESC) AS rank
    FROM products.items
    WHERE description ||| $query_text
    LIMIT 50
),
vector_ranked AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY description_embedding <=> $query_embedding) AS rank
    FROM products.items
    LIMIT 50
)
SELECT
    COALESCE(b.id, v.id) AS id,
    p.name,
    -- RRF formula: 1/(k + rank), k=60 is standard
    1.0 / (60 + COALESCE(b.rank, 1000)) + 1.0 / (60 + COALESCE(v.rank, 1000)) AS rrf_score
FROM bm25_ranked b
FULL OUTER JOIN vector_ranked v ON b.id = v.id
JOIN products.items p ON p.id = COALESCE(b.id, v.id)
ORDER BY rrf_score DESC
LIMIT 10;
```

---

## 4. Filtering and Facets

### 4.1 Filter Pushdown (BM25)

Numeric and boolean filters can be pushed into the BM25 index scan:

```sql
SELECT id, name, price, rating
FROM products.items
WHERE description ||| 'headphones'
  AND price BETWEEN 50 AND 150
  AND rating >= 4.0
  AND in_stock = true
ORDER BY pdb.score(id) DESC
LIMIT 10;
```

### 4.2 Faceted Search with Aggregations

**Single faceted query returning results + total count:**

```sql
SELECT
    id,
    name,
    price,
    pdb.score(id) AS score,
    pdb.agg('{"value_count": {"field": "id"}}') OVER () AS total_count
FROM products.items
WHERE description ||| 'wireless'
ORDER BY score DESC
LIMIT 10;
```

**Price histogram facet:**

```sql
SELECT
    id,
    name,
    price,
    pdb.agg('{"histogram": {"field": "price", "interval": 50}}') OVER () AS price_facets
FROM products.items
WHERE category = 'Electronics'
ORDER BY pdb.score(id) DESC
LIMIT 10;
```

**Category counts (terms aggregation):**

```sql
SELECT
    category,
    pdb.agg('{"value_count": {"field": "id"}}') AS count
FROM products.items
WHERE description ||| 'wireless'
GROUP BY category
ORDER BY category
LIMIT 10;
```

**Multiple aggregations:**

```sql
SELECT
    pdb.agg('{"avg": {"field": "price"}}') AS avg_price,
    pdb.agg('{"avg": {"field": "rating"}}') AS avg_rating,
    pdb.agg('{"value_count": {"field": "id"}}') AS total_count
FROM products.items
WHERE category = 'Electronics';
```

---

## 5. Mock Data Requirements

### 5.1 Product Categories

| Category | Subcategories | Count |
|----------|---------------|-------|
| Electronics | Headphones, Keyboards, Mice, Monitors, Cameras | 15 |
| Clothing | T-Shirts, Jackets, Pants, Shoes | 10 |
| Home & Garden | Furniture, Kitchen, Decor | 8 |
| Sports | Fitness, Outdoor, Team Sports | 7 |
| Books | Fiction, Non-Fiction, Technical | 5 |
| **Total** | | **45** |

### 5.2 Data Diversity Requirements

1. **Description variety**: Long (100+ words), medium (30-50 words), short (10-20 words)
2. **Price ranges**: Budget ($0-25), Mid-range ($25-100), Premium ($100-500), Luxury ($500+)
3. **Ratings distribution**: Mix of 3.0-5.0 ratings with varying review counts
4. **Overlapping vocabulary**: Some products share keywords for testing relevance ranking
5. **Unique terminology**: Technical jargon for testing semantic vs lexical gaps

### 5.3 products.json Structure

```json
{
  "products": [
    {
      "name": "Sony WH-1000XM5 Wireless Headphones",
      "description": "Industry-leading noise cancellation with Auto NC Optimizer. Crystal clear hands-free calling with 4 beamforming microphones. Up to 30-hour battery life with quick charging (3 min charge for 3 hours playback). Multipoint connection allows pairing with two Bluetooth devices simultaneously. Speak-to-Chat automatically pauses music when you start talking.",
      "brand": "Sony",
      "category": "Electronics",
      "subcategory": "Headphones",
      "tags": ["wireless", "bluetooth", "noise-cancellation", "premium"],
      "price": 349.99,
      "rating": 4.8,
      "review_count": 2847,
      "stock_quantity": 156,
      "in_stock": true,
      "featured": true,
      "attributes": {
        "color": "Black",
        "connectivity": "Bluetooth 5.2",
        "battery_life_hours": 30,
        "weight_grams": 250,
        "driver_size_mm": 30
      }
    }
  ]
}
```

---

## 6. Test Cases

### 6.1 SQL Test Scripts

Create in `docker-entrypoint-initdb.d/`:

| Script | Purpose |
|--------|---------|
| `10-products-schema.sql` | Schema, tables, indexes |
| `11-products-data.sql` | Load mock data from JSON |
| `12-bm25-search-tests.sql` | BM25 operator tests |
| `13-vector-search-tests.sql` | Vector similarity tests |
| `14-hybrid-search-tests.sql` | Combined search tests |
| `15-facet-aggregation-tests.sql` | Faceted search tests |

### 6.2 Rust Test Cases

Located in `pg_search_tests/src/bin/`:

```rust
// products_bm25_test.rs
#[tokio::test]
async fn test_bm25_match_disjunction() { /* ||| operator */ }

#[tokio::test]
async fn test_bm25_match_conjunction() { /* &&& operator */ }

#[tokio::test]
async fn test_bm25_field_specific_search() { /* name ||| 'keyboard' */ }

#[tokio::test]
async fn test_bm25_numeric_range_filter() { /* price:[50 TO 100] */ }

#[tokio::test]
async fn test_bm25_score_ordering() { /* pdb.score(id) */ }
```

```rust
// products_vector_test.rs
#[tokio::test]
async fn test_vector_cosine_similarity() { /* <=> operator */ }

#[tokio::test]
async fn test_vector_threshold_filter() { /* similarity > 0.7 */ }

#[tokio::test]
async fn test_vector_hnsw_index_usage() { /* EXPLAIN ANALYZE */ }
```

```rust
// products_hybrid_test.rs
#[tokio::test]
async fn test_hybrid_weighted_combination() { /* 70% vector + 30% BM25 */ }

#[tokio::test]
async fn test_hybrid_rrf_fusion() { /* Reciprocal Rank Fusion */ }

#[tokio::test]
async fn test_hybrid_with_filters() { /* category + price filter */ }
```

```rust
// products_facets_test.rs
#[tokio::test]
async fn test_facet_value_count() { /* pdb.agg value_count */ }

#[tokio::test]
async fn test_facet_histogram() { /* pdb.agg histogram */ }

#[tokio::test]
async fn test_facet_with_search() { /* search + aggregation */ }
```

---

## 7. Validation Queries

### 7.1 BM25 Validation

```sql
-- Test: Search for "wireless headphones" returns relevant products
SELECT name, pdb.score(id) AS score
FROM products.items
WHERE description ||| 'wireless headphones'
ORDER BY score DESC
LIMIT 5;

-- Expected: Sony WH-1000XM5 should rank highest
```

### 7.2 Vector Validation

```sql
-- Test: Semantic search for "audio equipment for music"
-- Should return headphones even without exact keyword match
SELECT name, 1 - (description_embedding <=> $embedding) AS similarity
FROM products.items
ORDER BY description_embedding <=> $embedding
LIMIT 5;
```

### 7.3 Hybrid Validation

```sql
-- Test: Query "best noise cancelling"
-- BM25 matches "noise" and "cancelling" keywords
-- Vector matches semantic concept of audio quality
-- Hybrid should combine both signals effectively
```

### 7.4 Facet Validation

```sql
-- Test: Category facets for "wireless" search
SELECT
    category,
    COUNT(*) as count
FROM products.items
WHERE description ||| 'wireless'
GROUP BY category
ORDER BY count DESC;

-- Expected: Electronics > Clothing > others
```

---

## 8. File Structure

```
irdb/
├── .claude/
│   └── spec.md                           # This specification
├── pg_search_tests/
│   ├── data/
│   │   └── products.json                 # Mock product data (45 products)
│   ├── sql_examples/
│   │   └── ... (existing examples)
│   └── src/bin/
│       ├── products_bm25_test.rs
│       ├── products_vector_test.rs
│       ├── products_hybrid_test.rs
│       └── products_facets_test.rs
└── docker-entrypoint-initdb.d/
    ├── 00-extensions.sql                 # Extensions (existing)
    ├── 01-ai-extensions.sql              # AI schema (existing)
    ├── ...
    ├── 10-products-schema.sql            # Products schema + indexes
    ├── 11-products-data.sql              # Load products from JSON
    ├── 12-bm25-search-tests.sql          # BM25 validation
    ├── 13-vector-search-tests.sql        # Vector validation
    ├── 14-hybrid-search-tests.sql        # Hybrid search validation
    └── 15-facet-aggregation-tests.sql    # Facet validation
```

---

## 9. API Quick Reference

### 9.1 pg_search (ParadeDB) Operators

| Operator | Name | Description |
|----------|------|-------------|
| `\|\|\|` | Match Disjunction | Match ANY token |
| `&&&` | Match Conjunction | Match ALL tokens |
| `===` | Exact Match | Literal string match (requires pdb.literal) |
| `@@@` | Legacy Query | Tantivy query syntax (deprecated) |

### 9.2 pg_search Functions

| Function | Description |
|----------|-------------|
| `pdb.score(key_field)` | Returns BM25 relevance score |
| `pdb.snippet(field)` | Returns highlighted text snippet |
| `pdb.agg(json_query)` | Executes aggregate over BM25 index |
| `pdb.all()` | Matches all documents |

### 9.3 pgvector Operators

| Operator | Distance Metric | Use Case |
|----------|-----------------|----------|
| `<=>` | Cosine | Normalized embeddings (most common) |
| `<->` | L2 (Euclidean) | General purpose |
| `<#>` | Inner Product | When magnitude matters |
| `<+>` | L1 (Manhattan) | Sparse vectors |

### 9.4 pgvector Index Types

| Index | Build Time | Memory | Recall | Best For |
|-------|------------|--------|--------|----------|
| HNSW | Slower | Higher | Better | Production queries |
| IVFFlat | Faster | Lower | Good | Large datasets, less recall-sensitive |

---

## 10. Implementation Notes

### 10.1 Embedding Generation

For testing, embeddings can be:
1. **Random vectors**: `SELECT array_agg(random())::vector(1536)` (for structure tests)
2. **Pre-computed**: Store real embeddings in products.json
3. **Generated on load**: Use external API during data loading

### 10.2 Performance Considerations

- BM25 index should be created AFTER data insertion
- HNSW index build benefits from `SET maintenance_work_mem = '4GB'`
- Faceted queries should use `LIMIT` on GROUP BY for performance
- Vacuum tables after bulk inserts: `VACUUM products.items`

### 10.3 Testing Order

1. Schema creation
2. Data loading
3. Index creation
4. BM25 tests (isolated)
5. Vector tests (isolated)
6. Hybrid tests (combined)
7. Facet tests (aggregations)

---

## Appendix A: Sample Queries Cheat Sheet

```sql
-- BM25: Find wireless products
SELECT * FROM products.items WHERE description ||| 'wireless' LIMIT 10;

-- BM25: Find products with BOTH terms
SELECT * FROM products.items WHERE description &&& 'wireless bluetooth' LIMIT 10;

-- BM25: Score and rank
SELECT name, pdb.score(id) FROM products.items WHERE name ||| 'keyboard' ORDER BY pdb.score(id) DESC;

-- Vector: Find similar to embedding
SELECT name, 1-(description_embedding <=> $emb) AS sim FROM products.items ORDER BY description_embedding <=> $emb LIMIT 10;

-- Hybrid: Combine BM25 + Vector
-- (see Section 3.3 for full query)

-- Facet: Count by category
SELECT category, pdb.agg('{"value_count": {"field": "id"}}') FROM products.items WHERE description ||| 'wireless' GROUP BY category;

-- Facet: Price histogram
SELECT pdb.agg('{"histogram": {"field": "price", "interval": 50}}') FROM products.items WHERE category = 'Electronics';
```
