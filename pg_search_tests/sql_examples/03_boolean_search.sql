-- Boolean Search Examples for ParadeDB pg_search v0.17.2
-- This demonstrates OR, AND, and NOT operators for combining search terms

-- Setup: Create test table and data
DROP TABLE IF EXISTS test_boolean CASCADE;

CREATE TABLE test_boolean (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

-- Insert test data BEFORE creating index
INSERT INTO test_boolean (name, description) VALUES
    ('Super Duper Widget', 'A very great product'),
    ('Mega Tron Robot', 'Another amazing robot'),
    ('Running Shoes', 'Fast shoes for running'),
    ('Walking Boots', 'Slow boots for walking'),
    ('Keyboard', 'Mechanical keyboard with switches');

-- Create BM25 index AFTER data is inserted
CREATE INDEX test_boolean_idx ON test_boolean
USING bm25 (id, name, description)
WITH (
    key_field='id',
    text_fields='{"name": {}, "description": {}}'
);

-- Example 1: OR operator - match documents with "running" OR "walking"
SELECT id, name, description
FROM test_boolean
WHERE test_boolean @@@ 'description:running OR description:walking'
ORDER BY id;
-- Expected: Returns both "Running Shoes" and "Walking Boots"

-- Example 2: OR operator on name field
SELECT id, name, description
FROM test_boolean
WHERE test_boolean @@@ 'name:robot OR name:widget'
ORDER BY id;
-- Expected: Returns "Super Duper Widget" and "Mega Tron Robot"

-- Example 3: AND operator - match documents with both terms
SELECT id, name, description
FROM test_boolean
WHERE test_boolean @@@ 'description:mechanical AND description:keyboard'
ORDER BY id;
-- Expected: Returns "Keyboard"

-- Example 4: Combining multiple fields with OR
SELECT id, name, description
FROM test_boolean
WHERE test_boolean @@@ 'name:shoes OR description:boots'
ORDER BY id;
-- Expected: Returns "Running Shoes" and "Walking Boots"

-- Example 5: Complex boolean query
SELECT id, name, description
FROM test_boolean
WHERE test_boolean @@@ '(description:fast OR description:slow) AND (name:shoes OR name:boots)'
ORDER BY id;
-- Expected: Returns "Running Shoes" and "Walking Boots"

-- Cleanup
DROP TABLE test_boolean CASCADE;
