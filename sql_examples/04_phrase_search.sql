-- Phrase Search Examples for ParadeDB pg_search v0.17.2
-- This demonstrates exact phrase matching with paradedb.phrase()

-- Setup: Create test table and data
DROP TABLE IF EXISTS test_phrase CASCADE;

CREATE TABLE test_phrase (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

-- Insert test data BEFORE creating index
INSERT INTO test_phrase (name, description) VALUES
    ('Super Duper Widget', 'A very great product'),
    ('Mega Tron Robot', 'Another amazing robot'),
    ('Running Shoes', 'Fast shoes for running'),
    ('Walking Boots', 'Slow boots for walking'),
    ('Keyboard', 'Mechanical keyboard with switches');

-- Create BM25 index AFTER data is inserted
CREATE INDEX test_phrase_idx ON test_phrase
USING bm25 (id, name, description)
WITH (
    key_field='id',
    text_fields='{"name": {}, "description": {}}'
);

-- Example 1: Phrase search - "mechanical keyboard" as exact phrase
-- The words must appear adjacent and in order
SELECT id, name, description
FROM test_phrase
WHERE test_phrase @@@ paradedb.phrase('description', ARRAY['mechanical', 'keyboard'])
ORDER BY id;
-- Expected: Returns "Keyboard"

-- Example 2: Phrase search that doesn't match
-- "very robot" - words exist separately but not as adjacent phrase
SELECT id, name, description
FROM test_phrase
WHERE test_phrase @@@ paradedb.phrase('description', ARRAY['very', 'robot'])
ORDER BY id;
-- Expected: Returns nothing (0 rows)

-- Example 3: Phrase search - "fast shoes"
SELECT id, name, description
FROM test_phrase
WHERE test_phrase @@@ paradedb.phrase('description', ARRAY['fast', 'shoes'])
ORDER BY id;
-- Expected: Returns "Running Shoes"

-- Example 4: Phrase search - "slow boots"
SELECT id, name, description
FROM test_phrase
WHERE test_phrase @@@ paradedb.phrase('description', ARRAY['slow', 'boots'])
ORDER BY id;
-- Expected: Returns "Walking Boots"

-- Example 5: Phrase search on name field - "super duper"
SELECT id, name, description
FROM test_phrase
WHERE test_phrase @@@ paradedb.phrase('name', ARRAY['super', 'duper'])
ORDER BY id;
-- Expected: Returns "Super Duper Widget"

-- Example 6: Three-word phrase
SELECT id, name, description
FROM test_phrase
WHERE test_phrase @@@ paradedb.phrase('description', ARRAY['mechanical', 'keyboard', 'with'])
ORDER BY id;
-- Expected: Returns "Keyboard"

-- Cleanup
DROP TABLE test_phrase CASCADE;
