-- Fuzzy Search Examples for ParadeDB pg_search v0.17.2
-- This demonstrates fuzzy term matching with paradedb.fuzzy_term()

-- Setup: Create test table and data
DROP TABLE IF EXISTS test_fuzzy CASCADE;

CREATE TABLE test_fuzzy (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

-- Insert test data BEFORE creating index
INSERT INTO test_fuzzy (name, description) VALUES
    ('Super Duper Widget', 'A very great product'),
    ('Mega Tron Robot', 'Another amazing robot'),
    ('Running Shoes', 'Fast shoes for running'),
    ('Walking Boots', 'Slow boots for walking'),
    ('Keyboard', 'Mechanical keyboard with switches');

-- Create BM25 index AFTER data is inserted
CREATE INDEX test_fuzzy_idx ON test_fuzzy
USING bm25 (id, name, description)
WITH (
    key_field='id',
    text_fields='{"name": {}, "description": {}}'
);

-- Example 1: Fuzzy term search - "dupr" (typo for "duper") matches "Super Duper Widget"
-- Note: BM25 tokenizes to lowercase, so search for lowercase terms
SELECT id, name, description
FROM test_fuzzy
WHERE test_fuzzy @@@ paradedb.fuzzy_term('name', 'dupr')
ORDER BY id;
-- Expected: Returns "Super Duper Widget"

-- Example 2: Exact match with field:term syntax
SELECT id, name, description
FROM test_fuzzy
WHERE test_fuzzy @@@ 'name:mega'
ORDER BY id;
-- Expected: Returns "Mega Tron Robot"

-- Example 3: Fuzzy term on description field
SELECT id, name, description
FROM test_fuzzy
WHERE test_fuzzy @@@ paradedb.fuzzy_term('description', 'walking')
ORDER BY id;
-- Expected: Returns "Walking Boots"

-- Example 4: Fuzzy term on name field
SELECT id, name, description
FROM test_fuzzy
WHERE test_fuzzy @@@ paradedb.fuzzy_term('name', 'keyboard')
ORDER BY id;
-- Expected: Returns "Keyboard"

-- Cleanup
DROP TABLE test_fuzzy CASCADE;
