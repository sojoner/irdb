# Plan: Integrate Apache AGE into IRDB PostgreSQL Image

## Executive Summary

Extend the current PostgreSQL image with **Apache AGE** (A Graph Extension) to enable graph database functionality alongside existing pg_search (BM25), pgvector, and pg_trgm extensions. This creates a unified knowledge representation platform supporting:

- **BM25 Full-Text Search** (ParadeDB pg_search v0.20.4)
- **Vector Similarity** (pgvector v0.8.0)
- **Graph Relationships** (Apache AGE v1.7.0)
- **Relational Data** (PostgreSQL 17.5)

---

## 1. Apache AGE Research Summary

### What is Apache AGE?

Apache AGE is a PostgreSQL extension that adds graph database capabilities on top of relational databases.

**Key Characteristics:**
- **Multi-model**: Combines relational + graph data models in single storage
- **OpenCypher Support**: Uses graph query language alongside SQL
- **Hybrid Querying**: Mix ANSI SQL with Cypher queries
- **Property Graphs**: Supports vertices (nodes) with properties and edges with relationships
- **Label Support**: Hierarchical graph label organization
- **Indexes**: Property indexes on vertices and edges
- **Multiple Graphs**: Query multiple independent graphs simultaneously

**Compatible Versions:**
- Latest: v1.7.0 (2024)
- Supports PostgreSQL 15, 16, 17, 18
- Current repo uses PG 17.5 ✓ (compatible)

**Build Requirements:**
```
Build Dependencies:
- gcc, glibc
- readline, readline-devel
- zlib, zlib-devel
- flex, bison
- PostgreSQL server dev files (postgresql-server-dev-17)
```

**Installation Methods:**
1. From source (C + PostgreSQL module)
2. Pre-built packages (if available)
3. Docker builds (compile in container)

---

## 2. Current State Analysis

### Dockerfile Structure (Multi-stage)

**Builder Stage:**
- Base: postgres:17.5-bookworm
- Installs: build tools, Rust, cargo-pgrx
- Builds: ParadeDB pg_search from source
- Output: Compiled .so files

**Runtime Stage:**
- Base: postgres:17.5-bookworm
- Copies: pg_search binaries from builder
- Installs: postgresql-contrib, pgvector
- Total size: ~500MB-600MB

**Current Extensions:**
- ✓ pgvector (pre-built package)
- ✓ pg_search (compiled via cargo-pgrx)
- ✓ pg_trgm (via postgresql-contrib)
- ✓ pg_stat_statements
- ✓ btree_gin
- ✗ Apache AGE (NOT YET)

### Test Infrastructure

**SQL Examples:**
- `sql_examples/00-13_*.sql` - comprehensive test suite
- Tests BM25, vector, hybrid, facets
- **Missing**: AGE/graph tests

---

## 3. Integration Strategy

### Phase 1: Extend Builder Stage

**Option A: Build from Source (Recommended)**
```dockerfile
# In BUILDER stage
ARG AGE_VERSION=1.7.0
ARG PG_MAJOR_VERSION=17

# Clone and build AGE using PostgreSQL module build system
RUN git clone --branch v${AGE_VERSION} \
    https://github.com/apache/age.git /tmp/age && \
    cd /tmp/age && \
    make && \
    make install
```

**Option B: Pre-built Binaries (if available)**
- Faster build, larger base image
- Check for PostgreSQL 17 compatibility

### Phase 2: Extend Runtime Stage

**Copy AGE Extension Files:**
```dockerfile
# Copy AGE binary from builder
COPY --from=builder /usr/lib/postgresql/17/lib/age.so \
    /usr/lib/postgresql/17/lib/

# Copy AGE SQL definitions
COPY --from=builder /usr/share/postgresql/17/extension/age* \
    /usr/share/postgresql/17/extension/
```

### Phase 3: Configure PostgreSQL

**Update postgresql.conf:**
```conf
# Add to shared_preload_libraries if needed
shared_preload_libraries = 'pg_search,age'

# AGE-specific settings (if needed)
# Check AGE docs for tuning parameters
```

**Alternative:** Skip preload if AGE doesn't require it (check docs)

### Phase 4: Add Initialization Scripts

**Create new script: `docker-entrypoint-initdb.d/04-age-setup.sql`**

Contents:
```sql
-- Create AGE extension
CREATE EXTENSION IF NOT EXISTS age;

-- Create schema for graph data
CREATE SCHEMA IF NOT EXISTS agens;

-- Grant permissions
GRANT USAGE ON SCHEMA agens TO postgres;
GRANT USAGE ON SCHEMA agens TO app;

-- Example: Create a simple graph for testing
SELECT * FROM ag_catalog.create_graph('test_graph');

\echo '✓ Apache AGE setup complete'
```

---

## 4. Enhanced Test Suite

### New SQL Test Files

#### File: `sql_examples/14_age_graph_setup.sql`
- Create graph schema
- Define vertices and edges
- Create property indexes
- Basic graph setup verification

#### File: `sql_examples/15_age_cypher_tests.sql`
- Node creation (MERGE, CREATE)
- Edge creation
- Path queries (MATCH)
- Pattern matching
- Property traversal

#### File: `sql_examples/16_age_hybrid_queries.sql`
- Combine SQL + Cypher
- Graph + relational joins
- Use Cypher results in SQL

#### File: `sql_examples/17_age_knowledge_graph_tests.sql`
- Full knowledge graph example
- Multi-model searches (text + vector + graph)
- Complex relationship traversals

#### File: `sql_examples/18_combined_ir_db_tests.sql`
- **Integration test combining ALL features:**
  - BM25 full-text search on document content
  - Vector similarity on embeddings
  - Graph relationships between entities
  - Hybrid ranking using all three modalities

---

## 5. Dockerfile Updates

### New Dockerfile Structure

```dockerfile
ARG POSTGRES_VERSION=17.5
ARG POSTGRES_VARIANT=bookworm
ARG PG_MAJOR_VERSION=17
ARG CARGO_PGRX_VERSION=0.16.1
ARG PARADEDB_VERSION=0.20.4
ARG AGE_VERSION=1.7.0

# === BUILDER STAGE ===
FROM postgres:${POSTGRES_VERSION}-${POSTGRES_VARIANT} AS builder

# Install all build dependencies
RUN apt-get update && apt-get install -y \
    build-essential flex bison readline-dev zlib1g-dev \
    # ... existing deps ...
    && rm -rf /var/lib/apt/lists/*

# Build ParadeDB (existing)
RUN ... paradedb build ...

# Build Apache AGE (NEW)
RUN git clone --branch v${AGE_VERSION} \
    https://github.com/apache/age.git /tmp/age && \
    cd /tmp/age && \
    make && \
    make install

# === RUNTIME STAGE ===
FROM postgres:${POSTGRES_VERSION}-${POSTGRES_VARIANT}

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    postgresql-contrib \
    postgresql-${PG_MAJOR_VERSION}-pgvector \
    && rm -rf /var/lib/apt/lists/*

# Copy ParadeDB (existing)
COPY --from=builder /usr/lib/postgresql/${PG_MAJOR_VERSION}/lib/pg_search.so ...
COPY --from=builder /usr/share/postgresql/${PG_MAJOR_VERSION}/extension/pg_search* ...

# Copy Apache AGE (NEW)
COPY --from=builder /usr/lib/postgresql/${PG_MAJOR_VERSION}/lib/age.so ...
COPY --from=builder /usr/share/postgresql/${PG_MAJOR_VERSION}/extension/age* ...

# Copy configs and init scripts
COPY postgresql.conf /etc/postgresql/postgresql.conf
COPY docker-entrypoint-initdb.d/ /docker-entrypoint-initdb.d/

USER postgres
CMD ["postgres", "-c", "config_file=/etc/postgresql/postgresql.conf"]
```

---

## 6. Build Time & Size Estimates

### Compilation Times
- **ParadeDB (current)**: ~10-15 minutes
- **Apache AGE (new)**: ~5-10 minutes
- **Total build time**: ~20-30 minutes (depends on system)

### Image Size Impact
- Base postgres:17.5: ~200MB
- Current with ParadeDB + pgvector: ~500-600MB
- With AGE added: ~550-650MB (+30-50MB)

### Cache Strategy
```dockerfile
# Separate cache mounts for each extension
RUN --mount=type=cache,target=/tmp/age/build \
    cd /tmp/age && make && make install
```

---

## 7. PostgreSQL Configuration

### postgresql.conf Updates

```conf
# Existing settings
shared_preload_libraries = 'pg_search,pg_stat_statements'

# May need AGE addition (verify in docs)
# If AGE requires preloading:
# shared_preload_libraries = 'pg_search,pg_stat_statements,age'

# AGE-specific tuning (check documentation)
# age.work_mem = 1GB  # Example setting
```

### shared_preload_libraries

**Status**: Check if AGE requires preloading
- Some extensions need preload (pg_stat_statements does)
- Some don't (ParadeDB, pgvector don't)
- May need `CREATE EXTENSION` only in init script

---

## 8. Test Strategy

### Test Execution Order

```bash
# 1. Run extension setup tests
psql -U postgres -d app -f sql_examples/00_setup_extensions.sql

# 2. Test each extension independently
psql -U postgres -d app -f sql_examples/10_bm25_search_tests.sql
psql -U postgres -d app -f sql_examples/11_vector_search_tests.sql
psql -U postgres -d app -f sql_examples/14_age_graph_setup.sql
psql -U postgres -d app -f sql_examples/15_age_cypher_tests.sql

# 3. Test combinations
psql -U postgres -d app -f sql_examples/12_hybrid_search_tests.sql
psql -U postgres -d app -f sql_examples/16_age_hybrid_queries.sql

# 4. Integration test (all three)
psql -U postgres -d app -f sql_examples/18_combined_ir_db_tests.sql
```

### Example AGE Test Script Structure

```sql
-- 14_age_graph_setup.sql
\echo '=== Apache AGE Graph Setup ==='

-- Create graph
SELECT * FROM ag_catalog.create_graph('knowledge_graph');

-- Create vertices (nodes)
SELECT * FROM cypher('knowledge_graph', $$
    CREATE (doc:Document {id: 1, title: 'PostgreSQL Guide'})
    RETURN doc
$$) AS (doc agtype);

-- Create edges (relationships)
SELECT * FROM cypher('knowledge_graph', $$
    MATCH (a:Document {id: 1}), (b:Document {id: 2})
    CREATE (a)-[r:REFERENCES]->(b)
    RETURN r
$$) AS (r agtype);

-- Query graph
SELECT * FROM cypher('knowledge_graph', $$
    MATCH (doc:Document)-[:REFERENCES]->(ref:Document)
    RETURN doc.title, ref.title
$$) AS (doc_title TEXT, ref_title TEXT);

\echo '✓ AGE tests complete'
```

---

## 9. Knowledge Representation Enhancement

### Three-Tier Architecture

**Tier 1: Lexical (Text Search)**
- BM25 full-text search via pg_search
- Keyword matching, phrase queries
- Document ranking by relevance

**Tier 2: Semantic (Vector Search)**
- Embedding similarity via pgvector
- Semantic meaning captured in vectors
- Cosine similarity, L2 distance
- Hybrid ranking with keywords

**Tier 3: Relational (Graph)**
- Entity relationships via Apache AGE
- Knowledge graphs (concepts, connections)
- Pattern matching and traversal
- Multi-hop queries

### Example: Multi-Modal Search

```sql
-- Find documents similar to query
-- Using ALL THREE representations:

WITH text_results AS (
    -- BM25: Find keyword matches
    SELECT id FROM documents 
    WHERE content ||| 'graph database'
    ORDER BY pdb.score(id) DESC
    LIMIT 20
),
vector_results AS (
    -- Vector: Find semantic similarity
    SELECT id FROM documents 
    WHERE embedding <=> query_embedding
    ORDER BY embedding <=> query_embedding
    LIMIT 20
),
graph_results AS (
    -- Graph: Find related entities
    SELECT DISTINCT doc_id FROM cypher('knowledge_graph', $$
        MATCH (d:Document)-[:RELATED_TO]->(e:Entity {type: 'GraphDB'})
        RETURN d, e
    $$) AS (d agtype, e agtype)
)
-- Combine scores: 30% text + 70% vector + graph boosting
SELECT id, (text_score * 0.3 + vector_score * 0.7) * graph_boost AS final_score
FROM combined_results
ORDER BY final_score DESC;
```

---

## 10. Implementation Checklist

### Phase 1: Dockerfile Updates
- [ ] Add AGE_VERSION build argument
- [ ] Add AGE build dependencies to builder
- [ ] Implement AGE compilation in builder stage
- [ ] Copy AGE binaries to runtime stage
- [ ] Update postgresql.conf if needed
- [ ] Test build: `docker build -t irdb:with-age .`

### Phase 2: Extension Initialization
- [ ] Create `04-age-setup.sql` initialization script
- [ ] Test extension loading: `SELECT * FROM pg_extension;`
- [ ] Create test graph schema
- [ ] Grant appropriate permissions

### Phase 3: Test Suite
- [ ] Create `14_age_graph_setup.sql`
- [ ] Create `15_age_cypher_tests.sql`
- [ ] Create `16_age_hybrid_queries.sql`
- [ ] Create `17_age_knowledge_graph_tests.sql`
- [ ] Create `18_combined_ir_db_tests.sql`
- [ ] Update `sql_examples/README.md`

### Phase 4: Documentation
- [ ] Add AGE examples to `docs/`
- [ ] Document graph schema design
- [ ] Document query patterns
- [ ] Update main README

### Phase 5: Validation
- [ ] Local Docker build test
- [ ] Run all SQL test suites
- [ ] Kubernetes deployment test
- [ ] Performance baseline

---

## 11. Build Optimization Tips

### Cache Layers
```dockerfile
# Place frequently changing instructions last
# AGE version change? Only rebuilds AGE, not ParadeDB
# ParadeDB version change? Rebuilds both (layer dependency)
```

### Parallel Builds
- Build ParadeDB and AGE in parallel? No (single stage)
- Could split into separate stages if independent

### Size Reduction
- Strip symbols: Add `CFLAGS="-s"` to builds
- Use alpine instead of bookworm? (May lack dependencies)
- Keep ParadeDB + AGE, skip pgvector? (Trade-off)

---

## 12. Potential Issues & Solutions

| Issue | Solution |
|-------|----------|
| AGE build fails on PG 17.5 | Verify PG version compatibility, check AGE release notes |
| Conflicting shared_preload_libraries | Test with/without AGE in preload, may only need `CREATE EXTENSION` |
| Large build time | Use cache mounts, consider pre-built binaries |
| Memory issues during build | Use `--memory` flag with Docker, or build on larger machine |
| Extension doesn't load at runtime | Check docker-entrypoint-initdb.d/ scripts run order |
| Cypher query syntax errors | AGE uses OpenCypher, not full Cypher (Neo4j); check compatibility |

---

## 13. Recommended Implementation Order

### Week 1: Setup & Build
1. Extend Dockerfile with AGE compilation
2. Test local Docker build
3. Verify `SELECT * FROM pg_extension;` shows all 4 extensions

### Week 2: SQL Tests
1. Create AGE setup script (04-age-setup.sql)
2. Build 14-15 test files (basic AGE functionality)
3. Run test suite locally

### Week 3: Integration
1. Create hybrid test files (16-18)
2. Document patterns and examples
3. Deploy to Kubernetes, test end-to-end

### Week 4: Documentation
1. Write AGE query guide
2. Document use cases (fraud detection, recommendations, etc.)
3. Create performance benchmarks
4. Update main README

---

## 14. References

- **Apache AGE GitHub**: https://github.com/apache/age
- **AGE Documentation**: https://age.apache.org/age-manual/master/
- **OpenCypher**: https://opencypher.org/
- **PostgreSQL Module Dev**: https://www.postgresql.org/docs/17/extend-pgxs.html
- **ParadeDB Docs**: https://docs.paradedb.com/
- **pgvector**: https://github.com/pgvector/pgvector

---

## Summary

**Result:**
A unified **Knowledge Representation Database** combining:
- **Text search** (BM25): Find what people wrote
- **Semantic search** (vectors): Understand meaning
- **Relationship search** (graphs): Discover connections

**Size Impact:** +30-50MB
**Build Time Impact:** +5-10 minutes
**Test Coverage:** +50+ new tests
