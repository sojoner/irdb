# ParadeDB pg_search SQL Examples

This directory contains SQL examples demonstrating various search capabilities of ParadeDB pg_search v0.17.2.

## Files Overview

| File | Description |
|------|-------------|
| `00_setup_extensions.sql` | **REQUIRED FIRST** - Initializes extensions, schema, tables, and functions |
| `01_fuzzy_search.sql` | Fuzzy term matching with `paradedb.fuzzy_term()` |
| `02_exact_term_search.sql` | Field-specific exact term matching with `field:term` syntax |
| `03_boolean_search.sql` | Boolean operators (OR, AND, NOT) for combining searches |
| `04_phrase_search.sql` | Exact phrase matching with `paradedb.phrase()` |
| `05_complete_setup.sql` | Complete workflow: table creation, indexing, all search types |
| `06_numeric_range_search.sql` | Numeric field searches with ranges and comparisons |
| `07_snippet_highlighting.sql` | Search result highlighting with `paradedb.snippet()` |
| `08_products_schema.sql` | Products schema and indexes for hybrid search testing |
| `09_products_data.sql` | Load mock product data from JSON file |
| `10_bm25_search_tests.sql` | BM25 full-text search validation tests |
| `11_vector_search_tests.sql` | Vector similarity search validation tests |
| `12_hybrid_search_tests.sql` | Hybrid search combining BM25 and vector search |
| `13_facet_aggregation_tests.sql` | Faceted search and aggregation tests |

## Quick Start - Kubernetes Deployment

### Step 1: Port-forward to the PostgreSQL database
```bash
# In one terminal, set up port-forward
kubectl port-forward -n databases svc/irdb-postgres-017-cnpg-rw 5432:5432
```

### Step 2: Initialize extensions and schema
```bash
# Run the setup script FIRST
psql -h localhost -U postgres -d app -p 5432 -f 00_setup_extensions.sql
```

Expected output:
```
✓ vector extension created
✓ pg_stat_statements extension created
✓ pg_trgm extension created
✓ btree_gin extension created
✓ pg_search extension created (or already exists)
✓ ai_data schema created
✓ ai_data.documents table created
✓ ai_data.chunks table created
✓ Vector similarity indexes created
✓ Text search index created
✓ hybrid_search function created
✓ generate_random_vector function created
✓ Permissions granted to app user
```

### Step 3: Run examples
```bash
# Run a single example
psql -h localhost -U postgres -d app -p 5432 -f 01_fuzzy_search.sql

# Run all examples in sequence
for f in 0*.sql 0[1-9]*.sql 1*.sql; do
    echo "Running $f..."
    psql -h localhost -U postgres -d app -p 5432 -f "$f"
done
```

## Quick Start - Local Docker Compose

### Step 1: Start the database
```bash
docker-compose up -d
```

### Step 2: Initialize extensions and schema
```bash
# Run the setup script FIRST
psql -h localhost -U postgres -d database -p 5432 -f 00_setup_extensions.sql
```

### Step 3: Run examples
```bash
psql -h localhost -U postgres -d database -p 5432 -f 01_fuzzy_search.sql
```

## Quick Start - General

1. **Connect to your database:**
   ```bash
   # Kubernetes (after port-forward)
   psql -h localhost -U postgres -d app -p 5432

   # Docker Compose
   psql -h localhost -U postgres -d database -p 5432
   ```

2. **Run setup script FIRST:**
   ```bash
   psql -h <host> -U postgres -d <database> -p 5432 -f 00_setup_extensions.sql
   ```

3. **Then run any example file:**
   ```bash
   psql -h <host> -U postgres -d <database> -p 5432 -f 01_fuzzy_search.sql
   ```

4. **Or copy-paste queries directly** into your SQL client.

## Key Concepts

### Critical Setup Order

**IMPORTANT:** Data must be inserted BEFORE creating the BM25 index!

```sql
-- 1. Create table
CREATE TABLE products (id SERIAL PRIMARY KEY, name TEXT, description TEXT);

-- 2. Insert data FIRST
INSERT INTO products (name, description) VALUES ('Product', 'Description');

-- 3. THEN create index
CREATE INDEX products_idx ON products USING bm25 (id, name, description)
WITH (key_field='id', text_fields='{"name": {}, "description": {}}');
```

### Text Tokenization

- BM25 **lowercases** all text during indexing
- Search terms must be **lowercase**
- Example: `"Super Duper"` is indexed as tokens `["super", "duper"]`
- Search: `'name:super'` ✅ works, `'name:Super'` ❌ fails

### Field Types

Specify field types in the index configuration:

```sql
CREATE INDEX idx ON table USING bm25 (id, text_field, num_field, bool_field)
WITH (
    key_field='id',
    text_fields='{"text_field": {}}',
    numeric_fields='{"num_field": {}}',
    boolean_fields='{"bool_field": {}}'
);
```

## Search Syntax Reference

### Basic Search

```sql
-- Field-specific search
WHERE table @@@ 'field:term'

-- Search across all text fields
WHERE table @@@ 'term'
```

### Boolean Operators

```sql
-- OR
WHERE table @@@ 'field1:term1 OR field2:term2'

-- AND
WHERE table @@@ 'field1:term1 AND field2:term2'

-- NOT
WHERE table @@@ 'field1:term1 AND NOT field2:term2'

-- Grouping
WHERE table @@@ '(field1:term1 OR field1:term2) AND field2:term3'
```

### Numeric Searches

```sql
-- Range (inclusive)
WHERE table @@@ 'price:[10 TO 100]'

-- Greater than
WHERE table @@@ 'rating:>4.5'

-- Less than
WHERE table @@@ 'price:<50'

-- Greater than or equal
WHERE table @@@ 'quantity:>=10'

-- Less than or equal
WHERE table @@@ 'rating:<=4.0'

-- Exact match
WHERE table @@@ 'quantity:50'
```

### Boolean Searches

```sql
-- Exact boolean value
WHERE table @@@ 'in_stock:true'
WHERE table @@@ 'active:false'
```

### Advanced Functions

```sql
-- Fuzzy term matching
WHERE table @@@ paradedb.fuzzy_term('field', 'term')

-- Exact phrase matching
WHERE table @@@ paradedb.phrase('field', ARRAY['word1', 'word2'])

-- Snippet/highlighting
SELECT paradedb.snippet(field) FROM table WHERE table @@@ 'search'
```

## Common Patterns

### E-commerce Product Search

```sql
-- Search by category and price range
SELECT name, price FROM products
WHERE products @@@ 'category:Electronics AND price:[50 TO 200]'
ORDER BY price;

-- Search with text and availability
SELECT name, price FROM products
WHERE products @@@ 'description:wireless AND in_stock:true'
ORDER BY price;
```

### Content Search with Highlighting

```sql
-- Find articles and show snippets
SELECT
    title,
    paradedb.snippet(content) as highlighted_content
FROM articles
WHERE articles @@@ 'content:database OR content:performance'
LIMIT 10;
```

### Fuzzy + Boolean Search

```sql
-- Fuzzy search combined with filters
SELECT name, category FROM products
WHERE products @@@ '(paradedb.fuzzy_term(''name'', ''keybaord'') OR name:mouse) AND category:Electronics';
```

## Testing Notes

### Features That Work in v0.17.2

✅ Exact term search (`field:term`)
✅ Boolean operators (OR, AND, NOT)
✅ Numeric ranges (`[min TO max]`, `>`, `<`, `>=`, `<=`)
✅ Boolean filters (`field:true`, `field:false`)
✅ Fuzzy term (`paradedb.fuzzy_term(field, term)`) - works for exact matches
✅ Phrase search (`paradedb.phrase(field, ARRAY[...])`)
✅ Snippet highlighting (`paradedb.snippet(field)`)

### Features That Don't Work as Expected

❌ Wildcard searches (`term*` for prefix matching)
❌ N-gram tokenization (`paradedb.ngram` type)
❌ Regex search (`paradedb.regex()`)
❌ Fuzzy with edit distance parameter (2-param version works, 3-param doesn't)

## Best Practices

1. **Insert data before creating index** - Critical for proper indexing
2. **Use lowercase search terms** - BM25 lowercases during tokenization
3. **Specify field types explicitly** - Use `text_fields`, `numeric_fields`, `boolean_fields`
4. **Test with small datasets first** - Verify index behavior before production
5. **Cast numeric results** - Use `price::float8` for proper type handling
6. **Clean up test tables** - Use `DROP TABLE ... CASCADE` when done

## Troubleshooting

### Issue: "access method 'bm25' does not exist"

This error occurs when the `pg_search` extension is not properly loaded.

**Solution for Kubernetes:**
```bash
# 1. Verify the setup script ran successfully
kubectl port-forward -n databases svc/irdb-postgres-017-cnpg-rw 5432:5432
psql -h localhost -U postgres -d app -p 5432 -f 00_setup_extensions.sql

# 2. Check if pg_search extension is loaded
psql -h localhost -U postgres -d app -p 5432 -c "SELECT extname FROM pg_extension;"

# 3. If pg_search is missing, it may need shared_preload_libraries
# The extension is available but requires preloading in postgresql.conf
# For now, try rerunning the setup script
```

### Issue: "database 'database' does not exist"

The Kubernetes deployment uses database name `app`, not `database`.

**Solution:**
```bash
# Use the correct database name for Kubernetes
psql -h localhost -U postgres -d app -p 5432 -f 00_setup_extensions.sql

# For Docker Compose, use 'database'
psql -h localhost -U postgres -d database -p 5432 -f 00_setup_extensions.sql
```

### Issue: "role 'app' does not exist"

The `app` role should be created by CloudNativePG, but if it's missing:

**Solution:**
```bash
# Create the app role manually
psql -h localhost -U postgres -d app -p 5432 -c "CREATE ROLE app WITH LOGIN ENCRYPTED PASSWORD 'app_password_123';"
```

### Issue: Tests failing with connection errors

**Solution:**
```bash
# Make sure port-forward is running in another terminal
kubectl port-forward -n databases svc/irdb-postgres-017-cnpg-rw 5432:5432

# Test the connection
psql -h localhost -U postgres -d app -p 5432 -c "SELECT version();"

# Verify DATABASE_URL environment variable
echo $DATABASE_URL
```

### Issue: "ERROR: vector type not found"

The `vector` extension is not created.

**Solution:**
```bash
# Re-run the setup script
psql -h localhost -U postgres -d app -p 5432 -f 00_setup_extensions.sql

# Or manually create extensions
psql -h localhost -U postgres -d app -p 5432 << EOF
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS btree_gin;
EOF
```

## Running Tests

Each SQL file is self-contained with setup and cleanup:

```bash
# Run a single example
psql -U app -d app -f sql_examples/01_fuzzy_search.sql

# Run all examples in sequence
for f in sql_examples/*.sql; do
    echo "Running $f..."
    psql -U app -d app -f "$f"
done
```

## Related Documentation

- [ParadeDB Official Docs](https://docs.paradedb.com/)
- [pg_search GitHub](https://github.com/paradedb/paradedb/tree/dev/pg_search)
- [BM25 Algorithm](https://en.wikipedia.org/wiki/Okapi_BM25)

## License

These examples are part of the IRDB project test suite.
