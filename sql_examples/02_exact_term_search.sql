-- Exact Term Search Examples for ParadeDB pg_search v0.17.2
-- This demonstrates field-specific exact term matching

-- Setup: Create test table and data
DROP TABLE IF EXISTS test_exact CASCADE;

CREATE TABLE test_exact (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

-- Insert test data BEFORE creating index
INSERT INTO test_exact (name, description) VALUES
    ('Super Duper Widget', 'A very great product'),
    ('Mega Tron Robot', 'Another amazing robot'),
    ('Running Shoes', 'Fast shoes for running'),
    ('Walking Boots', 'Slow boots for walking'),
    ('Keyboard', 'Mechanical keyboard with switches');

-- Create BM25 index AFTER data is inserted
CREATE INDEX test_exact_idx ON test_exact
USING bm25 (id, name, description)
WITH (
    key_field='id',
    text_fields='{"name": {}, "description": {}}'
);

-- Example 1: Search for "running" in description field
-- BM25 tokenizes and lowercases text, so search for lowercase terms
SELECT id, name, description
FROM test_exact
WHERE test_exact @@@ 'description:running'
ORDER BY id;
-- Expected: Returns "Running Shoes"

-- Example 2: Search for "walking" in description field
SELECT id, name, description
FROM test_exact
WHERE test_exact @@@ 'description:walking'
ORDER BY id;
-- Expected: Returns "Walking Boots"

-- Example 3: Search for "mechanical" in description field
SELECT id, name, description
FROM test_exact
WHERE test_exact @@@ 'description:mechanical'
ORDER BY id;
-- Expected: Returns "Keyboard"

-- Example 4: Search for "robot" in description field
SELECT id, name, description
FROM test_exact
WHERE test_exact @@@ 'description:robot'
ORDER BY id;
-- Expected: Returns "Mega Tron Robot"

-- Example 5: Search for "widget" in name field
SELECT id, name, description
FROM test_exact
WHERE test_exact @@@ 'name:widget'
ORDER BY id;
-- Expected: Returns "Super Duper Widget"

-- Cleanup
DROP TABLE test_exact CASCADE;
