-- Numeric Range Search Examples for ParadeDB pg_search v0.17.2
-- This demonstrates searching numeric fields with ranges and comparisons

-- Setup: Create test table with numeric fields
DROP TABLE IF EXISTS test_numeric CASCADE;

CREATE TABLE test_numeric (
    id SERIAL PRIMARY KEY,
    name TEXT,
    price DECIMAL(10, 2),
    rating DECIMAL(3, 2),
    quantity INTEGER
);

-- Insert test data BEFORE creating index
INSERT INTO test_numeric (name, price, rating, quantity) VALUES
    ('Budget Item', 9.99, 3.5, 100),
    ('Mid-range Item', 49.99, 4.2, 50),
    ('Premium Item', 199.99, 4.8, 20),
    ('Luxury Item', 599.99, 4.9, 5),
    ('Clearance Item', 5.99, 3.0, 200);

-- Create BM25 index with numeric fields
CREATE INDEX test_numeric_idx ON test_numeric
USING bm25 (id, name, price, rating, quantity)
WITH (
    key_field='id',
    text_fields='{"name": {}}',
    numeric_fields='{"price": {}, "rating": {}, "quantity": {}}'
);

-- Example 1: Range query - price between 10 and 50 (inclusive)
SELECT id, name, price::float8
FROM test_numeric
WHERE test_numeric @@@ 'price:[10 TO 50]'
ORDER BY price;
-- Expected: Returns "Mid-range Item" (49.99)

-- Example 2: Greater than query - rating > 4.5
SELECT id, name, rating::float8
FROM test_numeric
WHERE test_numeric @@@ 'rating:>4.5'
ORDER BY rating DESC;
-- Expected: Returns "Premium Item" (4.8) and "Luxury Item" (4.9)

-- Example 3: Less than query - price < 10
SELECT id, name, price::float8
FROM test_numeric
WHERE test_numeric @@@ 'price:<10'
ORDER BY price;
-- Expected: Returns "Budget Item" (9.99) and "Clearance Item" (5.99)

-- Example 4: Greater than or equal - quantity >= 50
SELECT id, name, quantity::int
FROM test_numeric
WHERE test_numeric @@@ 'quantity:>=50'
ORDER BY quantity DESC;
-- Expected: Returns items with quantity 100, 50, 200

-- Example 5: Less than or equal - rating <= 4.0
SELECT id, name, rating::float8
FROM test_numeric
WHERE test_numeric @@@ 'rating:<=4.0'
ORDER BY rating;
-- Expected: Returns "Clearance Item" (3.0) and "Budget Item" (3.5)

-- Example 6: Exact numeric match - quantity:50
SELECT id, name, quantity::int
FROM test_numeric
WHERE test_numeric @@@ 'quantity:50'
ORDER BY id;
-- Expected: Returns "Mid-range Item"

-- Example 7: Combining numeric and text search
SELECT id, name, price::float8
FROM test_numeric
WHERE test_numeric @@@ 'name:item AND price:[40 TO 100]'
ORDER BY price;
-- Expected: Returns "Mid-range Item"

-- Example 8: Multiple range conditions with AND
SELECT id, name, price::float8, rating::float8
FROM test_numeric
WHERE test_numeric @@@ 'price:[40 TO 250] AND rating:>4.0'
ORDER BY price;
-- Expected: Returns "Mid-range Item" and "Premium Item"

-- Example 9: Range with OR condition
SELECT id, name, price::float8
FROM test_numeric
WHERE test_numeric @@@ 'price:<10 OR price:>500'
ORDER BY price;
-- Expected: Returns "Clearance Item", "Budget Item", and "Luxury Item"

-- Cleanup
DROP TABLE test_numeric CASCADE;
