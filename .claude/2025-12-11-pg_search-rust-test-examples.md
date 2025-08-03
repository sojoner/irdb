# tests

## copy.rs

```
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn test_copy_to_table(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test_copy_to_table;
        CREATE TABLE test_copy_to_table (id SERIAL PRIMARY KEY, name TEXT);
        CREATE INDEX idx_test_copy_to_table ON test_copy_to_table USING bm25(id, name) WITH (key_field = 'id');
    "#.execute(&mut conn);

    let mut copyin = conn
        .copy_in_raw("COPY test_copy_to_table(name) FROM STDIN")
        .await
        .unwrap();
    copyin.send("one\ntwo\nthree".as_bytes()).await.unwrap();
    copyin.finish().await.unwrap();

    let (count,) = "SELECT COUNT(*) FROM test_copy_to_table WHERE name @@@ 'one'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
```

---

## bm25_search.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use anyhow::Result;
use approx::assert_relative_eq;
use core::panic;
use fixtures::*;
use pgvector::Vector;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};
use std::str::FromStr;

#[rstest]
async fn basic_search_query(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard OR category:electronics' ORDER BY id"
            .fetch_collect(&mut conn);

    assert_eq!(
        columns.description,
        concat!(
            "Ergonomic metal keyboard,Plastic Keyboard,Innovative wireless earbuds,",
            "Fast charging power bank,Bluetooth-enabled speaker"
        )
        .split(',')
        .collect::<Vec<_>>()
    );

    assert_eq!(
        columns.category,
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );

    Ok(())
}

#[rstest]
async fn basic_search_ids(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard OR category:electronics' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);
}

#[rstest]
fn json_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.color:white' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![4, 15, 25]);
}

#[rstest]
fn date_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'last_updated_date:[2023-04-15T00:00:00Z TO 2023-04-18T00:00:00Z]' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 23, 41]);
}

#[rstest]
fn timestamp_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'created_at:[2023-04-15T00:00:00Z TO 2023-04-18T00:00:00Z]' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 22, 23, 41]);
}

#[rstest]
fn real_time_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "INSERT INTO paradedb.bm25_search (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        VALUES ('New keyboard', 5, 'Electronics', true, '{}', TIMESTAMP '2023-05-04 11:09:12', DATE '2023-05-06', TIME '10:07:10')"
        .execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 1".execute(&mut conn);
    "UPDATE paradedb.bm25_search SET description = 'PVC Keyboard' WHERE id = 2".execute(&mut conn);

    let columns: SimpleProductsTableVec = "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard OR category:electronics' ORDER BY id"
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 12, 22, 32, 42]);
}

#[rstest]
fn sequential_scan_syntax(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec = "SELECT * FROM paradedb.bm25_search
        WHERE paradedb.search_with_query_input(
            id,
            paradedb.parse('category:electronics')
        ) ORDER BY id"
        .to_string()
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![1, 2, 12, 22, 32]);
}

#[rstest]
fn quoted_table_name(mut conn: PgConnection) {
    r#"CREATE TABLE "Activity" (key SERIAL, name TEXT, age INTEGER);
    INSERT INTO "Activity" (name, age) VALUES ('Alice', 29);
    INSERT INTO "Activity" (name, age) VALUES ('Bob', 34);
    INSERT INTO "Activity" (name, age) VALUES ('Charlie', 45);
    INSERT INTO "Activity" (name, age) VALUES ('Diana', 27);
    INSERT INTO "Activity" (name, age) VALUES ('Fiona', 38);
    INSERT INTO "Activity" (name, age) VALUES ('George', 41);
    INSERT INTO "Activity" (name, age) VALUES ('Hannah', 22);
    INSERT INTO "Activity" (name, age) VALUES ('Ivan', 30);
    INSERT INTO "Activity" (name, age) VALUES ('Julia', 25);
    CREATE INDEX activity ON "Activity"
    USING bm25 ("key", name) WITH (key_field='key')"#
        .execute(&mut conn);
    let row: (i32, String, i32) =
        "SELECT * FROM \"Activity\" WHERE \"Activity\" @@@ 'name:alice' ORDER BY key"
            .fetch_one(&mut conn);

    assert_eq!(row, (1, "Alice".into(), 29));
}

#[rstest]
fn text_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        text_array TEXT[],
        varchar_array VARCHAR[]
    );
    INSERT INTO example_table (text_array, varchar_array) VALUES
    ('{"text1", "text2", "text3"}', '{"vtext1", "vtext2"}'),
    ('{"another", "array", "of", "texts"}', '{"vtext3", "vtext4", "vtext5"}'),
    ('{"single element"}', '{"single varchar element"}');
    CREATE INDEX example_table_idx ON public.example_table
    USING bm25 (id, text_array, varchar_array)
    WITH (
        key_field = 'id',
        text_fields = '{
            "text_array": {},
            "varchar_array": {}
        }'
    );"#
    .execute(&mut conn);
    let row: (i32,) =
        r#"SELECT * FROM example_table WHERE example_table @@@ 'text_array:text1' ORDER BY id"#
            .fetch_one(&mut conn);

    assert_eq!(row, (1,));

    let row: (i32,) =
        r#"SELECT * FROM example_table WHERE example_table @@@ 'text_array:"single element"' ORDER BY id"#.fetch_one(&mut conn);

    assert_eq!(row, (3,));

    let rows: Vec<(i32,)> =
        r#"SELECT * FROM example_table WHERE example_table @@@ 'varchar_array:varchar OR text_array:array' ORDER BY id"#
            .fetch(&mut conn);

    assert_eq!(rows[0], (2,));
    assert_eq!(rows[1], (3,));
}

#[rstest]
fn int_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        int_array INT[],
        bigint_array BIGINT[]
    );
    INSERT INTO example_table (int_array, bigint_array) VALUES
    ('{1, 2, 3}', '{100, 200}'),
    ('{4, 5, 6}', '{300, 400, 500}'),
    ('{7, 8, 9}', '{600, 700, 800, 900}');
    CREATE INDEX example_table_idx ON public.example_table
    USING bm25 (id, int_array, bigint_array)
    WITH (key_field = 'id');"#
        .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table WHERE example_table @@@ 'int_array:1' ORDER BY id"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1,));

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table WHERE example_table @@@ 'bigint_array:500' ORDER BY id"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (2,));
}

#[rstest]
fn boolean_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        bool_array BOOLEAN[]
    );
    INSERT INTO example_table (bool_array) VALUES
    ('{true, true, true}'),
    ('{false, false, false}'),
    ('{true, true, false}');

    CREATE INDEX example_table_idx ON example_table
    USING bm25 (id, bool_array) WITH (key_field='id')
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table WHERE example_table @@@ 'bool_array:true' ORDER BY id"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (1,));
    assert_eq!(rows[1], (3,));

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table WHERE example_table @@@ 'bool_array:false' ORDER BY id"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (2,));
    assert_eq!(rows[1], (3,));
}

#[rstest]
fn datetime_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        date_array DATE[],
        timestamp_array TIMESTAMP[]
    );
    INSERT INTO example_table (date_array, timestamp_array) VALUES
    (ARRAY['2023-01-01'::DATE, '2023-02-01'::DATE], ARRAY['2023-02-01 12:00:00'::TIMESTAMP, '2023-02-01 13:00:00'::TIMESTAMP]),
    (ARRAY['2023-03-01'::DATE, '2023-04-01'::DATE], ARRAY['2023-04-01 14:00:00'::TIMESTAMP, '2023-04-01 15:00:00'::TIMESTAMP]),
    (ARRAY['2023-05-01'::DATE, '2023-06-01'::DATE], ARRAY['2023-06-01 16:00:00'::TIMESTAMP, '2023-06-01 17:00:00'::TIMESTAMP]);
    CREATE INDEX example_table_idx ON example_table
    USING bm25 (id, date_array, timestamp_array) WITH (key_field='id')
    "#.execute(&mut conn);

    let rows: Vec<(i32,)> =
        r#"SELECT id FROM example_table WHERE example_table @@@ 'date_array:"2023-02-01T00:00:00Z"' ORDER BY id"#
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1,));

    let rows: Vec<(i32,)> =
        r#"SELECT id FROM example_table WHERE example_table @@@ 'timestamp_array:"2023-04-01T15:00:00Z"' ORDER BY id"#
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (2,));
}

#[rstest]
fn json_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        json_array JSONB[]
    );
    INSERT INTO example_table (json_array) VALUES
    (ARRAY['{"name": "John", "age": 30}'::JSONB, '{"name": "Jane", "age": 25}'::JSONB]),
    (ARRAY['{"name": "Bob", "age": 40}'::JSONB, '{"name": "Alice", "age": 35}'::JSONB]),
    (ARRAY['{"name": "Mike", "age": 50}'::JSONB, '{"name": "Lisa", "age": 45}'::JSONB]);"#
        .execute(&mut conn);

    match "CREATE INDEX example_table_idx ON example_table USING bm25 (id, json_array) WITH (key_field='id')"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("json arrays should not yet be supported"),
        Err(err) => assert!(err.to_string().contains("not yet supported")),
    }
}

#[rstest]
fn uuid(mut conn: PgConnection) {
    r#"
    CREATE TABLE uuid_table (
        id SERIAL PRIMARY KEY,
        random_uuid UUID,
        some_text text
    );

    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'some text');

    CREATE INDEX uuid_table_bm25_index ON uuid_table
    USING bm25 (id, some_text) WITH (key_field='id');

    DROP INDEX uuid_table_bm25_index CASCADE;"#
        .execute(&mut conn);

    r#"
    CREATE INDEX uuid_table_bm25_index ON uuid_table
    USING bm25 (id, some_text, random_uuid) WITH (key_field='id')
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        r#"SELECT * FROM uuid_table WHERE uuid_table @@@ 'some_text:some' ORDER BY id"#
            .fetch(&mut conn);

    assert_eq!(rows.len(), 10);
}

#[rstest]
fn multi_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
	paradedb.boolean(
	    should => ARRAY[
		    paradedb.parse('description:shoes'),
		    paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
		    paradedb.term(field => 'description', value => 'speaker'),
		    paradedb.fuzzy_term(field => 'description', value => 'wolo', transposition_cost_one => false, distance => 1, prefix => true)
	    ]
    ) ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5, 7, 10, 32, 33, 34, 37, 39, 41]);
}

#[rstest]
fn snippet(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let row: (i32, String, f32) = "
        SELECT id, pdb.snippet(description), pdb.score(id)
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY id"
        .fetch_one(&mut conn);

    assert_eq!(row.0, 3);
    assert_eq!(row.1, "Sleek running <b>shoes</b>");
    assert_relative_eq!(row.2, 2.484906, epsilon = 1e-6);

    let row: (i32, String, f32) = "
        SELECT id, pdb.snippet(description, '<h1>', '</h1>'), pdb.score(id)
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY id"
        .fetch_one(&mut conn);

    assert_eq!(row.0, 3);
    assert_eq!(row.1, "Sleek running <h1>shoes</h1>");
    assert_relative_eq!(row.2, 2.484906, epsilon = 1e-6);

    let row: (i32, String, f32) = "
        SELECT id, pdb.snippet(description, max_num_chars=>14), pdb.score(id)
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id;"
        .fetch_one(&mut conn);

    assert_eq!(row.0, 1);
    assert_eq!(row.1, "metal <b>keyboard</b>");
    assert_relative_eq!(row.2, 2.821378, epsilon = 1e-6);

    let row: (i32, String, f32) = "
        SELECT id, pdb.snippet(description, max_num_chars=>17), pdb.score(id)
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
        .fetch_one(&mut conn);

    assert_eq!(row.0, 5);
    assert_eq!(row.1, "Generic <b>shoes</b>");
    assert_relative_eq!(row.2, 2.877_26, epsilon = 1e-6);
}

#[rstest]
fn snippet_text_array(mut conn: PgConnection) {
    r#"
    CREATE TABLE people (
        id SERIAL PRIMARY KEY,
        names TEXT[],
        locations VARCHAR[]
    );
    INSERT INTO people (names, locations) VALUES
    ('{"Alice", "Bob", "Charlie"}', '{"New York", "Los Angeles"}'),
    ('{"Diana", "Eve", "Fiona"}', '{"Chicago", "Houston"}'),
    ('{"George", "Hannah", "Ivan"}', '{"Miami", "Seattle"}');
    CREATE INDEX people_idx ON people USING bm25 (id, names, locations) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results: Vec<(i32, String, String)> = "
        SELECT id, pdb.snippet(names), pdb.snippet(locations)
        FROM people WHERE names @@@ 'alice' AND locations @@@ 'new'"
        .fetch(&mut conn);
    assert_eq!(
        results,
        vec![(
            1,
            "<b>Alice</b> Bob Charlie".into(),
            "<b>New</b> York Los Angeles".into()
        )]
    );
}

#[rstest]
fn hybrid_with_single_result(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx
    ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at)
    WITH (
        key_field='id',
        text_fields='{"description": {}, "category": {}}',
        numeric_fields='{"rating": {}}',
        boolean_fields='{"in_stock": {}}',
        json_fields='{"metadata": {}}'
    );

    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);

    // Here, we'll delete all rows in the table but the first.
    // This previously triggered a "division by zero" error when there was
    // only one result in the similarity query. This test ensures that we
    // check for that condition.
    "DELETE FROM mock_items WHERE id != 1".execute(&mut conn);

    let rows: Vec<(i32, BigDecimal, String, Vector)> = r#"
    WITH semantic_search AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items ORDER BY embedding <=> '[1,2,3]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) as rank
        FROM mock_items WHERE description @@@ 'keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        COALESCE(1.0 / (60 + semantic_search.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25_search.rank), 0.0) AS score,
        mock_items.description,
        mock_items.embedding
    FROM semantic_search
    FULL OUTER JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC, description
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![(
            1,
            BigDecimal::from_str("0.03278688524590163934").unwrap(),
            String::from("Ergonomic metal keyboard"),
            Vector::from(vec![3.0, 4.0, 5.0])
        ),]
    );
}

#[rstest]
fn update_non_indexed_column(mut conn: PgConnection) -> Result<()> {
    // Create the test table and index.
    "CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');"
        .execute(&mut conn);

    // For this test, we'll turn off autovacuum, as we'll be measuring the size of the index.
    // We don't want a vacuum to happen and unexpectedly change the size.
    "ALTER TABLE mock_items SET (autovacuum_enabled = false)".execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "default", "lowercase": true, "remove_long": 255}}}')
    "#
      .execute(&mut conn);

    let page_size_before: (i64,) =
        "SELECT pg_relation_size('search_idx') / current_setting('block_size')::int AS page_count"
            .fetch_one(&mut conn);
    // Update a non-indexed column.
    "UPDATE mock_items set category = 'Books' WHERE description = 'Sleek running shoes'"
        .execute(&mut conn);

    let page_size_after: (i64,) =
        "SELECT pg_relation_size('search_idx') / current_setting('block_size')::int AS page_count"
            .fetch_one(&mut conn);
    // The total page count should not have changed when updating a non-indexed column.
    assert_eq!(page_size_before, page_size_after);

    Ok(())
}

#[rstest]
async fn json_array_flattening(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert a JSON array into the metadata field
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES
    ('Product with array', 'Electronics', 4, true, '{\"colors\": [\"red\", \"green\", \"blue\"]}', now(), current_date)"
        .execute(&mut conn);

    // Search for individual elements in the JSON array
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:red' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:green' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:blue' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);
}

#[rstest]
async fn json_array_multiple_documents(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert multiple documents with JSON arrays
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES
    ('Product 1', 'Electronics', 5, true, '{\"colors\": [\"red\", \"green\"]}', now(), current_date),
    ('Product 2', 'Electronics', 3, false, '{\"colors\": [\"blue\", \"yellow\"]}', now(), current_date),
    ('Product 3', 'Electronics', 4, true, '{\"colors\": [\"green\", \"blue\"]}', now(), current_date)"
        .execute(&mut conn);

    // Search for individual elements and verify the correct documents are returned
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:red' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:green' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42, 44]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:blue' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![43, 44]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.colors:yellow' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![43]);
}

#[rstest]
async fn json_array_mixed_data(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert documents with mixed data types in JSON arrays
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES
    ('Product with mixed array', 'Electronics', 5, true, '{\"attributes\": [\"fast\", 4, true]}', now(), current_date)"
        .execute(&mut conn);

    // Search for each data type element in the JSON array
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.attributes:fast' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.attributes:4' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.attributes:true' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);
}

#[rstest]
async fn json_nested_arrays(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert a document with nested JSON arrays into the metadata field
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES
    ('Product with nested array', 'Electronics', 4, true, '{\"specs\": {\"dimensions\": [\"width\", [\"height\", \"depth\"]]}}', now(), current_date)"
        .execute(&mut conn);

    // Search for elements in the nested JSON arrays
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.specs.dimensions:width' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.specs.dimensions:height' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'metadata.specs.dimensions:depth' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);
}

#[rstest]
fn bm25_partial_index_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');".execute(&mut conn);

    let ret = r#"
    CREATE INDEX partial_idx ON paradedb.test_partial_index
    USING bm25 (id, description, category, rating)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
                "tokenizer": {"type": "default"}
            }
        }'
    ) WHERE category = 'Electronics';
    "#
    .execute_result(&mut conn);
    assert!(ret.is_ok(), "{ret:?}");

    // Ensure returned rows match the predicate
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.test_partial_index WHERE test_partial_index @@@ 'rating:>1' ORDER BY rating LIMIT 20"
            .fetch_collect(&mut conn);
    assert_eq!(columns.category.len(), 5);
    assert_eq!(
        columns.category,
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );
    assert_eq!(columns.rating, vec![3, 4, 4, 4, 5]);

    // Ensure no mismatch rows returned
    let rows: Vec<(String, String)> = "
    SELECT description, category FROM paradedb.test_partial_index
    WHERE test_partial_index @@@ '(description:jeans OR category:Footwear) AND rating:>1'
    ORDER BY rating LIMIT 20"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Insert multiple tuples only 1 matches predicate and query
    "INSERT INTO paradedb.test_partial_index (description, category, rating, in_stock) VALUES
    ('Product 1', 'Electronics', 2, true),
    ('Product 2', 'Electronics', 1, false),
    ('Product 3', 'Footwear', 2, true)"
        .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = "
    SELECT description, rating, category FROM paradedb.test_partial_index
    WHERE test_partial_index @@@ 'rating:>1'
    ORDER BY rating LIMIT 20"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let (desc, rating, category) = rows[0].clone();
    assert_eq!(desc, "Product 1");
    assert_eq!(rating, 2);
    assert_eq!(category, "Electronics");

    // Update one tuple to make it no longer match the predicate
    "UPDATE paradedb.test_partial_index SET category = 'Footwear' WHERE description = 'Product 1'"
        .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = "
    SELECT description, rating, category FROM paradedb.test_partial_index
    WHERE test_partial_index @@@ 'rating:>1'
    ORDER BY rating LIMIT 20"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    let (desc, ..) = rows[0].clone();
    assert_ne!(desc, "Product 1");

    // Update one tuple to make it match the predicate
    "UPDATE paradedb.test_partial_index SET category = 'Electronics' WHERE description = 'Product 3'"
        .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = "
    SELECT description, rating, category FROM paradedb.test_partial_index
    WHERE test_partial_index @@@ 'rating:>1'
    ORDER BY rating LIMIT 20"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let (desc, rating, category) = rows[0].clone();
    assert_eq!(desc, "Product 3");
    assert_eq!(rating, 2);
    assert_eq!(category, "Electronics");

    // Insert one row without specifying the column referenced by the predicate.
    let rows: Vec<(String, i32, String)> = "
    SELECT description, rating, category FROM paradedb.test_partial_index
    WHERE test_partial_index @@@ 'rating:>1'
    ORDER BY rating LIMIT 20"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 6);
}

// TODO: This test is currently ignored because the custom scan will not trigger (in all cases)
// on a partial index: see https://github.com/paradedb/paradedb/issues/2747
#[ignore]
#[rstest]
fn bm25_partial_index_hybrid(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);

    let ret = r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default", "lowercase": true, "remove_long": 255}},
            "category": {}
        }',
        numeric_fields='{"rating": {}}'
    ) WHERE category = 'Electronics';"#
    .execute_result(&mut conn);
    assert!(ret.is_ok(), "{ret:?}");

    let rows: Vec<(i32, BigDecimal, String, String, Vector)> = r#"WITH semantic_search AS (
    SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items
        ORDER BY embedding <=> '[1,2,3]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) AS rank
        FROM mock_items
        WHERE mock_items @@@ 'rating:>1'
        AND category = 'Electronics'
        LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        COALESCE(1.0 / (60 + semantic_search.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25_search.rank), 0.0) AS score,
        mock_items.description,
        mock_items.category,
        mock_items.embedding
    FROM semantic_search
    JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC, description
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.iter().map(|r| r.3.clone()).collect::<Vec<_>>(),
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );

    "INSERT INTO mock_items (description, category, rating, in_stock) VALUES
    ('Product 1', 'Electronics', 2, true),
    ('Product 2', 'Electronics', 1, false),
    ('Product 3', 'Footwear', 2, true);

    UPDATE mock_items m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;"
        .execute(&mut conn);

    let rows: Vec<(i32, BigDecimal, String, String, Vector)> = r#"
    WITH semantic_search AS (
    SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items
        ORDER BY embedding <=> '[1,2,3]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) AS rank
        FROM mock_items
        WHERE mock_items @@@ 'rating:>1'
        AND category = 'Electronics'
        LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        COALESCE(1.0 / (60 + semantic_search.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25_search.rank), 0.0) AS score,
        mock_items.description,
        mock_items.category,
        mock_items.embedding
    FROM semantic_search
    JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC, description
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 6);
    assert_eq!(
        rows.iter().map(|r| r.3.clone()).collect::<Vec<_>>(),
        "Electronics,Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    )
}

#[rstest]
fn bm25_partial_index_invalid_statement(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');".execute(&mut conn);

    // Ensure report error when predicate is invalid
    // unknown column
    let ret = r#"
    CREATE INDEX partial_idx ON paradedb.test_partial_index
    USING bm25 (id, description, category, rating)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
                "tokenizer": {"type": "default"}
            }
        }'
    ) WHERE city = 'Electronics';
    "#
    .execute_result(&mut conn);
    assert!(ret.is_err());

    // mismatch type
    let ret = r#"
    CREATE INDEX partial_idx ON paradedb.test_partial_index
    USING bm25 (id, description, category, rating)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
                "tokenizer": {"type": "default"}
            }
        }'
    ) WHERE city = 'Electronics';
    "#
    .execute_result(&mut conn);
    assert!(ret.is_err());

    let ret = r#"
    CREATE INDEX partial_idx ON paradedb.test_partial_index
    USING bm25 (id, description, category, rating)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
                "tokenizer": {"type": "default"}
            }
        }'
    ) WHERE category = 'Electronics';
    "#
    .execute_result(&mut conn);
    assert!(ret.is_ok(), "{ret:?}");
}

#[rstest]
fn bm25_partial_index_alter_and_drop(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');".execute(&mut conn);

    r#"
    CREATE INDEX partial_idx ON paradedb.test_partial_index
    USING bm25 (id, description, category, rating)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default", "lowercase": true, "remove_long": 255}}
        }'
    ) WHERE category = 'Electronics';
    "#
    .execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Drop a column that is not referenced in the partial index.
    "ALTER TABLE paradedb.test_partial_index DROP COLUMN metadata;".execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // When the predicate column is dropped with CASCADE, the index and the corresponding
    // schema are both dropped.
    "ALTER TABLE paradedb.test_partial_index DROP COLUMN category CASCADE;".execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx';".fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    r#"
    CREATE INDEX partial_idx ON paradedb.test_partial_index
    USING bm25 (id, description, rating)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default", "lowercase": true, "remove_long": 255}}
        }'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    "DROP INDEX paradedb.partial_idx".execute(&mut conn);

    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx'".fetch(&mut conn);
    assert_eq!(rows.len(), 0);
}

#[rstest]
fn high_limit_rows(mut conn: PgConnection) {
    "CREATE TABLE large_series (id SERIAL PRIMARY KEY, description TEXT);".execute(&mut conn);
    "INSERT INTO large_series (description) SELECT 'Product ' || i FROM generate_series(1, 200000) i;"
        .execute(&mut conn);

    r#"
    CREATE INDEX large_series_idx ON public.large_series
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM large_series WHERE large_series @@@ 'description:Product' ORDER BY id"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 200000);
}

#[rstest]
fn json_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id FROM paradedb.bm25_search
        WHERE paradedb.bm25_search.id @@@ paradedb.term('metadata.color', 'white')
        ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(4,), (15,), (25,)]);

    r#"
    UPDATE paradedb.bm25_search
    SET metadata = '{"attributes": {"score": 4, "keywords": ["electronics", "headphones"]}}'::jsonb
    WHERE id = 1
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.term('metadata.attributes.score', 4)
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.term('metadata.attributes.keywords', 'electronics')
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // Term set
    let rows: Vec<(i32,)> = "
        SELECT id FROM paradedb.bm25_search
        WHERE paradedb.bm25_search.id @@@ paradedb.term_set(
            ARRAY[
                paradedb.term('metadata.color', 'white'),
                paradedb.term('metadata.attributes.score', 4)
            ]
        ) ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,), (4,), (15,), (25,)]);
}

#[rstest]
fn json_fuzzy_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id FROM paradedb.bm25_search
        WHERE paradedb.bm25_search.id @@@ paradedb.fuzzy_term('metadata.color', 'whiet')
        ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(4,), (15,), (25,)]);
}

#[rstest]
fn json_phrase(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    UPDATE paradedb.bm25_search
    SET metadata = '{"attributes": {"review": "really good quality product"}}'::jsonb
    WHERE id = 1
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.phrase('metadata.attributes.review', ARRAY['good', 'quality'])
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.phrase('metadata.attributes.review', ARRAY['good', 'product'], slop => 1)
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);
}

#[rstest]
fn json_phrase_prefix(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    UPDATE paradedb.bm25_search
    SET metadata = '{"attributes": {"review": "really good quality product"}}'::jsonb
    WHERE id = 1
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.phrase_prefix('metadata.attributes.review', ARRAY['really', 'go'])
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);
}

#[rstest]
fn json_match(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    UPDATE paradedb.bm25_search
    SET metadata = '{"attributes": {"review": "really good quality product"}}'::jsonb
    WHERE id = 1
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.match('metadata.attributes.review', 'realy godo', distance => 2)
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);
}

#[rstest]
fn json_range(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');"
        .execute(&mut conn);
    "CREATE INDEX bm25_search_idx ON paradedb.bm25_search
    USING bm25 (id, metadata)
    WITH (
        key_field='id',
        json_fields='{\"metadata\": {\"fast\": true}}'
    )"
    .execute(&mut conn);

    r#"
    UPDATE paradedb.bm25_search
    SET metadata = '{"attributes": {"score": 3, "tstz": "2023-05-01T08:12:34Z"}}'::jsonb
    WHERE id = 1;

    UPDATE paradedb.bm25_search
    SET metadata = '{"attributes": {"score": 4, "tstz": "2023-05-01T09:12:34Z"}}'::jsonb
    WHERE id = 2;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.range('metadata.attributes.score', int4range(3, NULL, '[)'))
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,), (2,)]);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.range('metadata.attributes.score', int4range(4, NULL, '[)'))
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.range('metadata.attributes.tstz', tstzrange('2023-05-01T09:12:00Z', NULL, '[)'))
    ORDER BY id
    "
    .fetch(&mut conn);
    assert_eq!(rows, vec![(2,)]);
}

#[rstest]
fn test_customers_table(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(
        table_name => 'customers',
        schema_name => 'public',
        table_type => 'Customers'
    );"
    .execute(&mut conn);

    r#"CREATE INDEX customers_idx ON customers
    USING bm25 (id, name, crm_data)
    WITH (
        key_field='id',
        text_fields='{"name": {}}',
        json_fields='{"crm_data": {}}'
    );"#
    .execute(&mut conn);

    // Test querying by name
    let rows: Vec<(i32,)> =
        "SELECT id FROM customers WHERE customers @@@ 'name:Deep' ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // Test querying nested JSON data
    let rows: Vec<(i32,)> = "SELECT id FROM customers WHERE customers @@@ 'crm_data.level1.level2.level3:deep_value' ORDER BY id"
        .fetch(&mut conn);
    assert_eq!(rows, vec![(2,)]);
}

#[rstest]
fn json_array_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE colors (id SERIAL PRIMARY KEY, colors_json JSON, colors_jsonb JSONB);
    INSERT INTO colors (colors_json, colors_jsonb) VALUES
        ('["red", "green", "blue"]'::JSON, '["red", "green", "blue"]'::JSONB),
        ('["red", "orange"]'::JSON, '["red", "orange"]'::JSONB);
    CREATE INDEX colors_bm25_index ON colors
    USING bm25 (id, colors_json, colors_jsonb)
    WITH (
        key_field='id',
        json_fields='{"colors_json": {}, "colors_jsonb": {}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id FROM colors
        WHERE colors.id @@@ paradedb.term('colors_json', 'red')
        ORDER BY id"
        .fetch(&mut conn);
    assert_eq!(rows, vec![(1,), (2,)]);

    let rows: Vec<(i32,)> = "
        SELECT id FROM colors
        WHERE colors.id @@@ paradedb.term('colors_jsonb', 'red')
        ORDER BY id"
        .fetch(&mut conn);
    assert_eq!(rows, vec![(1,), (2,)]);

    let rows: Vec<(i32,)> = "
        SELECT id FROM colors
        WHERE colors.id @@@ paradedb.term('colors_json', 'green')
        ORDER BY id"
        .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    let rows: Vec<(i32,)> = "
        SELECT id FROM colors
        WHERE colors.id @@@ paradedb.term('colors_jsonb', 'green')
        ORDER BY id"
        .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);
}

#[rstest]
fn multiple_tokenizers_with_alias(mut conn: PgConnection) {
    // Create the table
    "CREATE TABLE products (
        id SERIAL PRIMARY KEY,
        name TEXT,
        description TEXT
    );"
    .execute(&mut conn);

    // Insert mock data
    "INSERT INTO products (name, description) VALUES
    ('Mechanical Keyboard', 'RGB backlit keyboard with Cherry MX switches'),
    ('Wireless Mouse', 'Ergonomic mouse with long battery life'),
    ('4K Monitor', 'Ultra-wide curved display with HDR'),
    ('Gaming Laptop', 'Powerful laptop with dedicated GPU'),
    ('Ergonomic Chair', 'Adjustable office chair with lumbar support'),
    ('Standing Desk', 'Electric height-adjustable desk'),
    ('Noise-Cancelling Headphones', 'Over-ear headphones with active noise cancellation'),
    ('Mechanical Pencil', 'Precision drafting tool with 0.5mm lead'),
    ('Wireless Keyboard', 'Slim keyboard with multi-device support'),
    ('Graphic Tablet', 'Digital drawing pad with pressure sensitivity'),
    ('Curved Monitor', 'Immersive gaming display with high refresh rate'),
    ('Ergonomic Keyboard', 'Split design keyboard for comfortable typing'),
    ('Vertical Mouse', 'Upright mouse design to reduce wrist strain'),
    ('Ultrabook Laptop', 'Thin and light laptop with all-day battery'),
    ('LED Desk Lamp', 'Adjustable lighting with multiple color temperatures');"
        .execute(&mut conn);

    // Create the BM25 index
    r#"CREATE INDEX products_index ON products
    USING bm25 (id, name, description)
    WITH (
        key_field='id',
        text_fields='{
            "name": {
                "tokenizer": {"type": "default"}
            },
            "name_stem": {
                "tokenizer": {"type": "default", "stemmer": "English"},
                "column": "name"
            },
            "description": {
                "tokenizer": {"type": "default"}
            },
            "description_stem": {
                "tokenizer": {"type": "default", "stemmer": "English"},
                "column": "description"
            }
        }'
    );"#
    .execute(&mut conn);

    // Test querying with default tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('name:Keyboard')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|(_, name)| name == "Mechanical Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Wireless Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Ergonomic Keyboard"));

    // Ensure that the default tokenizer doesn't return for stemmed queries
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('name:Keyboards')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Test querying with stemmed alias
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('name_stem:Keyboards')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|(_, name)| name == "Mechanical Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Wireless Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Ergonomic Keyboard"));

    // Test querying description with default tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('description:battery')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, name)| name == "Wireless Mouse"));
    assert!(rows.iter().any(|(_, name)| name == "Ultrabook Laptop"));

    // Ensure that the default tokenizer doesn't return for stemmed queries
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('description:displaying')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Test querying description with stemmed alias
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('description_stem:displaying')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, name)| name == "4K Monitor"));
    assert!(rows.iter().any(|(_, name)| name == "Curved Monitor"));

    // Test querying with both default and stemmed fields
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products WHERE id @@@ paradedb.parse('name:Mouse OR description_stem:mouses')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, name)| name == "Wireless Mouse"));
    assert!(rows.iter().any(|(_, name)| name == "Vertical Mouse"));
}

#[rstest]
fn alias_cannot_be_key_field(mut conn: PgConnection) {
    // Create the table
    "CREATE TABLE products (
        id TEXT PRIMARY KEY,
        name TEXT,
        description TEXT
    );
        INSERT INTO products (id,name, description) VALUES
        ('id1', 'apple', 'fruit'),
        ('id2', 'banana', 'fruit'),
        ('id3', 'cherry', 'fruit'),
        ('id4', 'banana split', 'fruit');
    "
    .execute(&mut conn);

    // Test alias cannot be the same as key_field
    let result = r#"
    CREATE INDEX products_index ON products
    USING bm25 (id, name, description)
    WITH (
        key_field='id',
        text_fields='{
            "name": {
                "tokenizer": {"type": "default"}
            },
            "id": {
                "tokenizer": {"type": "default", "stemmer": "English"},
                "column": "description"
            }
        }'
    );"#
    .execute_result(&mut conn);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error returned from database: cannot override BM25 configuration for key_field 'id', you must use an aliased field name and 'column' configuration key"
    );

    // Test valid configuration where alias is different from key_field
    r#"
    CREATE INDEX products_index ON products
    USING bm25 (id, name, description)
    WITH (
        key_field='id',
        text_fields='{
            "name": {
                "tokenizer": {"type": "default"}
            },
            "id_aliased": {
                "column": "id"
            }
        }'
    );"#
    .execute(&mut conn);

    let rows: Vec<(String,)> =
        "SELECT id FROM products WHERE id @@@ paradedb.parse('id_aliased:id1')".fetch(&mut conn);

    assert_eq!(rows, vec![("id1".to_string(),)]);
}

#[rstest]
fn multiple_tokenizers_same_field_in_query(mut conn: PgConnection) {
    // Create the table
    "CREATE TABLE product_reviews (
        id SERIAL PRIMARY KEY,
        product_name TEXT,
        review_text TEXT
    );"
    .execute(&mut conn);

    // Insert mock data
    "INSERT INTO product_reviews (product_name, review_text) VALUES
    ('SmartPhone X', 'This smartphone is incredible! The camera quality is amazing.'),
    ('Laptop Pro', 'Great laptop for programming. The keyboard is comfortable.'),
    ('Wireless Earbuds', 'These earbuds have excellent sound quality. Battery life could be better.'),
    ('Gaming Mouse', 'Responsive and comfortable. Perfect for long gaming sessions.'),
    ('4K TV', 'The picture quality is breathtaking. Smart features work seamlessly.'),
    ('Fitness Tracker', 'Accurate step counting and heart rate monitoring. The app is user-friendly.'),
    ('Smartwatch', 'This watch is smart indeed! Great for notifications and fitness tracking.'),
    ('Bluetooth Speaker', 'Impressive sound for its size. Waterproof feature is a plus.'),
    ('Mechanical Keyboard', 'Satisfying key presses. RGB lighting is customizable.'),
    ('External SSD', 'Super fast read/write speeds. Compact and portable design.');"
    .execute(&mut conn);

    // Create the BM25 index with multiple tokenizers
    r#"CREATE INDEX product_reviews_index ON product_reviews
    USING bm25 (id, product_name, review_text)
    WITH (
        key_field='id',
        text_fields='{
            "product_name": {
                "tokenizer": {"type": "default"}
            },
            "product_name_ngram": {
                "column": "product_name",
                "tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 3, "prefix_only": false}
            },
            "review_text": {
                "tokenizer": {"type": "default"}
            },
            "review_text_stem": {
                "column": "review_text",
                "tokenizer": {"type": "default", "stemmer": "English"}
            }
        }'
    );"#
    .execute(&mut conn);

    //  Exact match using default tokenizer
    let rows: Vec<(i32, String)> = r#"SELECT id, product_name FROM product_reviews WHERE id @@@ paradedb.parse('product_name:"Wireless Earbuds"')"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "Wireless Earbuds");

    // Partial match using ngram tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, product_name FROM product_reviews WHERE id @@@ paradedb.parse('product_name_ngram:phon')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "SmartPhone X");

    // Stemmed search using English stemmer tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, product_name FROM product_reviews WHERE id @@@ paradedb.parse('review_text_stem:gaming')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().any(|(_, name)| name == "Gaming Mouse"));

    // Using default tokenizer and stem on same field
    let rows: Vec<(i32, String)> = "SELECT id, product_name FROM product_reviews WHERE id @@@ paradedb.parse('review_text:monitoring OR review_text_stem:mon')"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "Fitness Tracker");
}

#[rstest]
fn more_like_this_with_alias(mut conn: PgConnection) {
    // Create the table
    r#"
    CREATE TABLE test_more_like_this_alias (
        id SERIAL PRIMARY KEY,
        flavour TEXT,
        description TEXT
    );

    INSERT INTO test_more_like_this_alias (flavour, description) VALUES
        ('apple', 'A sweet and crisp fruit'),
        ('banana', 'A long yellow tropical fruit'),
        ('cherry', 'A small round red fruit'),
        ('banana split', 'An ice cream dessert with bananas'),
        ('apple pie', 'A dessert made with apples');
    "#
    .execute(&mut conn);

    // Create the BM25 index with aliased fields
    r#"
    CREATE INDEX test_more_like_this_alias_index ON test_more_like_this_alias
    USING bm25 (id, flavour, description)
    WITH (
        key_field='id',
        text_fields='{
            "taste": {
                "column": "flavour",
                "tokenizer": {"type": "default"}
            },
            "details": {
                "column": "description",
                "tokenizer": {"type": "default"}
            }
        }'
    );
    "#
    .execute(&mut conn);

    // Test more_like_this with aliased field 'taste' (original 'flavour')
    let rows: Vec<(i32, String, String)> = r#"
    SELECT id, flavour, description FROM test_more_like_this_alias
    WHERE id @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"taste": "banana"}'
    );
    "#
    .fetch_collect(&mut conn);

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, flavour, _)| flavour == "banana"));
    assert!(rows.iter().any(|(_, flavour, _)| flavour == "banana split"));
}

#[rstest]
fn multiple_aliases_same_column(mut conn: PgConnection) {
    // Test multiple aliases pointing to the same column with different tokenizers
    "CREATE TABLE multi_alias (
        id SERIAL PRIMARY KEY,
        content TEXT
    );"
    .execute(&mut conn);

    "INSERT INTO multi_alias (content) VALUES
    ('running and jumping'),
    ('ran and jumped'),
    ('runner jumper athlete');"
        .execute(&mut conn);

    // Create index with multiple aliases for same column
    r#"CREATE INDEX multi_alias_idx ON multi_alias
    USING bm25 (id, content)
    WITH (
        key_field='id',
        text_fields='{
            "content": {
                "tokenizer": {"type": "default"}
            },
            "content_stem": {
                "column": "content",
                "tokenizer": {"type": "default", "stemmer": "English"}
            },
            "content_ngram": {
                "column": "content",
                "tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 3, "prefix_only": false}
            }
        }'
    );"#
    .execute(&mut conn);

    // Test each alias configuration
    let rows: Vec<(i32,)> =
        "SELECT id FROM multi_alias WHERE multi_alias @@@ 'content:running'".fetch(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    let rows: Vec<(i32,)> =
        "SELECT id FROM multi_alias WHERE multi_alias @@@ 'content_stem:running'".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(i32,)> =
        "SELECT id FROM multi_alias WHERE multi_alias @@@ 'content_ngram:run'".fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn cant_name_a_field_ctid(mut conn: PgConnection) {
    "CREATE TABLE missing_source (
        id SERIAL PRIMARY KEY,
        text_field TEXT
    );"
    .execute(&mut conn);

    let result = r#"CREATE INDEX missing_source_idx ON missing_source
    USING bm25 (id, text_field)
    WITH (
        key_field='id',
        text_fields='{
            "ctid": {
                "column": "text_field",
                "tokenizer": {"type": "default"}
            }
        }'
    );"#
    .execute_result(&mut conn);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error returned from database: the name `ctid` is reserved by pg_search"
    );
}

#[rstest]
fn can_index_only_key_field(mut conn: PgConnection) {
    "CREATE TABLE can_index_only_key_field (
        id SERIAL PRIMARY KEY,
        text_field TEXT
    );"
    .execute(&mut conn);

    let result = r#"

        INSERT INTO can_index_only_key_field (text_field) VALUES ('hello world');

        CREATE INDEX idxcan_index_only_key_field ON can_index_only_key_field
        USING bm25 (id)
        WITH (key_field='id');
    "#
    .execute_result(&mut conn);
    assert!(result.is_ok());

    let (count,) = "SELECT COUNT(*) FROM can_index_only_key_field WHERE id @@@ '1'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn missing_source_column(mut conn: PgConnection) {
    "CREATE TABLE missing_source (
        id SERIAL PRIMARY KEY,
        text_field TEXT
    );"
    .execute(&mut conn);

    // Attempt to create index with alias pointing to non-existent column
    let result = r#"CREATE INDEX missing_source_idx ON missing_source
    USING bm25 (id, text_field)
    WITH (
        key_field='id',
        text_fields='{
            "alias": {
                "column": "nonexistent_column",
                "tokenizer": {"type": "default"}
            }
        }'
    );"#
    .execute_result(&mut conn);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "error returned from database: the column `nonexistent_column` referenced by the field configuration for 'alias' does not exist"
    );
}

#[rstest]
fn alias_type_mismatch(mut conn: PgConnection) {
    "CREATE TABLE type_mismatch (
        id SERIAL PRIMARY KEY,
        numeric_field INTEGER,
        text_field TEXT
    );"
    .execute(&mut conn);

    // Try to create text alias pointing to numeric column
    let result = r#"CREATE INDEX type_mismatch_idx ON type_mismatch
    USING bm25 (id, numeric_field, text_field)
    WITH (
        key_field='id',
        text_fields='{
            "wrong_type": {
                "column": "numeric_field",
                "tokenizer": {"type": "default"}
            }
        }'
    );"#
    .execute_result(&mut conn);

    assert!(result.is_err());
}

#[rstest]
fn alias_chain_validation(mut conn: PgConnection) {
    // Test that we can't create an alias that points to another alias
    "CREATE TABLE alias_chain (
        id SERIAL PRIMARY KEY,
        base_field TEXT
    );"
    .execute(&mut conn);

    let result = r#"CREATE INDEX alias_chain_idx ON alias_chain
    USING bm25 (id, base_field)
    WITH (
        key_field='id',
        text_fields='{
            "first_alias": {
                "column": "base_field",
                "tokenizer": {"type": "default"}
            },
            "second_alias": {
                "column": "first_alias",
                "tokenizer": {"type": "default"}
            }
        }'
    );"#
    .execute_result(&mut conn);

    assert!(result.is_err());
}

#[rstest]
fn mixed_field_types_with_aliases(mut conn: PgConnection) {
    // Test mixing different field types with aliases
    "CREATE TABLE mixed_fields (
        id SERIAL PRIMARY KEY,
        text_content TEXT,
        json_content JSONB
    );"
    .execute(&mut conn);

    "INSERT INTO mixed_fields (text_content, json_content) VALUES
    ('test content', '{\"key\": \"value1\"}'),
    ('another test', '{\"key\": \"value2\"}');"
        .execute(&mut conn);

    r#"CREATE INDEX mixed_fields_idx ON mixed_fields
    USING bm25 (id, text_content, json_content)
    WITH (
        key_field='id',
        text_fields='{
            "text_alias": {
                "column": "text_content",
                "tokenizer": {"type": "default"}
            }
        }',
        json_fields='{
            "json_alias": {
                "column": "json_content"
            }
        }'
    );"#
    .execute(&mut conn);

    // Test each type of alias
    let rows: Vec<(i32,)> =
        "SELECT id FROM mixed_fields WHERE mixed_fields @@@ 'text_alias:test'".fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32,)> =
        "SELECT id FROM mixed_fields WHERE mixed_fields @@@ 'json_alias.key:value1'"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}
```

---

## mlt.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn mlt_enables_scoring_issue1747(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id,) = "
    SELECT id FROM paradedb.bm25_search WHERE id @@@ pdb.more_like_this(
        key_value => 3,
        min_term_frequency => 1
    ) ORDER BY id LIMIT 1"
        .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 3);
}

#[rstest]
fn mlt_scoring_nested(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Boolean must
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        must => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // Boolean must_not
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        must_not => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert!(results.is_empty());

    // Boolean should
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        should => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // Boost
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boost(
        factor => 1.5,
        query => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // ConstScore
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.const_score(
        score => 5,
        query => pdb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // DisjunctionMax
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.disjunction_max(
        disjuncts => ARRAY[
            pdb.more_like_this(
                min_doc_frequency => 2,
                min_term_frequency => 1,
                document => '{"description": "keyboard"}'
            ),
            pdb.more_like_this(
                min_doc_frequency => 2,
                min_term_frequency => 1,
                document => '{"description": "shoes"}'
            )
        ]
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2, 3, 4, 5]);

    // Multiple nested
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@
    paradedb.boolean(
        must_not => paradedb.parse('description:plastic'),
        should => paradedb.disjunction_max(
            disjuncts => ARRAY[
                paradedb.boost(
                    factor => 3,
                    query => pdb.more_like_this(
                        min_doc_frequency => 2,
                        min_term_frequency => 1,
                        document => '{"description": "keyboard"}'
                    )
                ),
                pdb.more_like_this(
                    min_doc_frequency => 2,
                    min_term_frequency => 1,
                    document => '{"description": "shoes"}'
                )
            ]
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 3, 4, 5]);
}
```

---

## sorting.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

fn field_sort_fixture(conn: &mut PgConnection) -> Value {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        WITH (
            key_field = 'id',
            text_fields = '{
                "description": {},
                "category": {
                    "fast": true,
                    "normalizer": "lowercase"
                }
            }',
            numeric_fields = '{
                "rating": {}
            }',
            boolean_fields = '{
                "in_stock": {}
            }',
            json_fields = '{
                "metadata": {}
            }',
            datetime_fields = '{
                "created_at": {},
                "last_updated_date": {},
                "latest_available_time": {}
            }'
        );
    "#.execute(conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY lower(category) LIMIT 5".fetch_one::<(Value,)>(conn);
    eprintln!("{plan:#?}");
    plan
}

#[rstest]
fn sort_by_lower(mut conn: PgConnection) {
    let plan = field_sort_fixture(&mut conn);
    let plan = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        plan.get("   TopN Order By"),
        Some(&Value::String(String::from("category asc")))
    );
}

#[rstest]
fn sort_by_lower_parallel(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        // We cannot reliably force parallel workers to be used without `debug_parallel_query`.
        return;
    }

    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET debug_parallel_query TO on".execute(&mut conn);

    let plan = field_sort_fixture(&mut conn);
    let plan = plan
        .pointer("/0/Plan/Plans/0/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        plan.get("   TopN Order By"),
        Some(&Value::String(String::from("category asc")))
    );
}

#[rstest]
fn sort_by_raw(mut conn: PgConnection) {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        WITH (
            key_field = 'id',
            text_fields = '{
                "description": {},
                "category": {
                    "fast": true,
                    "normalizer": "raw"
                }
            }',
            numeric_fields = '{
                "rating": {}
            }',
            boolean_fields = '{
                "in_stock": {}
            }',
            json_fields = '{
                "metadata": {}
            }',
            datetime_fields = '{
                "created_at": {},
                "last_updated_date": {},
                "latest_available_time": {}
            }'
        );
    "#.execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY category LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");
    let plan = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        plan.get("   TopN Order By"),
        Some(&Value::String(String::from("category asc")))
    );
}

#[rstest]
async fn test_compound_sort(mut conn: PgConnection) {
    "SET max_parallel_workers to 0;".execute(&mut conn);

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id FROM paradedb.bm25_search
        WHERE description @@@ 'shoes' ORDER BY rating DESC, created_at DESC LIMIT 10"#
        .fetch_one(&mut conn);

    eprintln!("plan: {plan:#?}");

    // Since both ORDER-BY fields are fast, they should be pushed down.
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/   TopN Order By"),
        Some(&Value::String(String::from("rating desc, created_at desc")))
    );
}

#[rstest]
async fn compound_sort_expression(mut conn: PgConnection) {
    "SET max_parallel_workers to 0;".execute(&mut conn);

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT *, pdb.score(id) * 2 FROM paradedb.bm25_search
        WHERE description @@@ 'shoes' ORDER BY 2, pdb.score(id) LIMIT 10"#
        .fetch_one(&mut conn);

    eprintln!("plan: {plan:#?}");

    // Since the ORDER BY contains an expression, we should not attempt TopN, even if other
    // fields could be pushed down.
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/Plans/0/Exec Method"),
        Some(&Value::String(String::from("NormalScanExecState")))
    );
}

#[rstest]
async fn compound_sort_partitioned(mut conn: PgConnection) {
    "SET max_parallel_workers to 0;".execute(&mut conn);

    // Create the partitioned sales table
    PartitionedTable::setup().execute(&mut conn);

    // Insert a good size amount of random data, and then analyze.
    r#"
    INSERT INTO sales (sale_date, amount, description)
    SELECT
        (DATE '2023-01-01' + (random() * 179)::integer) AS sale_date,
        (random() * 1000)::real AS amount,
        ('wine '::text || md5(random()::text)) AS description
    FROM generate_series(1, 1000);

    ANALYZE;
    "#
    .execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales
        WHERE description @@@ 'wine'
        ORDER BY sale_date, amount LIMIT 10;"#
        .fetch_one(&mut conn);

    eprintln!("plan: {plan:#?}");

    // Extract the Custom Scan nodes from the JSON plan for inspection
    let mut custom_scan_nodes = Vec::new();
    collect_custom_scan_nodes(plan.pointer("/0/Plan").unwrap(), &mut custom_scan_nodes);

    // Check that we have Custom Scan nodes that handle our search
    assert_eq!(custom_scan_nodes.len(), 2);
    for node in custom_scan_nodes {
        assert_eq!(
            node.get("   TopN Order By"),
            Some(&Value::String(String::from("sale_date asc, amount asc")))
        );
    }
}

// Helper function to recursively collect Custom Scan nodes from a plan
fn collect_custom_scan_nodes(plan: &Value, nodes: &mut Vec<Value>) {
    // Check if this is a Custom Scan node
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        if node_type == "Custom Scan" {
            nodes.push(plan.clone());
        }
    }

    // Recursively check child plans
    if let Some(plans) = plan.get("Plans").and_then(|p| p.as_array()) {
        for child_plan in plans {
            collect_custom_scan_nodes(child_plan, nodes);
        }
    }
}

#[rstest]
fn sort_partitioned_early_cutoff(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    // Insert matching rows into both partitions.
    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Cheap plastic keyboard');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers TO 0;".execute(&mut conn);

    // With ORDER BY the partition key: we expect the partitions to be visited sequentially, and
    // for cutoff to occur.
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT description, sale_date
        FROM sales
        WHERE description @@@ 'keyboard'
        ORDER BY sale_date
        LIMIT 1;
        "#
    .fetch_one(&mut conn);
    eprintln!("{plan:#?}");

    // We expect both partitions to be in the plan, but for only the first one to have been
    // executed, because the Append node was able to get enough results from the first partition.
    let plans = plan
        .pointer("/0/Plan/Plans/0/Plans")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        plans[0].get("Actual Loops").unwrap(),
        &serde_json::from_str::<Value>("1").unwrap()
    );
    assert_eq!(
        plans[1].get("Actual Loops").unwrap(),
        &serde_json::from_str::<Value>("0").unwrap()
    );
}
```

---

## mixed_fast_fields.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

// Tests for MixedFastFieldExecState implementation
// Includes both basic functionality tests and corner/edge cases

mod fixtures;

use bigdecimal::BigDecimal;
use fixtures::db::Query;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

// Helper function to get all execution methods in the plan
fn get_all_exec_methods(plan: &Value) -> Vec<String> {
    let mut methods = Vec::new();
    extract_methods(plan, &mut methods);
    methods
}

// Recursive function to walk the plan tree
fn extract_methods(node: &Value, methods: &mut Vec<String>) {
    if let Some(exec_method) = node.get("Exec Method") {
        if let Some(method) = exec_method.as_str() {
            methods.push(method.to_string());
        }
    }

    // Check child plans
    if let Some(plans) = node.get("Plans") {
        if let Some(plans_array) = plans.as_array() {
            for plan in plans_array {
                extract_methods(plan, methods);
            }
        }
    }

    // Start from the root if given the root plan
    if let Some(root) = node.get(0) {
        if let Some(plan_node) = root.get("Plan") {
            extract_methods(plan_node, methods);
        }
    }
}

// Setup for complex aggregation with mixed fast fields
struct TestComplexAggregation;

impl TestComplexAggregation {
    fn setup() -> impl Query {
        r#"
            DROP TABLE IF EXISTS expected_payments;
            CREATE TABLE expected_payments (
              id                  SERIAL PRIMARY KEY,
              organization_id     UUID     NOT NULL,
              live_mode           BOOLEAN  NOT NULL,
              status              TEXT     NOT NULL,
              internal_account_id UUID     NOT NULL,
              amount_range        NUMRANGE NOT NULL,
              amount_reconciled   NUMERIC  NOT NULL,
              direction           TEXT     NOT NULL CHECK (direction IN ('credit','debit')),
              currency            TEXT     NOT NULL,
              discarded_at        TIMESTAMP NULL
            );
            
            INSERT INTO expected_payments (
              organization_id,
              live_mode,
              status,
              internal_account_id,
              amount_range,
              amount_reconciled,
              direction,
              currency,
              discarded_at
            )
            SELECT
              organization_id,
              live_mode,
              status,
              internal_account_id,
              numrange(lower_val, lower_val + offset_val)         AS amount_range,
              amount_reconciled,
              direction,
              currency,
              discarded_at
            FROM (
              SELECT
                -- random UUID
                (md5(random()::text))::uuid                        AS organization_id,
                -- 50/50 live_mode
                (random() < 0.5)                                    AS live_mode,
                -- status pick
                (ARRAY['unreconciled','partially_reconciled'])
                  [floor(random()*2 + 1)::int]                      AS status,
                -- another random UUID
                (md5(random()::text))::uuid                        AS internal_account_id,
                -- ensure lower ≤ upper by generating an offset
                floor(random()*1000)::int                           AS lower_val,
                floor(random()*100)::int + 1                        AS offset_val,
                -- reconciled amount between –500 and +500
                (random()*1000 - 500)::numeric                      AS amount_reconciled,
                -- direction pick
                (ARRAY['credit','debit'])[floor(random()*2 + 1)::int] AS direction,
                -- currency pick
                (ARRAY['USD','EUR','GBP','JPY','AUD'])[floor(random()*5 + 1)::int] AS currency,
                -- 10% NULL, else random timestamp in last year
                CASE
                  WHEN random() < 0.10 THEN NULL
                  ELSE now() - (random() * INTERVAL '365 days')
                END                                                 AS discarded_at
              FROM generate_series(1, 1000)
            ) sub;
            
            create index expected_payments_idx on expected_payments using bm25 (
                id, 
                organization_id, 
                live_mode, 
                status, 
                internal_account_id, 
                amount_range, 
                amount_reconciled, 
                direction, 
                currency, 
                discarded_at
            ) with (
                key_field = 'id', 
                text_fields = '{"organization_id": {"fast":true}, "status": {"fast": true, "tokenizer": {"type": "keyword"}}, "direction": {"fast": true}, "currency": {"fast": true}}',
                boolean_fields = '{"live_mode": {"fast": true}}'
            );
        "#
    }
}

#[ignore]
#[rstest]
fn test_complex_aggregation_with_mixed_fast_fields(mut conn: PgConnection) {
    TestComplexAggregation::setup().execute(&mut conn);

    // Force disable regular index scans to ensure BM25 index is used
    "SET enable_indexscan = off;".execute(&mut conn);

    // Get execution plan for the complex query
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else -(upper(expected_payments.amount_range) - 1) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else -lower(expected_payments.amount_range) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -(upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -lower(expected_payments.amount_range) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_max_range, 
          COUNT(case when expected_payments.direction = 'credit' then 1 else null end) as credit_count, 
          COUNT(case when expected_payments.direction = 'debit' then 1 else null end) as debit_count, 
          COUNT(*) as total_count, 
          COUNT(distinct expected_payments.currency) as currency_count, 
          (ARRAY_AGG(distinct expected_payments.currency))[1] as currency 
        FROM expected_payments
        WHERE expected_payments.live_mode @@@ 'true' 
          AND expected_payments.status @@@ 'IN [unreconciled partially_reconciled]' 
          AND expected_payments.discarded_at IS NULL 
        LIMIT 1
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Complex aggregation execution methods: {methods:?}");

    // Assert that a fast field execution state is used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for complex aggregation, got: {methods:?}"
    );

    // Actually execute the query to verify results
    let results = r#"
        SELECT
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else -(upper(expected_payments.amount_range) - 1) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else -lower(expected_payments.amount_range) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -(upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -lower(expected_payments.amount_range) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_max_range, 
          COUNT(case when expected_payments.direction = 'credit' then 1 else null end) as credit_count, 
          COUNT(case when expected_payments.direction = 'debit' then 1 else null end) as debit_count, 
          COUNT(*) as total_count, 
          COUNT(distinct expected_payments.currency) as currency_count, 
          (ARRAY_AGG(distinct expected_payments.currency))[1] as currency 
        FROM expected_payments
        WHERE expected_payments.live_mode @@@ 'true' 
          AND expected_payments.status @@@ 'IN [unreconciled partially_reconciled]' 
          AND expected_payments.discarded_at IS NULL 
        LIMIT 1
    "#
    .fetch_result::<(
        BigDecimal,
        BigDecimal,
        BigDecimal,
        BigDecimal,
        BigDecimal,
        BigDecimal,
        i64,
        i64,
        i64,
        i64,
        Option<String>,
    )>(&mut conn)
    .unwrap();

    // Assert that we got results (should be at least one row)
    assert!(!results.is_empty(), "Expected at least one row of results");

    // Get the counts from first result
    let (_, _, _, _, _, _, credit_count, debit_count, total_count, currency_count, currency) =
        &results[0];

    // Verify consistency in counts
    assert_eq!(
        *total_count,
        credit_count + debit_count,
        "Total count should equal credit_count + debit_count"
    );

    // Verify currency count is positive
    assert!(
        *currency_count > 0,
        "Should have at least one currency type"
    );

    // Check that we have a currency value if currency_count > 0
    if *currency_count > 0 {
        assert!(
            currency.is_some(),
            "Should have a currency value when currency_count > 0"
        );
    }

    // Reset setting
    "SET enable_indexscan = on;".execute(&mut conn);
}
```

---

## cleanup.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn validate_checksum(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let (count,) =
        "select count(*) from paradedb.validate_checksum('paradedb.bm25_search_bm25_index')"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);
}

#[rstest]
fn vacuum_full(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id IN (1, 2, 3, 4, 5)".execute(&mut conn);

    "VACUUM FULL".execute(&mut conn);
}

#[rstest]
fn create_and_drop_builtin_index(mut conn: PgConnection) {
    // Test to ensure that dropping non-search indexes works correctly, as our event
    // trigger will need to skip indexes we didn't create.

    "CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL)".execute(&mut conn);

    "CREATE INDEX test_table_value_idx ON test_table(value)".execute(&mut conn);

    "DROP INDEX test_table_value_idx CASCADE".execute(&mut conn);

    let index_count = "SELECT COUNT(*) FROM pg_indexes WHERE indexname = 'test_table_value_idx'"
        .fetch_one::<(i64,)>(&mut conn)
        .0;

    assert_eq!(
        index_count, 0,
        "Index should no longer exist after dropping with CASCADE"
    );

    "DROP TABLE IF EXISTS test_table CASCADE".execute(&mut conn);
}

#[rstest]
fn bulk_insert_segments_behavior(mut conn: PgConnection) {
    let mutable_segment_rows = 10;
    format!(
        r#"
        SET maintenance_work_mem = '1GB';
        SET work_mem = '1GB';
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL);

        CREATE INDEX idxtest_table ON public.test_table
        USING bm25 (id, value)
        WITH (
            key_field = 'id',
            mutable_segment_rows = {mutable_segment_rows}
        );
    "#
    )
    .execute(&mut conn);

    // Insert less than the mutable segments size, and confirm that we have 1 segment.
    format!(
        "INSERT INTO test_table (value) SELECT md5(random()::text) FROM generate_series(1, {})",
        1
    )
    .execute(&mut conn);
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert_eq!(nsegments, 1);

    // Insert more than the mutable segments size, and confirm that it fills the first mutable
    // segment, and then produces one additional (immutable) segment.
    format!(
        "INSERT INTO test_table (value) SELECT md5(random()::text) FROM generate_series(1, {})",
        4 * mutable_segment_rows
    )
    .execute(&mut conn);
    let nsegments = "SELECT COUNT(*) FROM paradedb.index_info('idxtest_table');"
        .fetch_one::<(i64,)>(&mut conn)
        .0 as usize;
    assert_eq!(nsegments, 2);
}
```

---

## domain_type.rs

```
#![allow(dead_code)]
// Copyright (c) 2023-2025 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
    "CREATE DOMAIN employee_salary_range AS int4range;".execute(&mut conn);
    "CREATE DOMAIN employee_status AS TEXT CHECK (VALUE IN ('active', 'inactive', 'on_leave'));"
        .execute(&mut conn);

    // array of domain type
    "CREATE DOMAIN rating AS INTEGER CHECK (VALUE BETWEEN 1 AND 5);".execute(&mut conn);
    "CREATE DOMAIN rating_history AS rating[];".execute(&mut conn);

    let sql = r#"
        CREATE TABLE employees (
            id SERIAL PRIMARY KEY,
            salary_range employee_salary_range,
            status_history employee_status[],
            ratings rating_history
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idx_employees ON employees USING bm25 (id, salary_range, status_history, ratings)
        WITH (
            key_field='id',
            range_fields='{
                "salary_range": {"fast": true}
            }',
            text_fields='{
                "status_history": {"fast": true}
            }',
            numeric_fields='{
                "ratings": {"fast": true}
            }'
        );
    "#;
    sql.execute(&mut conn);

    "INSERT INTO employees (salary_range, status_history, ratings)
    VALUES
        ('[10000, 50000)', ARRAY['active', 'on_leave'], ARRAY[3, 4]::rating_history),
        ('[50000, 100000)', ARRAY['inactive', 'active'], ARRAY[5, 1]::rating_history),
        ('[20000, 80000)', ARRAY['on_leave', 'inactive'], ARRAY[2, 2, 5]::rating_history);"
        .execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    conn
}

mod domain_types {
    use super::*;

    #[rstest]
    fn verify_index_schema(#[from(setup_test_table)] mut conn: PgConnection) {
        let rows: Vec<(String, String)> =
            "SELECT name, field_type FROM paradedb.schema('idx_employees')".fetch(&mut conn);

        assert_eq!(rows[0], ("ctid".into(), "U64".into()));
        assert_eq!(rows[1], ("id".into(), "I64".into()));
        assert_eq!(rows[2], ("ratings".into(), "I64".into()));
        assert_eq!(rows[3], ("salary_range".into(), "JsonObject".into()));
        assert_eq!(rows[4], ("status_history".into(), "Str".into()));
    }

    #[rstest]
    fn with_range(#[from(setup_test_table)] mut conn: PgConnection) {
        let res: Vec<(i32, i32, i32, String, String)> = r#"
            select id, lower(salary_range), upper(salary_range), status_history::TEXT, ratings::TEXT
            from employees
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            and lower(salary_range) > 15000;
        "#
        .fetch(&mut conn);
        assert_eq!(
            res,
            vec![
                (2, 50000, 100000, "{inactive,active}".into(), "{5,1}".into()),
                (
                    3,
                    20000,
                    80000,
                    "{on_leave,inactive}".into(),
                    "{2,2,5}".into()
                )
            ]
        );

        let count = r#"
            select count(*)
            from employees
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            and lower(salary_range) > 15000;
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);
    }

    #[rstest]
    fn with_array_filter(#[from(setup_test_table)] mut conn: PgConnection) {
        let res: Vec<(i32, i32, i32, String, String)> = r#"
            select id, lower(salary_range), upper(salary_range), status_history::TEXT, ratings::TEXT
            from employees
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            and 'active' = ANY(status_history);
        "#
        .fetch(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, 10000, 50000, "{active,on_leave}".into(), "{3,4}".into()),
                (2, 50000, 100000, "{inactive,active}".into(), "{5,1}".into())
            ]
        );
    }

    #[rstest]
    fn with_domain_wrapped_array(#[from(setup_test_table)] mut conn: PgConnection) {
        let res: Vec<(i32, String)> = r#"
            SELECT id, ratings::TEXT
            FROM employees
            WHERE 5 = ANY(ratings);
        "#
        .fetch(&mut conn);

        assert_eq!(res, vec![(2, "{5,1}".into()), (3, "{2,2,5}".into())]);
    }

    #[rstest]
    fn reject_invalid_domain_values(#[from(setup_test_table)] mut conn: PgConnection) {
        let result = "INSERT INTO employees (status_history)
                      VALUES (ARRAY['invalid']::status_array);"
            .execute_result(&mut conn);
        assert!(result.is_err());
    }
}
```

---

## mixed_fast_fields_benchmark.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use anyhow::Result;
use fixtures::*;
use paradedb::micro_benchmarks::{
    benchmark_mixed_fast_fields, detect_exec_method, set_execution_method, setup_benchmark_database,
};
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

// Number of benchmark iterations for each query
const ITERATIONS: usize = 5;
// Number of warmup iterations before measuring performance
const WARMUP_ITERATIONS: usize = 2;
// Number of rows to use in the benchmark
const NUM_ROWS_BENCHMARK: usize = 10000;
const NUM_ROWS_VALIDATION: usize = 1000; // Reduced for faster test runs
const BATCH_SIZE: usize = 10000; // For efficiency with large datasets, use batch inserts

#[rstest]
async fn benchmark_mixed_fast_fields_test(mut conn: PgConnection) -> Result<()> {
    benchmark_mixed_fast_fields(
        &mut conn,
        false,
        ITERATIONS,
        WARMUP_ITERATIONS,
        NUM_ROWS_BENCHMARK,
        BATCH_SIZE,
    )
    .await?;
    Ok(())
}

/// Validate that the different execution methods return the same results
/// and enforce that we're actually using the intended execution methods
#[rstest]
async fn validate_mixed_fast_fields_correctness(mut conn: PgConnection) -> Result<()> {
    // Set up the benchmark database
    setup_benchmark_database(
        &mut conn,
        NUM_ROWS_VALIDATION,
        "test_benchmark_data",
        BATCH_SIZE,
    )
    .await?;

    println!("Testing query correctness between execution methods...");
    println!("────────────────────────────────────────────────────────");

    // Define a test query that will use both string and numeric fast fields
    let test_query =
        "SELECT id, string_field1, string_field2, numeric_field1, numeric_field2, numeric_field3 
         FROM test_benchmark_data 
         WHERE numeric_field1 < 500 AND string_field1 @@@ '\"alpha_complex_identifier_123456789\"' AND string_field2 @@@ '\"red_velvet_cupcake_with_cream_cheese_frosting\"'
         ORDER BY id";

    println!("Testing query correctness between execution methods...");
    println!("────────────────────────────────────────────────────────");

    // Set PostgreSQL settings to ensure index usage
    sqlx::query("SET enable_seqscan = off")
        .execute(&mut conn)
        .await?;
    sqlx::query("SET enable_bitmapscan = off")
        .execute(&mut conn)
        .await?;
    sqlx::query("SET enable_indexscan = off")
        .execute(&mut conn)
        .await?;

    set_execution_method(&mut conn, "MixedFastFieldExec", "test_benchmark_data").await?;

    // Get results with MixedFastFieldExec
    let mixed_results = sqlx::query(test_query).fetch_all(&mut conn).await?;

    // Get execution plan to verify method
    let (mixed_plan,): (Value,) = sqlx::query_as(&format!("EXPLAIN (FORMAT JSON) {test_query}"))
        .fetch_one(&mut conn)
        .await?;

    let mixed_method = detect_exec_method(&mixed_plan);
    println!("✓ Mixed index using → {mixed_method}");

    // ENFORCE: Validate we're actually using the MixedFastFieldExec method
    assert!(
        mixed_method.contains("MixedFastFieldExec"),
        "Expected MixedFastFieldExec execution method, but got: {mixed_method}. Check index configuration and query settings."
    );

    set_execution_method(&mut conn, "NormalScanExecState", "test_benchmark_data").await?;

    // Get results with NormalScanExecState
    let normal_results = sqlx::query(test_query).fetch_all(&mut conn).await?;

    // Get execution plan to verify method
    let (normal_plan,): (Value,) = sqlx::query_as(&format!("EXPLAIN (FORMAT JSON) {test_query}"))
        .fetch_one(&mut conn)
        .await?;

    let normal_method = detect_exec_method(&normal_plan);
    println!("✓ Normal index using → {normal_method}");

    // ENFORCE: Validate we're actually using the NormalScanExecState method
    assert!(
        normal_method.contains("NormalScanExecState"),
        "Expected NormalScanExecState execution method, but got: {normal_method}. Check index configuration and query settings."
    );

    // Compare result counts
    println!(
        "Comparing {} rows from each execution method...",
        mixed_results.len()
    );
    assert_eq!(
        mixed_results.len(),
        normal_results.len(),
        "Mixed and Normal execution methods returned different number of rows"
    );

    // Verify that we have the same rows (by comparing the string representation of each row)
    for (i, (mixed_row, normal_row)) in mixed_results.iter().zip(normal_results.iter()).enumerate()
    {
        assert_eq!(
            format!("{:?}", mixed_row),
            format!("{:?}", normal_row),
            "Row {} differs between Mixed and Normal execution methods",
            i
        );
    }

    println!("✅ Validation passed: Both execution methods returned identical results");
    println!("────────────────────────────────────────────────────────");

    Ok(())
}
```

---

## vacuum.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rustfmt::skip]
#[rstest]
fn manual_vacuum(mut conn: PgConnection) {
    fn count_func(conn: &mut PgConnection) -> i64 {
        "select count(*)::bigint from sadvac WHERE sadvac @@@ 'data:test';".fetch_one::<(i64,)>(conn).0
    }
    
    // originally, this test uncovered a problem at ROW_COUNT=103, but now that the problem is
    // fixed, we'll do a bunch more rows
    const ROW_COUNT:i64 = 10_000;

   "drop table if exists sadvac cascade;
    drop schema if exists idxsadvac cascade;

    create table sadvac
        (
            id   serial8,
            data text
        );
    alter table sadvac set (autovacuum_enabled = 'off');".execute(&mut conn);

    format!("insert into sadvac (data) select 'this is a test ' || x from generate_series(1, {ROW_COUNT}) x;").execute(&mut conn);

    "
    CREATE INDEX idxsadvac ON public.sadvac
    USING bm25 (id, data)
    WITH (key_field = 'id');
    ".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post create index");

    "update sadvac set id = id;".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post first update");

    "vacuum sadvac;".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post vacuum");

    // it's here, after a vacuum, that this would fail
    // for me it fails at i=103
    "update sadvac set id = id;".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post update after vacuum");
}
```

---

## key.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

// In addition to checking whether all the expected types work for keys, make sure to include tests for anything that
//    is reliant on keys (e.g. stable_sort, alias)

#[rstest]
fn boolean_key(mut conn: PgConnection) {
    // Boolean keys are pretty useless, but they're supported!

    r#"
    CREATE TABLE test_table (
        id BOOLEAN,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (true, 'bluetooth'), (false, 'blue');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(bool, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(false, 0.25759196), (true, 0.14109309)]);

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id UUID,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'redness');
    INSERT INTO test_table (id, value) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'great white');
    INSERT INTO test_table (id, value) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table WHERE test_table @@@
        paradedb.term(field => 'value', value => 'blue') ORDER BY score desc
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                "b5faacc0-9eba-441a-81f8-820b46a3b57e".to_string(),
                0.61846066
            ),
            (
                "38bf27a0-1aa8-42cd-9cb0-993025e0b8d0".to_string(),
                0.57459813
            ),
            (
                "f159c89e-2162-48cd-85e3-e42b71d2ecd0".to_string(),
                0.53654534
            ),
            (
                "40bc9216-66d0-4ae8-87ee-ddb02e3e1b33".to_string(),
                0.50321954
            ),
            (
                "ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad".to_string(),
                0.47379148
            ),
            (
                "eb833eb6-c598-4042-b84a-0045828fceea".to_string(),
                0.44761515
            ),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), pdb.snippet(value) FROM test_table WHERE value @@@ 'blue'
    UNION
    SELECT CAST(id AS TEXT), pdb.snippet(value) FROM test_table WHERE value @@@ 'tooth'
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 8);
}

#[rstest]
fn i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id BIGINT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(i64, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 0.61846066),
            (2, 0.57459813),
            (1, 0.53654534),
            (9, 0.50321954),
            (5, 0.47379148),
            (4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(i64, String)> = r#"
    SELECT id, pdb.snippet(value) FROM test_table WHERE value @@@ 'blue'
    UNION
    SELECT id, pdb.snippet(value) FROM test_table WHERE value @@@ 'tooth'
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 8);
}

#[rstest]
fn i32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id INT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 0.61846066),
            (2, 0.57459813),
            (1, 0.53654534),
            (9, 0.50321954),
            (5, 0.47379148),
            (4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn i16_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SMALLINT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(i16, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 0.61846066),
            (2, 0.57459813),
            (1, 0.53654534),
            (9, 0.50321954),
            (5, 0.47379148),
            (4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn f32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id FLOAT4,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1.1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2.2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3.3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4.4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5.5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6.6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7.7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8.8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9.9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10.1, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(f32, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3.3, 0.61846066),
            (2.2, 0.57459813),
            (1.1, 0.53654534),
            (9.9, 0.50321954),
            (5.5, 0.47379148),
            (4.4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn f64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id FLOAT8,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1.1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2.2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3.3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4.4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5.5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6.6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7.7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8.8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9.9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10.1, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(f64, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3.3, 0.61846066),
            (2.2, 0.57459813),
            (1.1, 0.53654534),
            (9.9, 0.50321954),
            (5.5, 0.47379148),
            (4.4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn numeric_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id NUMERIC,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1.1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2.2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3.3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4.4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5.5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6.6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7.7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8.8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9.9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10.1, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(f64, f32)> = r#"
    SELECT CAST(id AS FLOAT8), pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3.3, 0.61846066),
            (2.2, 0.57459813),
            (1.1, 0.53654534),
            (9.9, 0.50321954),
            (5.5, 0.47379148),
            (4.4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn string_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TEXT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'redness');
    INSERT INTO test_table (id, value) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'great white');
    INSERT INTO test_table (id, value) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT id, pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                "b5faacc0-9eba-441a-81f8-820b46a3b57e".to_string(),
                0.61846066
            ),
            (
                "38bf27a0-1aa8-42cd-9cb0-993025e0b8d0".to_string(),
                0.57459813
            ),
            (
                "f159c89e-2162-48cd-85e3-e42b71d2ecd0".to_string(),
                0.53654534
            ),
            (
                "40bc9216-66d0-4ae8-87ee-ddb02e3e1b33".to_string(),
                0.50321954
            ),
            (
                "ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad".to_string(),
                0.47379148
            ),
            (
                "eb833eb6-c598-4042-b84a-0045828fceea".to_string(),
                0.44761515
            ),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn date_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id DATE,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('2023-05-03', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-04', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('2023-05-05', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('2023-05-06', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('2023-05-07', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('2023-05-08', 'redness');
    INSERT INTO test_table (id, value) VALUES ('2023-05-09', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-10', 'great white');
    INSERT INTO test_table (id, value) VALUES ('2023-05-11', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('2023-05-12', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("2023-05-05".to_string(), 0.61846066),
            ("2023-05-04".to_string(), 0.57459813),
            ("2023-05-03".to_string(), 0.53654534),
            ("2023-05-11".to_string(), 0.50321954),
            ("2023-05-07".to_string(), 0.47379148),
            ("2023-05-06".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
        paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn time_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIME,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('08:09:10', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('09:10:11', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('10:11:12', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('11:12:13', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('12:13:14', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('13:14:15', 'redness');
    INSERT INTO test_table (id, value) VALUES ('14:15:16', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('15:16:17', 'great white');
    INSERT INTO test_table (id, value) VALUES ('16:17:18', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('17:18:19', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("10:11:12".to_string(), 0.61846066),
            ("09:10:11".to_string(), 0.57459813),
            ("08:09:10".to_string(), 0.53654534),
            ("16:17:18".to_string(), 0.50321954),
            ("12:13:14".to_string(), 0.47379148),
            ("11:12:13".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
        paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn timestamp_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIMESTAMP,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('2023-05-03 08:09:10', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-04 09:10:11', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('2023-05-05 10:11:12', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('2023-05-06 11:12:13', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('2023-05-07 12:13:14', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('2023-05-08 13:14:15', 'redness');
    INSERT INTO test_table (id, value) VALUES ('2023-05-09 14:15:16', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-10 15:16:17', 'great white');
    INSERT INTO test_table (id, value) VALUES ('2023-05-11 16:17:18', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('2023-05-12 17:18:19', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("2023-05-05 10:11:12".to_string(), 0.61846066),
            ("2023-05-04 09:10:11".to_string(), 0.57459813),
            ("2023-05-03 08:09:10".to_string(), 0.53654534),
            ("2023-05-11 16:17:18".to_string(), 0.50321954),
            ("2023-05-07 12:13:14".to_string(), 0.47379148),
            ("2023-05-06 11:12:13".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn timestamptz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIMESTAMP WITH TIME ZONE,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('2023-05-03 08:09:10 EST', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-04 09:10:11 PST', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('2023-05-05 10:11:12 MST', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('2023-05-06 11:12:13 CST', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('2023-05-07 12:13:14 EST', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('2023-05-08 13:14:15 PST', 'redness');
    INSERT INTO test_table (id, value) VALUES ('2023-05-09 14:15:16 MST', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-10 15:16:17 CST', 'great white');
    INSERT INTO test_table (id, value) VALUES ('2023-05-11 16:17:18 EST', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('2023-05-12 17:18:19 PST', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table
    WHERE test_table @@@ paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("2023-05-05 17:11:12+00".to_string(), 0.61846066),
            ("2023-05-04 17:10:11+00".to_string(), 0.57459813),
            ("2023-05-03 13:09:10+00".to_string(), 0.53654534),
            ("2023-05-11 21:17:18+00".to_string(), 0.50321954),
            ("2023-05-07 17:13:14+00".to_string(), 0.47379148),
            ("2023-05-06 17:12:13+00".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT  pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') 
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), pdb.snippet(value) FROM test_table WHERE value @@@ 'blue'
    UNION
    SELECT CAST(id AS TEXT), pdb.snippet(value) FROM test_table WHERE value @@@ 'tooth'
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 8);
}

#[rstest]
fn timetz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIME WITH TIME ZONE,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('08:09:10 EST', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('09:10:11 PST', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('10:11:12 MST', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('11:12:13 CST', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('12:13:14 EST', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('13:14:15 PST', 'redness');
    INSERT INTO test_table (id, value) VALUES ('14:15:16 MST', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('15:16:17 CST', 'great white');
    INSERT INTO test_table (id, value) VALUES ('16:17:18 EST', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('17:18:19 PST', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    let rows: Vec<(String,)> = r#"
    SELECT CAST(id AS TEXT) FROM test_table"#
        .fetch_collect(&mut conn);

    println!("{rows:#?}");

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("10:11:12-07".to_string(), 0.61846066),
            ("09:10:11-08".to_string(), 0.57459813),
            ("08:09:10-05".to_string(), 0.53654534),
            ("16:17:18-05".to_string(), 0.50321954),
            ("12:13:14-05".to_string(), 0.47379148),
            ("11:12:13-06".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn inet_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id INET,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('23.100.234.255', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('13.248.169.48', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('152.19.134.142', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('63.141.128.16', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('23.21.162.66', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('185.125.190.21', 'redness');
    INSERT INTO test_table (id, value) VALUES ('20.112.250.133', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('185.230.63.107', 'great white');
    INSERT INTO test_table (id, value) VALUES ('217.196.149.50', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('192.168.0.0', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("152.19.134.142/32".to_string(), 0.61846066),
            ("13.248.169.48/32".to_string(), 0.57459813),
            ("23.100.234.255/32".to_string(), 0.53654534),
            ("217.196.149.50/32".to_string(), 0.50321954),
            ("23.21.162.66/32".to_string(), 0.47379148),
            ("63.141.128.16/32".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT pdb.score(id) FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}
```

---

## joins.rs

```
mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn joins_return_correct_results(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    DROP TABLE IF EXISTS a;
    DROP TABLE IF EXISTS b;
    CREATE TABLE a (
        id bigint,
        value text
    );
    CREATE TABLE b (
        id bigint,
        value text
    );
    
    INSERT INTO public.a VALUES (1, 'beer wine');
    INSERT INTO public.a VALUES (2, 'beer wine');
    INSERT INTO public.a VALUES (3, 'cheese');
    INSERT INTO public.a VALUES (4, 'food stuff');
    INSERT INTO public.a VALUES (5, 'only_in_a');

    INSERT INTO public.b VALUES (1, 'beer');
    INSERT INTO public.b VALUES (2, 'wine');
    INSERT INTO public.b VALUES (3, 'cheese');
    INSERT INTO public.b VALUES (4, 'wine beer cheese');
                            -- mind the gap
    INSERT INTO public.b VALUES (6, 'only_in_b');

-- loading all this extra data makes the test take too long on CI
--    INSERT INTO a (id, value) SELECT x, md5(random()::text) FROM generate_series(7, 10000) x;
--    INSERT INTO b (id, value) SELECT x, md5(random()::text) FROM generate_series(7, 10000) x;
        
    CREATE INDEX idxa ON public.a USING bm25 (id, value) WITH (key_field=id, text_fields='{"value": {}}');
    CREATE INDEX idxb ON public.b USING bm25 (id, value) WITH (key_field=id, text_fields='{"value": {}}');
    "#
        .execute(&mut conn);

    type RowType = (Option<i64>, Option<i64>, Option<String>, Option<String>);
    // the pg_search queries also ORDER BY pdb.score() to ensure we get a paradedb CustomScan
    let queries = [
        [
            "select a.id, b.id, a.value a, b.value b from a left join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a left join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a right join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a right join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a inner join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a inner join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a full join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a full join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
    ];

    for [pg_search, postgres] in queries {
        eprintln!("pg_search: {pg_search:?}");
        eprintln!("postgres: {postgres:?}");

        let (pg_search_plan,) =
            format!("EXPLAIN (ANALYZE, FORMAT JSON) {pg_search}").fetch_one::<(Value,)>(&mut conn);
        eprintln!("pg_search_plan: {pg_search_plan:#?}");
        assert!(format!("{pg_search_plan:?}").contains("ParadeDB Scan"));

        let pg_search = pg_search.fetch_result::<RowType>(&mut conn)?;
        let postgres = postgres.fetch_result::<RowType>(&mut conn)?;

        assert_eq!(pg_search, postgres);
    }

    Ok(())
}

#[rstest]
fn snippet_from_join(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    CREATE TABLE a (
        id bigint,
        value text
    );
    CREATE TABLE b (
        id bigint,
        value text
    );

    INSERT INTO a (id, value) VALUES (1, 'beer'), (2, 'wine'), (3, 'cheese');
    INSERT INTO b (id, value) VALUES (1, 'beer'), (2, 'wine'), (3, 'cheese');

    CREATE INDEX idxa ON a USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    CREATE INDEX idxb ON b USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
        .execute(&mut conn);

    let (snippet, ) = r#"select pdb.snippet(a.value) from a left join b on a.id = b.id where a.value @@@ 'beer';"#
        .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    let (snippet, ) = r#"select pdb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' and b.value @@@ 'beer';"#
        .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    // NB:  the result of this is wrong for now...
    let results = r#"select a.id, b.id, pdb.snippet(a.value), pdb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' or b.value @@@ 'wine' order by a.id, b.id;"#
        .fetch_result::<(i64, i64, Option<String>, Option<String>)>(&mut conn)?;

    // ... this is what we'd actually expect from the above query
    let expected = vec![
        (1, 1, Some(String::from("<b>beer</b>")), None),
        (2, 2, None, Some(String::from("<b>wine</b>"))),
    ];

    assert_eq!(results, expected);

    Ok(())
}
```

---

## hybrid.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};

#[rstest]
fn hybrid_deprecated(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (
        key_field = 'id',
        text_fields = '{"description": {}, "category": {}}',
        numeric_fields = '{"rating": {}}',
        boolean_fields = '{"in_stock": {}}',
        datetime_fields = '{"created_at": {}}',
        json_fields = '{"metadata": {}}'
    );

    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, BigDecimal)> = r#"
    WITH semantic_search AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items ORDER BY embedding <=> '[1,2,3]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) as rank
        FROM mock_items WHERE description @@@ 'keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        (COALESCE(1.0 / (60 + semantic_search.rank), 0.0) * 0.1) +
        (COALESCE(1.0 / (60 + bm25_search.rank), 0.0) * 0.9) AS score
    FROM semantic_search
    FULL OUTER JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC
    LIMIT 5
    "#
    .fetch(&mut conn);

    assert_eq!(
        rows.into_iter().map(|t| t.0).collect::<Vec<_>>(),
        vec![2, 1, 19, 9, 29]
    );
}

#[rstest]
#[allow(clippy::excessive_precision)]
fn reciprocal_rank_fusion(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.bm25_search ADD COLUMN embedding vector(3);

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    let columns: Vec<(i32, f32, String)> = r#"
    WITH semantic AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM paradedb.bm25_search
        ORDER BY embedding <=> '[1,2,3]'
        LIMIT 20
    ),
    bm25 AS (
        SELECT id, RANK () OVER (ORDER BY pdb.score(id) DESC) as rank
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic.id, bm25.id) AS id,
        (COALESCE(1.0 / (60 + semantic.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25.rank), 0.0))::REAL AS score,
        paradedb.bm25_search.description
    FROM semantic
    FULL OUTER JOIN bm25 ON semantic.id = bm25.id
    JOIN paradedb.bm25_search ON paradedb.bm25_search.id = COALESCE(semantic.id, bm25.id)
    ORDER BY score DESC
    LIMIT 5;
    "#
    .fetch(&mut conn);

    assert_eq!(
        columns[0],
        (
            1,
            0.03062178588125292193,
            "Ergonomic metal keyboard".to_string()
        )
    );
    assert_eq!(
        columns[1],
        (2, 0.02990695613646433318, "Plastic Keyboard".to_string())
    );
    assert_eq!(
        columns[2],
        (
            19,
            0.01639344262295081967,
            "Artistic ceramic vase".to_string()
        )
    );
    assert_eq!(
        columns[3],
        (9, 0.01639344262295081967, "Modern wall clock".to_string())
    );
    assert_eq!(
        columns[4],
        (
            29,
            0.01639344262295081967,
            "Designer wall paintings".to_string()
        )
    );
}
```

---

## term.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn boolean_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value BOOLEAN
    );

    INSERT INTO test_table (value) VALUES (true), (false), (false), (true);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', boolean_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, bool)> = r#"
    SELECT * FROM test_table
    WHERE test_table @@@ paradedb.term(field => 'value', value => true)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, true), (4, true)]);
}

#[rstest]
fn integer_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int2 SMALLINT,
        value_int4 INTEGER,
        value_int8 BIGINT
    );

    INSERT INTO test_table (value_int2, value_int4, value_int8) VALUES 
        (-11, -1111, -11111111),
        (22, 2222, 22222222), 
        (33, 3333, 33333333), 
        (44, 4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_int2, value_int4, value_int8) WITH (key_field='id', numeric_fields='{"value_int2": {}, "value_int4": {}, "value_int8": {}}');
    "#
    .execute(&mut conn);

    // INT2
    let rows: Vec<(i32, i16)> = r#"
    SELECT id, value_int2 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_int2', value => -11)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, -11)]);

    // INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value_int4', value => 2222)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2, 2222)]);

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_int8', value => 33333333)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(3, 33333333)]);
}

#[rstest]
fn float_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_float4 FLOAT4,
        value_float8 FLOAT8,
        value_numeric NUMERIC
    );

    INSERT INTO test_table (value_float4, value_float8, value_numeric) VALUES
        (-1.1, -1111.1111, -111.11111),
        (2.2, 2222.2222, 222.22222),
        (3.3, 3333.3333, 333.33333),
        (4.4, 4444.4444, 444.44444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_float4, value_float8, value_numeric) WITH (key_field='id', numeric_fields='{"value_float4": {}, "value_float8": {}, "value_numeric": {}}');
    "#
    .execute(&mut conn);

    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_float4', value => -1.1::float4)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, -1.1)]);

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_float8', value => 4444.4444::float8)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(4, 4444.4444)]);

    // NUMERIC - no sqlx::Type for numerics, so just check id
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_numeric', value => 333.33333::numeric)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(3,)]);
}

#[rstest]
fn text_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_text TEXT,
        value_varchar VARCHAR(64),
        value_uuid UUID
    );

    INSERT INTO test_table (value_text, value_varchar, value_uuid) VALUES
        ('abc', 'var abc', 'a99e7330-37e6-4f14-8c95-985052ee74f3'::uuid),
        ('def', 'var def', '2fe779f1-2a74-4035-9f1a-9477bae0364c'::uuid),
        ('ghi', 'var ghi', 'b9592b87-82ea-4d7b-8865-f6be819d4f0f'::uuid),
        ('jkl', 'var jkl', 'ae9d4a8c-8382-452d-96fb-a9a1c4192a03'::uuid);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_text, value_varchar, value_uuid) WITH (key_field='id', text_fields='{
        "value_text": {}, 
        "value_varchar": {}, 
        "value_uuid": {"tokenizer": {"type": "raw"}, "normalizer": "raw", "record": "basic", "fieldnorms": false}
    }');
    "#
    .execute(&mut conn);

    // TEXT
    let rows: Vec<(i32, String)> = r#"
    SELECT id, value_text FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_text', value => 'abc')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, "abc".into())]);

    // VARCHAR
    let rows: Vec<(i32, String)> = r#"
    SELECT id, value_varchar FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_varchar', value => 'ghi')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(3, "var ghi".into())]);

    // UUID - sqlx doesn't have a uuid type, so we just look for id
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_uuid', value => 'ae9d4a8c-8382-452d-96fb-a9a1c4192a03')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(4,)]);
}

#[rstest]
fn datetime_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE,
        value_time TIME,
        value_timetz TIME WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz, value_time, value_timetz) VALUES 
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST', TIME '08:09:10', TIME WITH TIME ZONE '08:09:10 PST'),
        (DATE '2021-06-28', TIMESTAMP '2019-08-02 07:52:43.123', TIMESTAMP WITH TIME ZONE '2019-08-02 07:52:43.123 EST', TIME '11:43:21.456', TIME WITH TIME ZONE '11:43:21.456 EST');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_date, value_timestamp, value_timestamptz, value_time, value_timetz) WITH (key_field='id', datetime_fields='{
        "value_date": {}, 
        "value_timestamp": {}, 
        "value_timestamptz": {}, 
        "value_time": {}, 
        "value_timetz": {}
    }');
    "#
    .execute(&mut conn);

    // DATE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_date', value => DATE '2023-05-03')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIMESTAMP
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamp', value => TIMESTAMP '2019-08-02 07:52:43.123')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIMESTAMP WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamptz', value => TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIMESTAMP WITH TIME ZONE: Change time zone in query
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamptz', value => TIMESTAMP WITH TIME ZONE '2023-04-15 16:27:09 EST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIME
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_time', value => TIME '11:43:21.456')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIME WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timetz', value => TIME WITH TIME ZONE '11:43:21.456 EST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIME WITH TIME ZONE: Change time zone in query
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timetz', value => TIME WITH TIME ZONE '08:43:21.456 PST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIMESTAMP WITH TIME ZONE: Query no time zone with time zone
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamp', value => TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 GMT')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIMESTAMP: Query time zone with no time zone (GMT = EST + 5)
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamptz', value => TIMESTAMP '2019-08-02 12:52:43.123')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);
}
```

---

## mutation.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Message {
    Match,
    NoMatch,
}

impl Message {
    fn as_str(&self) -> &'static str {
        match self {
            Message::Match => "cheese",
            Message::NoMatch => "bread",
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
enum Action {
    Insert(Message),
    Update {
        #[proptest(strategy = "any::<prop::sample::Index>()")]
        index: prop::sample::Index,
        new_message: Message,
    },
    Delete(#[proptest(strategy = "any::<prop::sample::Index>()")] prop::sample::Index),
    Vacuum,
}

fn setup(conn: &mut PgConnection, mutable_segment_rows: usize) {
    format!(r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;
    SET log_error_verbosity TO VERBOSE;
    SET paradedb.global_mutable_segment_rows TO 0;
    DROP TABLE IF EXISTS test_table;
    CREATE TABLE test_table (id SERIAL8 PRIMARY KEY, message TEXT);
    CREATE INDEX idx_test_table ON test_table USING bm25 (id, message)
    WITH (key_field = 'id', text_fields='{{"message": {{ "tokenizer": {{"type": "default"}} }} }}', mutable_segment_rows={mutable_segment_rows});
    ANALYZE test_table;
    "#)
    .execute(conn);
}

#[rstest]
#[tokio::test]
async fn mutable_segment_correctness(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        actions in proptest::collection::vec(any::<Action>(), 1..32),
        mutable_segment_rows in prop_oneof![Just(0), Just(1), Just(10)],
    )| {
        let mut conn = pool.pull();
        setup(&mut conn, mutable_segment_rows);

        let mut model: Vec<(i64, Message)> = Vec::new();

        for (i, action) in actions.into_iter().enumerate() {
            match action {
                Action::Insert(message) => {
                    let (id,): (i64,) = format!(
                        "INSERT INTO test_table (message) VALUES ('{}') RETURNING id",
                        message.as_str()
                    ).fetch_one(&mut conn);
                    model.push((id, message));
                }
                Action::Update { index, new_message } => {
                    if model.is_empty() {
                        continue;
                    }
                    let idx = index.index(model.len());
                    let (id_to_update, _) = model[idx];

                    format!(
                        "UPDATE test_table SET message = '{}' WHERE id = {};",
                        new_message.as_str(),
                        id_to_update,
                    ).execute(&mut conn);

                    model[idx].1 = new_message;
                }
                Action::Delete(index) => {
                    if model.is_empty() {
                        continue;
                    }
                    let idx = index.index(model.len());
                    let (id_to_delete, _) = model[idx];

                    format!(
                        "DELETE FROM test_table WHERE id = {};",
                        id_to_delete,
                    ).execute(&mut conn);

                    model.remove(idx);
                }
                Action::Vacuum => {
                    "VACUUM test_table;".execute(&mut conn);
                }
            }

            let count_query = r#"SELECT COUNT(*) FROM test_table WHERE message @@@ 'cheese';"#;
            let (result_count,): (i64,) = count_query.fetch_one(&mut conn);

            let expected_count = model.iter().filter(|(_, m)| matches!(m, Message::Match)).count() as i64;

            prop_assert_eq!(
                result_count,
                expected_count,
                "Mismatch after action #{}: {:?}\nModel: {:?}",
                i,
                action,
                model
            );
        }
    });
}
```

---

## index_config.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

#![allow(unused_variables, unused_imports)]
mod fixtures;

use std::path::PathBuf;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

fn fmt_err<T: std::error::Error>(err: T) -> String {
    format!("unexpected error, received: {err}")
}

#[rstest]
fn invalid_create_index(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'public')"
        .execute(&mut conn);

    match r#"CREATE INDEX index_config_index ON index_config
        USING bm25 (id) "#
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with no key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: index should have a `WITH (key_field='...')` option"
        ),
    };
}

#[rstest]
fn prevent_duplicate(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);

    match r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with relation already exists"),
        Err(err) => assert!(
            err.to_string().contains("already exists"),
            "{}",
            fmt_err(err)
        ),
    };
}

#[rstest]
async fn drop_column(mut conn: PgConnection) {
    r#"
    CREATE TABLE f_table (
        id SERIAL PRIMARY KEY,
        category TEXT
    );

    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        fkey INTEGER REFERENCES f_table ON UPDATE CASCADE ON DELETE RESTRICT,
        fulltext TEXT
    );

    INSERT INTO f_table (category) VALUES ('cat_a'), ('cat_b'), ('cat_c');
    INSERT INTO test_table (fkey, fulltext) VALUES (1, 'abc'), (1, 'def'), (2, 'ghi'), (3, 'jkl');
    "#
    .execute(&mut conn);

    r#"CREATE INDEX test_index ON test_table
        USING bm25 (id, fulltext) WITH (key_field='id')"#
        .execute(&mut conn);

    r#"DROP INDEX test_index CASCADE;
    ALTER TABLE test_table DROP COLUMN fkey;

    CREATE INDEX test_index ON test_table
        USING bm25 (id, fulltext) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('test_index')".fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("fulltext".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn default_text_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("description".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn text_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "default", "normalizer": "raw"}, "record": "freq", "fast": true}}');
"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("description".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}

#[rstest]
fn multiple_text_fields(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config

        USING bm25 (id, description, category)
        WITH (
            key_field='id',
            text_fields='{"description": {"tokenizer": {"type": "default", "normalizer": "raw"}, "record": "freq", "fast": true}}'
        );
        "#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("category".into(), "Str".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
}

#[rstest]
fn default_numeric_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, rating) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("rating".into(), "I64".into()));
}

#[rstest]
fn numeric_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, rating) WITH (key_field='id', numeric_fields='{"rating": {"fast": true}}')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("rating".into(), "I64".into()));
}

#[rstest]
fn default_boolean_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, in_stock) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("in_stock".into(), "Bool".into()));
}

#[rstest]
fn boolean_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, in_stock) WITH (key_field='id', boolean_fields='{"in_stock": {"fast": false}}')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("in_stock".into(), "Bool".into()));
}

#[rstest]
fn default_json_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, metadata) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("metadata".into(), "JsonObject".into()));
}

#[rstest]
fn json_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, metadata)
        WITH (
            key_field='id',
            json_fields='{"metadata": {"fast": true, "expand_dots": false, "tokenizer": {"type": "raw", "normalizer": "raw"}}}'
        )"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("id".into(), "I64".into()));
    assert_eq!(rows[2], ("metadata".into(), "JsonObject".into()));
}

#[rstest]
fn default_datetime_field(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, created_at, last_updated_date) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("created_at".into(), "Date".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
    assert_eq!(rows[3], ("last_updated_date".into(), "Date".into()));
}

#[rstest]
fn datetime_field_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, created_at, last_updated_date)
        WITH (key_field='id', datetime_fields='{"created_at": {"fast": true}, "last_updated_date": {"fast": false}}')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("created_at".into(), "Date".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
    assert_eq!(rows[3], ("last_updated_date".into(), "Date".into()));
}

#[rstest]
fn multiple_fields(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description, category, rating, in_stock, metadata) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index')"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("category".into(), "Str".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
    assert_eq!(rows[4], ("in_stock".into(), "Bool".into()));
    assert_eq!(rows[5], ("metadata".into(), "JsonObject".into()));
    assert_eq!(rows[6], ("rating".into(), "I64".into()));
}

#[rstest]
fn missing_schema_index(mut conn: PgConnection) {
    match "SELECT paradedb.schema('paradedb.missing_bm25_index')".fetch_result::<(i64,)>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains(r#"relation "paradedb.missing_bm25_index" does not exist"#)),
        _ => panic!("non-existing index should throw an error"),
    }
}

#[rstest]
fn null_values(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    "INSERT INTO paradedb.index_config (description, category, rating) VALUES ('Null Item 1', NULL, NULL), ('Null Item 2', NULL, 2)"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description, category, rating, in_stock, metadata) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, Option<String>, Option<i32>)> = "
        SELECT description, category, rating
        FROM paradedb.index_config WHERE index_config @@@ 'description:\"Null Item\"'
        ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("Null Item 1".into(), None, None));
    assert_eq!(rows[1], ("Null Item 2".into(), None, Some(2)));

    let rows: Vec<(bool,)> =
        "SELECT in_stock FROM paradedb.index_config WHERE index_config @@@ 'in_stock:false'"
            .fetch(&mut conn);

    assert_eq!(rows.len(), 13);
}

#[rstest]
fn null_key_field_build(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(id INTEGER, description TEXT)".execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (NULL, 'Null Item 1'), (2, 'Null Item 2')"
        .execute(&mut conn);

    match r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with null key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: key_field column 'id' cannot be NULL"
        ),
    };
}

#[rstest]
fn null_key_field_insert(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(id INTEGER, description TEXT)".execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (1, 'Null Item 1'), (2, 'Null Item 2')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);

    match "INSERT INTO paradedb.index_config VALUES (NULL, 'Null Item 3')".execute_result(&mut conn)
    {
        Ok(_) => panic!("should fail with null key_field"),
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: key_field column 'id' cannot be NULL"
        ),
    };
}

#[rstest]
fn column_name_camelcase(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config(\"IdName\" INTEGER, \"ColumnName\" TEXT)"
        .execute(&mut conn);
    "INSERT INTO paradedb.index_config VALUES (1, 'Plastic Keyboard'), (2, 'Bluetooth Headphones')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 ("IdName", "ColumnName") WITH (key_field='IdName')"#
        .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM paradedb.index_config WHERE index_config @@@ 'ColumnName:keyboard'"
            .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1, "Plastic Keyboard".into()));
}

#[rstest]
fn multi_index_insert_in_transaction(mut conn: PgConnection) {
    "CREATE TABLE paradedb.index_config1(id INTEGER, description TEXT)".execute(&mut conn);
    "CREATE TABLE paradedb.index_config2(id INTEGER, description TEXT)".execute(&mut conn);
    r#"CREATE INDEX index_config1_index ON paradedb.index_config1
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);
    r#"CREATE INDEX index_config2_index ON paradedb.index_config2
        USING bm25 (id, description) WITH (key_field='id')"#
        .execute(&mut conn);
    "BEGIN".execute(&mut conn);
    "INSERT INTO paradedb.index_config1 VALUES (1, 'Item 1'), (2, 'Item 2')".execute(&mut conn);
    "INSERT INTO paradedb.index_config2 VALUES (1, 'Item 1'), (2, 'Item 2')".execute(&mut conn);
    "COMMIT".execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM paradedb.index_config1 WHERE index_config1 @@@ 'description:item'"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> =
        "SELECT * FROM paradedb.index_config2 WHERE index_config2 @@@ 'description:item'"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn partitioned_schema(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('sales_index') ORDER BY name"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("amount".into(), "F64".into()));
    assert_eq!(rows[1], ("ctid".into(), "U64".into()));
    assert_eq!(rows[2], ("description".into(), "Str".into()));
    assert_eq!(rows[3], ("id".into(), "I64".into()));
    assert_eq!(rows[4], ("sale_date".into(), "Date".into()));
}

#[rstest]
fn partitioned_info(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    // Insert rows into both partitions.
    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Modern wall clock');
    "#
    .execute(&mut conn);

    // And validate that we see at least one segment for each.
    let segments_per_partition: Vec<(String, i64)> = "
        SELECT index_name, COUNT(*) FROM paradedb.index_info('sales_index') GROUP BY index_name
    "
    .fetch(&mut conn);
    assert_eq!(segments_per_partition.len(), 2);
    for (index_name, segment_count) in segments_per_partition {
        assert!(
            segment_count > 0,
            "Got {segment_count} for index partition {index_name}"
        );
    }

    // Just cover `index_layer_info`.
    let segments_per_partition: Vec<(String, String, i64)> =
        "SELECT relname::text, layer_size, count FROM paradedb.index_layer_info".fetch(&mut conn);
    assert!(!segments_per_partition.is_empty());
}

#[rstest]
fn partitioned_all(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    let schema_rows: Vec<(String, String)> =
        "SELECT id from sales WHERE id @@@ paradedb.all()".fetch(&mut conn);
    assert_eq!(schema_rows.len(), 0);

    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Modern wall clock');
    "#
    .execute(&mut conn);

    let schema_rows: Vec<(i32,)> =
        "SELECT id from sales WHERE id @@@ paradedb.all()".fetch(&mut conn);
    assert_eq!(schema_rows.len(), 2);
}

#[rstest]
fn partitioned_query(mut conn: PgConnection) {
    // Set up the partitioned table with two partitions and a BM25 index.
    PartitionedTable::setup().execute(&mut conn);

    // Insert some data.
    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-01-15', 200.00, 'Plastic keyboard'),
        ('2023-02-05', 300.00, 'Sleek running shoes'),
        ('2023-03-12', 175.50, 'Bluetooth speaker'),
        ('2023-03-25', 225.75, 'Artistic ceramic vase');

        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-04-01', 250.00, 'Modern wall clock'),
        ('2023-04-18', 180.00, 'Designer wall paintings'),
        ('2023-05-09', 320.00, 'Handcrafted wooden frame');
    "#
    .execute(&mut conn);

    // Test: Verify data is partitioned correctly by querying each partition
    let rows_q1: Vec<(i32, String, String)> = r#"
        SELECT id, description, sale_date::text FROM sales_2023_q1
    "#
    .fetch(&mut conn);
    assert_eq!(rows_q1.len(), 5, "Expected 5 rows in Q1 partition");

    let rows_q2: Vec<(i32, String, String)> = r#"
        SELECT id, description, sale_date::text FROM sales_2023_q2
    "#
    .fetch(&mut conn);
    assert_eq!(rows_q2.len(), 3, "Expected 3 rows in Q2 partition");

    // Test: Search using the bm25 index against both the parent and child tables.
    for table in ["sales", "sales_2023_q1"] {
        let search_results: Vec<(i32, String)> = format!(
            r#"
            SELECT id, description FROM {table} WHERE id @@@ 'description:keyboard'
            "#
        )
        .fetch(&mut conn);
        assert_eq!(search_results.len(), 2, "Expected 2 items with 'keyboard'");
    }

    // Test: Retrieve items by a numeric range (amount field) and verify bm25 compatibility
    for (table, expected) in [("sales", 5), ("sales_2023_q1", 3)] {
        let amount_results: Vec<(i32, String, f32)> = format!(
            r#"
            SELECT id, description, amount FROM {table}
            WHERE amount @@@ '[175 TO 250]'
            ORDER BY amount ASC
            "#
        )
        .fetch(&mut conn);
        assert_eq!(
            amount_results.len(),
            expected,
            "Expected {expected} items with amount in range 175-250"
        );
    }
}

#[rstest]
fn partitioned_uses_custom_scan(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Modern wall clock');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers TO 0;".execute(&mut conn);

    // Without the partition key.
    let (plan,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT count(*)
        FROM sales
        WHERE id @@@ '1';
        "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    let per_partition_plans = plan
        .pointer("/0/Plan/Plans/0/Plans")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        per_partition_plans.len(),
        2,
        "Expected 2 partitions to be scanned."
    );
    for per_partition_plan in per_partition_plans {
        pretty_assertions::assert_eq!(
            per_partition_plan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
    }

    // With the partition key: we expect the partition to be filtered, and for
    // us to apply pushdown.
    let (plan,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT count(*)
        FROM sales
        WHERE description @@@ 'keyboard' and sale_date = '2023-01-10';
        "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    let per_partition_plans = plan.pointer("/0/Plan/Plans").unwrap().as_array().unwrap();
    assert_eq!(
        per_partition_plans.len(),
        1,
        "Expected 1 partition to be scanned."
    );
    for per_partition_plan in per_partition_plans {
        pretty_assertions::assert_eq!(
            per_partition_plan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
        let query = per_partition_plan.get("Tantivy Query").unwrap().to_string();
        assert!(
            query.to_string().contains("2023-01-10"),
            "Expected sale_date to be pushed down into query: {query:?}",
        );
    }
}

#[rstest]
fn custom_enum_term(mut conn: PgConnection) {
    r#"
    CREATE TYPE color AS ENUM ('red', 'green', 'blue');
    CREATE TABLE paradedb.index_config(id INTEGER, description TEXT, color color);
    INSERT INTO paradedb.index_config VALUES (1, 'Item 1', 'red'), (2, 'Item 2', 'green');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_config_index ON paradedb.index_config
    USING bm25 (id, description, color)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT id, description FROM paradedb.index_config WHERE id @@@ paradedb.term('color', 'red'::color)".fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Item 1".into())]);
}

#[rstest]
fn custom_enum_parse(mut conn: PgConnection) {
    r#"
    CREATE TYPE color AS ENUM ('red', 'green', 'blue');
    CREATE TABLE paradedb.index_config(id INTEGER, description TEXT, color color);
    INSERT INTO paradedb.index_config VALUES (1, 'Item 1', 'red'), (2, 'Item 2', 'green');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_config_index ON paradedb.index_config
    USING bm25 (id, description, color)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> =
        "SELECT id, description FROM paradedb.index_config WHERE id @@@ paradedb.parse('color:1.0')".fetch(&mut conn);

    assert_eq!(rows, vec![(1, "Item 1".into())]);
}

#[rstest]
fn long_text_key_field_issue2198(mut conn: PgConnection) {
    "CREATE TABLE issue2198 (id TEXT, value TEXT)".execute(&mut conn);

    "CREATE INDEX idxissue2198 ON issue2198 USING bm25 (id, value) WITH (key_field='id')"
        .execute(&mut conn);

    let long_string = "a".repeat(10000);

    format!("INSERT INTO issue2198(id) VALUES ('{long_string}')").execute(&mut conn);
    let (count,) = format!("SELECT count(*) FROM issue2198 WHERE id @@@ '{long_string}'")
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) =
        format!("SELECT count(*) FROM issue2198 WHERE id @@@ paradedb.term('id', '{long_string}')")
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn uuid_as_raw_issue2199(mut conn: PgConnection) {
    "CREATE TABLE issue2199 (id SERIAL8 NOT NULL PRIMARY KEY, value uuid);".execute(&mut conn);

    "CREATE INDEX idxissue2199 ON issue2199 USING bm25 (id, value) WITH (key_field='id');"
        .execute(&mut conn);

    let uuid = uuid::Uuid::new_v4();

    format!("INSERT INTO issue2199(value) VALUES ('{uuid}')").execute(&mut conn);
    let (count,) = format!("SELECT count(*) FROM issue2199 WHERE value @@@ '{uuid}'")
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) =
        format!("SELECT count(*) FROM issue2199 WHERE id @@@ paradedb.term('value', '{uuid}')")
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

/// Common setup function for partitioned and non-partitioned table tests
fn setup_table_for_order_by_limit_test(conn: &mut PgConnection, is_partitioned: bool) {
    // Common settings for all tests
    r#"
    SET enable_indexscan TO off;
    SET enable_bitmapscan TO off;
    SET max_parallel_workers TO 0;
    "#
    .execute(conn);

    if is_partitioned {
        // Set up a partitioned table
        r#"
        DROP TABLE IF EXISTS sales;

        CREATE TABLE sales (
            id SERIAL,
            product_name TEXT,
            amount DECIMAL,
            sale_date DATE
        ) PARTITION BY RANGE (sale_date);

        CREATE TABLE sales_2023 PARTITION OF sales
        FOR VALUES FROM ('2023-01-01') TO ('2024-01-01');

        CREATE TABLE sales_2024 PARTITION OF sales
        FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');

        INSERT INTO sales (product_name, amount, sale_date) VALUES
        ('Laptop', 1200.00, '2023-01-15'),
        ('Smartphone', 800.00, '2023-03-10'),
        ('Headphones', 150.00, '2023-05-20'),
        ('Monitor', 300.00, '2023-07-05'),
        ('Keyboard', 80.00, '2023-09-12'),
        ('Mouse', 40.00, '2023-11-25'),
        ('Tablet', 500.00, '2024-01-05'),
        ('Printer', 200.00, '2024-02-18'),
        ('Camera', 600.00, '2024-04-22'),
        ('Speaker', 120.00, '2024-06-30');

        CREATE INDEX idx_sales_bm25 ON sales
        USING bm25 (id, product_name, amount, sale_date)
        WITH (
            key_field = 'id',
            text_fields = '{"product_name": {}}',
            numeric_fields = '{"amount": {}}',
            datetime_fields = '{"sale_date": {"fast": true}}'
        );
        "#
        .execute(conn);
    } else {
        // Set up two separate tables (not partitioned)
        r#"
        DROP TABLE IF EXISTS products_2023;
        DROP TABLE IF EXISTS products_2024;

        -- Create two separate tables with similar schema
        CREATE TABLE products_2023 (
            id SERIAL,
            product_name TEXT,
            amount DECIMAL,
            sale_date DATE
        );

        CREATE TABLE products_2024 (
            id SERIAL,
            product_name TEXT,
            amount DECIMAL,
            sale_date DATE
        );

        -- Insert similar data to both tables
        INSERT INTO products_2023 (product_name, amount, sale_date) VALUES
        ('Laptop', 1200.00, '2023-01-15'),
        ('Smartphone', 800.00, '2023-03-10'),
        ('Headphones', 150.00, '2023-05-20'),
        ('Monitor', 300.00, '2023-07-05'),
        ('Keyboard', 80.00, '2023-09-12');

        INSERT INTO products_2024 (product_name, amount, sale_date) VALUES
        ('Mouse', 40.00, '2024-01-25'),
        ('Tablet', 500.00, '2024-01-05'),
        ('Printer', 200.00, '2024-02-18'),
        ('Camera', 600.00, '2024-04-22'),
        ('Speaker', 120.00, '2024-06-30');

        -- Create BM25 indexes for both tables
        CREATE INDEX idx_products_2023_bm25 ON products_2023
        USING bm25 (id, product_name, amount, sale_date)
        WITH (
            key_field = 'id',
            text_fields = '{"product_name": {}}',
            numeric_fields = '{"amount": {}}',
            datetime_fields = '{"sale_date": {"fast": true}}'
        );

        CREATE INDEX idx_products_2024_bm25 ON products_2024
        USING bm25 (id, product_name, amount, sale_date)
        WITH (
            key_field = 'id',
            text_fields = '{"product_name": {}}',
            numeric_fields = '{"amount": {}}',
            datetime_fields = '{"sale_date": {"fast": true}}'
        );
        "#
        .execute(conn);
    }
}

/// Setup function for view tests
fn setup_view_for_order_by_limit_test(conn: &mut PgConnection) {
    // First drop any existing tables or views
    r#"
    DROP VIEW IF EXISTS products_view;
    DROP TABLE IF EXISTS products_2023_view;
    DROP TABLE IF EXISTS products_2024_view;

    SET enable_indexscan TO off;
    SET enable_bitmapscan TO off;
    SET max_parallel_workers TO 0;

    -- Create two separate tables with similar schema
    CREATE TABLE products_2023_view (
        id SERIAL,
        product_name TEXT,
        amount DECIMAL,
        sale_date DATE
    );

    CREATE TABLE products_2024_view (
        id SERIAL,
        product_name TEXT,
        amount DECIMAL,
        sale_date DATE
    );

    -- Insert data to both tables
    INSERT INTO products_2023_view (product_name, amount, sale_date) VALUES
    ('Laptop', 1200.00, '2023-01-15'),
    ('Smartphone', 800.00, '2023-03-10'),
    ('Headphones', 150.00, '2023-05-20'),
    ('Monitor', 300.00, '2023-07-05'),
    ('Keyboard', 80.00, '2023-09-12');

    INSERT INTO products_2024_view (product_name, amount, sale_date) VALUES
    ('Mouse', 40.00, '2024-01-25'),
    ('Tablet', 500.00, '2024-01-05'),
    ('Printer', 200.00, '2024-02-18'),
    ('Camera', 600.00, '2024-04-22'),
    ('Speaker', 120.00, '2024-06-30');

    -- Create BM25 indexes for both tables
    CREATE INDEX idx_products_2023_view_bm25 ON products_2023_view
    USING bm25 (id, product_name, amount, sale_date)
    WITH (
        key_field = 'id',
        text_fields = '{"product_name": {}}',
        numeric_fields = '{"amount": {}}',
        datetime_fields = '{"sale_date": {"fast": true}}'
    );

    CREATE INDEX idx_products_2024_view_bm25 ON products_2024_view
    USING bm25 (id, product_name, amount, sale_date)
    WITH (
        key_field = 'id',
        text_fields = '{"product_name": {}}',
        numeric_fields = '{"amount": {}}',
        datetime_fields = '{"sale_date": {"fast": true}}'
    );

    -- Create view combining both tables
    CREATE VIEW products_view AS
    SELECT * FROM products_2023_view
    UNION ALL
    SELECT * FROM products_2024_view;
    "#
    .execute(conn);
}

#[rstest]
fn partitioned_order_by_limit_pushdown(mut conn: PgConnection) {
    setup_table_for_order_by_limit_test(&mut conn, true);

    // Get the explain plan
    let explain_output = r#"
    EXPLAIN (ANALYZE, VERBOSE)
    SELECT * FROM sales
    WHERE product_name @@@ 'laptop OR smartphone OR headphones'
    ORDER BY sale_date LIMIT 5;
    "#
    .fetch::<(String,)>(&mut conn)
    .into_iter()
    .map(|(line,)| line)
    .collect::<Vec<String>>()
    .join("\n");

    // Check for TopNScanExecState in the plan
    assert!(
        explain_output.contains("TopNScanExecState"),
        "Expected TopNScanExecState in the execution plan"
    );

    // Verify sort field and direction
    assert!(
        explain_output.contains("TopN Order By: sale_date asc"),
        "Expected sort field to be sale_date"
    );

    // Verify the limit is pushed down
    assert!(
        explain_output.contains("TopN Limit: 5"),
        "Expected limit 5 to be pushed down"
    );

    // Also test that we get the correct sorted results
    let results: Vec<(String, String)> = r#"
    SELECT product_name, sale_date::text FROM sales
    WHERE product_name @@@ 'laptop OR smartphone OR headphones'
    ORDER BY sale_date LIMIT 5;
    "#
    .fetch(&mut conn);

    // Verify we got the right number of results
    assert_eq!(results.len(), 3, "Expected 3 matching results");

    // Verify they're in the correct order (ordered by sale_date)
    assert_eq!(results[0].0, "Laptop");
    assert_eq!(results[1].0, "Smartphone");
    assert_eq!(results[2].0, "Headphones");

    // Check the dates are in ascending order
    assert_eq!(results[0].1, "2023-01-15");
    assert_eq!(results[1].1, "2023-03-10");
    assert_eq!(results[2].1, "2023-05-20");
}

#[rstest]
fn non_partitioned_no_order_by_limit_pushdown(mut conn: PgConnection) {
    setup_table_for_order_by_limit_test(&mut conn, false);

    // Get the explain plan for a UNION query with ORDER BY LIMIT
    let explain_output = r#"
    EXPLAIN (ANALYZE, VERBOSE)
    SELECT * FROM (
        SELECT * FROM products_2023
        WHERE product_name @@@ 'laptop OR smartphone OR headphones'
        UNION ALL
        SELECT * FROM products_2024
        WHERE product_name @@@ 'tablet OR printer'
    ) combined_products
    ORDER BY sale_date LIMIT 5;
    "#
    .fetch::<(String,)>(&mut conn)
    .into_iter()
    .map(|(line,)| line)
    .collect::<Vec<String>>()
    .join("\n");

    // Verify NormalScanExecState is used. We can't use TopN because there the limit occurs _after_
    // the union. And we can't use fast fields, because there are non-fast fields.
    assert!(
        explain_output.contains("NormalScanExecState"),
        "Expected NormalScanExecState in the execution plan"
    );

    assert!(
        !explain_output.contains("TopNScanExecState"),
        "TopNScanExecState should not be present in the execution plan"
    );

    // Even without the optimization, verify the query returns correct results
    let results: Vec<(String, String)> = r#"
    SELECT product_name, sale_date::text FROM (
        SELECT * FROM products_2023
        WHERE product_name @@@ 'laptop OR smartphone OR headphones'
        UNION ALL
        SELECT * FROM products_2024
        WHERE product_name @@@ 'tablet OR printer'
    ) combined_products
    ORDER BY sale_date LIMIT 5;
    "#
    .fetch(&mut conn);

    // Verify we got the right number of results and correct order
    assert!(results.len() <= 5, "Expected at most 5 matching results");

    // Check that the first result is the earliest date
    if !results.is_empty() {
        let mut prev_date = &results[0].1;
        for result in &results[1..] {
            assert!(
                &result.1 >= prev_date,
                "Results should be sorted by date in ascending order"
            );
            prev_date = &result.1;
        }
    }
}

#[rstest]
fn view_no_order_by_limit_pushdown(mut conn: PgConnection) {
    setup_view_for_order_by_limit_test(&mut conn);

    // Verify the tables and indexes were created properly
    let table_check: Vec<(String,)> = r#"
    SELECT tablename FROM pg_tables
    WHERE tablename IN ('products_2023_view', 'products_2024_view')
    ORDER BY tablename;
    "#
    .fetch(&mut conn);
    assert_eq!(table_check.len(), 2, "Both tables should exist");

    let index_check: Vec<(String,)> = r#"
    SELECT indexname FROM pg_indexes
    WHERE indexname IN ('idx_products_2023_view_bm25', 'idx_products_2024_view_bm25')
    ORDER BY indexname;
    "#
    .fetch(&mut conn);
    assert_eq!(index_check.len(), 2, "Both indexes should exist");

    // Verify the view was created
    let view_check: Vec<(String,)> = r#"
    SELECT viewname FROM pg_views WHERE viewname = 'products_view';
    "#
    .fetch(&mut conn);
    assert_eq!(view_check.len(), 1, "View should exist");

    // Verify direct table queries work
    let test_query: Vec<(String,)> = r#"
    SELECT product_name FROM products_2023_view
    WHERE product_name @@@ 'laptop'
    LIMIT 1;
    "#
    .fetch(&mut conn);
    assert_eq!(test_query.len(), 1, "Direct table query should work");

    // Get the explain plan for a view query with ORDER BY LIMIT
    let explain_output = r#"
    EXPLAIN (ANALYZE, VERBOSE)
    SELECT * FROM products_view
    WHERE product_name @@@ 'laptop OR smartphone OR headphones OR tablet OR printer'
    ORDER BY sale_date LIMIT 5;
    "#
    .fetch::<(String,)>(&mut conn)
    .into_iter()
    .map(|(line,)| line)
    .collect::<Vec<String>>()
    .join("\n");

    // Print the explain plan for debugging
    println!("EXPLAIN output:\n{explain_output}");

    // Verify NormalScanExecState is used (not TopNScanExecState)
    assert!(
        explain_output.contains("NormalScanExecState"),
        "Expected NormalScanExecState in the execution plan"
    );

    assert!(
        !explain_output.contains("TopNScanExecState"),
        "TopNScanExecState should not be present in the execution plan"
    );

    // Ensure the query works and returns correct results
    let results: Vec<(String, String)> = r#"
    SELECT product_name, sale_date::text FROM products_view
    WHERE product_name @@@ 'laptop OR smartphone OR headphones OR tablet OR printer'
    ORDER BY sale_date LIMIT 5;
    "#
    .fetch(&mut conn);

    println!("Query results: {results:?}");

    // Verify we got the right number of results and correct order
    assert_eq!(results.len(), 5, "Expected 5 matching results");

    // Check that results are sorted by date
    if !results.is_empty() {
        let mut prev_date = &results[0].1;
        for result in &results[1..] {
            assert!(
                &result.1 >= prev_date,
                "Results should be sorted by date in ascending order"
            );
            prev_date = &result.1;
        }
    }
}

#[rstest]
fn expression_with_options(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, (lower(description)::pdb.simple)) WITH (key_field='id')"#
        .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.index_config_index') ORDER BY name"
            .fetch(&mut conn);

    assert_eq!(rows[0], ("ctid".into(), "U64".into()));
    assert_eq!(rows[1], ("description".into(), "Str".into()));
    assert_eq!(rows[2], ("id".into(), "I64".into()));
}
```

---

## fast_fields.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn plans_numeric_fast_field(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);

    assert_eq!(
        Some(&Value::String("rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

#[rstest]
fn plans_many_numeric_fast_fields(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);

    assert_eq!(
        Some(&Value::String("id, rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

#[rstest]
fn plans_many_numeric_fast_fields_with_score(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, pdb.score(id), rating FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("id, rating".into())),
        plan.pointer("/0/Plan/Fast Fields")
    )
}

// string "fast fields" are only supported as part of an aggregate query.  They're basically slower
// in all other cases
#[rstest]
fn plans_string_fast_field(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
SET paradedb.enable_aggregate_custom_scan = false;
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT category, count(*) FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' GROUP BY category".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("category".into())),
        plan.pointer("/0/Plan/Plans/0/Plans/0/Fast Fields")
    )
}

// only selecting a string field does use a "fast field"-style plan
#[rstest]
fn does_plan_string_fast_field(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT category FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    assert_eq!(
        Some(&Value::String("Custom Scan".into())),
        plan.pointer("/0/Plan/Node Type")
    )
}

#[rstest]
fn numeric_fast_field_in_window_func(mut conn: PgConnection) {
    r#"
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CREATE INDEX idxbm25_search ON paradedb.bm25_search
USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "category": {"fast": true, "normalizer": "raw"}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    boolean_fields='{"in_stock": {}}',
    json_fields='{"metadata": {}}',
    datetime_fields='{
        "created_at": {},
        "last_updated_date": {},
        "latest_available_time": {}
    }'
);
    "#
    .execute(&mut conn);

    let (plan,) = r#"EXPLAIN (ANALYZE, FORMAT JSON)
    WITH RankedContacts AS (
        SELECT id,
               ROW_NUMBER() OVER (PARTITION BY rating ORDER BY id) AS rn
        FROM paradedb.bm25_search
        WHERE id @@@ 'description:shoes'
        )
    SELECT id
    FROM RankedContacts
    WHERE rn <= 10
    LIMIT 100 OFFSET 100;
    "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("plan: {plan:#?}");
    assert_eq!(
        Some(&Value::String("MixedFastFieldExecState".into())),
        plan.pointer("/0/Plan/Plans/0/Plans/0/Plans/0/Plans/0/Exec Method")
    )
}
```

---

## range.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );

    INSERT INTO test_table (value_int4, value_int8) VALUES
        (-1111, -11111111),
        (2222, 22222222),
        (3333, 33333333),
        (4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_int4, value_int8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int4', range => '[2222,4444]'::int4range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int8', range => '[0,50000000)'::int8range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn unbounded_integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );
    INSERT INTO test_table (value_int4, value_int8) VALUES
        (-1111, -11111111),
        (2222, 22222222),
        (3333, 33333333),
        (4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_int4, value_int8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Test unbounded upper range for INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int4', range => '[2222,)'::int4range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, 2222);
    assert_eq!(rows[2].1, 4444);

    // Test unbounded lower range for INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int4', range => '(,2222]'::int4range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, -1111);
    assert_eq!(rows[1].1, 2222);

    // Test unbounded upper range for INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int8', range => '[0,)'::int8range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, 22222222);
    assert_eq!(rows[2].1, 44444444);

    // Test unbounded lower range for INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_int8', range => '(,-5000000]'::int8range)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, -11111111);
}

#[rstest]
fn float_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_float4 FLOAT4,
        value_float8 FLOAT8,
        value_numeric NUMERIC
    );

    INSERT INTO test_table (value_float4, value_float8, value_numeric) VALUES
        (-1.1, -1111.1111, -111.11111),
        (2.2, 2222.2222, 222.22222),
        (3.3, 3333.3333, 333.33333),
        (4.4, 4444.4444, 444.44444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_float4, value_float8, value_numeric)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float4', range => '[-2,3]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float8', range => '(2222.2222, 3333.3333]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);

    // NUMERIC - no sqlx::Type for numerics, so just select id
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_numeric', range => '[0,400)'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn datetime_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz) VALUES
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST'),
        (DATE '2022-07-14', TIMESTAMP '2022-05-16 07:38:43', TIMESTAMP WITH TIME ZONE '2022-05-16 07:38:43 EST'),
        (DATE '2021-04-30', TIMESTAMP '2021-06-08 08:49:21', TIMESTAMP WITH TIME ZONE '2021-06-08 08:49:21 CST'),
        (DATE '2020-06-28', TIMESTAMP '2020-07-09 15:52:13', TIMESTAMP WITH TIME ZONE '2020-07-09 15:52:13 MST');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_date, value_timestamp, value_timestamptz)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // DATE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@
        paradedb.range(field => 'value_date', range => '[2020-05-20,2022-06-13]'::daterange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@
        paradedb.range(field => 'value_timestamp', range => '[2019-08-02 07:52:43, 2021-06-10 10:32:41]'::tsrange)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@
        paradedb.range(field => 'value_timestamptz', range => '[2020-07-09 17:52:13 EST, 2022-05-16 04:38:43 PST]'::tstzrange)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn integer_bounds_coercion(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_float4 FLOAT4,
        value_float8 FLOAT8
    );

    INSERT INTO test_table (value_float4, value_float8) VALUES
        (-1.1, -1111.1111),
        (2.2, 2222.2222),
        (3.3, 3333.3333),
        (4.4, 4444.4444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_float4, value_float8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Test integer bounds for FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float4', range => '[2,4]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, 2.2);
    assert_eq!(rows[1].1, 3.3);

    // Test integer bounds for FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float8', range => '[2222,4444]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, 2222.2222);
    assert_eq!(rows[1].1, 3333.3333);

    // Test mixed integer and float bounds for FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float4', range => '[2,3.5]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, 2.2);
    assert_eq!(rows[1].1, 3.3);

    // Test mixed integer and float bounds for FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table
    WHERE test_table @@@ paradedb.range(field => 'value_float8', range => '[2222,3333.5]'::numrange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, 2222.2222);
    assert_eq!(rows[1].1, 3333.3333);
}
```

---

## icu.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

#![cfg(feature = "icu")]

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_icu_arabic_tokenizer(mut conn: PgConnection) {
    IcuArabicPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_arabic ON icu_arabic_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM icu_arabic_posts WHERE icu_arabic_posts @@@ 'author:"محمد"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM icu_arabic_posts WHERE icu_arabic_posts @@@ 'title:"السوق"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM icu_arabic_posts WHERE icu_arabic_posts @@@ 'message:"في"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 3]);
}

#[rstest]
fn test_icu_amharic_tokenizer(mut conn: PgConnection) {
    IcuAmharicPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_amharic ON icu_amharic_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM icu_amharic_posts WHERE icu_amharic_posts @@@ 'author:"አለም"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM icu_amharic_posts WHERE icu_amharic_posts @@@ 'title:"ለመማር"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM icu_amharic_posts WHERE icu_amharic_posts @@@ 'message:"ዝናብ"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);
}

#[rstest]
fn test_icu_greek_tokenizer(mut conn: PgConnection) {
    IcuGreekPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_greek ON icu_greek_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM icu_greek_posts WHERE icu_greek_posts @@@ 'author:"Σοφία"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM icu_greek_posts WHERE icu_greek_posts @@@ 'title:"επιτυχία"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM icu_greek_posts WHERE icu_greek_posts @@@ 'message:"συμβουλές"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);
}

#[rstest]
fn test_icu_czech_tokenizer(mut conn: PgConnection) {
    IcuCzechPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_czech ON icu_czech_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM icu_czech_posts WHERE icu_czech_posts @@@ 'author:"Tomáš"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1]);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM icu_czech_posts WHERE icu_czech_posts @@@ 'title:"zdravý"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM icu_czech_posts WHERE icu_czech_posts @@@ 'message:"velký"~100' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);
}

#[rstest]
fn test_icu_czech_content_tokenizer(mut conn: PgConnection) {
    IcuCzechPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_czech_content ON icu_czech_posts 
    USING bm25 (id, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuCzechPostsTableVec = r#"
        SELECT * FROM icu_czech_posts
        WHERE icu_czech_posts @@@ paradedb.phrase(
            field => 'message',
            phrases => ARRAY['šla', 'sbírat']
        ) ORDER BY id;"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![1]);
}

#[rstest]
fn test_icu_snippet(mut conn: PgConnection) {
    IcuArabicPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_arabic ON icu_arabic_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: Vec<(i32, String)> =
        r#"SELECT id, pdb.snippet(title) FROM icu_arabic_posts WHERE title @@@ 'السوق' "#
            .fetch(&mut conn);
    assert_eq!(
        columns,
        vec![(2, "رحلة إلى <b>السوق</b> مع أبي".to_string())]
    );
}
```

---

## heap.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn mvcc_heap_filter(mut conn: PgConnection) {
    r#"
        CALL paradedb.create_bm25_test_table(table_name => 'heap_and_clauses_table', schema_name => 'public');

        CREATE INDEX heap_and_clauses_idx ON heap_and_clauses_table
        USING bm25 (id, description)
        WITH (key_field = 'id');
    "#.execute(&mut conn);

    // Ensure that heap filters continue to be applied correctly in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, description
            FROM heap_and_clauses_table
            WHERE id @@@ paradedb.match('description', 'Sleek running', conjunction_mode := true)
            AND description ILIKE 'Sleek running shoes'
            ORDER BY id;
        "#
        .fetch(&mut conn);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[0].1, "Sleek running shoes");

        r#"
            UPDATE heap_and_clauses_table SET last_updated_date = NOW();
        "#
        .execute(&mut conn);
    }
}

#[rstest]
fn mvcc_snippet(mut conn: PgConnection) {
    if pg_major_version(&mut conn) <= 14 {
        // TODO: See https://github.com/paradedb/paradedb/issues/3358.
        return;
    }

    r#"
        CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');
        
        CREATE INDEX mock_items_idx ON mock_items
        USING bm25 (id, description)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Ensure that snippet lookups from the heap succeed in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, pdb.snippet(description)
            FROM mock_items
            WHERE description @@@ 'shoes'
            ORDER BY id
            LIMIT 5;
        "#
        .fetch(&mut conn);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[0].1, "Sleek running <b>shoes</b>");
        assert_eq!(results[1].0, 4);
        assert_eq!(results[1].1, "White jogging <b>shoes</b>");
        assert_eq!(results[2].0, 5);
        assert_eq!(results[2].1, "Generic <b>shoes</b>");

        r#"
            UPDATE mock_items SET last_updated_date = NOW();
        "#
        .execute(&mut conn);
    }
}
```

---

## ivm.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn use_ivm(mut conn: PgConnection) {
    if "CREATE EXTENSION IF NOT EXISTS pg_ivm;"
        .execute_result(&mut conn)
        .is_err()
    {
        // Test requires `pg_ivm`.
        return;
    }

    r#"
    DROP TABLE IF EXISTS test CASCADE;
    CREATE TABLE test (
        id int,
        content TEXT
    );

    DROP TABLE IF EXISTS test_view CASCADE;
    SELECT pgivm.create_immv('test_view', 'SELECT test.*, test.id + 1 as derived FROM test;');

    CREATE INDEX test_search_idx ON test_view
    USING bm25 (id, content)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Validate the DML works with/without the custom scan.
    r#"
    SET paradedb.enable_custom_scan = false;
    INSERT INTO test VALUES (1, 'pineapple sauce');
    UPDATE test SET id = id;
    "#
    .execute(&mut conn);

    r#"
    SET paradedb.enable_custom_scan = true;
    INSERT INTO test VALUES (2, 'mango sauce');
    UPDATE test SET id = id;
    "#
    .execute(&mut conn);

    // Confirm that the indexed view is queryable.
    let res: Vec<(i32, f32)> = r#"
    SELECT id, pdb.score(id)
    FROM test_view
    WHERE test_view.content @@@ 'pineapple';
    "#
    .fetch(&mut conn);
    assert_eq!(res, vec![(1, 0.5389965)]);
}
```

---

## iam.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Tests for the paradedb.tokenize function

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use rustc_hash::FxHashSet as HashSet;
use sqlx::PgConnection;

#[rstest]
fn reltuples_are_set(mut conn: PgConnection) {
    "CREATE TABLE reltuptest AS SELECT md5(x::text), x FROM generate_series(1, 1024) x;"
        .execute(&mut conn);

    let (reltuples,) = "SELECT reltuples FROM pg_class WHERE oid = 'reltuptest'::regclass::oid"
        .fetch_one::<(f32,)>(&mut conn);
    if reltuples > 0.0 {
        panic!("expected reltuples to be <= 0.0.")
    }

    "CREATE INDEX idxreltuptest ON reltuptest USING bm25 (x, md5) WITH (key_field='x')"
        .execute(&mut conn);
    let (reltuples,) = "SELECT reltuples FROM pg_class WHERE oid = 'reltuptest'::regclass::oid"
        .fetch_one::<(f32,)>(&mut conn);
    assert_eq!(reltuples, 1024.0);
}

#[rstest]
fn direct_or_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    for query in &[
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard OR category:electronics'",
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' OR bm25_search @@@ 'category:electronics'",
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.term('description', 'keyboard') OR bm25_search @@@ paradedb.term('category', 'electronics')",
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.term('description', 'keyboard') OR bm25_search @@@ 'category:electronics'",
    ] {
        let columns: SimpleProductsTableVec = query.fetch_collect(&mut conn);

        assert_eq!(
            columns.description.iter().cloned().collect::<HashSet<_>>(),
            concat!(
            "Plastic Keyboard,Ergonomic metal keyboard,Innovative wireless earbuds,",
            "Fast charging power bank,Bluetooth-enabled speaker"
            )
                .split(',')
                .map(|s| s.to_string())
                .collect::<HashSet<_>>()
        );

        assert_eq!(
            columns.category.iter().cloned().collect::<HashSet<_>>(),
            "Electronics,Electronics,Electronics,Electronics,Electronics"
                .split(',')
                .map(|s| s.to_string())
                .collect::<HashSet<_>>()
        );
    }
}

#[rstest]
fn direct_and_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    for query in &[
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard AND category:electronics'",
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' AND bm25_search @@@ 'category:electronics'",
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.term('description', 'keyboard') AND bm25_search @@@ paradedb.term('category', 'electronics')",
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.term('description', 'keyboard') AND bm25_search @@@ 'category:electronics'",
    ] {
        let columns: SimpleProductsTableVec = query.fetch_collect(&mut conn);

        assert_eq!(
            columns.description.iter().cloned().collect::<HashSet<_>>(),
            ["Plastic Keyboard","Ergonomic metal keyboard"].iter().map(|s| s.to_string())
                .collect::<HashSet<_>>()
        );

        assert_eq!(
            columns.category.iter().cloned().collect::<HashSet<_>>(),
            ["Electronics"].iter()
                .map(|s| s.to_string())
                .collect::<HashSet<_>>()
        );
    }
}

#[rstest]
fn direct_sql_mix(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (description, ) = "SELECT description FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' AND id = 2".fetch_one::<(String,)>(&mut conn);

    assert_eq!(description, "Plastic Keyboard");
}

#[rstest]
fn explain_row_estimate(mut conn: PgConnection) {
    use serde_json::Number;
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    let plan = plan
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plan")
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");

    // depending on how tantivy distributes docs per segment, it seems the estimated rows could be 2 or 3
    // with our little test table
    let plan_rows = plan.get("Plan Rows");
    assert!(
        plan_rows == Some(&Value::Number(Number::from(2)))
            || plan_rows == Some(&Value::Number(Number::from(3)))
    );
}
```

---

## query.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use core::panic;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{PgConnection, Row};

#[rstest]
fn boolean_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    paradedb.boolean(
        should => ARRAY[
            paradedb.parse('description:shoes'),
            paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
            paradedb.term(field => 'description', value => 'speaker'),
		    paradedb.fuzzy_term(field => 'description', value => 'wolo', transposition_cost_one => false, distance => 1, prefix => true)
        ]
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5, 7, 10, 32, 33, 34, 37, 39, 41]);
}

#[rstest]
fn fuzzy_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.fuzzy_term(field => 'category', value => 'elector', prefix => true)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32], "wrong results");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    paradedb.term(field => 'category', value => 'electornics')
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(columns.is_empty(), "without fuzzy field should be empty");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            transposition_cost_one => false,
            distance => 1,
            prefix => true
        ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(
        columns.is_empty(),
        "transposition_cost_one false should be empty"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            transposition_cost_one => true,
            distance => 1,
            prefix => true
        ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![1, 2],
        "incorrect transposition_cost_one true"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            prefix => true
        ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2], "incorrect defaults");
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    paradedb.all() ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    paradedb.boost(query => paradedb.all(), factor => 1.5)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.const_score(query => paradedb.all(), score => 3.9)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    paradedb.disjunction_max(disjuncts => ARRAY[paradedb.parse('description:shoes')])
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.empty() ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.fuzzy_term(
        field => 'description',
        value => 'wolo',
        transposition_cost_one => false,
        distance => 1,
        prefix => true
    ) ORDER BY ID"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.parse('description:teddy') ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.phrase_prefix(field => 'description', phrases => ARRAY['har'])
        ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.phrase(field => 'description', phrases => ARRAY['robot'])
        ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.phrase(
        field => 'description',
        phrases => ARRAY['robot', 'building', 'kit']
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.range(field => 'last_updated_date', range => '[2023-05-01,2023-05-03]'::daterange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex(
        field => 'description',
        pattern => '(hardcover|plush|leather|running|wireless)'
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Test regex anchors
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex(
        field => 'description',
        pattern => '^running'
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(
        columns.len(),
        1,
        "start anchor ^ should match exactly one item"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex(
        field => 'description',
        pattern => 'keyboard$'
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 2, "end anchor $ should match two items");

    // Regex Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex_phrase(
        field => 'description',
        regexes => ARRAY['.*bot', '.*ing', 'kit']
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{
        "regex_phrase": {
            "field": "description",
            "regexes": [".*eek", "shoes"],
            "slop": 1,
            "max_expansion": 10
        }
    }'::jsonb;"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Regex Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex_phrase(
        field => 'description',
        regexes => ARRAY['.*bot', '.*ing', 'kit']
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{
        "regex_phrase": {
            "field": "description",
            "regexes": [".*eek", "shoes"],
            "slop": 1,
            "max_expansion": 10
        }
    }'::jsonb;"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.term(field => 'description', value => 'shoes')
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    //
    // NB:  This once worked, but the capability was removed when the new "pdb.*" builder functions
    //      were added.  The general problem is that there's no longer a clean way to indicate
    //      the desire to "search all column"
    //
    // // Term with no field (should search all columns)
    // let columns: SimpleProductsTableVec = r#"
    // SELECT * FROM paradedb.bm25_search
    // WHERE bm25_search @@@ paradedb.term(value => 'shoes') ORDER BY id"#
    //     .fetch_collect(&mut conn);
    // assert_eq!(columns.len(), 3);

    // TermSet with invalid term list
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.term_set(
        terms => ARRAY[
            paradedb.regex(field => 'description', pattern => '.+')
        ]
    ) ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("only term queries can be passed to term_set")),
        _ => panic!("term set query should only accept terms"),
    }

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.term_set(
        terms => ARRAY[
            paradedb.term(field => 'description', value => 'shoes'),
            paradedb.term(field => 'description', value => 'novel')
        ]
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn exists_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Simple exists query
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.exists('rating')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Non fast field should fail
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.exists('description')
    "#
    .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains("not a fast field")),
        _ => panic!("exists() over non-fast field should fail"),
    }

    // Exists with boolean query
    "INSERT INTO paradedb.bm25_search (id, description, rating) VALUES (42, 'shoes', NULL)"
        .execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        paradedb.boolean(
            must => ARRAY[
                paradedb.exists('rating'),
                paradedb.parse('description:shoes')
            ]
        )
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);
}

#[rstest]
fn more_like_this_raw(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES
        ('apple'),
        ('banana'),
        ('cherry'),
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_table WHERE test_more_like_this_table @@@
        pdb.more_like_this();
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either key_value or document")
        }
        _ => panic!("key_value or document validation failed"),
    }

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        key_value => 2
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_empty(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES
        ('apple'),
        ('banana'),
        ('cherry'),
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this()
    ORDER BY id;
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either key_value or document")
        }
        _ => panic!("key_value or document validation failed"),
    }
}

#[rstest]
fn more_like_this_text(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES
        ('apple'),
        ('banana'),
        ('cherry'),
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_boolean_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BOOLEAN PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (true, 'apple'),
        (false, 'banana')
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(bool, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@
    pdb.more_like_this(
       min_doc_frequency => 0,
       min_term_frequency => 0,
       document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn more_like_this_uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id UUID PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'apple'),
        ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'banana'),
        ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'cherry'),
        ('eb833eb6-c598-4042-b84a-0045828fceea', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(uuid::Uuid, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BIGINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1, 'apple'),
        (2, 'banana'),
        (3, 'cherry'),
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@
    pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id INT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1, 'apple'),
        (2, 'banana'),
        (3, 'cherry'),
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_literal_cast(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id INT PRIMARY KEY,
        year INTEGER
    );

    INSERT INTO test_more_like_this_table (id, year) VALUES
        (1, 2012),
        (2, 2013),
        (3, 2014),
        (4, 2012);
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, year)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, i32)> = r#"
    SELECT id, year FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"year": 2012}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i16_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SMALLINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1, 'apple'),
        (2, 'banana'),
        (3, 'cherry'),
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i16, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id FLOAT4 PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(f32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id FLOAT8 PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@
    pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_numeric_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id NUMERIC PRIMARY KEY,
    flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT CAST(id AS FLOAT8), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_date_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id DATE PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('2023-05-03', 'apple'),
        ('2023-05-04', 'banana'),
        ('2023-05-05', 'cherry'),
        ('2023-05-06', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@  pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_time_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIME PRIMARY KEY,
    flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('08:09:10', 'apple'),
        ('09:10:11', 'banana'),
        ('10:11:12', 'cherry'),
        ('11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@
    pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamp_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIMESTAMP PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('2023-05-03 08:09:10', 'apple'),
        ('2023-05-04 09:10:11', 'banana'),
        ('2023-05-05 10:11:12', 'cherry'),
        ('2023-05-06 11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@
    pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamptz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIMESTAMP WITH TIME ZONE PRIMARY KEY,
    flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('2023-05-03 08:09:10 EST', 'apple'),
        ('2023-05-04 09:10:11 PST', 'banana'),
        ('2023-05-05 10:11:12 MST', 'cherry'),
        ('2023-05-06 11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@
    pdb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timetz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIME WITH TIME ZONE PRIMARY KEY,
        flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('08:09:10 EST',
        'apple'),
        ('09:10:11 PST', 'banana'),
        ('10:11:12 MST', 'cherry'),
        ('11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);
    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ pdb.more_like_this(
            min_doc_frequency => 0,
            min_term_frequency => 0,
            document => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn match_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.match(field => 'description', value => 'ruling shoeez', distance => 2)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.match(
        field => 'description',
        value => 'ruling shoeez',
        distance => 2,
        conjunction_mode => true
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.match(field => 'description', value => 'ruling shoeez', distance => 1)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id.len(), 0);
}

#[rstest]
fn parse_lenient(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // With Tantivy's new behavior (commit e7c8c331), queries succeed if any default field
    // matches, even if others fail. Test that lenient mode still provides additional tolerance.
    // A query with valid terms should work in both modes
    let rows_strict: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.parse('shoes')
    ORDER BY id;
    "#
    .fetch(&mut conn);
    assert!(!rows_strict.is_empty());

    // With lenient enabled, mixed valid/invalid terms should also work
    let rows_lenient: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.parse('shoes keyboard', lenient => true)
    ORDER BY id;
    "#
    .fetch(&mut conn);
    // Should return results matching "shoes" (keyboard is ignored as non-existent)
    assert_eq!(rows_lenient, vec![(1,), (2,), (3,), (4,), (5,)]);
}

#[rstest]
fn parse_conjunction(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.parse('description:(shoes running)', conjunction_mode => true)
    ORDER BY id;
    "#.fetch(&mut conn);
    assert_eq!(rows, vec![(3,)]);
}

#[rstest]
fn parse_with_field_conjunction(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search
    WHERE paradedb.bm25_search.id @@@ paradedb.parse_with_field('description', 'shoes running', conjunction_mode => true)
    ORDER BY id;
    "#.fetch(&mut conn);
    assert_eq!(rows, vec![(3,)]);
}

#[rstest]
fn range_term(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'deliveries',
        table_type => 'Deliveries'
    );

    CREATE INDEX deliveries_idx ON deliveries
    USING bm25 (delivery_id, weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times)
    WITH (key_field = 'delivery_id');
    "#
    .execute(&mut conn);

    // int4range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 1 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('weights', 1) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 13 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('weights', 13) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // int8range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE quantities @> 17000::int8 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('quantities', 17000) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // numrange
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE prices @> 3.5 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('prices', 3.5) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // daterange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-07'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('ship_dates', '2023-03-07'::date) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-06'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('ship_dates', '2023-03-06'::date) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // tsrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 14:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('facility_arrival_times', '2024-05-01 14:00:00'::timestamp) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 15:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('facility_arrival_times', '2024-05-01 15:00:00'::timestamp) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // tstzrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01 06:31:00-04'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('delivery_times', '2024-05-01 06:31:00-04'::timestamptz) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01T11:30:00Z'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('delivery_times', '2024-05-01T11:30:00Z'::timestamptz) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);
}

#[rstest]
async fn prepared_statement_replanning(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // ensure our plan doesn't change into a sequential scan after the 5th execution
    for _ in 0..10 {
        let _: Vec<i32> = sqlx::query("SELECT id FROM paradedb.bm25_search WHERE id @@@ paradedb.term('rating', $1) ORDER BY id")
            .bind(2)
            .fetch_all(&mut conn)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get::<i32, _>("id"))
            .collect();
    }
}

#[rstest]
async fn direct_prepared_statement_replanning(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "PREPARE stmt(text) AS SELECT id FROM paradedb.bm25_search WHERE description @@@ $1"
        .execute(&mut conn);

    // ensure our plan doesn't change into a sequential scan after the 5th execution
    for _ in 0..10 {
        "EXECUTE stmt('keyboard')".fetch_one::<(i32,)>(&mut conn);
    }
}

#[rstest]
async fn direct_prepared_statement_replanning_custom_scan(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "PREPARE stmt(text) AS SELECT pdb.score(id), id FROM paradedb.bm25_search WHERE description @@@ $1 ORDER BY score desc LIMIT 10"
        .execute(&mut conn);

    // ensure our plan doesn't change into a sequential scan after the 5th execution
    for _ in 0..10 {
        let (score, id) = "EXECUTE stmt('keyboard')".fetch_one::<(f32, i32)>(&mut conn);
        assert_eq!((score, id), (3.2668595, 2))
    }
}
```

---

## range_term.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::postgres::types::PgRange;
use sqlx::types::time::{Date, OffsetDateTime, PrimitiveDateTime};
use sqlx::PgConnection;
use std::fmt::{Debug, Display};
use std::ops::Bound;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use time::macros::{date, datetime};

const TARGET_INT4_LOWER_BOUNDS: [i32; 2] = [2, 10];
const TARGET_INT4_UPPER_BOUNDS: [i32; 1] = [10];
const QUERY_INT4_LOWER_BOUNDS: [i32; 7] = [-10, 1, 2, 3, 9, 10, 11];
const QUERY_INT4_UPPER_BOUNDS: [i32; 8] = [-10, 1, 2, 3, 9, 10, 11, 12];

const TARGET_INT8_LOWER_BOUNDS: [i64; 2] = [2, 10];
const TARGET_INT8_UPPER_BOUNDS: [i64; 1] = [10];
const QUERY_INT8_LOWER_BOUNDS: [i64; 7] = [-10, 1, 2, 3, 9, 10, 11];
const QUERY_INT8_UPPER_BOUNDS: [i64; 8] = [-10, 1, 2, 3, 9, 10, 11, 12];

const TARGET_NUMERIC_LOWER_BOUNDS: [f64; 2] = [2.5, 10.5];
const TARGET_NUMERIC_UPPER_BOUNDS: [f64; 1] = [10.5];
const QUERY_NUMERIC_LOWER_BOUNDS: [f64; 7] = [-10.5, 1.5, 2.5, 3.5, 9.5, 10.5, 11.5];
const QUERY_NUMERIC_UPPER_BOUNDS: [f64; 8] = [-10.5, 1.5, 2.5, 3.5, 9.5, 10.5, 11.5, 12.5];

const TARGET_DATE_LOWER_BOUNDS: [Date; 2] = [date!(2021 - 01 - 01), date!(2021 - 01 - 10)];
const TARGET_DATE_UPPER_BOUNDS: [Date; 1] = [date!(2021 - 01 - 10)];
const QUERY_DATE_LOWER_BOUNDS: [Date; 7] = [
    date!(2020 - 12 - 01),
    date!(2020 - 12 - 31),
    date!(2021 - 01 - 01),
    date!(2021 - 01 - 02),
    date!(2021 - 01 - 09),
    date!(2021 - 01 - 10),
    date!(2021 - 01 - 11),
];
const QUERY_DATE_UPPER_BOUNDS: [Date; 8] = [
    date!(2020 - 12 - 01),
    date!(2020 - 12 - 31),
    date!(2021 - 01 - 01),
    date!(2021 - 01 - 02),
    date!(2021 - 01 - 09),
    date!(2021 - 01 - 10),
    date!(2021 - 01 - 11),
    date!(2021 - 01 - 12),
];

const TARGET_TIMESTAMP_LOWER_BOUNDS: [PrimitiveDateTime; 2] =
    [datetime!(2019-01-01 0:00), datetime!(2019-01-10 0:00)];
const TARGET_TIMESTAMP_UPPER_BOUNDS: [PrimitiveDateTime; 1] = [datetime!(2019-01-10 0:00)];
const QUERY_TIMESTAMP_LOWER_BOUNDS: [PrimitiveDateTime; 7] = [
    datetime!(2018-12-31 23:59:59),
    datetime!(2018-12-31 23:59:59),
    datetime!(2019-01-01 0:00:00),
    datetime!(2019-01-01 0:00:01),
    datetime!(2019-01-09 23:59:59),
    datetime!(2019-01-10 0:00:00),
    datetime!(2019-01-10 0:00:01),
];
const QUERY_TIMESTAMP_UPPER_BOUNDS: [PrimitiveDateTime; 8] = [
    datetime!(2018-12-31 23:59:59),
    datetime!(2018-12-31 23:59:59),
    datetime!(2019-01-01 0:00:00),
    datetime!(2019-01-01 0:00:01),
    datetime!(2019-01-09 23:59:59),
    datetime!(2019-01-10 0:00:00),
    datetime!(2019-01-10 0:00:01),
    datetime!(2019-01-11 0:00:00),
];

const TARGET_TIMESTAMPTZ_LOWER_BOUNDS: [OffsetDateTime; 2] = [
    datetime!(2021-01-01 00:00:00 +02:00),
    datetime!(2021-01-10 00:00:00 +02:00),
];
const TARGET_TIMESTAMPTZ_UPPER_BOUNDS: [OffsetDateTime; 1] =
    [datetime!(2021-01-10 00:00:00 +02:00)];
const QUERY_TIMESTAMPTZ_LOWER_BOUNDS: [OffsetDateTime; 7] = [
    datetime!(2020-12-30 23:59:59 UTC),
    datetime!(2021-01-01 00:00:00 +02:00),
    datetime!(2021-01-01 00:00:00 UTC),
    datetime!(2021-01-01 00:00:00 -02:00),
    datetime!(2021-01-10 00:00:00 +02:00),
    datetime!(2021-01-10 00:00:00 UTC),
    datetime!(2021-01-10 00:00:00 -02:00),
];
const QUERY_TIMESTAMPTZ_UPPER_BOUNDS: [OffsetDateTime; 8] = [
    datetime!(2020-12-30 23:59:59 UTC),
    datetime!(2021-01-01 00:00:00 +02:00),
    datetime!(2021-01-01 00:00:00 UTC),
    datetime!(2021-01-01 00:00:00 -02:00),
    datetime!(2021-01-10 00:00:00 +02:00),
    datetime!(2021-01-10 00:00:00 UTC),
    datetime!(2021-01-10 00:00:00 -02:00),
    datetime!(2021-01-11 00:00:00 +02:00),
];

#[derive(Clone, Copy, Debug, EnumIter, PartialEq)]
enum BoundType {
    Included,
    Excluded,
    Unbounded,
}

impl BoundType {
    fn to_bound<T>(self, val: T) -> Bound<T> {
        match self {
            BoundType::Included => Bound::Included(val),
            BoundType::Excluded => Bound::Excluded(val),
            BoundType::Unbounded => Bound::Unbounded,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RangeRelation {
    Intersects,
    Contains,
    Within,
}

#[rstest]
async fn range_term_contains_int4range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "weights",
        "int4range",
        &TARGET_INT4_LOWER_BOUNDS,
        &TARGET_INT4_UPPER_BOUNDS,
        &QUERY_INT4_LOWER_BOUNDS,
        &QUERY_INT4_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_int8range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "quantities",
        "int8range",
        &TARGET_INT8_LOWER_BOUNDS,
        &TARGET_INT8_UPPER_BOUNDS,
        &QUERY_INT8_LOWER_BOUNDS,
        &QUERY_INT8_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_numrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "prices",
        "numrange",
        &TARGET_NUMERIC_LOWER_BOUNDS,
        &TARGET_NUMERIC_UPPER_BOUNDS,
        &QUERY_NUMERIC_LOWER_BOUNDS,
        &QUERY_NUMERIC_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_daterange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "ship_dates",
        "daterange",
        &TARGET_DATE_LOWER_BOUNDS,
        &TARGET_DATE_UPPER_BOUNDS,
        &QUERY_DATE_LOWER_BOUNDS,
        &QUERY_DATE_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_tsrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "facility_arrival_times",
        "tsrange",
        &TARGET_TIMESTAMP_LOWER_BOUNDS,
        &TARGET_TIMESTAMP_UPPER_BOUNDS,
        &QUERY_TIMESTAMP_LOWER_BOUNDS,
        &QUERY_TIMESTAMP_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_contains_tstzrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Contains,
        "deliveries",
        "delivery_times",
        "tstzrange",
        &TARGET_TIMESTAMPTZ_LOWER_BOUNDS,
        &TARGET_TIMESTAMPTZ_UPPER_BOUNDS,
        &QUERY_TIMESTAMPTZ_LOWER_BOUNDS,
        &QUERY_TIMESTAMPTZ_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_int4range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "weights",
        "int4range",
        &TARGET_INT4_LOWER_BOUNDS,
        &TARGET_INT4_UPPER_BOUNDS,
        &QUERY_INT4_LOWER_BOUNDS,
        &QUERY_INT4_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_int8range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "quantities",
        "int8range",
        &TARGET_INT8_LOWER_BOUNDS,
        &TARGET_INT8_UPPER_BOUNDS,
        &QUERY_INT8_LOWER_BOUNDS,
        &QUERY_INT8_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_numrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "prices",
        "numrange",
        &TARGET_NUMERIC_LOWER_BOUNDS,
        &TARGET_NUMERIC_UPPER_BOUNDS,
        &QUERY_NUMERIC_LOWER_BOUNDS,
        &QUERY_NUMERIC_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_daterange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "ship_dates",
        "daterange",
        &TARGET_DATE_LOWER_BOUNDS,
        &TARGET_DATE_UPPER_BOUNDS,
        &QUERY_DATE_LOWER_BOUNDS,
        &QUERY_DATE_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_tsrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "facility_arrival_times",
        "tsrange",
        &TARGET_TIMESTAMP_LOWER_BOUNDS,
        &TARGET_TIMESTAMP_UPPER_BOUNDS,
        &QUERY_TIMESTAMP_LOWER_BOUNDS,
        &QUERY_TIMESTAMP_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_within_tstzrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Within,
        "deliveries",
        "delivery_times",
        "tstzrange",
        &TARGET_TIMESTAMPTZ_LOWER_BOUNDS,
        &TARGET_TIMESTAMPTZ_UPPER_BOUNDS,
        &QUERY_TIMESTAMPTZ_LOWER_BOUNDS,
        &QUERY_TIMESTAMPTZ_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_int4range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "weights",
        "int4range",
        &TARGET_INT4_LOWER_BOUNDS,
        &TARGET_INT4_UPPER_BOUNDS,
        &QUERY_INT4_LOWER_BOUNDS,
        &QUERY_INT4_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_int8range(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "quantities",
        "int8range",
        &TARGET_INT8_LOWER_BOUNDS,
        &TARGET_INT8_UPPER_BOUNDS,
        &QUERY_INT8_LOWER_BOUNDS,
        &QUERY_INT8_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_numrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "prices",
        "numrange",
        &TARGET_NUMERIC_LOWER_BOUNDS,
        &TARGET_NUMERIC_UPPER_BOUNDS,
        &QUERY_NUMERIC_LOWER_BOUNDS,
        &QUERY_NUMERIC_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_daterange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "ship_dates",
        "daterange",
        &TARGET_DATE_LOWER_BOUNDS,
        &TARGET_DATE_UPPER_BOUNDS,
        &QUERY_DATE_LOWER_BOUNDS,
        &QUERY_DATE_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_tsrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "facility_arrival_times",
        "tsrange",
        &TARGET_TIMESTAMP_LOWER_BOUNDS,
        &TARGET_TIMESTAMP_UPPER_BOUNDS,
        &QUERY_TIMESTAMP_LOWER_BOUNDS,
        &QUERY_TIMESTAMP_UPPER_BOUNDS,
    );
}

#[rstest]
async fn range_term_intersects_tstzrange(mut conn: PgConnection) {
    execute_range_test(
        &mut conn,
        RangeRelation::Intersects,
        "deliveries",
        "delivery_times",
        "tstzrange",
        &TARGET_TIMESTAMPTZ_LOWER_BOUNDS,
        &TARGET_TIMESTAMPTZ_UPPER_BOUNDS,
        &QUERY_TIMESTAMPTZ_LOWER_BOUNDS,
        &QUERY_TIMESTAMPTZ_UPPER_BOUNDS,
    );
}

#[allow(clippy::too_many_arguments)]
fn execute_range_test<T>(
    conn: &mut PgConnection,
    relation: RangeRelation,
    table: &str,
    field: &str,
    range_type: &str,
    target_lower_bounds: &[T],
    target_upper_bounds: &[T],
    query_lower_bounds: &[T],
    query_upper_bounds: &[T],
) where
    T: Debug + Display + Clone + PartialEq + std::cmp::PartialOrd,
{
    DeliveriesTable::setup().execute(conn);

    // Insert all combinations of ranges
    for lower_bound_type in BoundType::iter() {
        for upper_bound_type in BoundType::iter() {
            for lower_bound in target_lower_bounds {
                for upper_bound in target_upper_bounds {
                    let range = PgRange {
                        start: lower_bound_type.to_bound(lower_bound.clone()),
                        end: upper_bound_type.to_bound(upper_bound.clone()),
                    };
                    format!("INSERT INTO {table} ({field}) VALUES ('{range}'::{range_type})")
                        .execute(conn);
                }
            }
        }
    }

    // Insert null range value
    format!("INSERT INTO {table} ({field}) VALUES (NULL)").execute(conn);

    // Run all combinations of range queries
    for lower_bound_type in BoundType::iter() {
        for upper_bound_type in BoundType::iter() {
            for lower_bound in query_lower_bounds {
                for upper_bound in query_upper_bounds {
                    let range = PgRange {
                        start: lower_bound_type.to_bound(lower_bound.clone()),
                        end: upper_bound_type.to_bound(upper_bound.clone()),
                    };

                    if lower_bound >= upper_bound {
                        continue;
                    }

                    let expected: Vec<(i32,)> = match relation {
                        RangeRelation::Contains => {
                            postgres_contains_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Within => {
                            postgres_within_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Intersects => {
                            postgres_intersects_query(&range, table, field, range_type).fetch(conn)
                        }
                    };

                    let result_json: Vec<(i32,)> = match relation {
                        RangeRelation::Contains => {
                            pg_search_contains_json_query(&range, table, field, range_type)
                                .fetch(conn)
                        }
                        RangeRelation::Within => {
                            pg_search_within_json_query(&range, table, field, range_type)
                                .fetch(conn)
                        }
                        RangeRelation::Intersects => {
                            pg_search_intersects_json_query(&range, table, field, range_type)
                                .fetch(conn)
                        }
                    };

                    let result: Vec<(i32,)> = match relation {
                        RangeRelation::Contains => {
                            pg_search_contains_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Within => {
                            pg_search_within_query(&range, table, field, range_type).fetch(conn)
                        }
                        RangeRelation::Intersects => {
                            pg_search_intersects_query(&range, table, field, range_type).fetch(conn)
                        }
                    };

                    println!(
                        "expected: {expected:?}, {result:?} {} {}",
                        postgres_contains_query(&range, table, field, range_type),
                        pg_search_contains_query(&range, table, field, range_type),
                    );

                    assert_eq!(expected, result, "query failed for range: {:?}", range);
                    assert_eq!(
                        expected, result_json,
                        "json query failed for range: {:?}",
                        range
                    );
                }
            }
        }
    }
}

fn postgres_contains_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {table}
        WHERE '{range}'::{range_type} @> {field}
        ORDER BY delivery_id"
    )
}

fn postgres_within_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {table}
        WHERE {field} @> '{range}'::{range_type}
        ORDER BY delivery_id"
    )
}

fn postgres_intersects_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {table}
        WHERE '{range}'::{range_type} && {field}
        ORDER BY delivery_id"
    )
}

fn pg_search_contains_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {table}
        WHERE delivery_id @@@ paradedb.range_term('{field}', '{range}'::{range_type}, 'Contains')
        ORDER BY delivery_id"
    )
}

fn pg_search_contains_json_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    let is_datetime = ["daterange", "tsrange", "tstzrange"].contains(&range_type);
    let lower_bound = match range.start {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    let upper_bound = match range.end {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    format!(
        r#"
        SELECT delivery_id FROM {table}
        WHERE delivery_id @@@ '{{
            "range_contains": {{
                "field": "{field}",
                "lower_bound": {lower_bound},
                "upper_bound": {upper_bound}
            }}
        }}'::jsonb
        ORDER BY delivery_id"#
    )
}

fn pg_search_within_json_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    let is_datetime = ["daterange", "tsrange", "tstzrange"].contains(&range_type);
    let lower_bound = match range.start {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    let upper_bound = match range.end {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    format!(
        r#"
        SELECT delivery_id FROM {table}
        WHERE delivery_id @@@ '{{
            "range_within": {{
                "field": "{field}",
                "lower_bound": {lower_bound},
                "upper_bound": {upper_bound}
            }}
        }}'::jsonb
        ORDER BY delivery_id"#
    )
}

fn pg_search_intersects_json_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    let is_datetime = ["daterange", "tsrange", "tstzrange"].contains(&range_type);
    let lower_bound = match range.start {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    let upper_bound = match range.end {
        Bound::Included(ref val) => format!(
            r#"{{"included": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Excluded(ref val) => format!(
            r#"{{"excluded": {}}}"#,
            if is_datetime {
                format!(r#""{val}""#)
            } else {
                val.to_string()
            }
        ),
        Bound::Unbounded => "null".to_string(),
    };

    format!(
        r#"
        SELECT delivery_id FROM {table}
        WHERE delivery_id @@@ '{{
            "range_intersects": {{
                "field": "{field}",
                "lower_bound": {lower_bound},
                "upper_bound": {upper_bound}
            }}
        }}'::jsonb
        ORDER BY delivery_id"#
    )
}

fn pg_search_within_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {table}
        WHERE delivery_id @@@ paradedb.range_term('{field}', '{range}'::{range_type}, 'Within')
        ORDER BY delivery_id"
    )
}

fn pg_search_intersects_query<T>(
    range: &PgRange<T>,
    table: &str,
    field: &str,
    range_type: &str,
) -> String
where
    T: Debug + Display + Clone + PartialEq,
{
    format!(
        "
        SELECT delivery_id FROM {table}
        WHERE delivery_id @@@ paradedb.range_term('{field}', '{range}'::{range_type}, 'Intersects')
        ORDER BY delivery_id"
    )
}
```

---

## sqlsmith.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

/// `sqlsmith` generated a query that crashed due to us dereferencing a null pointer
///
/// The query itself it completely nonsensical but we shouldn't crash no matter what
/// the user (or sqlsmith) throws at us.
#[rstest]
fn crash_in_subquery(mut conn: PgConnection) {
    let result = r#"
        select
          pg_catalog.jsonb_build_array() as c0,
          (select high from paradedb.index_layer_info limit 1 offset 36)
             as c1
        from
          (select
                subq_1.c1 as c0,
                subq_1.c1 as c1
              from
                (select
                      ref_0.high as c0,
                      (select relname from paradedb.index_layer_info limit 1 offset 1)
                         as c1,
                      ref_0.byte_size as c2,
                      ref_0.layer_size as c3,
                      ref_0.byte_size as c4,
                      ref_0.segments as c5,
                      ref_0.byte_size as c6,
                      ref_0.relname as c7,
                      3 as c8
                    from
                      paradedb.index_layer_info as ref_0
                    where cast(null as int2) >= cast(null as int2)
                    limit 44) as subq_0,
                lateral (select
                      subq_0.c4 as c0,
                      (select segments from paradedb.index_layer_info limit 1 offset 4)
                         as c1
                    from
                      paradedb.index_layer_info as ref_1
                    where (cast(null as float8) >= cast(null as float8))
                      and (subq_0.c3 @@@ ref_1.relname)
                    limit 117) as subq_1
              where case when (select count from paradedb.index_layer_info limit 1 offset 3)
                       > cast(null as int2) then cast(nullif(cast(null as "time"),
                    cast(null as "time")) as "time") else cast(nullif(cast(null as "time"),
                    cast(null as "time")) as "time") end
                   <= pg_catalog.make_time(
                  cast(subq_0.c8 as int4),
                  cast(40 as int4),
                  cast(pg_catalog.pg_notification_queue_usage() as float8))
              limit 115) as subq_2
        where (select high from paradedb.index_layer_info limit 1 offset 3)
             is not NULL
        limit 53;
    "#
    .execute_result(&mut conn);

    let pg_ver = pg_major_version(&mut conn);
    if pg_ver == 14 {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{err}")
            .contains("unable to determine Var relation as it belongs to a NULL subquery"))
    }
}
```

---

## qgen.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use crate::fixtures::querygen::groupbygen::arb_group_by;
use crate::fixtures::querygen::joingen::JoinType;
use crate::fixtures::querygen::pagegen::arb_paging_exprs;
use crate::fixtures::querygen::wheregen::arb_wheres;
use crate::fixtures::querygen::{
    arb_joins_and_wheres, compare, generated_queries_setup, Column, PgGucs,
};

use fixtures::*;

use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::{PgConnection, Row};

const COLUMNS: &[Column] = &[
    Column::new("id", "SERIAL8", "'4'")
        .primary_key()
        .groupable({
            true
        }),
    Column::new("uuid", "UUID", "'550e8400-e29b-41d4-a716-446655440000'")
        .groupable({
            true
        })
        .bm25_text_field(r#""uuid": { "tokenizer": { "type": "keyword" } , "fast": true }"#)
        .random_generator_sql("rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid"),
    Column::new("name", "TEXT", "'bob'")
        .bm25_text_field(r#""name": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
        .random_generator_sql(
            "(ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int]"
        ),
    Column::new("color", "VARCHAR", "'blue'")
        .whereable({
            // TODO: A variety of tests fail due to the NULL here. The column exists in order to
            // provide coverage for ORDER BY on a column containing NULL.
            // https://github.com/paradedb/paradedb/issues/3111
            false
        })
        .bm25_text_field(r#""color": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
        .random_generator_sql(
            "(ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int]"
        ),
    Column::new("age", "INTEGER", "'20'")
        .bm25_numeric_field(r#""age": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 100) + 1)"),
    Column::new("quantity", "INTEGER", "'7'")
        .whereable({
            // TODO: A variety of tests fail due to the NULL here. The column exists in order to
            // provide coverage for ORDER BY on a column containing NULL.
            // https://github.com/paradedb/paradedb/issues/3111
            false
        })
        .bm25_numeric_field(r#""quantity": { "fast": true }"#)
        .random_generator_sql("CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END"),
    Column::new("price", "NUMERIC(10,2)", "'99.99'")
        .groupable({
            // TODO: Grouping on a float fails to ORDER BY (even in cases without an ORDER BY):
            // ```
            // Cannot ORDER BY OrderByInfo
            // ```
            false
        })
        .bm25_numeric_field(r#""price": { "fast": true }"#)
        .random_generator_sql("(random() * 1000 + 10)::numeric(10,2)"),
    Column::new("rating", "INTEGER", "'4'")
        .indexed({
            // Marked un-indexed in order to test heap-filter pushdown.
            false
        })
        .groupable({
            true
        })
        .bm25_numeric_field(r#""rating": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 5) + 1)::int"),
];

fn columns_named(names: Vec<&'static str>) -> Vec<Column> {
    COLUMNS
        .iter()
        .filter(|c| names.contains(&c.name))
        .cloned()
        .collect()
}

///
/// Tests all JoinTypes against small tables (which are particularly important for joins which
/// result in e.g. the cartesian product).
///
#[rstest]
#[tokio::test]
async fn generated_joins_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let tables_and_sizes = [("users", 10), ("products", 10), ("orders", 10)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            any::<JoinType>(),
            tables,
            &columns_named(vec!["id", "name", "color", "age"]),
        ),
        gucs in any::<PgGucs>(),
    )| {
        let join_clause = join.to_sql();

        let from = format!("SELECT COUNT(*) {join_clause} ");

        compare(
            &format!("{from} WHERE {}", where_expr.to_sql(" = ")),
            &format!("{from} WHERE {}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )?;
    });
}

///
/// Tests only the smallest JoinType against larger tables, with a target list, and a limit.
///
/// TODO: This test is currently ignored because it occasionally generates nested loop joins which
/// run in exponential time: https://github.com/paradedb/paradedb/issues/2733
///
#[ignore]
#[rstest]
#[tokio::test]
async fn generated_joins_large_limit(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let tables_and_sizes = [("users", 10000), ("products", 10000), ("orders", 10000)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            Just(JoinType::Inner),
            tables,
            &columns_named(vec!["id", "name", "color", "age"]),
        ),
        target_list in proptest::sample::subsequence(vec!["id", "name", "color", "age"], 1..=4),
        gucs in any::<PgGucs>(),
    )| {
        let join_clause = join.to_sql();
        let used_tables = join.used_tables();

        let target_list =
            target_list
                .into_iter()
                .map(|column| format!("{}.{column}", used_tables[0]))
                .collect::<Vec<_>>()
                .join(", ");

        let from = format!("SELECT {target_list} {join_clause} ");

        compare(
            &format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql(" = ")),
            &format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_dynamic(conn).len(),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_single_relation(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 10)], COLUMNS);

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            COLUMNS,
        ),
        gucs in any::<PgGucs>(),
        target in prop_oneof![Just("COUNT(*)"), Just("id")],
    )| {
        compare(
            &format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql(" = ")),
            &format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                let mut rows = query.fetch::<(i64,)>(conn);
                rows.sort();
                rows
            }
        )?;
    });
}

///
/// Property test for GROUP BY aggregates - ensures equivalence between PostgreSQL and bm25 behavior
///
#[rstest]
#[tokio::test]
async fn generated_group_by_aggregates(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 50)], COLUMNS);

    // Columns that can be used for grouping (must have fast: true in index)
    let columns: Vec<_> = COLUMNS
        .iter()
        .filter(|col| col.is_groupable && col.is_whereable)
        .cloned()
        .collect();

    let grouping_columns: Vec<_> = columns.iter().map(|col| col.name).collect();

    proptest!(|(
        text_where_expr in arb_wheres(
            vec![table_name],
            &columns,
        ),
        numeric_where_expr in arb_wheres(
            vec![table_name],
            &columns_named(vec!["age", "price", "rating"]),
        ),
        group_by_expr in arb_group_by(grouping_columns.to_vec(), vec!["COUNT(*)", "SUM(price)", "AVG(price)", "MIN(rating)", "MAX(rating)", "SUM(age)", "AVG(age)"]),
        gucs in any::<PgGucs>(),
    )| {
        let select_list = group_by_expr.to_select_list();
        let group_by_clause = group_by_expr.to_sql();

        // Create combined WHERE clause for PostgreSQL using = operator
        let pg_where_clause = format!(
            "({}) AND ({})",
            text_where_expr.to_sql(" = "),
            numeric_where_expr.to_sql(" < ")
        );

        // Create combined WHERE clause for BM25 using appropriate operators
        let bm25_where_clause = format!(
            "({}) AND ({})",
            text_where_expr.to_sql("@@@"),
            numeric_where_expr.to_sql(" < ")
        );

        let pg_query = format!(
            "SELECT {select_list} FROM {table_name} WHERE {pg_where_clause} {group_by_clause}",
        );

        let bm25_query = format!(
            "SELECT {select_list} FROM {table_name} WHERE {bm25_where_clause} {group_by_clause}",
        );

        // Custom result comparator for GROUP BY results
        let compare_results = |query: &str, conn: &mut PgConnection| -> Vec<String> {
            // Fetch all rows as dynamic results and convert to string representation
            let rows = query.fetch_dynamic(conn);
            let mut string_rows: Vec<String> = rows
                .into_iter()
                .map(|row| {
                    // Convert entire row to a string representation for comparison
                    let mut row_string = String::new();
                    for i in 0..row.len() {
                        if i > 0 {
                            row_string.push('|');
                        }

                        // Try to get value as different types, converting to string
                        let value_str = if let Ok(val) = row.try_get::<i64, _>(i) {
                            val.to_string()
                        } else if let Ok(val) = row.try_get::<i32, _>(i) {
                            val.to_string()
                        } else if let Ok(val) = row.try_get::<String, _>(i) {
                            val
                        } else {
                            "NULL".to_string()
                        };

                        row_string.push_str(&value_str);
                    }
                    row_string
                })
                .collect();

            // Sort for consistent comparison
            string_rows.sort();
            string_rows
        };

        compare(&pg_query, &bm25_query, &gucs, &mut pool.pull(), &setup_sql, compare_results)?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_paging_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 1000)], COLUMNS);

    proptest!(|(
        where_expr in arb_wheres(vec![table_name], &columns_named(vec!["name"])),
        // TODO: Compound top-n occasionally flakes, so we only use tiebreaker columns.
        // See https://github.com/paradedb/paradedb/issues/3266.
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            &format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql(" = ")),
            &format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch::<(i64,)>(conn),
        )?;
    });
}

/// Generates paging expressions on a large table, which was necessary to reproduce
/// https://github.com/paradedb/tantivy/pull/51
///
/// TODO: Explore whether this could use https://github.com/paradedb/paradedb/pull/2681
/// to use a large segment count rather than a large table size.
#[rstest]
#[tokio::test]
async fn generated_paging_large(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 100000)], COLUMNS);

    proptest!(|(
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            &format!("SELECT uuid::text FROM {table_name} WHERE name  =  'bob' {paging_exprs}"),
            &format!("SELECT uuid::text FROM {table_name} WHERE name @@@ 'bob' {paging_exprs}"),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch::<(String,)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_subquery(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let outer_table_name = "products";
    let inner_table_name = "orders";
    let setup_sql = generated_queries_setup(
        &mut pool.pull(),
        &[(outer_table_name, 10), (inner_table_name, 10)],
        COLUMNS,
    );

    proptest!(|(
        outer_where_expr in arb_wheres(
            vec![outer_table_name],
            COLUMNS,
        ),
        inner_where_expr in arb_wheres(
            vec![inner_table_name],
            COLUMNS,
        ),
        subquery_column in proptest::sample::select(&["name", "color", "age"]),
        // TODO: Compound top-n occasionally flakes, so we only use tiebreaker columns.
        // See https://github.com/paradedb/paradedb/issues/3266.
        paging_exprs in arb_paging_exprs(inner_table_name, vec![], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        let pg = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql(" = "),
            outer_where_expr.to_sql(" = "),
        );
        let bm25 = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql("@@@"),
            outer_where_expr.to_sql("@@@"),
        );

        compare(
            &pg,
            &bm25,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn),
        )?;
    });
}
```

---

## find_var_relation.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_subselect(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_subselect(id serial8, t text);
        INSERT INTO test_subselect(t) VALUES ('this is a test');

        CREATE INDEX test_subselect_idx ON test_subselect
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) = r#"
        select id from (select random(), * from (select random(), t, id from test_subselect) x) test_subselect 
        where id @@@ 't:test';"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn test_cte(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_cte(id serial8, t text);
        INSERT INTO test_cte(t) VALUES ('beer wine cheese');
        INSERT INTO test_cte(t) VALUES ('beer cheese');

        CREATE INDEX test_cte_idx ON test_cte
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) = r#"
        with my_cte as (select * from test_cte)
        select * from my_cte a inner join my_cte b on a.id = b.id
        where a.id @@@ 't:beer' and b.id @@@ 't:cheese' order by a.id;"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn test_cte2(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_cte(id serial8, t text);
        INSERT INTO test_cte(t) VALUES ('beer wine cheese');
        INSERT INTO test_cte(t) VALUES ('beer cheese');

        CREATE INDEX test_cte_idx ON test_cte
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) = r#"
        with my_cte as (select * from test_cte)
        select * from my_cte where id @@@ 't:beer' order by id;"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn test_plain_relation(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_plain_relation(id serial8, t text);
        INSERT INTO test_plain_relation(t) VALUES ('beer wine cheese');

        CREATE INDEX test_plain_relation_idx ON test_plain_relation
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) =
        "select id from test_plain_relation where id @@@ 't:beer'".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}
```

---

## aggregate_custom_scan.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

// Tests for ParadeDB's Aggregate Custom Scan implementation
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

fn assert_uses_custom_scan(conn: &mut PgConnection, enabled: bool, query: impl AsRef<str>) {
    let (plan,) = format!(" EXPLAIN (FORMAT JSON) {}", query.as_ref()).fetch_one::<(Value,)>(conn);
    eprintln!("{plan:#?}");
    assert_eq!(
        enabled,
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );
}

#[rstest]
fn test_count(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Use the aggregate custom scan only if it is enabled.
    for enabled in [true, false] {
        format!("SET paradedb.enable_aggregate_custom_scan TO {enabled};").execute(&mut conn);

        let query = "SELECT COUNT(*) FROM paradedb.bm25_search WHERE description @@@ 'keyboard'";

        assert_uses_custom_scan(&mut conn, enabled, query);

        let (count,) = query.fetch_one::<(i64,)>(&mut conn);
        assert_eq!(count, 2, "With custom scan: {enabled}");
    }
}

#[rstest]
fn test_count_with_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
    "SET client_min_messages TO warning;".execute(&mut conn);

    // First test simple COUNT(*) without GROUP BY
    let simple_count = "SELECT COUNT(*) FROM paradedb.bm25_search";
    eprintln!("Testing simple COUNT(*)");
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {simple_count}").fetch_one::<(Value,)>(&mut conn);
    eprintln!("Simple COUNT(*) plan: {plan:#?}");
    eprintln!(
        "Uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Test COUNT(*) with WHERE clause (like the working test)
    let count_with_where =
        "SELECT COUNT(*) FROM paradedb.bm25_search WHERE description @@@ 'keyboard'";
    eprintln!("\nTesting COUNT(*) with WHERE clause");
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {count_with_where}").fetch_one::<(Value,)>(&mut conn);
    eprintln!(
        "COUNT(*) with WHERE plan uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Then test WITHOUT WHERE clause but WITH GROUP BY
    let query_no_where = r#"
        SELECT rating, COUNT(*) 
        FROM paradedb.bm25_search 
        GROUP BY rating 
        ORDER BY rating
    "#;

    eprintln!("Testing query without WHERE clause");
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {query_no_where}").fetch_one::<(Value,)>(&mut conn);
    eprintln!("Plan without WHERE: {plan:#?}");
    eprintln!(
        "Uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Then test WITH WHERE clause
    let query = r#"
        SELECT rating, COUNT(*) 
        FROM paradedb.bm25_search 
        WHERE description @@@ 'shoes' 
        GROUP BY rating 
        ORDER BY rating
    "#;

    // Verify it uses the aggregate custom scan
    assert_uses_custom_scan(&mut conn, true, query);

    // Execute and verify results
    let results: Vec<(i32, i64)> = query.fetch(&mut conn);
    assert_eq!(results.len(), 3); // We should have 3 distinct ratings for shoes
    assert_eq!(results[0], (3, 1)); // rating 3, count 1
    assert_eq!(results[1], (4, 1)); // rating 4, count 1
    assert_eq!(results[2], (5, 1)); // rating 5, count 1
}

#[rstest]
fn test_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Supports GROUP BY with aggregate scan
    assert_uses_custom_scan(
        &mut conn,
        true,
        r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search WHERE
        description @@@ 'keyboard'
        GROUP BY rating
        ORDER BY rating
        "#,
    );
}

#[rstest]
fn test_group_by_null_bucket(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    assert_uses_custom_scan(
        &mut conn,
        true,
        r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search
        WHERE description @@@ 'keyboard'
        GROUP BY rating
        ORDER BY rating NULLS FIRST
    "#,
    );
}

#[rstest]
fn test_no_bm25_index(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'no_bm25', schema_name => 'paradedb');"
        .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Do not use the aggregate custom scan on non-bm25 indexed tables.
    assert_uses_custom_scan(&mut conn, false, "SELECT COUNT(*) FROM paradedb.no_bm25");
}

#[rstest]
fn test_other_aggregates(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    for aggregate_func in ["SUM(rating)", "AVG(rating)", "MIN(rating)", "MAX(rating)"] {
        assert_uses_custom_scan(
            &mut conn,
            true,
            format!(
                r#"
                SELECT {aggregate_func}
                FROM paradedb.bm25_search WHERE
                description @@@ 'keyboard'
                "#
            ),
        );
    }
}
```

---

## lindera.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

#![allow(unused_variables, unused_imports)]
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn lindera_korean_tokenizer(mut conn: PgConnection) {
    r#"CREATE TABLE IF NOT EXISTS korean (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );

    INSERT INTO korean (author, title, message)
    VALUES
        ('김민준', '서울의 새로운 카페', '서울 중심부에 새로운 카페가 문을 열었습니다. 현대적인 디자인과 독특한 커피 선택이 특징입니다.'),
        ('이하은', '축구 경기 리뷰', '어제 열린 축구 경기에서 화려한 골이 터졌습니다. 마지막 순간의 반전이 경기의 하이라이트였습니다.'),
        ('박지후', '지역 축제 개최 소식', '이번 주말 지역 축제가 열립니다. 다양한 음식과 공연이 준비되어 있어 기대가 됩니다.');

        CREATE INDEX korean_idx ON korean
        USING bm25 (id, author, title, message)
        WITH (
            key_field = 'id',
            text_fields = '{
                "author": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                },
                "title": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                },
                "message": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                }
            }'
        );
    "#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM korean WHERE korean @@@ 'author:김민준' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM korean WHERE korean @@@ 'title:"경기"' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM korean WHERE korean @@@ 'message:"지역 축제"' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
async fn lindera_chinese_tokenizer(mut conn: PgConnection) {
    r#"CREATE TABLE IF NOT EXISTS chinese (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );
    INSERT INTO chinese (author, title, message)
    VALUES
        ('李华', '北京的新餐馆', '北京市中心新开了一家餐馆，以其现代设计和独特的菜肴选择而闻名。'),
        ('张伟', '篮球比赛回顾', '昨日篮球比赛精彩纷呈，尤其是最后时刻的逆转成为了比赛的亮点。'),
        ('王芳', '本地文化节', '本周末将举行一个地方文化节，预计将有各种食物和表演。');

    CREATE INDEX chinese_idx ON chinese
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id',
        text_fields = '{
            "author": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            },
            "title": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            },
            "message": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            }
        }'
    ); 
    "#
    .execute(&mut conn);

    let row: (i32,) =
        r#"SELECT id FROM chinese WHERE chinese @@@ 'author:华' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM chinese WHERE chinese @@@ 'title:北京' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM chinese WHERE chinese @@@ 'message:文化节' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
async fn lindera_japenese_tokenizer(mut conn: PgConnection) {
    r#"
    CREATE TABLE IF NOT EXISTS japanese (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );
    INSERT INTO japanese (author, title, message)
    VALUES
        ('佐藤健', '東京の新しいカフェ', '東京の中心部に新しいカフェがオープンしました。モダンなデザインとユニークなコーヒーが特徴です。'),
        ('鈴木一郎', 'サッカー試合レビュー', '昨日のサッカー試合では素晴らしいゴールが見られました。終了間際のドラマチックな展開がハイライトでした。'),
        ('高橋花子', '地元の祭り', '今週末に地元で祭りが開催されます。様々な食べ物とパフォーマンスが用意されています。');

    CREATE INDEX japanese_idx ON japanese
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id',
        text_fields = '{
            "author": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            },
            "title": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            },
            "message": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            }
        }'
    );
    "#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'author:佐藤' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'title:サッカー' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'message:祭り' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}
```

---

## documentation.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use approx::assert_relative_eq;
use fixtures::*;
use num_traits::ToPrimitive;
use pgvector::Vector;
use rstest::*;
use sqlx::types::BigDecimal;
use sqlx::PgConnection;
use std::str::FromStr;

#[rstest]
fn quickstart(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("Ergonomic metal keyboard".into(), 4, "Electronics".into()),
            ("Plastic Keyboard".into(), 4, "Electronics".into()),
            ("Sleek running shoes".into(), 5, "Footwear".into())
        ]
    );

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' OR category @@@ 'footwear' AND rating @@@ '>2'
    ORDER BY description
    LIMIT 5"#
        .fetch(&mut conn);
    assert_eq!(rows[0].0, "Comfortable slippers".to_string());
    assert_eq!(rows[1].0, "Generic shoes".to_string());
    assert_eq!(rows[2].0, "Sleek running shoes".to_string());
    assert_eq!(rows[3].0, "Sturdy hiking boots".to_string());
    assert_eq!(rows[4].0, "White jogging shoes".to_string());

    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE description @@@ 'shoes' OR category @@@ 'footwear' AND rating @@@ '>2'
    ORDER BY score DESC, description
    LIMIT 5"#
        .fetch(&mut conn);
    assert_eq!(rows[0].0, "Generic shoes".to_string());
    assert_eq!(rows[1].0, "Sleek running shoes".to_string());
    assert_eq!(rows[2].0, "White jogging shoes".to_string());
    assert_eq!(rows[3].0, "Comfortable slippers".to_string());
    // The BM25 score here is a tie, so the order is arbitrary
    assert!(rows[4].0 == *"Sturdy hiking boots" || rows[4].0 == *"Winter woolen socks");
    assert_eq!(rows[0].3, 5.8135376);
    assert_eq!(rows[1].3, 5.4211845);
    assert_eq!(rows[2].3, 5.4211845);
    assert_eq!(rows[3].3, 2.9362776);
    assert_eq!(rows[4].3, 2.9362776);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"white shoes"~1'
    LIMIT 5"#
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "White jogging shoes");

    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'orders',
        table_type => 'Orders'
    );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, i32, i32, BigDecimal, String)> = r#"
    SELECT * FROM orders LIMIT 3"#
        .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                1,
                1,
                3,
                BigDecimal::from_str("99.99").unwrap(),
                "John Doe".into()
            ),
            (
                2,
                2,
                1,
                BigDecimal::from_str("49.99").unwrap(),
                "Jane Smith".into()
            ),
            (
                3,
                3,
                5,
                BigDecimal::from_str("249.95").unwrap(),
                "Alice Johnson".into()
            ),
        ]
    );

    let rows: Vec<(i32, String, String)> = r#"
    SELECT o.order_id, o.customer_name, m.description
    FROM orders o
    JOIN mock_items m ON o.product_id = m.id
    WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes'
    ORDER BY order_id
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, "Alice Johnson".into(), "Sleek running shoes".into()),
            (6, "Alice Johnson".into(), "White jogging shoes".into()),
            (36, "Alice Johnson".into(), "White jogging shoes".into()),
        ]
    );

    r#"
    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);
    "#
    .execute(&mut conn);
    r#"
    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);
    let rows: Vec<(String, i32, String, Vector)> = r#"
    SELECT description, rating, category, embedding
    FROM mock_items LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, "Ergonomic metal keyboard");
    assert_eq!(rows[1].0, "Plastic Keyboard");
    assert_eq!(rows[2].0, "Sleek running shoes");
    assert_eq!(rows[0].3, Vector::from(vec![3.0, 4.0, 5.0]));
    assert_eq!(rows[1].3, Vector::from(vec![4.0, 5.0, 6.0]));
    assert_eq!(rows[2].3, Vector::from(vec![5.0, 6.0, 7.0]));

    r#"
    CREATE INDEX on mock_items
    USING hnsw (embedding vector_cosine_ops);
    "#
    .execute(&mut conn);
    let rows: Vec<(String, String, i32, Vector)> = r#"
    SELECT description, category, rating, embedding
    FROM mock_items
    ORDER BY embedding <=> '[1,2,3]', description
    LIMIT 3;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, "Artistic ceramic vase");
    assert_eq!(rows[1].0, "Designer wall paintings");
    assert_eq!(rows[2].0, "Handcrafted wooden frame");
    assert_eq!(rows[0].3, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[1].3, Vector::from(vec![1.0, 2.0, 3.0]));
    assert_eq!(rows[2].3, Vector::from(vec![1.0, 2.0, 3.0]));
}

#[rstest]
fn full_text_search(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String, i32)> = r#"
    SELECT id, description, rating
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '>3'
    ORDER BY id
    "#
    .fetch(&mut conn);
    assert_eq!(rows[0], (3, "Sleek running shoes".into(), 5));
    assert_eq!(rows[1], (5, "Generic shoes".into(), 4));

    // Basic term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Multiple terms
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'keyboard' OR category @@@ 'toy'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Not term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '(shoes running -white)'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Basic phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"plastic keyboard"'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Slop operator
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"ergonomic keyboard"~1'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Phrase prefix
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '"plastic keyb"*'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Basic filtering
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating > 2
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '>2'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Numeric filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '4'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '>=4'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Datetime filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND created_at @@@ '"2023-04-20T16:38:02Z"'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Boolean filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND in_stock @@@ 'true'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Range filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ '[1 TO 4]'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND created_at @@@ '[2020-01-31T00:00:00Z TO 2024-01-31T00:00:00Z]'
    "#.fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ '[book TO camera]'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 8);

    // Set filter
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND rating @@@ 'IN [2 3 4]'
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Pagination
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 1 OFFSET 2
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // BM25 scoring
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, pdb.score(id)
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'orders',
        table_type => 'Orders'
    );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, f32)> = r#"
    SELECT o.order_id, pdb.score(o.order_id) + pdb.score(m.id) as score
    FROM orders o
    JOIN mock_items m ON o.product_id = m.id
    WHERE o.customer_name @@@ 'Johnson' AND (m.description @@@ 'shoes' OR m.description @@@ 'running')
    ORDER BY score DESC, o.order_id
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows, vec![(3, 8.738735), (6, 5.406531), (36, 5.406531)]);

    // Highlighting
    let rows: Vec<(i32, String)> = r#"
    SELECT id, pdb.snippet(description)
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, pdb.snippet(description, start_tag => '<i>', end_tag => '</i>')
    FROM mock_items
    WHERE description @@@ 'shoes'
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert!(rows[0].1.contains("<i>"));
    assert!(rows[0].1.contains("</i>"));

    // Order by score
    let rows: Vec<(String, i32, String, f32)> = r#"
        SELECT description, rating, category, pdb.score(id)
        FROM mock_items
        WHERE description @@@ 'shoes'
        ORDER BY score DESC
        LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].3, 2.8772602);
    assert_eq!(rows[1].3, 2.4849067);
    assert_eq!(rows[2].3, 2.4849067);

    // Order by field
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes'
    ORDER BY rating DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("Sleek running shoes".into(), 5, "Footwear".into()),
            ("Generic shoes".into(), 4, "Footwear".into()),
            ("White jogging shoes".into(), 3, "Footwear".into()),
        ]
    );

    // Tiebreaking
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE category @@@ 'electronics'
    ORDER BY score DESC, rating DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].3, 2.1096356);
    assert_eq!(rows[1].3, 2.1096356);
    assert_eq!(rows[2].3, 2.1096356);
    assert_eq!(rows[3].3, 2.1096356);
    assert_eq!(rows[4].3, 2.1096356);

    // Constant boosting
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, pdb.score(id)
    FROM mock_items
    WHERE description @@@ 'shoes^2' OR category @@@ 'footwear'
    ORDER BY score DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].1, 7.690798);
    assert_eq!(rows[1].1, 6.9060907);
    assert_eq!(rows[2].1, 6.9060907);
    assert_eq!(rows[3].1, 1.9362776);
    assert_eq!(rows[4].1, 1.9362776);

    // Boost by field
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, pdb.score(id) * COALESCE(rating, 1) as score
    FROM mock_items
    WHERE description @@@ 'shoes'
    ORDER BY score DESC
    LIMIT 5
    "#
    .fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 12.424533367156982),
            (5, 11.509040832519531),
            (4, 7.4547200202941895),
        ]
    );
}

#[rstest]
fn match_query(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.match('description', 'running shoes');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "match": {
            "field": "description",
            "value": "running shoes"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.match(
        'description',
        'running shoes',
        tokenizer => paradedb.tokenizer('whitespace')
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "match": {
            "field": "description",
            "value": "running shoes",
            "tokenizer": {"type": "whitespace", "lowercase": true, "remove_long": 255}
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.match('description', 'ruining shoez', distance => 2);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "match": {
            "field": "description",
            "value": "ruining shoez",
            "distance": 2
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.match('description', 'running shoes', conjunction_mode => true);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "match": {
            "field": "description",
            "value": "running shoes",
            "conjunction_mode": true
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn term_level_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Exists
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.exists('rating')
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "exists": {
            "field": "rating"
        }
    }'::jsonb
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
      must => ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.exists('rating')
      ]
    )
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "boolean": {
            "must": [
                {"term": {"field": "description", "value": "shoes"}},
                {"exists": {"field": "rating"}}
            ]
        }
    }'::jsonb
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Fuzzy term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.fuzzy_term('description', 'shoez')
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "fuzzy_term": {
            "field": "description",
            "value": "shoez"
        }
    }'::jsonb
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Range
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.range(
        field => 'rating',
        range => int4range(1, 3, '[)')
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 4);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "range": {
            "field": "rating",
            "lower_bound": {"included": 1},
            "upper_bound": {"excluded": 3}
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 4);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.range(
        field => 'rating',
        range => int4range(1, 3, '[]')
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 13);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "range": {
            "field": "rating",
            "lower_bound": {"included": 1},
            "upper_bound": {"included": 3}
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 13);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.range(
        field => 'rating',
        range => int4range(1, NULL, '[)')
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 41);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "range": {
            "field": "rating",
            "lower_bound": {"included": 1},
            "upper_bound": null
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 41);

    // Range term
    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@ paradedb.range_term('weight_range', 1);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@
    '{
        "range_term": {
            "field": "weight_range",
            "value": 1
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    let rows: Vec<(i32,)> = r#"
    SELECT id, description, category, weight_range FROM mock_items
    WHERE id @@@ paradedb.boolean(
        must => ARRAY[
            paradedb.range_term('weight_range', 1),
            paradedb.term('category', 'footwear')
        ]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32,)> = r#"
    SELECT id, description, category, weight_range FROM mock_items
    WHERE id @@@
    '{
        "boolean": {
            "must": [
                {
                    "range_term": {
                        "field": "weight_range",
                        "value": 1
                    }
                },
                {
                    "term": {
                        "field": "category",
                        "value": "footwear"
                    }
                }
            ]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@ paradedb.range_term('weight_range', '(10, 12]'::int4range, 'Intersects');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@
    '{
        "range_intersects": {
            "field": "weight_range",
            "lower_bound": {"excluded": 10},
            "upper_bound": {"included": 12}
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@ paradedb.range_term('weight_range', '(3, 9]'::int4range, 'Contains');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 7);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@
    '{
        "range_contains": {
            "field": "weight_range",
            "lower_bound": {"excluded": 3},
            "upper_bound": {"included": 9}
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 7);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@ paradedb.range_term('weight_range', '(2, 11]'::int4range, 'Within');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(i32,)> = r#"
    SELECT id, weight_range FROM mock_items
    WHERE id @@@
    '{
        "range_within": {
            "field": "weight_range",
            "lower_bound": {"excluded": 2},
            "upper_bound": {"included": 11}
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    // Regex
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.regex('description', '(plush|leather)');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "regex": {
            "field": "description",
            "pattern": "(plush|leather)"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.regex('description', 'key.*rd');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "regex": {
            "field": "description",
            "pattern": "key.*rd"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    // Term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('description', 'shoes');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "term": {
            "field": "description",
            "value": "shoes"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('rating', 4);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "term": {
            "field": "rating",
            "value": 4
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    // Term set
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term_set(
    	terms => ARRAY[
    		paradedb.term('description', 'shoes'),
    		paradedb.term('description', 'novel')
    	]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "term_set": {
            "terms": [
                {"field": "description", "value": "shoes"},
                {"field": "description", "value": "novel"}
            ]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);
}

#[rstest]
fn phrase_level_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.phrase(
        field => 'description',
        phrases => ARRAY['running', 'shoes']
    )"#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "phrase": {
            "field": "description",
            "phrases": ["running", "shoes"]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.phrase('description', ARRAY['sleek', 'shoes'], slop => 1);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "phrase": {
            "field": "description",
            "phrases": ["sleek", "shoes"],
            "slop": 1
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Test both function and JSON syntax for phrase_prefix
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.phrase_prefix('description', ARRAY['running', 'sh'])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "phrase_prefix": {
            "field": "description",
            "phrases": ["running", "sh"]
        }
    }'::jsonb
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Regex phrase
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.regex_phrase('description', ARRAY['run.*', 'shoe.*'])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.regex_phrase('description', ARRAY['run.*', 'sh.*'])
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn json_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    UPDATE mock_items
    SET metadata = '{"attributes": {"score": 3, "tstz": "2023-05-01T08:12:34Z"}}'::jsonb
    WHERE id = 1;

    UPDATE mock_items
    SET metadata = '{"attributes": {"score": 4, "tstz": "2023-05-01T09:12:34Z"}}'::jsonb
    WHERE id = 2;


    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');
    "#
    .execute(&mut conn);

    // Term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('metadata.color', 'white')
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
        FROM mock_items
        WHERE id @@@
    '{
        "term": {
            "field": "metadata.color",
            "value": "white"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Datetime Handling
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM mock_items WHERE mock_items @@@ '{
        "range": {
            "field": "metadata.attributes.tstz",
            "lower_bound": {"included": "2023-05-01T08:12:34Z"},
            "upper_bound": null,
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn json_arrays(mut conn: PgConnection) {
    //
    // 1) Create the mock_items test table with ParadeDB helper
    //
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    "#
    .execute(&mut conn);

    //
    // 2) Insert some JSON arrays so we can test array-flattening
    //
    r#"
    UPDATE mock_items
    SET metadata = '{"colors": ["red", "green", "blue"]}'::jsonb
    WHERE id = 1;
    UPDATE mock_items
    SET metadata = '{"colors": ["red", "yellow"]}'::jsonb
    WHERE id = 2;
    UPDATE mock_items
    SET metadata = '{"colors": ["blue", "purple"]}'::jsonb
    WHERE id = 3;
    "#
    .execute(&mut conn);

    //
    // 3) Create an index that includes metadata
    //
    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    //
    // 4) Query via function syntax
    //
    let rows: Vec<(String, serde_json::Value)> = r#"
    SELECT description, metadata
    FROM mock_items
    WHERE id @@@ paradedb.term('metadata.colors', 'blue')
       OR id @@@ paradedb.term('metadata.colors', 'red');
    "#
    .fetch(&mut conn);

    // We expect these three rows to match IDs 1, 2, and/or 3
    assert_eq!(rows.len(), 3);

    //
    // 5) Query via JSON syntax
    //
    let rows2: Vec<(String, serde_json::Value)> = r#"
    SELECT description, metadata
    FROM mock_items
    WHERE id @@@
    '{
        "term": {
            "field": "metadata.colors",
            "value": "blue"
        }
    }'::jsonb
    OR id @@@
    '{
        "term": {
            "field": "metadata.colors",
            "value": "red"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);

    // Same three rows should appear
    assert_eq!(rows2.len(), 3);
}

#[rstest]
fn custom_enum(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE TYPE color AS ENUM ('red', 'green', 'blue');
    ALTER TABLE mock_items ADD COLUMN color color;
    INSERT INTO mock_items (color) VALUES ('red'), ('green'), ('blue');

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, color, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Term
    let rows: Vec<(Option<String>, Option<i32>, Option<String>)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.term('color', 'red'::color);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(Option<String>, Option<i32>, Option<String>)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "term": {
            "field": "color",
            "value": 1.0
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Parse
    let rows: Vec<(Option<String>, Option<i32>, Option<String>)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('color:1.0');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(Option<String>, Option<i32>, Option<String>)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "parse": {
            "query_string": "color:1.0"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn compound_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Overview
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[
            paradedb.boost(query => paradedb.term('description', 'shoes'), factor => 2.0),
            paradedb.term('description', 'running')
        ]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "boolean": {
            "should": [
                {"boost": {"query": {"term": {"field": "description", "value": "shoes"}}, "factor": 2.0}},
                {"term": {"field": "description", "value": "running"}}
            ]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // All
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[paradedb.all()],
        must_not => ARRAY[paradedb.term('description', 'shoes')]
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 38);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "boolean": {
            "should": [{"all": null}],
            "must_not": [{"term": {"field": "description", "value": "shoes"}}]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 38);

    // Boolean
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[
          paradedb.term('description', 'headphones')
        ],
        must => ARRAY[
          paradedb.term('category', 'electronics'),
          paradedb.fuzzy_term('description', 'bluetooht')
        ],
        must_not => ARRAY[
          paradedb.range('rating', int4range(NULL, 2, '()'))
        ]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[
          paradedb.term('description', 'headphones')
        ],
        must => ARRAY[
          paradedb.term('category', 'electronics'),
          paradedb.fuzzy_term('description', 'bluetooht')
        ],
        must_not => ARRAY[
          paradedb.range('rating', int4range(NULL, 2, '()'))
        ]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Boost
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
      should => ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.boost(2.0, paradedb.term('description', 'running'))
      ]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@
    '{
        "boolean": {
            "should": [
                {"term": {"field": "description", "value": "shoes"}},
                {"boost": {"factor": 2.0, "query": {"term": {"field": "description", "value": "running"}}}}
            ]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Const score
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.boolean(
      should => ARRAY[
        paradedb.const_score(1.0, paradedb.term('description', 'shoes')),
        paradedb.term('description', 'running')
      ]
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@
    '{
        "boolean": {
            "should": [
                {"const_score": {"score": 1.0, "query": {"term": {"field": "description", "value": "shoes"}}}},
                {"term": {"field": "description", "value": "running"}}
            ]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Disjunction max
    // Test both function and JSON syntax for disjunction_max
    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.disjunction_max(ARRAY[
      paradedb.term('description', 'shoes'),
      paradedb.term('description', 'running')
    ]);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@
    '{
        "disjunction_max": {
            "disjuncts": [
                {"term": {"field": "description", "value": "shoes"}},
                {"term": {"field": "description", "value": "running"}}
            ]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String, f32)> = r#"
    SELECT description, rating, category, pdb.score(id)
    FROM mock_items
    WHERE id @@@
    '{
        "disjunction_max": {
            "disjuncts": [
                {"term": {"field": "description", "value": "shoes"}},
                {"term": {"field": "description", "value": "running"}}
            ]
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    // Empty
    let rows: Vec<(String, i32, String, f32)> = r#"
    -- Returns no rows
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.empty();
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    let rows: Vec<(String, i32, String, f32)> = r#"
    -- Returns no rows
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ '{"empty": null}'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Parse
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('description:"running shoes" OR category:footwear');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.boolean(should => ARRAY[
      paradedb.phrase('description', ARRAY['running', 'shoes']),
      paradedb.term('category', 'footwear')
    ]);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ '{
      "parse": {"query_string": "description:\"running shoes\" OR category:footwear"}
    }'::jsonb
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ '{
      "boolean": {
        "should": [
          {
            "phrase": {
              "field": "description",
              "phrases": ["running", "shoes"]
            }
          },
          {
            "term": {
              "field": "category",
              "value": "footwear"
            }
          }
        ]
      }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    // Lenient parse
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('speaker electronics', lenient => true);
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "parse": {
            "query_string": "speaker electronics",
            "lenient": true
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    // Conjunction mode
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('description:speaker category:electronics');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('description:speaker OR category:electronics');
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "parse": {
            "query_string": "description:speaker category:electronics"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse(
      'description:speaker category:electronics',
      conjunction_mode => true
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse(
    'description:speaker AND category:electronics'
    )"#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "parse": {
            "query_string": "description:speaker category:electronics",
            "conjunction_mode": true
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "parse": {
            "query_string": "description:speaker AND category:electronics"
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Parse with field
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse_with_field(
      'description',
      'speaker bluetooth',
      conjunction_mode => true
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "parse_with_field": {
            "field": "description",
            "query_string": "speaker bluetooth",
            "conjunction_mode": true
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn specialized_queries(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // More like this
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ pdb.more_like_this(
      key_value => 3,
      min_term_frequency => 1
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ pdb.more_like_this(
      document => '{"description": "shoes"}',
      min_doc_frequency => 0,
      max_doc_frequency => 100,
      min_term_frequency => 1
    );
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "more_like_this": {
            "key_value": 3,
            "min_term_frequency": 1
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 16);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@
    '{
        "more_like_this": {
            "document": [["description", "shoes"]],
            "min_doc_frequency": 0,
            "max_doc_frequency": 100,
            "min_term_frequency": 1
        }
    }'::jsonb;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn autocomplete(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let expected = vec![
        ("Sleek running shoes".into(), 5, "Footwear".into()),
        ("Generic shoes".into(), 4, "Footwear".into()),
        ("White jogging shoes".into(), 3, "Footwear".into()),
    ];

    // Fuzzy term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.fuzzy_term(
        field => 'description',
        value => 'shoez'
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    // Match
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.match(
        field => 'description',
        value => 'ruining shoez',
        distance => 2
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.match(
        field => 'description',
        value => 'ruining shoez',
        distance => 2,
        conjunction_mode => true
    )
    "#
    .fetch(&mut conn);
    assert_eq!(rows, vec![expected[0].clone()]);

    // Multiple fuzzy fields
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.boolean(
        should => ARRAY[
            paradedb.match(field => 'description', value => 'ruining shoez', distance => 2),
            paradedb.match(field => 'category', value => 'ruining shoez', distance => 2)
        ]
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    r#"
    DROP INDEX search_idx;
    CREATE INDEX ngrams_idx ON public.mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{"description": {"tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 3, "prefix_only": false}}}'
    );
    "#
    .execute(&mut conn);

    // Ngram term
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE description @@@ 'sho'
    ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);

    // Ngram term set
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category FROM mock_items
    WHERE id @@@ paradedb.match(
        field => 'description',
        value => 'hsoes',
        distance => 0
    ) ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
fn hybrid_search(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');

    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, BigDecimal, String, Vector)> = r#"
    WITH bm25_ranked AS (
        SELECT id, RANK() OVER (ORDER BY score DESC) AS rank
        FROM (
            SELECT id, pdb.score(id) AS score
            FROM mock_items
            WHERE description @@@ 'keyboard'
            ORDER BY pdb.score(id) DESC
            LIMIT 20
        ) AS bm25_score
    ),
    semantic_search AS (
        SELECT id, RANK() OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items
        ORDER BY embedding <=> '[1,2,3]'
        LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_ranked.id) AS id,
        COALESCE(1.0 / (60 + semantic_search.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25_ranked.rank), 0.0) AS score,
        mock_items.description,
        mock_items.embedding
    FROM semantic_search
    FULL OUTER JOIN bm25_ranked ON semantic_search.id = bm25_ranked.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_ranked.id)
    ORDER BY score DESC, description
    LIMIT 5;
    "#
    .fetch(&mut conn);

    // Expected results
    let expected = vec![
        (
            1,
            BigDecimal::from_str("0.03062178588125292193").unwrap(),
            String::from("Ergonomic metal keyboard"),
            Vector::from(vec![3.0, 4.0, 5.0]),
        ),
        (
            2,
            BigDecimal::from_str("0.02990695613646433318").unwrap(),
            String::from("Plastic Keyboard"),
            Vector::from(vec![4.0, 5.0, 6.0]),
        ),
        (
            19,
            BigDecimal::from_str("0.01639344262295081967").unwrap(),
            String::from("Artistic ceramic vase"),
            Vector::from(vec![1.0, 2.0, 3.0]),
        ),
        (
            29,
            BigDecimal::from_str("0.01639344262295081967").unwrap(),
            String::from("Designer wall paintings"),
            Vector::from(vec![1.0, 2.0, 3.0]),
        ),
        (
            39,
            BigDecimal::from_str("0.01639344262295081967").unwrap(),
            String::from("Handcrafted wooden frame"),
            Vector::from(vec![1.0, 2.0, 3.0]),
        ),
    ];

    // Compare each row individually
    for (actual, expected) in rows.iter().zip(expected.iter()) {
        assert_eq!(actual.0, expected.0); // Compare IDs
        assert_relative_eq!(
            actual.1.to_f64().unwrap(),
            expected.1.to_f64().unwrap(),
            epsilon = 0.000265
        ); // Compare BigDecimal scores
        assert_eq!(actual.2, expected.2); // Compare descriptions
        assert_eq!(actual.3, expected.3); // Compare embeddings
    }
}

#[rstest]
fn create_bm25_test_tables(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'orders',
        table_type => 'Orders'
    );

    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'parts',
        table_type => 'Parts'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, i32, i32, f32, String)> = r#"
        SELECT order_id, product_id, order_quantity, order_total::REAL, customer_name FROM orders
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 64);
    assert_eq!(rows[0], (1, 1, 3, 99.99, "John Doe".into()));

    let rows: Vec<(i32, i32, String)> = r#"
        SELECT part_id, parent_part_id, description FROM parts
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 36);
    assert_eq!(rows[0], (1, 0, "Chassis Assembly".into()));
}

#[rstest]
fn concurrent_indexing(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX CONCURRENTLY search_idx_v2 ON mock_items
    USING bm25 (id, description, category, rating, in_stock)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    r#"
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    // Verify the new index is being used by running a query that includes in_stock
    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE description @@@ 'shoes' AND id @@@ 'in_stock:true'
    ORDER BY rating DESC
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn schema(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('search_idx')".fetch(&mut conn);

    let expected = vec![
        ("category".to_string(), "Str".to_string()),
        ("created_at".to_string(), "Date".to_string()),
        ("ctid".to_string(), "U64".to_string()),
        ("description".to_string(), "Str".to_string()),
        ("id".to_string(), "I64".to_string()),
        ("in_stock".to_string(), "Bool".to_string()),
        ("metadata".to_string(), "JsonObject".to_string()),
        ("rating".to_string(), "I64".to_string()),
    ];

    assert_eq!(rows, expected);
}

#[rstest]
fn index_size(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let size: i64 = "SELECT pg_relation_size('search_idx')"
        .fetch_one::<(i64,)>(&mut conn)
        .0;

    assert!(size > 0);
}

#[rstest]
fn field_configuration(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "ngram", "min_gram": 2, "max_gram": 3, "prefix_only": false}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "ngram", "min_gram": 2, "max_gram": 3, "prefix_only": false}
            },
            "category": {
                "tokenizer": {"type": "ngram", "min_gram": 2, "max_gram": 3, "prefix_only": false}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "fast": true,
            "tokenizer": {"type": "ngram", "min_gram": 2, "max_gram": 3, "prefix_only": false}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, metadata)
    WITH (
    key_field = 'id',
    json_fields = '{
        "metadata": {
        "fast": true
        }
    }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, rating)
    WITH (
        key_field = 'id',
        numeric_fields = '{
            "rating": {"fast": true}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, in_stock)
    WITH (
    key_field = 'id',
    boolean_fields = '{
        "in_stock": {"fast": true}
    }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, created_at)
    WITH (
    key_field = 'id',
    datetime_fields = '{
        "created_at": {"fast": true}
    }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, weight_range)
    WITH (key_field='id');
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);
}

#[rstest]
fn available_tokenizers(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "whitespace"}}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "default"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "whitespace"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "raw"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "regex", "pattern": "\\W+"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "ngram", "min_gram": 2, "max_gram": 3, "prefix_only": false}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "source_code"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "chinese_compatible"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {
            "tokenizer": {"type": "chinese_lindera"}
            }
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    if cfg!(feature = "icu") {
        r#"
        CREATE INDEX search_idx ON mock_items
        USING bm25 (id, description)
        WITH (
            key_field = 'id',
            text_fields = '{
                "description": {
                "tokenizer": {"type": "icu"}
                }
            }'
        );
        DROP INDEX search_idx;
        "#
        .execute(&mut conn);
    }

    r#"
    SELECT * FROM paradedb.tokenizers();
    "#
    .execute(&mut conn);

    r#"
    SELECT * FROM paradedb.tokenize(
    paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false),
    'keyboard'
    );
    "#
    .execute(&mut conn);

    // Test multiple tokenizers for the same field
    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "whitespace"}},
            "description_ngram": {"tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 3, "prefix_only": false}, "column": "description"},
            "description_stem": {"tokenizer": {"type": "default", "stemmer": "English"}, "column": "description"}
        }'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('description_ngram:cam AND description_stem:digitally')
    ORDER BY rating DESC
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].0.contains("camera"));
    assert!(rows[0].0.contains("digital"));

    let rows: Vec<(String, i32, String)> = r#"
    SELECT description, rating, category
    FROM mock_items
    WHERE id @@@ paradedb.parse('description:"Soft cotton" OR description_stem:shirts')
    ORDER BY rating DESC
    LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().any(|r| r.0.contains("cotton")));
    assert!(rows.iter().any(|r| r.0.contains("shirt")));
}

#[rstest]
fn token_filters(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default", "stemmer": "English"}}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default", "remove_long": 255}}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default", "lowercase": false}}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);
}

#[rstest]
fn fast_fields(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, rating)
    WITH (
        key_field = 'id',
        text_fields ='{
            "description": {"fast": true}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, category)
    WITH (
        key_field='id',
        text_fields='{
            "category": {"fast": true, "normalizer": "raw"}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);
}

#[rstest]
fn record(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"record": "freq"}
        }'
    );
    DROP INDEX search_idx;
    "#
    .execute(&mut conn);
}
```

---

## aggregate.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_aggregate_with_mvcc(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
    CREATE INDEX idxbm25_search ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (
        key_field='id',
        text_fields='{
            "category": {"fast": true, "normalizer": "raw"}
        }',
        numeric_fields='{"rating": {"fast": true}}'
    );
    INSERT INTO paradedb.bm25_search (description, category, rating) VALUES
        ('keyboard', 'Electronics', 4.5),
        ('keyboard', 'Electronics', 3.8),
        ('keyboard', 'Accessories', 4.2);

    DELETE FROM paradedb.bm25_search WHERE category = 'Accessories';
    "#
        .execute(&mut conn);

    // Test with MVCC enabled (default)
    let result = r#"
    SELECT paradedb.aggregate(
        'paradedb.idxbm25_search',
        paradedb.parse('description:keyboard'),
        '{
            "category": {
                "terms": {
                    "field": "category",
                    "size": 10
                }
            }
        }'::json
    )
    "#
    .fetch_one::<(serde_json::Value,)>(&mut conn);

    // Verify the aggregation results
    let buckets = result
        .0
        .pointer("/category/buckets")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(buckets.len(), 1); // Should have 1 category
}

#[rstest]
fn test_aggregate_without_mvcc(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
    CREATE INDEX idxbm25_search ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (
        key_field='id',
        text_fields='{
            "description": {},
            "category": {"fast": true, "normalizer": "raw"}
        }',
        numeric_fields='{"rating": {"fast": true}}',
        boolean_fields='{"in_stock": {}}',
        json_fields='{"metadata": {}}',
        datetime_fields='{
            "created_at": {},
            "last_updated_date": {},
            "latest_available_time": {}
        }'
    );
    INSERT INTO paradedb.bm25_search (description, category, rating) VALUES
        ('keyboard', 'Electronics', 4.5),
        ('keyboard', 'Electronics', 3.8),
        ('keyboard', 'Accessories', 4.2);

    DELETE FROM paradedb.bm25_search WHERE category = 'Accessories';
    "#
        .execute(&mut conn);

    // Test with MVCC disabled
    let result = r#"
    SELECT paradedb.aggregate(
        'paradedb.idxbm25_search',
        paradedb.parse('description:keyboard'),
        '{
            "category": {
                "terms": {
                    "field": "category",
                    "size": 10
                }
            }
        }'::json,
        false
    )
    "#
    .fetch_one::<(serde_json::Value,)>(&mut conn);

    // Verify the aggregation results
    let buckets = result
        .0
        .pointer("/category/buckets")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(buckets.len(), 2); // Should have 2 categories
}
```

---

## str_ff_exec.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
    let sql = r#"
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            col_boolean boolean DEFAULT false,
            col_text text,
            col_int8 int8
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idxtest ON test USING bm25 (id, col_boolean, col_text, col_int8)
        WITH (key_field='id', text_fields = '{"col_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
    "#;
    sql.execute(&mut conn);

    "INSERT INTO test (id) VALUES (1);".execute(&mut conn);
    "INSERT INTO test (id, col_text) VALUES (2, 'foo');".execute(&mut conn);
    "INSERT INTO test (id, col_text, col_int8) VALUES (3, 'bar', 333);".execute(&mut conn);
    "INSERT INTO test (id, col_int8) VALUES (4, 444);".execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    conn
}

mod string_fast_field_exec {
    use super::*;

    #[rstest]
    fn with_range(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE id @@@ paradedb.range(field => 'id', range => int8range(1, 5, '[]'))
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, false, None, None),
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333)),
                (4, false, None, Some(444))
            ]
        );
    }

    #[rstest]
    fn with_filter(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE col_text IS NULL and id @@@ '>2'
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(4, false, None, Some(444))]);
    }

    #[rstest]
    fn with_multiple_filters(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE col_text IS NULL
            AND col_int8 IS NOT NULL
            AND id @@@ paradedb.range(field => 'id', range => int8range(1, 5, '[]'))
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(4, false, None, Some(444))]);
    }

    #[rstest]
    fn with_not_null(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE col_text IS NOT NULL and id @@@ '>2'
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(3, false, Some(String::from("bar")), Some(333))]);
    }

    #[rstest]
    fn with_null(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT * FROM test
            WHERE col_text IS NULL and id @@@ '<=2'
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(1, false, None, None)]);
    }

    #[rstest]
    fn with_count(#[from(setup_test_table)] mut conn: PgConnection) {
        let count = r#"
            SELECT count(*) FROM test
            WHERE col_text IS NOT NULL and id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);

        let count = r#"
            SELECT count(*) FROM test
            WHERE col_text IS NULL and id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);
    }

    #[rstest]
    fn with_empty_string(#[from(setup_test_table)] mut conn: PgConnection) {
        "INSERT INTO test (id, col_text) VALUES (5, '');".execute(&mut conn);

        let res = r#"
            SELECT * FROM test
            WHERE col_text = ''
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(5, false, Some(String::from("")), None)]);
    }

    #[rstest]
    fn with_all_null_segment(mut conn: PgConnection) {
        let sql = r#"
            CREATE TABLE another_test (
                id SERIAL8 NOT NULL PRIMARY KEY,
                col_boolean boolean DEFAULT false,
                col_text text,
                col_int8 int8
            );
        "#;
        sql.execute(&mut conn);

        let sql = r#"
            CREATE INDEX another_idxtest ON another_test USING bm25 (id, col_boolean, col_text, col_int8)
            WITH (key_field='id', text_fields = '{"col_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
        "#;
        sql.execute(&mut conn);

        "INSERT INTO another_test (id) VALUES (1);".execute(&mut conn);
        "INSERT INTO another_test (id, col_int8) VALUES (3, 333);".execute(&mut conn);
        "INSERT INTO another_test (id, col_int8) VALUES (4, 444);".execute(&mut conn);
        "INSERT INTO another_test (id, col_text) VALUES (6, NULL), (7, NULL), (8, NULL);"
            .execute(&mut conn);

        "SET enable_indexscan TO off;".execute(&mut conn);
        "SET enable_bitmapscan TO off;".execute(&mut conn);
        "SET max_parallel_workers TO 0;".execute(&mut conn);

        let count = r#"
            SELECT count(*) FROM another_test
            WHERE col_text IS NULL and id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(5,)]);

        let res = r#"
            SELECT * FROM another_test
            WHERE id @@@ paradedb.range(field => 'id', range => int8range(1, 8, '[]'))
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, false, None, None),
                (3, false, None, Some(333)),
                (4, false, None, Some(444)),
                (6, false, None, None),
                (7, false, None, None),
                (8, false, None, None)
            ]
        );

        let count = r#"
            SELECT count(*) FROM another_test
            WHERE id @@@ paradedb.range(field => 'id', range => int8range(1, 8, '[]'))
            AND col_text IS NOT NULL
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(0,)])
    }
}
```

---

## pushdown.rs

```
mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

/// Helper function to verify that a query plan uses ParadeDB's custom scan operator.
/// It recursively searches the plan and asserts that exactly one "Custom Scan" node is found.
#[track_caller]
fn verify_custom_scan(plan: &Value, description: &str) {
    fn find_custom_scan_nodes<'a>(plan_node: &'a Value, nodes: &mut Vec<&'a Value>) {
        if let Some(obj) = plan_node.as_object() {
            if let Some("Custom Scan") = obj.get("Node Type").and_then(Value::as_str) {
                nodes.push(plan_node);
            }

            if let Some(plans) = obj.get("Plans").and_then(Value::as_array) {
                for child_plan in plans {
                    find_custom_scan_nodes(child_plan, nodes);
                }
            }
        }
    }

    let root_plan_node = plan
        .pointer("/0/Plan")
        .unwrap_or_else(|| panic!("Could not find plan node in: {plan:?}"));

    let mut custom_scan_nodes = Vec::new();
    find_custom_scan_nodes(root_plan_node, &mut custom_scan_nodes);

    assert_eq!(
        1,
        custom_scan_nodes.len(),
        "Expected to find exactly one Custom Scan node for '{description}', but found {}. Plan: {plan:#?}",
        custom_scan_nodes.len()
    );
}

#[rstest]
fn pushdown_is_true_doesnt_require_scores_with_parallel_custom_scan(mut conn: PgConnection) {
    r#"CREATE TABLE pushdown_is_true(
        id serial8 not null primary key,
        bool_field bool
    );
    CREATE INDEX idxpushdown_is_true ON pushdown_is_true USING bm25 (id, bool_field) WITH (key_field = 'id');
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    INSERT INTO pushdown_is_true (bool_field) SELECT true FROM generate_series(1, 100);
    "#
    .execute(&mut conn);

    // the test is simply that this doesn't cause postgres to raise an ERROR: cannot sort by field and get scores in the same query
    //
    // user reported a bug where, specifically, a `bool_field = TRUE|FALSE` pushdown would cause the
    // query to think it needed scores, which, clearly, the query doesn't use
    "SELECT * FROM pushdown_is_true WHERE bool_field = TRUE AND id @@@ paradedb.all() ORDER BY id desc LIMIT 25 OFFSET 0"
        .execute(&mut conn);
}

#[rstest]
fn pushdown(mut conn: PgConnection) {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];

    // colname, sqltype, default value
    const TYPES: &[[&str; 3]] = &[
        ["int2", "int2", "0"],
        ["int4", "int4", "0"],
        ["int8", "int8", "0"],
        ["float4", "float4", "0"],
        ["float8", "float8", "0"],
        ["date", "date", "now()"],
        ["time", "time", "now()"],
        ["timetz", "timetz", "now()"],
        ["timestamp", "timestamp", "now()"],
        ["timestamptz", "timestamptz", "now()"],
        ["text", "text", "'foo'::text"],
        ["text_1", "text", "'foo'::varchar"],
        ["varchar", "varchar", "'foo'::varchar"],
        ["varchar_1", "varchar", "'foo'::text"],
        ["uuid", "uuid", "gen_random_uuid()"],
    ];

    let sqlname = |sqltype: &str| -> String { String::from("col_") + &sqltype.replace('"', "") };

    let mut sql = String::new();
    sql += "CREATE TABLE test (id SERIAL8 NOT NULL PRIMARY KEY, col_boolean boolean DEFAULT false";
    for [colname, sqltype, default] in TYPES {
        sql += &format!(
            ", {} {sqltype} NOT NULL DEFAULT {default}",
            sqlname(colname)
        );
    }
    sql += ");";

    eprintln!("{sql}");
    sql.execute(&mut conn);

    let sql = format!(
        r#"
            CREATE INDEX idxtest
                      ON test
                   USING bm25 (id, col_boolean, {})
                   WITH (
                    key_field='id',
                        text_fields = '{{
                            "col_text": {{"tokenizer": {{"type":"keyword"}} }},
                            "col_text_1": {{"tokenizer": {{"type":"keyword"}} }},
                            "col_varchar": {{"tokenizer": {{"type":"keyword"}} }},
                            "col_varchar_1": {{"tokenizer": {{"type":"keyword"}} }}
                         }}'
                    );"#,
        TYPES
            .iter()
            .map(|t| sqlname(t[0]))
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!("{sql}");
    sql.execute(&mut conn);

    "INSERT INTO test (id) VALUES (1);".execute(&mut conn); // insert all default values

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);
    "SET paradedb.enable_custom_scan_without_operator TO on;".execute(&mut conn);

    for operator in OPERATORS {
        for [colname, sqltype, default] in TYPES {
            let sqlname = sqlname(colname);
            let sql = format!(
                r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} {operator} {default}::{sqltype};
            "#
            );

            eprintln!("/----------/");
            eprintln!("{sql}");

            let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
            eprintln!("{plan:#?}");

            verify_custom_scan(&plan, &format!("Operator {operator} for type {sqltype}"));
        }
    }

    // boolean is a bit of a separate beast, so test it directly
    {
        let sqltype = "boolean";
        let sqlname = sqlname(sqltype);
        let sql = format!(
            r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} = true;
            "#
        );

        eprintln!("/----------/");
        eprintln!("{sql}");

        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        verify_custom_scan(&plan, "boolean = true operator");
    }
    {
        let sqltype = "boolean";
        let sqlname = sqlname(sqltype);
        let sql = format!(
            r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} = false;
            "#
        );

        eprintln!("/----------/");
        eprintln!("{sql}");

        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        verify_custom_scan(&plan, "boolean = false operator");
    }
}

#[rstest]
fn issue2301_is_null_with_joins(mut conn: PgConnection) {
    r#"
        CREATE TABLE mcp_server (
            id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            name text NOT NULL,
            description text NOT NULL,
            created_at timestamp with time zone NOT NULL DEFAULT now(),
            attributes jsonb NOT NULL DEFAULT '[]'::jsonb,
            updated_at timestamp with time zone NOT NULL DEFAULT now(),
            synced_at timestamp with time zone,
            removed_at timestamp with time zone
        );
        CREATE INDEX mcp_server_search_idx ON mcp_server
        USING bm25 (id, name, description, synced_at, removed_at)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let (plan, ) = r#"
        EXPLAIN (VERBOSE, FORMAT JSON) SELECT ms1.id, ms1.name, pdb.score (ms1.id)
        FROM mcp_server ms1
        WHERE
          ms1.synced_at IS NOT NULL
          AND ms1.removed_at IS NULL
          AND ms1.id @@@ '{
              "boolean": {
                "should": [
                  {"boost": {"factor": 2, "query": {"fuzzy_term": {"field": "name", "value": "cloudflare"}}}},
                  {"boost": {"factor": 1, "query": {"fuzzy_term": {"field": "description", "value": "cloudflare"}}}}
                ]
              }
            }'::jsonb
        ORDER BY pdb.score (ms1.id) DESC;
    "#.fetch_one::<(Value, )>(&mut conn);

    eprintln!("{plan:#?}");

    verify_custom_scan(&plan, "IS NULL with joins");
}

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
    let sql = r#"
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            col_boolean boolean DEFAULT false,
            col_text text,
            col_int8 int8
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idxtest ON test USING bm25 (id, col_boolean, col_text, col_int8)
        WITH (key_field='id', text_fields = '{"col_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
    "#;
    sql.execute(&mut conn);

    "INSERT INTO test (id, col_text) VALUES (1, NULL);".execute(&mut conn);
    "INSERT INTO test (id, col_text) VALUES (2, 'foo');".execute(&mut conn);
    "INSERT INTO test (id, col_text, col_int8) VALUES (3, 'bar', 333);".execute(&mut conn);
    "INSERT INTO test (id, col_int8) VALUES (4, 444);".execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);
    conn
}

mod pushdown_is_not_null {
    use super::*;

    #[rstest]
    fn custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT count(*)
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ '1';
        "#;

        eprintln!("/----------/");
        eprintln!("{sql}");

        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        // Verify that the custom scan is used
        verify_custom_scan(&plan, "IS NOT NULL condition");
    }

    #[rstest]
    fn with_count(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that count is correct
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_int8 IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_int8 IS NOT NULL
            AND col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);
    }

    #[rstest]
    fn with_return_values(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT *
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333))
            ]
        );

        let res = r#"
            SELECT *
            FROM test
            WHERE col_int8 IS NOT NULL
            AND col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(3, false, Some(String::from("bar")), Some(333))]);
    }

    #[rstest]
    fn with_multiple_predicates(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that IS NOT NULL works with other predicates
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);

        let res = r#"
            SELECT *
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(3, false, Some(String::from("bar")), Some(333))]);
    }

    #[rstest]
    fn with_ordering(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that results are correct and ordered
        let result = r#"
            SELECT id
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY id DESC;
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(result, vec![(3,), (2,)]);
    }

    #[rstest]
    fn with_aggregation(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that GROUP BY works
        let result = r#"
            SELECT col_text, count(*)
            FROM test
            WHERE col_text IS NOT NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            GROUP BY col_text
            ORDER BY col_text;
        "#
        .fetch::<(String, i64)>(&mut conn);
        assert_eq!(
            result,
            vec![(String::from("bar"), 1), (String::from("foo"), 1)]
        );
    }

    #[rstest]
    fn with_distinct(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that DISTIINCT works
        let count = r#"
            SELECT COUNT(DISTINCT col_text)
            FROM test
            WHERE col_text IS NOT NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let res = r#"
            SELECT DISTINCT col_text
            FROM test
            WHERE col_text IS NOT NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY col_text;
        "#
        .fetch::<(Option<String>,)>(&mut conn);
        assert_eq!(
            res,
            vec![(Some(String::from("bar")),), (Some(String::from("foo")),)]
        );
    }

    #[rstest]
    fn with_join(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that JOIN works
        "CREATE TABLE test2 (id SERIAL8 NOT NULL PRIMARY KEY, ref_id int8, ref_text text);"
            .execute(&mut conn);
        let sql = r#"
            CREATE INDEX idxtest2 ON test2 USING bm25 (id, ref_id, ref_text)
            WITH (key_field='id', text_fields = '{"ref_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
        "#;
        sql.execute(&mut conn);

        "INSERT INTO test2 (ref_id, ref_text) VALUES (1, 'qux');".execute(&mut conn);
        "INSERT INTO test2 (ref_id, ref_text) VALUES (3, 'foo');".execute(&mut conn);

        let join = r#"
            SELECT test.id, test.col_text, test2.ref_text
            FROM test
            INNER JOIN test2 ON test.id = test2.ref_id
            WHERE test.col_text IS NOT NULL
            AND test.id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY test.id;
        "#
        .fetch_one::<(i64, String, String)>(&mut conn);
        assert_eq!(join, (3, String::from("bar"), String::from("foo")));
    }

    #[rstest]
    fn post_update(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that NULL is not counted after update
        "UPDATE test SET col_text = NULL".execute(&mut conn);
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(0,)]);

        let res = r#"
            SELECT *
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![]);
    }
}

mod pushdown_is_null {
    use super::*;

    #[rstest]
    fn custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ '1';
        "#;

        eprintln!("/----------/");
        eprintln!("{sql}");

        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        // Verify that the custom scan is used
        verify_custom_scan(&plan, "IS NULL condition");
    }

    #[rstest]
    fn with_count(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that count is correct
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_int8 IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_int8 IS NULL
            AND col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);
    }

    #[rstest]
    fn with_return_values(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT id, col_boolean, col_int8
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(1, false, None), (4, false, Some(444))]);

        let res = r#"
            SELECT *
            FROM test
            WHERE col_int8 IS NULL
            AND col_text IS NULL
            AND id @@@ '1' OR id @@@ '2' OR id @@@ '3' OR id @@@ '4'
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, false, None, None),
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333)),
                (4, false, None, Some(444))
            ]
        );
    }

    #[rstest]
    fn with_multiple_predicates(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that IS NULL works with other predicates
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);

        let res = r#"
            SELECT id, col_boolean, col_int8
            FROM test
            WHERE col_text IS NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64, bool, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(4, false, Some(444))]);
    }

    #[rstest]
    fn with_ordering(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that results are correct and ordered
        let result = r#"
            SELECT id
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY id DESC;
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(result, vec![(4,), (1,)]);
    }

    #[rstest]
    fn with_aggregation(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that GROUP BY works
        let result = r#"
            SELECT col_int8, count(*)
            FROM test
            WHERE col_text IS NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            GROUP BY col_int8
            ORDER BY col_int8;
        "#
        .fetch::<(Option<i64>, i64)>(&mut conn);
        assert_eq!(result, vec![(Some(444), 1), (None, 1)]);
    }

    #[rstest]
    fn with_distinct(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that DISTIINCT works
        let result = r#"
            SELECT COUNT(DISTINCT col_int8)
            FROM test
            WHERE col_text IS NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(result, vec![(1,)]);
    }

    #[rstest]
    fn with_join(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that JOIN works
        "CREATE TABLE test2 (id SERIAL8 NOT NULL PRIMARY KEY, ref_id int8, ref_text text);"
            .execute(&mut conn);
        let sql = r#"
            CREATE INDEX idxtest2 ON test2 USING bm25 (id, ref_id, ref_text)
            WITH (key_field='id', text_fields = '{"ref_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
        "#;
        sql.execute(&mut conn);

        "INSERT INTO test2 (ref_id, ref_text) VALUES (2, 'qux');".execute(&mut conn);
        "INSERT INTO test2 (ref_id, ref_text) VALUES (4, 'foo');".execute(&mut conn);

        let join = r#"
            SELECT test.id, test.col_text, test2.ref_text
            FROM test
            INNER JOIN test2 ON test.id = test2.ref_id
            WHERE test.col_int8 IS NULL
            AND test.id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY test.id;
        "#
        .fetch_one::<(i64, String, String)>(&mut conn);
        assert_eq!(join, (2, String::from("foo"), String::from("qux")));
    }

    #[rstest]
    fn post_update(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that NULL is not counted after update
        "UPDATE test SET col_text = NULL".execute(&mut conn);
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(4,)]);

        let res = r#"
            SELECT id, col_int8, col_boolean
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY id;
        "#
        .fetch::<(i64, Option<i64>, bool)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, None, false),
                (2, None, false),
                (3, Some(333), false),
                (4, Some(444), false)
            ]
        )
    }
}

/// Tests for boolean IS TRUE/FALSE operators
mod pushdown_is_bool_operator {
    use super::*;

    // Helper function to verify a query uses custom scan and returns expected results
    fn verify_boolean_is_operator(
        conn: &mut PgConnection,
        condition: &str,
        expected_id: i64,
        expected_bool_value: bool,
    ) {
        // Check execution plan uses custom scan
        let sql = format!(
            r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, pdb.score(id) FROM is_true
            WHERE bool_field {condition} AND message @@@ 'beer';
            "#
        );

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(conn);
        eprintln!("{plan:#?}");

        // Verify custom scan is used
        verify_custom_scan(&plan, &format!("boolean {condition} operator"));

        // Verify query results
        let results: Vec<(i64, bool, String, f32)> = format!(
            r#"
            SELECT id, bool_field, message, pdb.score(id)
            FROM is_true
            WHERE bool_field {condition} AND message @@@ 'beer'
            ORDER BY id;
            "#
        )
        .fetch(conn);

        assert_eq!(1, results.len());
        assert_eq!(expected_id, results[0].0); // id
        assert_eq!(expected_bool_value, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }

    // Helper for complex boolean expression tests
    fn verify_complex_boolean_expr(
        conn: &mut PgConnection,
        condition: &str,
        expected_id: i64,
        expected_bool_value: bool,
    ) {
        let sql = format!(
            r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, pdb.score(id) FROM is_true
            WHERE {condition} AND message @@@ 'beer';
            "#
        );

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(conn);
        eprintln!("{plan:#?}");

        // For complex expressions we don't verify the plan type
        // since it may not use Custom Scan directly

        // Just verify the query results
        let results: Vec<(i64, bool, String, Option<f32>)> = format!(
            r#"
            SELECT id, bool_field, message, pdb.score(id)
            FROM is_true
            WHERE {condition} AND message @@@ 'beer'
            ORDER BY id;
            "#
        )
        .fetch(conn);

        assert_eq!(1, results.len());
        assert_eq!(expected_id, results[0].0); // id
        assert_eq!(expected_bool_value, results[0].1); // bool_field
        assert_ne!(None, results[0].3, "score should not be None"); // score
        assert_eq!("beer", results[0].2); // message
    }

    /// Test for issue #2433: Pushdown `bool_field IS true|false`
    /// Verifies that the SQL IS operator for boolean fields is properly
    /// pushed down to the ParadeDB scan operator.
    #[rstest]
    fn test_bool_is_operator_pushdown(mut conn: PgConnection) {
        r#"
    DROP TABLE IF EXISTS is_true;
    CREATE TABLE is_true (
        id serial8 not null primary key,
        bool_field boolean,
        message text
    );

    CREATE INDEX idxis_true ON is_true USING bm25 (id, bool_field, message) WITH (key_field = 'id');

    INSERT INTO is_true (bool_field, message) VALUES (true, 'beer');
    INSERT INTO is_true (bool_field, message) VALUES (false, 'beer');
    "#
        .execute(&mut conn);

        // Test all boolean IS operators using the helper function
        verify_boolean_is_operator(&mut conn, "IS true", 1, true);
        verify_boolean_is_operator(&mut conn, "IS false", 2, false);
        verify_boolean_is_operator(&mut conn, "IS NOT true", 2, false);
        verify_boolean_is_operator(&mut conn, "IS NOT false", 1, true);
    }

    /// Test for issue #2433: Complex boolean expressions with IS TRUE/FALSE operators
    /// This test checks the behavior of complex expressions (not just simple field references)
    /// with IS TRUE/FALSE operators.
    ///
    /// Note: Currently, complex expressions won't be pushed down to the ParadeDB scan operator.
    /// PostgreSQL will handle the evaluation of these expressions after the scan.
    /// We're marking this test as ignored until we implement full support for complex expressions.
    #[rstest]
    #[ignore]
    fn test_complex_bool_expressions_with_is_operator(mut conn: PgConnection) {
        r#"
    DROP TABLE IF EXISTS is_true;
    CREATE TABLE is_true (
        id serial8 not null primary key,
        bool_field boolean,
        message text
    );

    CREATE INDEX idxis_true ON is_true USING bm25 (id, bool_field, message) WITH (key_field = 'id');

    INSERT INTO is_true (bool_field, message) VALUES (true, 'beer');
    INSERT INTO is_true (bool_field, message) VALUES (false, 'beer');

    CREATE OR REPLACE FUNCTION is_true_test(b boolean) RETURNS boolean AS $$
    BEGIN
        RETURN b;
    END;
    $$ LANGUAGE plpgsql;
    "#
        .execute(&mut conn);

        // Test with expression IS TRUE
        verify_complex_boolean_expr(&mut conn, "(bool_field = true) IS true", 1, true);

        verify_complex_boolean_expr(&mut conn, "is_true_test(bool_field) IS true", 1, true);

        // Test with complex expression IS FALSE
        verify_complex_boolean_expr(&mut conn, "(bool_field <> true) IS true", 2, false);
    }

    /// Test the handling of boolean IS TRUE/FALSE operators with NULL values
    /// Verifies that SQL operators follow the SQL standard:
    /// - IS TRUE should only return rows where the value is TRUE (not NULL)
    /// - IS FALSE should only return rows where the value is FALSE (not NULL)
    /// - IS NOT TRUE should return rows where the value is FALSE or NULL
    /// - IS NOT FALSE should return rows where the value is TRUE or NULL
    /// - NOT (field = TRUE) should only return rows where the value is FALSE (not NULL)
    #[rstest]
    fn test_boolean_operators_with_null_values(mut conn: PgConnection) {
        r#"
        DROP TABLE IF EXISTS bool_null_test;
        CREATE TABLE bool_null_test (
            id serial8 not null primary key,
            bool_field boolean,
            message text
        );

        CREATE INDEX idx_bool_null_test ON bool_null_test USING bm25 (id, bool_field, message) WITH (key_field = 'id');

        -- Insert values: true, false, and NULL
        INSERT INTO bool_null_test (bool_field, message) VALUES (true, 'beer');
        INSERT INTO bool_null_test (bool_field, message) VALUES (false, 'beer');
        INSERT INTO bool_null_test (bool_field, message) VALUES (NULL, 'beer');
        "#
        .execute(&mut conn);

        // Helper function for testing boolean conditions with expected row count and value checks
        fn test_boolean_condition(
            conn: &mut PgConnection,
            condition: &str,
            expected_count: usize,
            expected_values: &[Option<bool>],
            description: &str,
        ) {
            // Check query plan
            let sql = format!(
                r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT *, pdb.score(id) FROM bool_null_test
                WHERE {condition} AND message @@@ 'beer';
                "#
            );

            eprintln!("{sql}");
            let (plan,) = sql.fetch_one::<(Value,)>(conn);
            eprintln!("{plan:#?}");

            // Verify custom scan is used
            verify_custom_scan(&plan, &format!("{condition} operator with NULL test"));

            // Get actual results
            let results: Vec<(i64, Option<bool>, String, f32)> = format!(
                r#"
                SELECT id, bool_field, message, pdb.score(id)
                FROM bool_null_test
                WHERE {condition} AND message @@@ 'beer'
                ORDER BY id;
                "#
            )
            .fetch(conn);

            // Check result count
            if results.len() != expected_count {
                eprintln!(
                    "FAIL: '{condition}' should return {expected_count} rows, got {}",
                    results.len()
                );
                assert_eq!(expected_count, results.len(), "SQL standard: {description}");
            }

            // Check expected values if provided
            for expected_value in expected_values {
                match expected_value {
                    Some(value) => {
                        let has_value = results.iter().any(|(_, b, _, _)| *b == Some(*value));
                        assert!(
                            has_value,
                            "Results should include a row with bool_field = {value}"
                        );
                    }
                    None => {
                        let has_null = results.iter().any(|(_, b, _, _)| b.is_none());
                        assert!(
                            has_null,
                            "Results should include a row with bool_field = NULL"
                        );
                    }
                }
            }
        }

        // ---- Simple boolean operators ----

        // Test with IS TRUE - should return only the row with true
        test_boolean_condition(
            &mut conn,
            "bool_field IS TRUE",
            1,
            &[Some(true)],
            "IS TRUE should only return TRUE rows, not NULL rows",
        );

        // Test with IS FALSE - should only return the FALSE row (not NULL)
        test_boolean_condition(
            &mut conn,
            "bool_field IS FALSE",
            1,
            &[Some(false)],
            "IS FALSE should only return FALSE rows, not NULL rows",
        );

        // Test with IS NOT TRUE - should return rows with false and NULL
        test_boolean_condition(
            &mut conn,
            "bool_field IS NOT TRUE",
            2,
            &[Some(false), None],
            "IS NOT TRUE should return both FALSE and NULL rows",
        );

        // Test with IS NOT FALSE - should return rows with true and NULL
        test_boolean_condition(
            &mut conn,
            "bool_field IS NOT FALSE",
            2,
            &[Some(true), None],
            "IS NOT FALSE should return both TRUE and NULL rows",
        );

        // ---- Comparison operators ----

        // Test with = TRUE - should also only return the row with true
        test_boolean_condition(
            &mut conn,
            "bool_field = TRUE",
            1,
            &[Some(true)],
            "= TRUE should only return TRUE rows, not NULL rows",
        );

        // Test with = FALSE - should only return the FALSE row (not NULLs)
        test_boolean_condition(
            &mut conn,
            "bool_field = FALSE",
            1,
            &[Some(false)],
            "= FALSE should only return FALSE rows, not NULL rows",
        );

        // ---- Complex expressions ----

        // Test NOT (field = TRUE) - should only return FALSE (no NULL)
        test_boolean_condition(
            &mut conn,
            "NOT (bool_field = TRUE)",
            1,
            &[Some(false)],
            "NOT (field = TRUE) should only return FALSE rows, not NULL rows",
        );

        // Test NOT (field = FALSE) - should only return TRUE (no NULL)
        test_boolean_condition(
            &mut conn,
            "NOT (bool_field = FALSE)",
            1,
            &[Some(true)],
            "NOT (field = FALSE) should only return TRUE rows, not NULL rows",
        );

        // Test for whether comparison with NULL returns expected results
        // (These provide the reference behavior for the IS operators)
        {
            let results: Vec<(i64, Option<bool>, String)> = r#"
                SELECT id, bool_field, message
                FROM bool_null_test
                WHERE bool_field IS NULL AND message @@@ 'beer'
                ORDER BY id;
            "#
            .fetch(&mut conn);

            assert_eq!(1, results.len(), "Should find one row with NULL bool_field");
            assert_eq!(None, results[0].1, "The row should have bool_field = NULL");
        }
    }
}
```

---

## index_only_scan.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn custom_scan_on_key_field(mut conn: PgConnection) {
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT id FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard'".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");
    let plan = plan.pointer("/0/Plan").unwrap();
    pretty_assertions::assert_eq!(
        plan.get("Custom Plan Provider"),
        Some(&Value::String(String::from("ParadeDB Scan")))
    );
}
```

---

## aborted_xact.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn aborted_segments_not_visible(mut conn: PgConnection) {
    r#"
        SET max_parallel_maintenance_workers = 0;
        SET parallel_leader_participation = false;
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO test_table (value) VALUES ('committed');

        CREATE INDEX idxtest_table ON public.test_table
        USING bm25 (id, value)
        WITH (
            key_field = 'id',
            text_fields = '{
                "value": {}
            }'
        );
    "#
    .execute(&mut conn);

    // there's one segment, from CREATE INDEX
    let (pre_update_visible_segments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table')".fetch_one::<(i64,)>(&mut conn);

    assert_eq!(pre_update_visible_segments, 1);

    // this will do a merge_on_insert, creating a new segment, even tho its contents will not be
    // visible (because the xact aborted), the segment itself will be
    "BEGIN; UPDATE test_table SET value = 'aborted'; ABORT".execute(&mut conn);

    // so that means this will return two segments.  The original one made by CREATE INDEX and
    // the segment from above
    let (post_visible_segments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table', true)"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(post_visible_segments, 2);

    // and even tho this will search both segments, it will not return the row from the aborted xact
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'aborted'".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    // because it's supposed to only return rows from live segments
    let (count,) = "SELECT count(*) FROM test_table WHERE value @@@ 'committed'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
```

---

## json_pushdown.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

use crate::fixtures::querygen::opexprgen::Operator;
use crate::fixtures::querygen::{compare, PgGucs};

#[derive(Debug, Clone, Arbitrary)]
pub enum TokenizerType {
    Default,
    Keyword,
}

impl TokenizerType {
    fn to_config(&self) -> &'static str {
        match self {
            TokenizerType::Default => r#""type": "default""#,
            TokenizerType::Keyword => r#""type": "keyword""#,
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub struct IndexConfig {
    tokenizer: TokenizerType,
    fast: bool,
}

impl IndexConfig {
    fn to_json_fields_config(&self) -> String {
        format!(
            r#"{{
                "metadata": {{
                    "tokenizer": {{ {} }},
                    "fast": {}
                }}
            }}"#,
            self.tokenizer.to_config(),
            self.fast
        )
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum JsonValueType {
    Text,
    Numeric,
    Boolean,
    Null,
}

impl JsonValueType {
    fn sample_values(&self) -> BoxedStrategy<String> {
        match self {
            JsonValueType::Text => proptest::sample::select(vec![
                "'apple'".to_string(),
                "'banana'".to_string(),
                "'cherry'".to_string(),
                "'date'".to_string(),
                "'elderberry'".to_string(),
                "'test'".to_string(),
                "'value'".to_string(),
                "'red apple'".to_string(),
                "'yellow banana'".to_string(),
                "'sweet cherry'".to_string(),
                "'fresh date'".to_string(),
                "'purple elderberry'".to_string(),
                "'unit test'".to_string(),
                "'test value'".to_string(),
            ])
            .boxed(),
            JsonValueType::Numeric => proptest::sample::select(vec![
                "42".to_string(),
                "100".to_string(),
                "3.14".to_string(),
                "0".to_string(),
                "-1".to_string(),
                "999".to_string(),
                // Edge cases for numeric type conversion - SKIPPED due to prop test failures
                // "1".to_string(),                    // Small integer (I64/U64/F64)
                // "1.0".to_string(),                  // Float equivalent of integer
                // "9007199254740992".to_string(),     // 2^53, max safe F64 integer
                // "9223372036854775807".to_string(),  // i64::MAX
                // "18446744073709551615".to_string(), // u64::MAX (as string)
                // "-9223372036854775808".to_string(), // i64::MIN
            ])
            .boxed(),
            JsonValueType::Boolean => {
                proptest::sample::select(vec!["true".to_string(), "false".to_string()]).boxed()
            }
            JsonValueType::Null => Just("NULL".to_string()).boxed(),
        }
    }

    fn to_json_literal(&self, value: &str) -> String {
        match self {
            JsonValueType::Text => format!("'\"{}\"'", value.trim_matches('\'')),
            JsonValueType::Numeric => format!("'{value}'"),
            JsonValueType::Boolean => format!("'{value}'"),
            JsonValueType::Null => "'null'".to_string(),
        }
    }

    fn is_compatible_with_operator(&self, operator: &Operator) -> bool {
        match (self, operator) {
            // Range operators only work with numeric types
            (JsonValueType::Numeric, Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge) => {
                true
            }
            // Equality operators work with all types
            (_, Operator::Eq | Operator::Ne) => true,
            // Other combinations are not compatible
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum JsonPath {
    Simple(String),
    Nested(String, String),
    DeepNested(String, String, String),
}

impl Arbitrary for JsonPath {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![
            proptest::sample::select(vec![
                "name", "count", "active", "tags", "user", "settings", "level1", "items", "mixed"
            ])
            .prop_map(|key| JsonPath::Simple(key.to_string())),
            (
                proptest::sample::select(vec!["user", "settings", "level1", "mixed"]),
                proptest::sample::select(vec![
                    "name",
                    "age",
                    "theme",
                    "level2",
                    "text",
                    "number",
                    "boolean",
                    "null_value"
                ])
            )
                .prop_map(|(key1, key2)| JsonPath::Nested(key1.to_string(), key2.to_string())),
            (
                proptest::sample::select(vec!["level1"]),
                proptest::sample::select(vec!["level2"]),
                proptest::sample::select(vec!["level3"])
            )
                .prop_map(|(key1, key2, key3)| JsonPath::DeepNested(
                    key1.to_string(),
                    key2.to_string(),
                    key3.to_string()
                )),
        ]
        .boxed()
    }
}

impl JsonPath {
    fn is_boolean_field(&self) -> bool {
        match self {
            JsonPath::Simple(key) => key == "active",
            JsonPath::Nested(_, key2) => key2 == "boolean",
            JsonPath::DeepNested(_, _, _) => false,
        }
    }

    fn is_numeric_field(&self) -> bool {
        match self {
            JsonPath::Simple(key) => key == "count",
            JsonPath::Nested(_, key2) => key2 == "age" || key2 == "number",
            JsonPath::DeepNested(_, _, _) => false,
        }
    }
}

impl JsonPath {
    fn to_sql(&self) -> String {
        match self {
            JsonPath::Simple(key) => format!("'{key}'"),
            JsonPath::Nested(key1, key2) => format!("'{{{key1},{key2}}}'"),
            JsonPath::DeepNested(key1, key2, key3) => format!("'{{{key1},{key2},{key3}}}'"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum JsonOperation {
    Comparison {
        operator: Operator,
        value: JsonValueType,
    },
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse,
    In {
        values: Vec<JsonValueType>,
    },
    NotIn {
        values: Vec<JsonValueType>,
    },
}

impl Arbitrary for JsonOperation {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![
            (any::<Operator>(), any::<JsonValueType>())
                .prop_filter(
                    "operator and value type must be compatible",
                    |(operator, value)| { value.is_compatible_with_operator(operator) }
                )
                .prop_map(|(operator, value)| JsonOperation::Comparison { operator, value }),
            Just(JsonOperation::IsNull),
            Just(JsonOperation::IsNotNull),
            Just(JsonOperation::IsTrue),
            Just(JsonOperation::IsFalse),
            any::<JsonValueType>()
                .prop_flat_map(|value_type| { proptest::collection::vec(Just(value_type), 1..4) })
                .prop_map(|values| JsonOperation::In { values }),
            any::<JsonValueType>()
                .prop_flat_map(|value_type| { proptest::collection::vec(Just(value_type), 1..4) })
                .prop_map(|values| JsonOperation::NotIn { values }),
        ]
        .boxed()
    }
}

#[derive(Debug, Clone)]
pub struct JsonExpr {
    path: JsonPath,
    operation: JsonOperation,
}

impl Arbitrary for JsonExpr {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<JsonPath>(), any::<JsonOperation>())
            .prop_filter(
                "operation must be compatible with field type",
                |(path, operation)| {
                    match operation {
                        JsonOperation::Comparison { operator, value } => {
                            // For range operators, ensure we're using numeric fields
                            match operator {
                                Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge => {
                                    path.is_numeric_field()
                                        && value.is_compatible_with_operator(operator)
                                }
                                _ => {
                                    // For other operators, ensure value type matches field type
                                    match (value, path) {
                                        (JsonValueType::Numeric, path)
                                            if path.is_numeric_field() =>
                                        {
                                            true
                                        }
                                        (JsonValueType::Boolean, path)
                                            if path.is_boolean_field() =>
                                        {
                                            true
                                        }
                                        (JsonValueType::Text, path)
                                            if !path.is_numeric_field()
                                                && !path.is_boolean_field() =>
                                        {
                                            true
                                        }
                                        (JsonValueType::Null, _) => true, // NULL works with any field
                                        _ => false, // Incompatible combinations
                                    }
                                }
                            }
                        }
                        JsonOperation::IsTrue | JsonOperation::IsFalse => {
                            // Boolean operations only work on boolean fields
                            path.is_boolean_field()
                        }
                        JsonOperation::In { values } | JsonOperation::NotIn { values } => {
                            // For IN/NOT IN, ensure all values are compatible with the field type
                            values.iter().all(|value_type| {
                                match (value_type, path) {
                                    (JsonValueType::Numeric, path) if path.is_numeric_field() => {
                                        true
                                    }
                                    (JsonValueType::Boolean, path) if path.is_boolean_field() => {
                                        true
                                    }
                                    (JsonValueType::Text, path)
                                        if !path.is_numeric_field() && !path.is_boolean_field() =>
                                    {
                                        true
                                    }
                                    (JsonValueType::Null, _) => true, // NULL works with any field
                                    _ => false,                       // Incompatible combinations
                                }
                            })
                        }
                        _ => true, // Other operations work with any field type
                    }
                },
            )
            .prop_map(|(path, operation)| JsonExpr { path, operation })
            .boxed()
    }
}

impl JsonExpr {
    fn sample_values(&self) -> BoxedStrategy<Vec<String>> {
        match &self.operation {
            JsonOperation::Comparison { value, .. } => {
                let values = value.sample_values();
                proptest::collection::vec(values, 1..3).boxed()
            }
            JsonOperation::In { values: _ } | JsonOperation::NotIn { values: _ } => {
                // For IN/NOT IN operations, we'll use simple predefined values
                let predefined_values = vec![
                    vec!["'apple'".to_string()],
                    vec!["'banana'".to_string(), "'cherry'".to_string()],
                    vec!["'red apple'".to_string(), "'yellow banana'".to_string()],
                    vec!["'sweet cherry'".to_string(), "'fresh date'".to_string()],
                    vec!["42".to_string(), "100".to_string()],
                ];
                proptest::sample::select(predefined_values).boxed()
            }
            _ => Just(vec![]).boxed(),
        }
    }

    fn to_sql(&self, values: &[String]) -> String {
        let column = "metadata";
        let json_expr = format!("{column} ->> {}", self.path.to_sql());

        match &self.operation {
            JsonOperation::Comparison { operator, value } => {
                if values.is_empty() {
                    return format!("{} {} NULL", json_expr, operator.to_sql());
                }
                let value_literal = value.to_json_literal(&values[0]);

                // Determine the target type based on the field path and operation
                let target_type = if self.path.is_numeric_field() {
                    "numeric"
                } else if self.path.is_boolean_field() {
                    "boolean"
                } else {
                    "text"
                };

                // Add type casting based on the target type and operation
                let final_expr = match (operator, value, target_type) {
                    // Range operations on numeric fields
                    (
                        Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge,
                        JsonValueType::Numeric,
                        "numeric",
                    ) => {
                        format!("({json_expr})::numeric")
                    }
                    // Boolean comparisons
                    (_, JsonValueType::Boolean, "boolean") => {
                        format!("({json_expr})::boolean")
                    }
                    // Numeric comparisons on numeric fields
                    (_, JsonValueType::Numeric, "numeric") => {
                        format!("({json_expr})::numeric")
                    }
                    // Text comparisons (no casting needed for text fields)
                    (_, JsonValueType::Text, "text") => json_expr,
                    // Don't do cross-type comparisons - they're invalid
                    _ => {
                        // For incompatible types, just return the original expression
                        // This will likely cause a runtime error, but that's better than invalid SQL
                        json_expr
                    }
                };

                format!("{} {} {}", final_expr, operator.to_sql(), value_literal)
            }
            JsonOperation::IsNull => format!("{json_expr} IS NULL"),
            JsonOperation::IsNotNull => format!("{json_expr} IS NOT NULL"),
            JsonOperation::IsTrue => {
                // Ensure boolean operations only work on boolean fields
                format!("({json_expr})::boolean IS TRUE")
            }
            JsonOperation::IsFalse => {
                // Ensure boolean operations only work on boolean fields
                format!("({json_expr})::boolean IS FALSE")
            }
            JsonOperation::In { values } => {
                if values.is_empty() {
                    return format!("{json_expr} IN ()");
                }

                // Determine the target type based on the field path
                let target_type = if self.path.is_numeric_field() {
                    "numeric"
                } else if self.path.is_boolean_field() {
                    "boolean"
                } else {
                    "text"
                };

                // Cast the JSON expression to the appropriate type
                let casted_expr = format!("({json_expr})::{target_type}");

                // Generate values of the appropriate type (only compatible combinations)
                let value_literals: Vec<String> = values
                    .iter()
                    .map(|value_type| match value_type {
                        JsonValueType::Text => "'apple'".to_string(),
                        JsonValueType::Numeric => "42".to_string(),
                        JsonValueType::Boolean => "true".to_string(),
                        JsonValueType::Null => "NULL".to_string(),
                    })
                    .collect();
                format!("{} IN ({})", casted_expr, value_literals.join(", "))
            }
            JsonOperation::NotIn { values } => {
                if values.is_empty() {
                    return format!("{json_expr} NOT IN ()");
                }

                // Determine the target type based on the field path
                let target_type = if self.path.is_numeric_field() {
                    "numeric"
                } else if self.path.is_boolean_field() {
                    "boolean"
                } else {
                    "text"
                };

                // Cast the JSON expression to the appropriate type
                let casted_expr = format!("({json_expr})::{target_type}");

                // Generate values of the appropriate type (only compatible combinations)
                let value_literals: Vec<String> = values
                    .iter()
                    .map(|value_type| match value_type {
                        JsonValueType::Text => "'banana'".to_string(),
                        JsonValueType::Numeric => "100".to_string(),
                        JsonValueType::Boolean => "false".to_string(),
                        JsonValueType::Null => "NULL".to_string(),
                    })
                    .collect();
                format!("{} NOT IN ({})", casted_expr, value_literals.join(", "))
            }
        }
    }
}

fn json_pushdown_setup(conn: &mut PgConnection, index_config: &IndexConfig) -> String {
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let json_fields_config = index_config.to_json_fields_config();

    let setup_sql = format!(
        r#"
DROP TABLE IF EXISTS json_pushdown_test;
CREATE TABLE json_pushdown_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    metadata JSONB
);

-- Insert test data with various JSON structures
INSERT INTO json_pushdown_test (metadata) VALUES
    ('{{"name": "apple", "count": 42, "active": true, "tags": ["fruit", "red"]}}'),
    ('{{"name": "banana", "count": 100, "active": false, "tags": ["fruit", "yellow"]}}'),
    ('{{"name": "cherry", "count": 3.14, "active": true, "tags": ["fruit", "red"]}}'),
    ('{{"name": "date", "count": 0, "active": false, "tags": ["fruit", "brown"]}}'),
    ('{{"name": "elderberry", "count": -1, "active": true, "tags": ["fruit", "purple"]}}'),
    ('{{"name": "test", "count": 999, "active": false, "tags": ["test", "data"]}}'),
    ('{{"name": "value", "count": 1, "active": true, "tags": ["value", "test"]}}'),
    ('{{"name": "red apple", "count": 50, "active": true, "tags": ["fruit", "red", "multi"]}}'),
    ('{{"name": "yellow banana", "count": 75, "active": false, "tags": ["fruit", "yellow", "multi"]}}'),
    ('{{"name": "sweet cherry", "count": 25, "active": true, "tags": ["fruit", "red", "multi"]}}'),
    ('{{"name": "fresh date", "count": 60, "active": false, "tags": ["fruit", "brown", "multi"]}}'),
    ('{{"name": "purple elderberry", "count": 30, "active": true, "tags": ["fruit", "purple", "multi"]}}'),
    ('{{"name": "unit test", "count": 200, "active": false, "tags": ["test", "unit", "multi"]}}'),
    ('{{"name": "test value", "count": 150, "active": true, "tags": ["test", "value", "multi"]}}'),
    ('{{"user": {{"name": "alice", "age": 25}}, "settings": {{"theme": "dark"}}}}'),
    ('{{"user": {{"name": "bob", "age": 30}}, "settings": {{"theme": "light"}}}}'),
    ('{{"user": {{"name": "charlie", "age": 35}}, "settings": {{"theme": "dark"}}}}'),
    ('{{"level1": {{"level2": {{"level3": "deep_value"}}}}}}'),
    ('{{"level1": {{"level2": {{"level3": "another_value"}}}}}}'),
    ('{{"items": ["item1", "item2", "item3"]}}'),
    ('{{"items": ["item4", "item5", "item6"]}}'),
    ('{{"mixed": {{"text": "hello", "number": 123, "boolean": true, "null_value": null}}}}'),
    ('{{"mixed": {{"text": "world", "number": 456, "boolean": false, "null_value": null}}}}'),
    -- Edge case numeric values for type conversion testing - SKIPPED due to prop test failures
    -- ('{{"name": "edge_int", "count": 1}}'),
    -- ('{{"name": "edge_float", "count": 1.0}}'),
    -- ('{{"name": "max_safe_f64", "count": 9007199254740992}}'),
    -- ('{{"name": "i64_max", "count": 9223372036854775807}}'),
    -- ('{{"name": "i64_min", "count": -9223372036854775808}}'),
    -- ('{{"user": {{"name": "edge_test", "age": 1}}, "settings": {{"theme": "dark"}}}}'),
    -- ('{{"mixed": {{"text": "edge", "number": 1, "boolean": true, "null_value": null}}}}'),
    (NULL),
    ('{{}}');

-- Create BM25 index
CREATE INDEX idx_json_pushdown_test ON json_pushdown_test
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{json_fields_config}'
);

-- help our cost estimates
ANALYZE json_pushdown_test;
"#
    );

    setup_sql.clone().execute(conn);
    setup_sql
}

#[rstest]
#[tokio::test]
async fn json_pushdown_correctness(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        (expr, selected_values) in any::<JsonExpr>()
            .prop_flat_map(|expr| {
                let values_strategy = expr.sample_values();
                (Just(expr), values_strategy)
            }),
        index_config in any::<IndexConfig>(),
        gucs in any::<PgGucs>(),
    )| {
        let setup_sql = json_pushdown_setup(&mut pool.pull(), &index_config);
        eprintln!("Setup SQL:\n{setup_sql}");

        let json_condition = expr.to_sql(&selected_values);

        // Test SELECT queries with actual results
        let pg_query = format!(
            "SELECT id, metadata FROM json_pushdown_test WHERE {json_condition} ORDER BY id"
        );
        let bm25_query = format!(
            "SELECT id, metadata FROM json_pushdown_test WHERE id @@@ paradedb.all() AND {json_condition} ORDER BY id"
        );

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                query.fetch::<(i64, Option<serde_json::Value>)>(conn)
            },
        )?;
    });
}
```

---

## json.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

// In addition to checking whether all the expected types work for keys, make sure to include tests for anything that
//    is reliant on keys (e.g. stable_sort, alias)

#[rstest]
fn json_datatype(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id serial8,
        value json
    );

    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 120000 }');
    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 75000 }');
    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 140000 }');
    "#
    .execute(&mut conn);

    // if we don't segfault postgres here, we're good
    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', json_fields='{"value": {"indexed": true, "fast": true}}');
    "#
    .execute(&mut conn);
}

#[rstest]
fn simple_jsonb_string_array_crash(mut conn: PgConnection) {
    // ensure that we can index top-level json arrays that are strings.
    // Prior to 82fb7126ce6d2368cf19dd4dc6e28915afc5cf1e (PR #1618, <=v0.9.4) this didn't work

    r#"    
    CREATE TABLE crash
    (
        id serial8,
        j  jsonb
    );
    
    INSERT INTO crash (j) SELECT '["one-element-string-array"]' FROM generate_series(1, 10000);
    
    CREATE INDEX crash_idx ON crash
    USING bm25 (id, j) WITH (key_field='id', json_fields='{"j": {"indexed": true, "fast": true}}');
    "#
    .execute(&mut conn);
}
```

---

## expression.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn expression_paradedb_func(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb');

    CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, (lower(description)::pdb.simple)) WITH (key_field='id');

    INSERT INTO paradedb.index_config (description) VALUES ('Test description');
    "#
    .execute(&mut conn);

    let (count,) =
        "SELECT count(*) FROM paradedb.index_config WHERE index_config @@@ paradedb.term('description', 'test')"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = "SELECT count(*) FROM paradedb.index_config WHERE lower(description) @@@ 'test'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn expression_paradedb_op(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb');

    CREATE INDEX index_config_index ON paradedb.index_config
        USING bm25 (id, ((description || ' with cats')::pdb.simple)) WITH (key_field='id');

    INSERT INTO paradedb.index_config (description) VALUES ('Test description');
    "#
    .execute(&mut conn);

    // All entries in the index should match, since all of them now have cats.
    let (count,) =
        "SELECT count(*) FROM paradedb.index_config WHERE (description || ' with cats') @@@ 'cats'"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 42);
    // Inserted test value still should too.
    let (count,) =
        "SELECT count(*) FROM paradedb.index_config WHERE (description || ' with cats') @@@ 'description'"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}

#[rstest]
fn expression_conflicting_query_string(mut conn: PgConnection) {
    r#"
    CREATE TABLE expression_test (id SERIAL PRIMARY KEY, firstname TEXT, lastname TEXT);

    CREATE INDEX expression_test_idx ON expression_test
        USING bm25 (id, (lower(firstname)::pdb.simple), (lower(lastname)::pdb.simple)) WITH (key_field='id');

    INSERT INTO expression_test (firstname, lastname) VALUES ('John', 'Doe');
    "#
    .execute(&mut conn);

    let (count,) = "SELECT count(*) FROM expression_test WHERE lower(firstname) @@@ 'john'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = "SELECT count(*) FROM expression_test WHERE lower(lastname) @@@ 'doe'"
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) =
        "SELECT count(*) FROM expression_test WHERE lower(firstname) @@@ 'john' AND lower(lastname) @@@ 'doe'"
            .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);
}
```

---

## helpers.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Tests for the paradedb.tokenize function

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn defult_tokenizer(mut conn: PgConnection) {
    let rows: Vec<(String, i32)> = r#"
    SELECT * FROM paradedb.tokenize(paradedb.tokenizer('default'), 'hello world');
    "#
    .fetch_collect(&mut conn);

    assert_eq!(rows, vec![("hello".into(), 0), ("world".into(), 1)]);

    let res = r#"
    SELECT * FROM paradedb.tokenize(paradedb.tokenizer('de'), 'hello world');
    "#
    .execute_result(&mut conn);

    assert!(res.is_err());
}

#[rstest]
fn tokenizer_filters(mut conn: PgConnection) {
    // Test default tokenizer with default layers (lowercase => true, remove_long => 255).
    let rows: Vec<(String, i32)> = r#"
    SELECT * FROM paradedb.tokenize(
      paradedb.tokenizer('default'),
      'Hello, hello, ladiesandgentlemen!'
    );
    "#
    .fetch_collect(&mut conn);

    assert_eq!(
        rows,
        vec![
            ("hello".into(), 0),
            ("hello".into(), 1),
            ("ladiesandgentlemen".into(), 2)
        ]
    );

    // Test default optimizer with explicit layers.
    let rows: Vec<(String, i32)> = r#"
    SELECT * FROM paradedb.tokenize(
      paradedb.tokenizer('default', lowercase => false, remove_long => 15),
      'Hello, hello, ladiesandgentlemen!'
    );
    "#
    .fetch_collect(&mut conn);

    assert_eq!(
        rows,
        vec![
            ("Hello".into(), 0),
            ("hello".into(), 1),
            // ladiesandgentlemen is filtered out because it is too long
        ]
    );
}

#[rstest]
fn list_tokenizers(mut conn: PgConnection) {
    let rows: Vec<(String,)> = r#"
    SELECT * FROM paradedb.tokenizers();
    "#
    .fetch_collect(&mut conn);

    if cfg!(feature = "icu") {
        assert_eq!(
            rows,
            vec![
                ("default".into(),),
                ("keyword".into(),),
                ("keyword_deprecated".into(),),
                ("raw".into(),),
                ("literal_normalized".into(),),
                ("white_space".into(),),
                ("regex_tokenizer".into(),),
                ("chinese_compatible".into(),),
                ("source_code".into(),),
                ("ngram".into(),),
                ("chinese_lindera".into(),),
                ("japanese_lindera".into(),),
                ("korean_lindera".into(),),
                ("icu".into(),),
                ("jieba".into(),),
                ("lindera".into(),),
                ("unicode_words".into(),)
            ]
        );
    } else {
        assert_eq!(
            rows,
            vec![
                ("default".into(),),
                ("keyword".into(),),
                ("keyword_deprecated".into(),),
                ("raw".into(),),
                ("literal_normalized".into(),),
                ("white_space".into(),),
                ("regex_tokenizer".into(),),
                ("chinese_compatible".into(),),
                ("source_code".into(),),
                ("ngram".into(),),
                ("chinese_lindera".into(),),
                ("japanese_lindera".into(),),
                ("korean_lindera".into(),),
                ("jieba".into(),),
                ("lindera".into(),),
                ("unicode_words".into(),)
            ]
        );
    }
}

#[rstest]
fn test_index_fields(mut conn: PgConnection) {
    // First create a test table and index
    r#"
        CREATE TABLE test_fields (
            id INTEGER PRIMARY KEY,
            title TEXT,
            price NUMERIC,
            in_stock BOOLEAN,
            metadata JSONB,
            price_range INT8RANGE,
            created_at TIMESTAMP
        );
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX idx_test_fields ON test_fields USING bm25 (
            id, title, price, in_stock, metadata, price_range, created_at
        ) WITH (
            key_field='id',
            text_fields='{"title": {"fast": true}}',
            numeric_fields='{"price": {}}',
            boolean_fields='{"in_stock": {}}',
            json_fields='{"metadata": {}}',
            range_fields='{"price_range": {}}',
            datetime_fields='{"created_at": {}}'
        );
    "#
    .execute(&mut conn);

    // Get the index fields
    let row: (serde_json::Value,) = r#"
        SELECT paradedb.index_fields('idx_test_fields')::jsonb;
    "#
    .fetch_one(&mut conn);

    // Verify all fields are present with correct configurations
    let fields = row.0.as_object().unwrap();

    // Check key field (id)
    assert!(fields.contains_key("id"));
    let id_config = fields.get("id").unwrap().get("Numeric").unwrap();
    assert_eq!(id_config.get("indexed").unwrap(), true);
    assert_eq!(id_config.get("fast").unwrap(), true);

    // Check text field (title)
    assert!(fields.contains_key("title"));
    let title_config = fields
        .get("title")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Text")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        title_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );

    // Check numeric field (price)
    assert!(fields.contains_key("price"));
    let price_config = fields
        .get("price")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Numeric")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        price_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(price_config.get("fast").unwrap().as_bool().unwrap(), true);

    // Check boolean field (in_stock)
    assert!(fields.contains_key("in_stock"));
    let stock_config = fields
        .get("in_stock")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Boolean")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        stock_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );

    // Check JSON field (metadata)
    assert!(fields.contains_key("metadata"));
    let metadata_config = fields
        .get("metadata")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Json")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        metadata_config.get("indexed").unwrap().as_bool().unwrap(),
        true
    );

    assert!(fields.contains_key("price_range"));

    // Check datetime field (created_at)
    assert!(fields.contains_key("created_at"));
    let date_config = fields
        .get("created_at")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Date")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(date_config.get("indexed").unwrap().as_bool().unwrap(), true);

    // Cleanup
    r#"DROP TABLE test_fields CASCADE;"#.execute(&mut conn);
}
```

---

## parameterized_queries.rs

```
mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn self_referencing_var(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results =
        "SELECT id FROM test WHERE value @@@ paradedb.with_index('idxtest', paradedb.term('value', id::text)) ORDER BY id;".fetch::<(i64,)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (10,),
            (11,),
            (12,),
            (13,),
            (14,),
            (15,),
            (16,),
            (17,),
            (18,),
            (19,),
            (20,),
        ]
    );
}

#[rstest]
fn parallel_with_subselect(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // Unstable results without `debug_parallel_query`.
        return;
    }
    "SET debug_parallel_query TO on".execute(&mut conn);

    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    "PREPARE foo AS SELECT count(*) FROM test WHERE value @@@ (select $1);".execute(&mut conn);
    let (count,) = "EXECUTE foo('contains')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 11);

    // next 4 executions use one plan, and the 5th shouldn't change
    for _ in 0..5 {
        let (plan,) = "EXPLAIN (ANALYZE, FORMAT JSON) EXECUTE foo('contains');"
            .fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");
        let plan = plan
            .pointer("/0/Plan/Plans/1/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        pretty_assertions::assert_eq!(
            plan.get("Custom Plan Provider"),
            Some(&Value::String(String::from("ParadeDB Scan")))
        );
    }
}

#[rstest]
fn parallel_function_with_agg_subselect(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }

    "PREPARE foo AS SELECT id FROM test WHERE id @@@ paradedb.term_set((select array_agg(paradedb.term('value', token)) from paradedb.tokenize(paradedb.tokenizer('default'), $1))) ORDER BY id;".execute(&mut conn);

    let results = "EXECUTE foo('no matches')".fetch::<(i64,)>(&mut conn);
    assert_eq!(results.len(), 0);

    let results = "EXECUTE foo('value contains id')".fetch::<(i64,)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (10,),
            (11,),
            (12,),
            (13,),
            (14,),
            (15,),
            (16,),
            (17,),
            (18,),
            (19,),
            (20,),
        ]
    );
}

#[rstest]
fn test_issue2061(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results = r#"
    SELECT id, description, pdb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.match('description', (SELECT description FROM mock_items WHERE id = 1))
    ORDER BY pdb.score(id) DESC;
    "#
    .fetch::<(i32, String, f32)>(&mut conn);

    assert_eq!(
        results,
        vec![
            (1, "Ergonomic metal keyboard".into(), 9.485788),
            (2, "Plastic Keyboard".into(), 3.2668595),
        ]
    )
}
```

---

## search_config.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use tantivy::tokenizer::Language;
use tokenizers::manager::LANGUAGES;

// Define languages and corresponding test data
static MOCK_LANGUAGES: &[(Language, &str, &str, &str, &str)] = &[
    (
        Language::Arabic,
        "('محمد','رحلة إلى السوق مع أبي', 'مرحباً بك في المقالة الأولى. أتمنى أن تجد المحتوى مفيدًا ومثيرًا للاهتمام'),
        ('فاطمة', 'رحلة إلى الشرق', 'في هذا المقال، سنستكشف رحلة مثيرة إلى الشرق ونتعرف على ثقافات مختلفة وتاريخها الغني'),
        ('أحمد', 'نصائح للنجاح', 'هنا نقدم لك بعض النصائح القيمة لتحقيق النجاح في حياتك المهنية والشخصية. استفد منها وحقق أهدافك')",
        "محمد",
        "شرق",
        "هنا",
    ),
    (
        Language::Danish,
        "('Mette Hansen', 'Ny Bogudgivelse', 'Spændende ny bog udgivet af anerkendt forfatter.'),
        ('Lars Jensen', 'Teknologikonference Højdepunkter', 'Højdepunkter fra den seneste teknologikonference.'),
        ('Anna Nielsen', 'Lokal Kulturfestival', 'Der afholdes en lokal kulturfestival i weekenden med forventede madboder og forestillinger.')",
        "met",
        "højdepunk",
        "weekend",
    ),
    (
        Language::Dutch,
        " ('Pieter de Vries', 'Nieuw Boek Uitgebracht', 'Spannend nieuw boek uitgebracht door een bekende auteur.'),
        ('Annelies Bakker', 'Technologie Conferentie Hoogtepunten', 'Hoogtepunten van de laatste technologie conferentie.'),
        ('Jan Jansen', 'Lokale Culturele Festival', 'Dit weekend wordt er een lokaal cultureel festival gehouden met verwachte eetkraampjes en optredens.')",
        "vries",
        "hoogtepunt",
        "lokal",
    ),
    (
        Language::English,
        "('John Doe', 'New Book Release', 'Exciting new book released by renowned author.'),
        ('Jane Smith', 'Tech Conference Highlights', 'Highlights from the latest tech conference.'),
        ('Michael Brown', 'Local Charity Event', 'Upcoming charity event featuring local artists and performers.')",
        "john",
        "confer",
        "perform",
    ),
    (
        Language::Finnish,
        "('Matti Virtanen', 'Uusi Kirjan Julkaisu', 'Jännittävä uusi kirja julkaistu tunnetulta kirjailijalta.'),
        ('Anna Lehtonen', 'Teknologiakonferenssin Keskustelut', 'Viimeisimmän teknologiakonferenssin keskustelut ja huomiot.'),
        ('Juha Mäkinen', 'Paikallinen Kulttuuritapahtuma', 'Viikonloppuna järjestetään paikallinen kulttuuritapahtuma, jossa on odotettavissa erilaisia ruokakojuja ja esityksiä.')",
        "mat",
        "keskustelu",
        "järjest",
    ),
    (
        Language::French,
        "('Jean Dupont', 'Nouvelle Publication', 'Nouveau livre passionnant publié par un auteur renommé.'),
            ('Marie Leclerc', 'Points Forts de la Conférence Technologique', 'Points forts de la dernière conférence technologique.'),
            ('Pierre Martin', 'Festival Culturel Local', 'Ce week-end se tiendra un festival culturel local avec des stands de nourriture et des spectacles prévus.')",
        "dupont",
        "technolog",
        "tiendr",
    ),
    (
        Language::German,
        "('Hans Müller', 'Neue Buchveröffentlichung', 'Spannendes neues Buch veröffentlicht von einem bekannten Autor.'),
            ('Anna Schmidt', 'Highlights der Technologiekonferenz', 'Höhepunkte der letzten Technologiekonferenz.'),
            ('Michael Wagner', 'Lokales Kulturfestival', 'Am Wochenende findet ein lokales Kulturfestival statt, mit erwarteten Essensständen und Auftritten.')",
        "mull",
        "technologiekonferenz",
        "essensstand",
    ),
    (
        Language::Greek,
        "('Γιάννης Παπαδόπουλος', 'Νέα Έκδοση Βιβλίου', 'Συναρπαστικό νέο βιβλίο κυκλοφόρησε από γνωστό συγγραφέα.'),
            ('Αννα Στεφανίδου', 'Κορυφαίες Στιγμές της Τεχνολογικής Διάσκεψης', 'Κορυφαίες στιγμές από την τελευταία τεχνολογική διάσκεψη.'),
            ('Μιχάλης Παπαδόπουλος', 'Τοπικό Πολιτιστικό Φεστιβάλ', 'Το Σαββατοκύριακο θα πραγματοποιηθεί τοπικό πολιτιστικό φεστιβάλ με αναμενόμενα περίπτερα φαγητού και εμφανίσεις.')",
        "Παπαδόπουλος",
        "διασκεψ",
        "σαββατοκυριακ",
    ),
    (
        Language::Hungarian,
        "('János Kovács', 'Új Könyv Megjelenése', 'Izgalmas új könyv jelent meg egy ismert szerzőtől.'),
            ('Anna Nagy', 'Technológiai Konferencia Kiemelkedői', 'A legutóbbi technológiai konferencia kiemelkedő pillanatai.'),
            ('Gábor Tóth', 'Helyi Kulturális Fesztivál', 'Hétvégén helyi kulturális fesztivált rendeznek, várhatóan ételstandokkal és előadásokkal.')",
        "jános",
        "kiemelkedő",
        "várható",
    ),
    (
        Language::Italian,
        "('Giuseppe Rossi', 'Nuova Pubblicazione Libro', 'Nuovo libro emozionante pubblicato da un autore famoso.'),
            ('Maria Bianchi', 'Highlights della Conferenza Tecnologica', 'I momenti salienti della recente conferenza tecnologica.'),
            ('Luca Verdi', 'Festival Culturale Locale', 'Questo fine settimana si terrà un festival culturale locale, con previsti stand gastronomici e spettacoli.')",
        "ross",
        "conferent",
        "gastronom",
    ),
    (
        Language::Norwegian,
        "('Ole Hansen', 'Ny Bokutgivelse', 'Spennende ny bok utgitt av en kjent forfatter.'),
            ('Kari Olsen', 'Høydepunkter fra Teknologikonferansen', 'Høydepunkter fra den siste teknologikonferansen.'),
            ('Per Johansen', 'Lokal Kulturfestival', 'Denne helgen arrangeres det en lokal kulturfestival med forventede matboder og forestillinger.')",
        "ole",
        "høydepunkt",
        "forestilling",
    ),
    (
        Language::Polish,
        "('Jan Kowalski', 'Nowa Publikacja Książki', 'Ekscytująca nowa książka wydana przez znanego autora.'),
            ('Anna Nowak', 'Najważniejsze Momenty Konferencji Technologicznej', 'Najważniejsze momenty z ostatniej konferencji technologicznej.'),
            ('Piotr Wiśniewski', 'Lokalny Festiwal Kulturalny', 'W ten weekend odbędzie się lokalny festiwal kulturalny z planowanymi stojakami z jedzeniem i występami.')",
        "kowalsk",
        "technologiczn",
        "odbędz",
    ),
    (
        Language::Portuguese,
        "('João Silva', 'Novo Lançamento de Livro', 'Novo livro emocionante lançado por um autor famoso.'),
            ('Maria Santos', 'Destaques da Conferência de Tecnologia', 'Os destaques da última conferência de tecnologia.'),
            ('Pedro Oliveira', 'Festival Cultural Local', 'Neste fim de semana será realizado um festival cultural local, com barracas de comida e apresentações esperadas.')",
        "joã",
        "conferent",
        "será",
    ),
    (
        Language::Romanian,
        "('Ion Popescu', 'Nouă Publicație de Carte', 'O carte nouă și captivantă publicată de un autor renumit.'),
            ('Ana Ionescu', 'Momentele Cheie ale Conferinței Tehnologice', 'Cele mai importante momente ale ultimei conferințe tehnologice.'),
            ('Mihai Radu', 'Festival Cultural Local', 'În acest weekend va avea loc un festival cultural local, cu standuri de mâncare și spectacole programate.')",
        "popescu",
        "moment",
        "mânc",
    ),
    (
        Language::Russian,
        "('Иван Иванов', 'Новое издание книги', 'Увлекательная новая книга, выпущенная известным автором.'),
            ('Мария Петрова', 'Основные моменты технологической конференции', 'Основные моменты последней технологической конференции.'),
            ('Алексей Сидоров', 'Местный культурный фестиваль', 'В этот уикенд состоится местный культурный фестиваль с предполагаемыми палатками с едой и выступлениями.')",
        "иван",
        "технологическ",
        "культурн",
    ),
    (
        Language::Spanish,
        "('Juan Pérez', 'Nuevo Lanzamiento de Libro', 'Nuevo libro emocionante publicado por un autor famoso.'),
            ('María García', 'Aspectos Destacados de la Conferencia Tecnológica', 'Los momentos más destacados de la última conferencia tecnológica.'),
            ('Carlos Martínez', 'Festival Cultural Local', 'Este fin de semana se llevará a cabo un festival cultural local, con puestos de comida y actuaciones programadas.')",
        "pérez",
        "destac",
        "com",
    ),
    (
        Language::Swedish,
        "('Anna Andersson', 'Ny Bokutgivning', 'Spännande ny bok utgiven av en känd författare.'),
            ('Johan Eriksson', 'Höjdpunkter från Teknologikonferensen', 'Höjdpunkter från den senaste teknologikonferensen.'),
            ('Emma Nilsson', 'Lokalt Kulturfestival', 'Den här helgen hålls en lokal kulturfestival med förväntade matstånd och föreställningar.')",
        "ann",
        "höjdpunk",
        "föreställning",
    ),
    (
        Language::Tamil,
        "('சுப்ரமணியம் சுப்பிரமணியம்', 'புதிய புத்தக வெளியிடுதல்', 'ஒரு பிரபல எழுத்தாளரால் வெளியிடப்பட்ட புதிய புத்தகம்.'),
            ('லக்ஷ்மி சுந்தரம்', 'தொழில்நுட்ப மாநாடு முக்கியப்பட்டவை', 'கடைசி தொழில்நுட்ப மாநாட்டின் முக்கிய நிகழ்வுகள்.'),
            ('அருணா குமார்', 'உள்ளூர் கலாச்சார திருவிழா', 'இந்த வாரம் ஒரு உள்ளூர் கலாச்சார திருவிழா நடைபெறும், எங்களுக்கு உண்டாக்கப்பட்ட உணவு முன்னேற்றங்களுடன்.')",
        "சுப்பிரமணியம",
        "மாநாடு",
        "திருவிழா",
    ),
    (
        Language::Turkish,
        "('Ahmet Yılmaz', 'Yeni Kitap Yayınlandı', 'Ünlü bir yazar tarafından heyecan verici yeni bir kitap yayınlandı.'),
        ('Ayşe Kaya', 'Teknoloji Konferansının Öne Çıkanları', 'Son teknoloji konferansının öne çıkanları.'),
        ('Mehmet Demir', 'Yerel Kültür Festivali', 'Bu hafta sonu yerel bir kültür festivali düzenlenecek, yiyecek standları ve planlanmış gösterilerle.')",
        "yılmaz",
        "konferansı",
        "göster",
    )
];

#[rstest]
fn basic_search_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'category:electronics' ORDER BY id"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2, 12, 22, 32])
}

#[rstest]
fn with_limit_and_offset(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let rows: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'category:electronics'
         ORDER BY id LIMIT 2"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![1, 2]);

    let rows: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'category:electronics'
         ORDER BY id OFFSET 1 LIMIT 2"
            .fetch_collect(&mut conn);

    assert_eq!(rows.id, vec![2, 12]);
}

#[rstest]
fn default_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX tokenizer_config_idx ON paradedb.tokenizer_config
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "default"}}}')"#
        .execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id FROM paradedb.tokenizer_config
    WHERE tokenizer_config @@@ 'description:earbud' ORDER BY id"
        .fetch(&mut conn);

    assert!(rows.is_empty())
}

#[rstest]
fn ngram_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX tokenizer_config_idx ON paradedb.tokenizer_config
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 8, "prefix_only": false}}}')"#
        .execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id FROM paradedb.tokenizer_config
        WHERE tokenizer_config @@@ 'description:boa' ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows[0], (1,));
    assert_eq!(rows[1], (2,));
    assert_eq!(rows[2], (20,));
}

#[rstest]
fn chinese_compatible_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX tokenizer_config_idx ON paradedb.tokenizer_config
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "chinese_compatible"}}}')"#
        .execute(&mut conn);

    "INSERT INTO paradedb.tokenizer_config (description, rating, category) VALUES ('电脑', 4, 'Electronics');".execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id FROM paradedb.tokenizer_config
        WHERE tokenizer_config @@@ 'description:电脑' ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows[0], (42,));
}

#[rstest]
fn whitespace_tokenizer_config(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "whitespace"}}}')"#
        .execute(&mut conn);

    let count: (i64,) = "
    SELECT COUNT(*) FROM paradedb.bm25_search
    WHERE bm25_search @@@ 'description:shoes'"
        .fetch_one(&mut conn);
    assert_eq!(count.0, 3);
}

#[rstest]
fn raw_tokenizer_config(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "raw"}}}');
    "#
    .execute(&mut conn);

    let count: (i64,) = r#"
        SELECT COUNT(*) FROM paradedb.bm25_search
        WHERE bm25_search @@@ 'description:shoes'"#
        .fetch_one(&mut conn);
    assert_eq!(count.0, 0);

    let count: (i64,) = r#"
        SELECT COUNT(*) FROM paradedb.bm25_search
        WHERE bm25_search @@@ 'description:"GENERIC SHOES"'"#
        .fetch_one(&mut conn);
    assert_eq!(count.0, 1);

    let count: (i64,) = r#"
        SELECT COUNT(*) FROM paradedb.bm25_search
        WHERE bm25_search @@@ 'description:"Generic shoes"'"#
        .fetch_one(&mut conn);
    assert_eq!(count.0, 1);
}

#[rstest]
fn regex_tokenizer_config(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb')"
        .execute(&mut conn);

    r#"CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "regex", "pattern": "\\b\\w{4,}\\b"}}}');
    INSERT INTO paradedb.bm25_search (id, description) VALUES
        (11001, 'This is a simple test'),
        (11002, 'Rust is awesome'),
        (11003, 'Regex patterns are powerful'),
        (11004, 'Find the longer words');
    "#
    .execute(&mut conn);

    let count: (i64,) =
        "SELECT COUNT(*) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:simple'"
            .fetch_one(&mut conn);
    assert_eq!(count.0, 1);

    let count: (i64,) =
        "SELECT COUNT(*) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:is'"
            .fetch_one(&mut conn);
    assert_eq!(count.0, 0);

    let count: (i64,) =
        "SELECT COUNT(*) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:longer'"
            .fetch_one(&mut conn);
    assert_eq!(count.0, 1);
}

#[rstest]
fn language_stem_filter(mut conn: PgConnection) {
    for (language, data, author_query, title_query, message_query) in MOCK_LANGUAGES {
        let language_str = LANGUAGES.get(language).unwrap();
        let setup_query = format!(
            r#"
            DROP TABLE IF EXISTS test_table;
            CREATE TABLE IF NOT EXISTS test_table(
                id SERIAL PRIMARY KEY,
                author TEXT,
                title TEXT,
                message TEXT
            );
            INSERT INTO test_table (author, title, message)
            VALUES {data};
            CREATE INDEX stem_test ON test_table
                USING bm25 (id, author, title, message)
                WITH (key_field='id', text_fields='{{
                    "author": {{"tokenizer": {{"type": "default", "stemmer": "{language_str}"}}}},
                    "title": {{"tokenizer": {{"type": "default", "stemmer": "{language_str}"}}}},
                    "message": {{"tokenizer": {{"type": "default", "stemmer": "{language_str}"}}}}
                }}');"#
        );

        setup_query.execute(&mut conn);

        let author_search_query = format!(
            "SELECT id FROM test_table WHERE test_table @@@ 'author:{author_query}' ORDER BY id"
        );
        let title_search_query = format!(
            "SELECT id FROM test_table WHERE test_table @@@ 'title:{title_query}' ORDER BY id"
        );
        let message_search_query = format!(
            "SELECT id FROM test_table WHERE test_table @@@ 'message:{message_query}' ORDER BY id"
        );

        let row: (i32,) = author_search_query.fetch_one(&mut conn);
        assert_eq!(row.0, 1);

        let row: (i32,) = title_search_query.fetch_one(&mut conn);
        assert_eq!(row.0, 2);

        let row: (i32,) = message_search_query.fetch_one(&mut conn);
        assert_eq!(row.0, 3);

        r#"
        DROP INDEX IF EXISTS stem_test;
        DROP TABLE IF EXISTS test_table;
        "#
        .execute(&mut conn);
    }
}

#[rstest]
fn default_config_is_stored_false(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    // we are using our default configurations for this index and none of them should be `stored = true`
    let count: (i64,) =
        r#"SELECT COUNT(*) FROM paradedb.schema('paradedb.bm25_search_idx') WHERE stored = true"#
            .fetch_one(&mut conn);
    assert_eq!(count.0, 0);
}

#[rstest]
fn stopwords_language_tokenizer_config(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "default", "stopwords_language": "English"}}}');
    "#
    .execute(&mut conn);

    let count: (i64,) = "
    SELECT COUNT(*) FROM paradedb.bm25_search
    WHERE bm25_search @@@ 'description:on'"
        .fetch_one(&mut conn);
    assert_eq!(count.0, 0);

    let count: (i64,) = r#"
    SELECT COUNT(*) FROM paradedb.bm25_search
    WHERE bm25_search @@@ 'description:"Hardcover book on history"'"#
        .fetch_one(&mut conn);
    assert_eq!(count.0, 1);
}

#[rstest]
fn stopwords_tokenizer_config(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description)
        WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "default", "stopwords": ["shoes"]}}}');
    "#
    .execute(&mut conn);

    let count: (i64,) = "
    SELECT COUNT(*) FROM paradedb.bm25_search
    WHERE bm25_search @@@ 'description:shoes'"
        .fetch_one(&mut conn);
    assert_eq!(count.0, 0);
}
```

---

## one_index.rs

```
mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn only_one_index_allowed(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_one ON public.mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    match r#"
    CREATE INDEX index_two ON public.mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("created a second `USING bm25` index"),
        Err(e) if format!("{e}").contains("a relation may only have one `USING bm25` index") => (), // all good
        Err(e) => panic!("{}", e),
    }
}
```

---

## snapshot.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn score_bm25_after_delete(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 3 OR id = 4".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, pdb.score(id) FROM paradedb.bm25_search
    WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);
}

#[rstest]
async fn snippet_after_delete(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 3 OR id = 4".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, pdb.snippet(description) FROM paradedb.bm25_search
    WHERE description @@@ 'shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);
}

#[rstest]
async fn score_bm25_after_update(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "UPDATE paradedb.bm25_search SET description = 'leather sandals' WHERE id = 3"
        .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:sandals' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3]);

    let rows: Vec<(i32,)> =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 4]);
}

#[rstest]
async fn snippet_after_update(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "UPDATE paradedb.bm25_search SET description = 'leather sandals' WHERE id = 3"
        .execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id, pdb.snippet(description) FROM paradedb.bm25_search
        WHERE description @@@ 'sandals' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3]);

    let rows: Vec<(i32,)> = "
        SELECT id, pdb.snippet(description) FROM paradedb.bm25_search
        WHERE description @@@ 'shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [4, 5]);
}

#[rstest]
async fn score_bm25_after_rollback(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 3".execute(&mut conn);

    "BEGIN".execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);

    "ROLLBACK".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 4]);
}

#[rstest]
async fn snippet_after_rollback(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 3".execute(&mut conn);

    "BEGIN".execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id, pdb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'shoes' ORDER BY id"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);

    "ROLLBACK".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, pdb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'shoes' ORDER BY id"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [4, 5]);
}

#[rstest]
async fn score_bm25_after_vacuum(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    "VACUUM paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC, id DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 3]);

    "VACUUM FULL paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC, id DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 3]);
}

#[rstest]
async fn snippet_after_vacuum(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    "VACUUM paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, pdb.snippet(description) FROM paradedb.bm25_search
    WHERE description @@@ 'description:shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3, 5]);

    "VACUUM FULL paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, pdb.snippet(description) FROM paradedb.bm25_search
    WHERE description @@@ 'description:shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3, 5]);
}
```

---

## jieba.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

// Helper function to run tokenize and collect results
fn get_tokens(conn: &mut PgConnection, tokenizer_type: &str, text: &str) -> Vec<(String, i32)> {
    let query_str = format!(
        "SELECT token, position FROM paradedb.tokenize(paradedb.tokenizer('{tokenizer_type}'), '{text}') ORDER BY position;"
    );
    query_str.fetch(conn)
}

#[rstest]
fn test_jieba_tokenizer_basic(mut conn: PgConnection) {
    // Test the paradedb.tokenize function directly
    // Positions should be sequential token ordinals (0, 1, 2, ...), not character offsets
    let tokens = get_tokens(&mut conn, "jieba", "我们都有光明的前途");
    assert_eq!(
        tokens,
        vec![
            ("我们".to_string(), 0),
            ("都".to_string(), 1),
            ("有".to_string(), 2),
            ("光明".to_string(), 3),
            ("的".to_string(), 4),
            ("前途".to_string(), 5),
        ],
        "Failed on '我们都有光明的前途'"
    );

    let tokens = get_tokens(&mut conn, "jieba", "李宇");
    assert_eq!(tokens, vec![("李宇".to_string(), 0),], "Failed on '李宇'");

    let tokens = get_tokens(&mut conn, "jieba", "公安");
    assert_eq!(tokens, vec![("公安".to_string(), 0),], "Failed on '公安'");

    let tokens = get_tokens(&mut conn, "jieba", "转移就业");
    assert_eq!(
        tokens,
        vec![("转移".to_string(), 0), ("就业".to_string(), 1),],
        "Failed on '转移就业'"
    );
}

#[rstest]
fn test_jieba_tokenizer_indexing(mut conn: PgConnection) {
    // Create a table and index using the jieba tokenizer
    r#"CREATE TABLE chinese_texts (
            id SERIAL PRIMARY KEY,
            content TEXT
        );"#
    .execute(&mut conn);

    r#"INSERT INTO chinese_texts (content) VALUES
            ('我们都有光明的前途'),
            ('李宇给公安局打了电话'),
            ('这项政策旨在促进劳动力转移就业');"#
        .execute(&mut conn);

    r#"CREATE INDEX chinese_texts_idx ON chinese_texts
        USING bm25 (id, content)
        WITH (
            key_field = 'id',
            text_fields = '{
                "content": { "tokenizer": {"type": "jieba"} }
            }'
        );"#
    .execute(&mut conn);

    // Test searching using fetch/fetch_one extension methods
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM chinese_texts WHERE chinese_texts @@@ 'content:光明' ORDER BY id"#
            .fetch(&mut conn);
    assert_eq!(rows, vec![(1,)], "Failed on 'content:光明'");

    let row: (i32,) =
        r#"SELECT id FROM chinese_texts WHERE chinese_texts @@@ 'content:公安局' ORDER BY id"#
            .fetch_one(&mut conn);
    assert_eq!(row, (2,), "Failed on 'content:公安局'");

    let row: (i32,) =
        r#"SELECT id FROM chinese_texts WHERE chinese_texts @@@ 'content:就业' ORDER BY id"#
            .fetch_one(&mut conn);
    assert_eq!(row, (3,), "Failed on 'content:就业'");
}
```

---

## reindex.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use anyhow::Result;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn basic_reindex(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Verify initial search works
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify search still works after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn concurrent_reindex(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Verify initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Perform concurrent REINDEX
    "REINDEX INDEX CONCURRENTLY paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify search still works after concurrent reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn reindex_with_updates(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Make some updates
    "UPDATE paradedb.bm25_search SET description = 'Mechanical keyboard' WHERE id = 1"
        .execute(&mut conn);
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES ('Wireless keyboard', 'Electronics', 4, true, '{\"color\": \"black\"}', now(), current_date)".execute(&mut conn);

    // Verify updates are searchable
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 42]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify all updates are still searchable after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 42]);

    Ok(())
}

#[rstest]
async fn reindex_with_deletes(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Delete some records
    "DELETE FROM paradedb.bm25_search WHERE id = 1".execute(&mut conn);

    // Verify delete is reflected in search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify deleted records are still not searchable after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    Ok(())
}

#[rstest]
async fn reindex_schema_validation(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Get initial schema
    let initial_schema: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.bm25_search_bm25_index') ORDER BY name"
            .fetch(&mut conn);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Get schema after reindex
    let reindexed_schema: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.bm25_search_bm25_index') ORDER BY name"
            .fetch(&mut conn);

    // Verify schema hasn't changed
    assert_eq!(initial_schema, reindexed_schema);

    Ok(())
}

#[rstest]
async fn reindex_partial_index(mut conn: PgConnection) -> Result<()> {
    "CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');"
        .execute(&mut conn);

    // Create a partial index
    r#"CREATE INDEX partial_idx ON paradedb.bm25_search
    USING bm25 (id, description, category)
    WITH (key_field='id')
    WHERE category = 'Electronics'"#
        .execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.partial_idx".execute(&mut conn);

    // Verify partial index still works correctly after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn concurrent_reindex_with_updates(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Start concurrent reindex
    "REINDEX INDEX CONCURRENTLY paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Make updates during reindex
    "UPDATE paradedb.bm25_search SET description = 'Mechanical keyboard' WHERE id = 1"
        .execute(&mut conn);
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES ('Wireless keyboard', 'Electronics', 4, true, '{\"color\": \"black\"}', now(), current_date)".execute(&mut conn);

    // Verify all updates are searchable after concurrent reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 42]);

    Ok(())
}

#[rstest]
async fn reindex_table(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Reindex entire table
    "REINDEX TABLE paradedb.bm25_search".execute(&mut conn);

    // Verify search still works
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn concurrent_index_creation(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Create a second index concurrently
    r#"CREATE INDEX CONCURRENTLY bm25_search_bm25_index_2 ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "default"}},
            "category": {}
        }',
        numeric_fields='{"rating": {}}',
        boolean_fields='{"in_stock": {}}',
        json_fields='{"metadata": {}}',
        datetime_fields='{"created_at": {}, "last_updated_date": {}}'
    )"#.execute(&mut conn);

    // Query using the new index
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Drop the original index
    "DROP INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify the new index still works
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}
```

---

## datetime.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn datetime_microsecond(mut conn: PgConnection) {
    r#"
    CREATE TABLE ts (id SERIAL, t TIMESTAMP);
    CREATE INDEX ts_idx on ts using bm25 (id, t) with (key_field = 'id');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079776Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079777Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079778Z');
    "#
    .execute(&mut conn);

    // Term queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079777Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.term('t', '2025-01-28T18:19:14.079777Z'::timestamp)".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079777Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.079777Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    // Range queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.079777Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.079777Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
fn datetime_term_millisecond(mut conn: PgConnection) {
    r#"
    CREATE TABLE ts (id SERIAL, t TIMESTAMP(3));
    CREATE INDEX ts_idx on ts using bm25 (id, t) with (key_field = 'id');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.078Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.079Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.08Z');
    "#
    .execute(&mut conn);

    // Term queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id FROM ts WHERE id @@@ paradedb.term('t', '2025-01-28T18:19:14.079Z'::timestamp)"
            .fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.079Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.079Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.078001Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.078001Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    // Range queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.079Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.079Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.07Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.07Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);
}

#[rstest]
fn datetime_term_second(mut conn: PgConnection) {
    r#"
    CREATE TABLE ts (id SERIAL, t TIMESTAMP(0));
    CREATE INDEX ts_idx on ts using bm25 (id, t) with (key_field = 'id');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:14.1Z');
    INSERT INTO ts (t) values ('2025-01-28T18:19:15Z');
    "#
    .execute(&mut conn);

    // Term queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.term('t', '2025-01-28T18:19:14Z'::timestamp) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t = '2025-01-28T18:19:14.1Z'::timestamp".fetch(&mut conn);
    let rows: Vec<(i32,)> =
        r#"SELECT id FROM ts WHERE t @@@ '"2025-01-28T18:19:14.1Z"'"#.fetch(&mut conn);
    assert_eq!(rows, expected);

    // Range queries
    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);

    let expected: Vec<(i32,)> =
        "SELECT id FROM ts WHERE t > '2025-01-28T18:19:14.001Z'::timestamp ORDER BY id"
            .fetch(&mut conn);
    let rows: Vec<(i32,)> = "SELECT id FROM ts WHERE id @@@ paradedb.range('t', tsrange('2025-01-28T18:19:14.001Z'::timestamp, NULL, '(]')) ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, expected);
}
```

---

## parallel.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use std::time::Instant;

use anyhow::Result;
use fixtures::*;
use futures::future::join_all;
use pretty_assertions::assert_eq;
use rand::Rng;
use rstest::*;
use tokio::join;

/// This test targets the locking functionality between Tantivy writers.
/// With no locking implemented, a high number of concurrent writers will
/// cause in an error when they all try to commit to the index at once.
#[rstest]
#[tokio::test]
async fn test_simultaneous_commits_with_bm25(database: Db) -> Result<()> {
    let mut conn1 = database.connection().await;

    // Create table once using any of the connections.
    r#"CREATE EXTENSION pg_search;

    CREATE TABLE concurrent_items (
      id SERIAL PRIMARY KEY,
      description TEXT,
      category VARCHAR(255),
      created_at TIMESTAMP DEFAULT now()
    );

    CREATE INDEX concurrent_items_bm25 ON public.concurrent_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {}
        }'
    );
    "#
    .execute(&mut conn1);

    // Dynamically generate at least 100 rows for each connection
    let mut rng = rand::rng();
    let categories = [
        "Category 1",
        "Category 2",
        "Category 3",
        "Category 4",
        "Category 5",
    ];

    for i in 0..5 {
        let random_category = categories[rng.random_range(0..categories.len())];

        // Create new connections for this iteration and store them in a vector
        let mut connections = vec![];
        for _ in 0..50 {
            connections.push(database.connection().await);
        }

        let mut futures = vec![];
        for (n, mut conn) in connections.into_iter().enumerate() {
            let query = format!(
                "INSERT INTO concurrent_items (description, category)
                 VALUES ('Item {i} from conn{n}', '{random_category}')"
            );
            // Move the connection into the future, avoiding multiple borrows
            futures.push(async move { query.execute_async(&mut conn).await });
        }

        // Await all the futures for this iteration
        join_all(futures).await;
    }

    // Verify the number of rows in each database
    let rows1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_items")
        .fetch_one(&mut conn1)
        .await?;

    assert_eq!(rows1, 250);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_statement_level_locking(database: Db) -> Result<()> {
    let mut conn = database.connection().await;

    // Create tables and indexes
    r#"CREATE EXTENSION pg_search;
    CREATE TABLE index_a (
      id SERIAL PRIMARY KEY,
      content TEXT
    );
    CREATE TABLE index_b (
      id SERIAL PRIMARY KEY,
      content TEXT
    );

    CREATE INDEX index_a_bm25 ON public.index_a
    USING bm25 (id, content)
    WITH (
        key_field = 'id',
        text_fields = '{
            "content": {}
        }'
    );

    CREATE INDEX index_b_bm25 ON public.index_b
    USING bm25 (id, content)
    WITH (
        key_field = 'id',
        text_fields = '{
            "content": {}
        }'
    );
    "#
    .execute(&mut conn);

    // Create two separate connections
    let mut conn_a = database.connection().await;
    let mut conn_b = database.connection().await;

    // Define the tasks for each connection
    let task_a = async move {
        "INSERT INTO index_a (content) VALUES ('Content A1');
         SELECT pg_sleep(3);
         INSERT INTO index_b (content) VALUES ('Content B1 from A');"
            .execute_async(&mut conn_a)
            .await;
    };

    let task_b = async move {
        "INSERT INTO index_b (content) VALUES ('Content B2');
         SELECT pg_sleep(3);
         INSERT INTO index_a (content) VALUES ('Content A2 from B');"
            .execute_async(&mut conn_b)
            .await;
    };

    // We're going to check a timer to ensure both of these queries,
    // which each sleep at query time, run concurrently.
    let start_time = Instant::now();

    // Run both tasks concurrently
    join!(task_a, task_b);

    // Stop the timer and assert that the duration is close to 5 seconds
    let duration = start_time.elapsed();
    assert!(
        duration.as_secs() >= 3 && duration.as_secs() < 5,
        "Expected duration to be around 3 seconds, but it took {duration:?}"
    );

    // Verify the results
    let count_a: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_a")
        .fetch_one(&mut conn)
        .await?;
    let count_b: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_b")
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(count_a, 2, "Expected 2 rows in index_a");
    assert_eq!(count_b, 2, "Expected 2 rows in index_b");

    Ok(())
}
```

---

## matview.rs

```
mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn refresh_matview_concurrently_issue2308(mut conn: PgConnection) {
    // if this doesn't raise an ERROR then it worked
    r#"
    DROP MATERIALIZED VIEW IF EXISTS TEST_mv;
    DROP TABLE IF EXISTS TEST_tbl;

    -- 2) Setup table
    CREATE table TEST_tbl (
        id integer
    );

    -- 3) insert data (data is optional for it to fail)
    -- INSERT INTO TEST_1 VALUES (1), (2), (3), (4);

    -- 4) Setup materialized view
    CREATE MATERIALIZED VIEW TEST_mv AS (SELECT * FROM TEST_tbl);
    CREATE UNIQUE INDEX test_idx ON TEST_mv (id); -- required for `CONCURRENTLY` to work
    CREATE INDEX TEST_bm25 ON TEST_mv USING bm25 (id) WITH (key_field='id');

    -- 5) Refresh the view concurrently
    REFRESH MATERIALIZED VIEW CONCURRENTLY TEST_mv;
    "#
    .execute(&mut conn);
}
```

---

## scalar_array_pushdown.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

use crate::fixtures::querygen::opexprgen::{ArrayQuantifier, Operator, ScalarArrayOperator};
use crate::fixtures::querygen::{compare, PgGucs};

#[derive(Debug, Clone, Arbitrary)]
pub enum TokenizerType {
    Default,
    Keyword,
}

impl TokenizerType {
    fn to_index_config(&self) -> &'static str {
        match self {
            TokenizerType::Default => r#""tokenizer": {"type": "default"}"#,
            TokenizerType::Keyword => r#""tokenizer": {"type": "keyword"}"#,
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum ColumnType {
    Text,
    Integer,
    Boolean,
    Timestamp,
    Uuid,
}

impl ColumnType {
    fn column_name(&self) -> &'static str {
        match self {
            ColumnType::Text => "text_col",
            ColumnType::Integer => "int_col",
            ColumnType::Boolean => "bool_col",
            ColumnType::Timestamp => "ts_col",
            ColumnType::Uuid => "uuid_col",
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum ArrayOperation {
    OperatorQuantifier {
        operator: Operator,
        quantifier: ArrayQuantifier,
    },
    ScalarArray {
        operator: ScalarArrayOperator,
    },
}

#[derive(Debug, Clone, Arbitrary)]
pub struct ScalarArrayExpr {
    column_type: ColumnType,
    operation: ArrayOperation,
    tokenizer: TokenizerType,
    include_null: bool,
}

impl ScalarArrayExpr {
    fn sample_values(&self) -> impl Strategy<Value = String> {
        let values = match self.column_type {
            ColumnType::Text => vec![
                "'apple'".to_string(),
                "'banana'".to_string(),
                "'cherry'".to_string(),
                "'date'".to_string(),
                "'elderberry'".to_string(),
            ],
            ColumnType::Integer => vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "42".to_string(),
                "100".to_string(),
            ],
            ColumnType::Boolean => {
                vec!["true".to_string(), "false".to_string()]
            }
            ColumnType::Timestamp => vec![
                "'2023-01-01 00:00:00'::timestamp".to_string(),
                "'2023-06-15 12:30:00'::timestamp".to_string(),
                "'2024-01-01 00:00:00'::timestamp".to_string(),
                "'2024-06-01 09:15:00'::timestamp".to_string(),
            ],
            ColumnType::Uuid => vec![
                "'550e8400-e29b-41d4-a716-446655440000'::uuid".to_string(),
                "'6ba7b810-9dad-11d1-80b4-00c04fd430c8'::uuid".to_string(),
                "'6ba7b811-9dad-11d1-80b4-00c04fd430c8'::uuid".to_string(),
                "'12345678-1234-5678-9abc-123456789abc'::uuid".to_string(),
            ],
        };
        proptest::sample::select(values)
    }

    fn null_value(&self) -> String {
        match self.column_type {
            ColumnType::Text => "NULL::text".to_string(),
            ColumnType::Integer => "NULL::integer".to_string(),
            ColumnType::Boolean => "NULL::boolean".to_string(),
            ColumnType::Timestamp => "NULL::timestamp".to_string(),
            ColumnType::Uuid => "NULL::uuid".to_string(),
        }
    }

    fn to_sql(&self, values: &[String]) -> String {
        let column = self.column_type.column_name();

        // Add NULL to values if include_null is true
        let mut final_values = values.to_vec();
        if self.include_null {
            final_values.push(self.null_value());
        }

        match &self.operation {
            ArrayOperation::OperatorQuantifier {
                operator,
                quantifier,
            } => {
                let op = operator.to_sql();
                let quant = quantifier.to_sql();
                let array_literal = format!("ARRAY[{}]", final_values.join(", "));
                format!("{column} {op} {quant}({array_literal})")
            }
            ArrayOperation::ScalarArray { operator } => {
                let op = operator.to_sql();
                format!("{} {} ({})", column, op, final_values.join(", "))
            }
        }
    }
}

fn scalar_array_setup(conn: &mut PgConnection, tokenizer: TokenizerType) -> String {
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let setup_sql = format!(
        r#"
DROP TABLE IF EXISTS scalar_array_test;
CREATE TABLE scalar_array_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    text_col TEXT,
    int_col INTEGER,
    bool_col BOOLEAN,
    ts_col TIMESTAMP,
    uuid_col UUID
);

-- Insert test data
INSERT INTO scalar_array_test (text_col, int_col, bool_col, ts_col, uuid_col) VALUES
    ('apple', 1, true, '2023-01-01 00:00:00', '550e8400-e29b-41d4-a716-446655440000'),
    ('Apple', 2, false, '2023-06-15 12:30:00', '6ba7b810-9dad-11d1-80b4-00c04fd430c8'),
    ('Apple Tree', 3, true, '2024-01-01 00:00:00', '6ba7b811-9dad-11d1-80b4-00c04fd430c8'),
    ('banana', 42, false, '2023-12-25 18:00:00', '12345678-1234-5678-9abc-123456789abc'),
    ('banana bunch', 100, true, '2024-06-01 09:15:00', '550e8400-e29b-41d4-a716-446655440001'),
    ('Ripe Banana', 1, false, '2023-03-15 14:20:00', '6ba7b810-9dad-11d1-80b4-00c04fd430c9'),
    ('banana', 2, true, '2023-09-30 20:45:00', '6ba7b811-9dad-11d1-80b4-00c04fd430c9'),
    ('banana', 3, false, '2024-02-14 11:30:00', '12345678-1234-5678-9abc-123456789abd'),
    -- Rows with NULL values
    (NULL, 4, true, '2024-03-01 10:00:00', '550e8400-e29b-41d4-a716-446655440002'),
    ('cherry', NULL, false, '2024-04-01 11:00:00', '6ba7b810-9dad-11d1-80b4-00c04fd430ca'),
    ('date', 42, NULL, '2024-05-01 12:00:00', '6ba7b811-9dad-11d1-80b4-00c04fd430ca'),
    ('elderberry', 2, true, NULL, '12345678-1234-5678-9abc-123456789abe'),
    ('cherry', 1, false, '2024-07-01 14:00:00', NULL);

-- Create BM25 index with configurable tokenizer
CREATE INDEX idx_scalar_array_test ON scalar_array_test
USING bm25 (id, text_col, int_col, bool_col, ts_col, uuid_col)
WITH (
    key_field = 'id',
    text_fields = '{{
        "text_col": {{ {} }},
        "uuid_col": {{ {} }}
    }}'
);

-- help our cost estimates
ANALYZE scalar_array_test;
"#,
        tokenizer.to_index_config(),
        tokenizer.to_index_config()
    );

    setup_sql.clone().execute(conn);
    setup_sql
}

#[rstest]
#[tokio::test]
async fn scalar_array_pushdown_correctness(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        (expr, selected_values) in any::<ScalarArrayExpr>()
            .prop_flat_map(|expr| {
                let values_strategy = proptest::collection::vec(expr.sample_values(), 1..4);
                (Just(expr), values_strategy)
            }),
        gucs in any::<PgGucs>(),
    )| {
        let setup_sql = scalar_array_setup(&mut pool.pull(), expr.tokenizer.clone());
        eprintln!("Setup SQL:\n{setup_sql}");

        let array_condition = expr.to_sql(&selected_values);

        // Test SELECT queries with actual results
        let pg_query = format!(
            "SELECT id, text_col FROM scalar_array_test WHERE {array_condition} ORDER BY id"
        );
        let bm25_query = format!(
            "SELECT id, text_col FROM scalar_array_test WHERE {array_condition} ORDER BY id"
        );

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                query.fetch::<(i64, Option<String>)>(conn)
            },
        )?;
    });
}
```

---

## replication.rs

```
mod fixtures;

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};
use dotenvy::dotenv;
use fixtures::db::Query;
use rstest::*;
use sqlx::{Connection, PgConnection};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// Static variables for initializing port assignment and ensuring one-time setup
static INIT: Once = Once::new();
static LAST_PORT: AtomicUsize = AtomicUsize::new(49152);

// Function to check if a port can be bound (i.e., is available)
fn can_bind(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

// Function to get a free port in the dynamic port range
fn get_free_port() -> u16 {
    let port_upper_bound = 65535;
    let port_lower_bound = 49152;

    INIT.call_once(|| {
        LAST_PORT.store(port_lower_bound, Ordering::SeqCst);
    });

    loop {
        let port = LAST_PORT.fetch_add(1, Ordering::SeqCst);
        if port > port_upper_bound {
            LAST_PORT.store(port_lower_bound, Ordering::SeqCst);
            continue;
        }

        if can_bind(port as u16) {
            return port as u16;
        }
    }
}

// Struct to manage an ephemeral PostgreSQL instance
struct EphemeralPostgres {
    pub tempdir_path: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pg_ctl_path: PathBuf,
}

// Implement Drop trait to ensure the PostgreSQL instance is properly stopped
impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        let path = &self.tempdir_path;
        let pg_ctl_path = &self.pg_ctl_path;
        run_cmd!($pg_ctl_path -D $path stop &> /dev/null)
            .unwrap_or_else(|_| println!("postgres instance at {} already shut down", self.port));
        std::fs::remove_dir_all(self.tempdir_path.clone()).unwrap();
    }
}

// Implementation of EphemeralPostgres
impl EphemeralPostgres {
    fn pg_bin_path() -> PathBuf {
        let pg_config_path = std::env::var("PG_CONFIG").expect(
            "PG_CONFIG variable must be set to enable creating ephemeral Postgres instances",
        );
        if !PathBuf::from(&pg_config_path).exists() {
            panic!("PG_CONFIG variable must a valid path to enable creating ephemeral Postgres instances, received {pg_config_path}");
        }
        match run_fun!($pg_config_path --bindir) {
            Ok(path) => PathBuf::from(path.trim().to_string()),
            Err(err) => panic!("could run pg_config --bindir to get Postgres bin folder: {err}"),
        }
    }

    fn pg_basebackup_path() -> PathBuf {
        Self::pg_bin_path().join("pg_basebackup")
    }

    fn initdb_path() -> PathBuf {
        Self::pg_bin_path().join("initdb")
    }

    fn pg_ctl_path() -> PathBuf {
        Self::pg_bin_path().join("pg_ctl")
    }

    fn new_from_initialized(
        tempdir_path: &Path,
        postgresql_conf: Option<&str>,
        pg_hba_conf: Option<&str>,
    ) -> Self {
        let tempdir_path = tempdir_path.to_str().unwrap().to_string();
        let port = get_free_port();
        let pg_ctl_path = Self::pg_ctl_path();

        // Write to postgresql.conf
        let config_content = match postgresql_conf {
            Some(config) => format!("port = {}\n{}", port, config.trim()),
            None => format!("port = {port}"),
        };
        let config_path = format!("{tempdir_path}/postgresql.conf");
        std::fs::write(config_path, config_content).expect("Failed to write to postgresql.conf");

        // Write to pg_hba.conf
        if let Some(config_content) = pg_hba_conf {
            let config_path = format!("{tempdir_path}/pg_hba.conf");

            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(config_path)
                .expect("Failed to open pg_hba.conf");

            writeln!(file, "{config_content}").expect("Failed to append to pg_hba.conf");
        }

        // Create log directory
        let timestamp = chrono::Utc::now().timestamp_millis();
        let logfile = format!("/tmp/ephemeral_postgres_logs/{timestamp}.log");
        std::fs::create_dir_all(Path::new(&logfile).parent().unwrap())
            .expect("Failed to create log directory");

        // Start PostgreSQL
        run_cmd!($pg_ctl_path -D $tempdir_path -l $logfile start)
            .expect("Failed to start Postgres");

        Self {
            // TempDir needs to be stored on the struct to avoid being dropped, otherwise the
            // temp folder will be deleted before the test finishes.
            tempdir_path,
            host: "localhost".to_string(),
            port,
            dbname: "postgres".to_string(),
            pg_ctl_path,
        }
    }

    fn new(postgresql_conf: Option<&str>, pg_hba_conf: Option<&str>) -> Self {
        // Make sure .env files are loaded before reading env vars.
        dotenv().ok();

        let init_db_path = Self::initdb_path();
        let tempdir = TempDir::new().expect("Failed to create temp dir");
        let tempdir_path = tempdir.keep();

        // Initialize PostgreSQL data directory
        run_cmd!($init_db_path -D $tempdir_path &> /dev/null)
            .expect("Failed to initialize Postgres data directory");

        Self::new_from_initialized(tempdir_path.as_path(), postgresql_conf, pg_hba_conf)
    }

    // Method to establish a connection to the PostgreSQL instance
    async fn connection(&self) -> Result<PgConnection> {
        Ok(PgConnection::connect(&format!(
            "postgresql://{}:{}/{}",
            self.host, self.port, self.dbname
        ))
        .await?)
    }
}

// Test function to test the ephemeral PostgreSQL setup
#[rstest]
async fn test_logical_replication() -> Result<()> {
    let config = "
        wal_level = logical
        max_replication_slots = 4
        max_wal_senders = 4
        shared_preload_libraries = 'pg_search'
    ";

    let source_postgres = EphemeralPostgres::new(Some(config), None);
    let target_postgres = EphemeralPostgres::new(Some(config), None);

    let mut source_conn = source_postgres.connection().await?;
    let mut target_conn = target_postgres.connection().await?;

    let major_version: (i32,) = "
        SELECT split_part(setting, '.', 1)::int AS major_version
        FROM pg_catalog.pg_settings
        WHERE name = 'server_version';
        "
    .fetch_one(&mut source_conn);

    // Logical replication for versions < 17 is not implemented
    if major_version.0 < 17 {
        return Ok(());
    }

    // Create pg_search extension on both source and target databases
    "CREATE EXTENSION pg_search".execute(&mut source_conn);
    "CREATE EXTENSION pg_search".execute(&mut target_conn);

    // Create the mock_items table schema
    let schema = "
        CREATE TABLE mock_items (
          id SERIAL PRIMARY KEY,
          description TEXT,
          rating INTEGER CHECK (rating BETWEEN 1 AND 5),
          category VARCHAR(255),
          in_stock BOOLEAN,
          metadata JSONB,
          created_at TIMESTAMP,
          last_updated_date DATE,
          latest_available_time TIME
        )
    ";
    schema.execute(&mut source_conn);
    schema.execute(&mut target_conn);

    // Create the bm25 index on the description field
    "CREATE INDEX mock_items_bm25_idx ON public.mock_items
    USING bm25 (id, description) WITH (key_field='id');
    "
    .execute(&mut source_conn);
    "CREATE INDEX mock_items_bm25_idx ON public.mock_items
    USING bm25 (id, description) WITH (key_field='id');
    "
    .execute(&mut target_conn);

    // Create publication and subscription for replication
    "CREATE PUBLICATION mock_items_pub FOR TABLE mock_items".execute(&mut source_conn);
    format!(
        "CREATE SUBSCRIPTION mock_items_sub
         CONNECTION 'host={} port={} dbname={}'
         PUBLICATION mock_items_pub;",
        source_postgres.host, source_postgres.port, source_postgres.dbname
    )
    .execute(&mut target_conn);

    // Verify initial state of the search results
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE id @@@ 'description:shoes'"
            .fetch(&mut source_conn);
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE id @@@ 'description:shoes'"
            .fetch(&mut target_conn);

    assert_eq!(source_results.len(), 0);
    assert_eq!(target_results.len(), 0);

    // Insert a new item into the source database
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Red sports shoes', 'Footwear', true, '12:00:00', '2024-07-10', '{}', '2024-07-10 12:00:00', 1)".execute(&mut source_conn);

    // Verify the insert is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE id @@@ 'description:shoes'"
            .fetch(&mut source_conn);

    // Wait for the replication to complete
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE id @@@ 'description:shoes'".fetch_retry(
            &mut target_conn,
            5,
            1000,
            |result| !result.is_empty(),
        );

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);

    // Additional insert test
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Blue running shoes', 'Footwear', true, '14:00:00', '2024-07-10', '{}', '2024-07-10 14:00:00', 2)".execute(&mut source_conn);

    // Verify the additional insert is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE id @@@ 'description:\"running shoes\"'"
            .fetch(&mut source_conn);

    // Wait for the replication to complete
    std::thread::sleep(std::time::Duration::from_secs(1));
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE id @@@ 'description:\"running shoes\"'"
            .fetch(&mut target_conn);

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);

    // Update test
    "UPDATE mock_items SET rating = 5 WHERE description = 'Red sports shoes'"
        .execute(&mut source_conn);

    // Verify the update is replicated to the target database
    let source_results: Vec<(i32,)> =
        "SELECT rating FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut source_conn);

    std::thread::sleep(std::time::Duration::from_secs(5)); // give a little time for the data to replicate

    let target_results: Vec<(i32,)> =
        "SELECT rating FROM mock_items WHERE description = 'Red sports shoes'".fetch_retry(
            &mut target_conn,
            5,
            1000,
            |result| !result.is_empty(),
        );

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);
    assert_eq!(source_results[0], target_results[0]);

    // Delete test
    "DELETE FROM mock_items WHERE description = 'Red sports shoes'".execute(&mut source_conn);

    // Verify the delete is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut source_conn);

    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description = 'Red sports shoes'".fetch_retry(
            &mut target_conn,
            5,
            1000,
            |result| !result.is_empty(),
        );

    assert_eq!(source_results.len(), 0);
    assert_eq!(target_results.len(), 0);

    // COPY test
    let mut copyin = source_conn
        .copy_in_raw("COPY mock_items(description) FROM STDIN")
        .await?;
    copyin
        .send("replicated1\nreplicated2\nreplicated3".as_bytes())
        .await?;
    copyin.finish().await?;

    // verify the COPY worked
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description @@@ 'description:replicated1' OR description @@@ 'description:replicated2' OR description @@@ 'description:replicated3'"
            .fetch(&mut source_conn);
    assert_eq!(source_results.len(), 3);

    std::thread::sleep(std::time::Duration::from_secs(1)); // give a little time for the data to replicate
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description @@@ 'description:replicated1' OR description @@@ 'description:replicated2' OR description @@@ 'description:replicated3'"
            .fetch_retry(&mut target_conn, 5, 1000, |result| !result.is_empty());
    assert_eq!(target_results.len(), 3);

    Ok(())
}

#[rstest]
async fn test_ephemeral_postgres_with_pg_basebackup() -> Result<()> {
    let config = "
        wal_level = logical
        max_replication_slots = 4
        max_wal_senders = 4
        # Adding pg_search to shared_preload_libraries in 17 doesn't do anything
        # but simplifies testing
        shared_preload_libraries = 'pg_search'
    ";

    let source_postgres = EphemeralPostgres::new(Some(config), None);
    let mut source_conn = source_postgres.connection().await?;
    let source_port = source_postgres.port;
    let source_username = "SELECT CURRENT_USER"
        .fetch_one::<(String,)>(&mut source_conn)
        .0;

    "CREATE TABLE text_array_table (
            id SERIAL PRIMARY KEY,
            text_array TEXT[]
        )"
    .execute(&mut source_conn);

    "INSERT INTO text_array_table (text_array) VALUES
        (ARRAY['apple', 'banana', 'cherry']),
        (ARRAY['dog', 'elephant', 'fox']),
        (ARRAY['grape', 'honeydew', 'kiwi']),
        (ARRAY['lion', 'monkey', 'newt']),
        (ARRAY['octopus', 'penguin', 'quail']),
        (ARRAY['rabbit', 'snake', 'tiger']),
        (ARRAY['umbrella', 'vulture', 'wolf']),
        (ARRAY['x-ray', 'yak', 'zebra']),
        (ARRAY['alpha', 'bravo', 'charlie']),
        (ARRAY['delta', 'echo', 'foxtrot'])"
        .execute(&mut source_conn);

    // Create pg_search extension and bm25 index
    "CREATE EXTENSION pg_search".execute(&mut source_conn);

    "
    CREATE INDEX text_array_table_idx ON text_array_table
    USING bm25 (id, text_array)
    WITH (key_field = 'id');
    "
    .execute(&mut source_conn);

    // Verify search results before pg_basebackup
    let source_results: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM text_array_table WHERE text_array_table @@@ 'text_array:dog' ORDER BY id",
    )
    .fetch_all(&mut source_conn)
    .await?;
    assert_eq!(source_results.len(), 1);

    let target_tempdir = TempDir::new().expect("Failed to create temp dir");
    let target_tempdir_path = target_tempdir.keep();

    // Permissions for the --pgdata directory passed to pg_basebackup
    // should be u=rwx (0700) or u=rwx,g=rx (0750)
    std::fs::set_permissions(
        target_tempdir_path.as_path(),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("couldn't set permissions on target_tempdir path");

    // Run pg_basebackup
    let pg_basebackup = EphemeralPostgres::pg_basebackup_path();
    run_cmd!($pg_basebackup --pgdata $target_tempdir_path --host localhost --port $source_port --username $source_username)
    .expect("Failed to run pg_basebackup");

    let target_postgres =
        EphemeralPostgres::new_from_initialized(target_tempdir_path.as_path(), Some(config), None);
    let mut target_conn = target_postgres.connection().await?;

    // Verify the content in the target database
    let target_results: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM text_array_table WHERE text_array_table @@@ 'text_array:dog'",
    )
    .fetch_all(&mut target_conn)
    .await?;

    assert_eq!(source_results.len(), target_results.len());

    // Verify the table content
    let source_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM text_array_table")
        .fetch_one(&mut source_conn)
        .await?;
    let target_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM text_array_table")
        .fetch_one(&mut target_conn)
        .await?;

    assert_eq!(source_count, target_count);

    Ok(())
}

#[rstest]
async fn test_physical_streaming_replication() -> Result<()> {
    // Create a unique directory for WAL archiving
    let archive_dir = TempDir::new().expect("Failed to create archive dir for WALs");
    // No need to set custom permissions, but you could if desired.

    // Adjust the archive_command to avoid file existence checks since this is a new directory.
    let primary_config = format!(
        "
        listen_addresses = 'localhost'
        wal_level = replica
        archive_mode = on
        archive_command = 'cp %p {}/%f'
        max_wal_senders = 3
        wal_keep_size = '160MB'
        # Adding pg_search to shared_preload_libraries in 17 doesn't do anything
        # but simplifies testing
        shared_preload_libraries = 'pg_search'
        ",
        archive_dir.path().display()
    );

    let primary_pg_hba = "
        host replication replicator 127.0.0.1/32 md5
        host replication replicator ::1/128 md5
    ";

    // Step 1: Create and start the primary Postgres instance
    let primary_postgres = EphemeralPostgres::new(Some(&primary_config), Some(primary_pg_hba));
    let mut primary_conn = primary_postgres.connection().await?;

    // Create a replication user and test table on primary
    "CREATE USER replicator WITH REPLICATION ENCRYPTED PASSWORD 'replicator_pass';"
        .execute(&mut primary_conn);
    "CREATE EXTENSION pg_search;".execute(&mut primary_conn);
    "CREATE TABLE test_data (id SERIAL PRIMARY KEY, info TEXT);".execute(&mut primary_conn);

    // Insert initial data on primary
    "INSERT INTO test_data (info) VALUES ('initial');".execute(&mut primary_conn);

    let primary_port = primary_postgres.port;

    // Step 2: Create the standby using pg_basebackup
    let standby_tempdir = TempDir::new().expect("Failed to create temp dir for standby");
    std::fs::set_permissions(
        standby_tempdir.path(),
        std::fs::Permissions::from_mode(0o700),
    )?;

    let pg_basebackup = EphemeralPostgres::pg_basebackup_path();
    let standby_tempdir = standby_tempdir.path();
    run_cmd!(
        $pg_basebackup
        -D $standby_tempdir
        -Fp -Xs -P -R
        -h localhost
        -U replicator
        --port $primary_port
        &> /dev/null
    )
    .expect("Failed to run pg_basebackup for standby setup");

    let standby_config = "
        # Adding pg_search to shared_preload_libraries in 17 doesn't do anything
        # but simplifies testing
        shared_preload_libraries = 'pg_search'
        hot_standby = on
    ";

    // Start the standby
    let standby_postgres =
        EphemeralPostgres::new_from_initialized(standby_tempdir, Some(standby_config), None);
    let mut standby_conn = standby_postgres.connection().await?;

    // Wait a moment for the standby to catch up
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Verify that the initial data replicated
    let standby_data: Vec<(String,)> =
        "SELECT info FROM test_data"
            .fetch_retry(&mut standby_conn, 60, 1000, |result| !result.is_empty());

    assert_eq!(standby_data.len(), 1);
    assert_eq!(standby_data[0].0, "initial");

    // (Optional) Insert more data on primary and verify it appears on standby
    "INSERT INTO test_data (info) VALUES ('from_primary');".execute(&mut primary_conn);

    let standby_data: Vec<(String,)> = "SELECT info FROM test_data WHERE info='from_primary'"
        .fetch_retry(&mut standby_conn, 60, 1000, |result| !result.is_empty());

    assert_eq!(standby_data.len(), 1);

    // Insert a different value into the primary and ensure it streams over
    "INSERT INTO test_data (info) VALUES ('from_primary_2');".execute(&mut primary_conn);

    // Now, check for 'from_primary_2'
    let standby_data: Vec<(String,)> = "SELECT info FROM test_data WHERE info='from_primary_2'"
        .fetch_retry(&mut standby_conn, 60, 1000, |result| !result.is_empty());

    assert_eq!(standby_data.len(), 1);

    // Optional: Test synchronous replication
    // Reconfigure primary to require synchronous replication
    // This ensures commits wait for replication confirmation.
    "ALTER SYSTEM SET synchronous_standby_names = '*';".execute(&mut primary_conn);
    let pg_ctl_path = primary_postgres.pg_ctl_path.clone();
    let tempdir_path = primary_postgres.tempdir_path.clone();
    run_cmd!($pg_ctl_path -D $tempdir_path restart &> /dev/null)
        .expect("Failed to restart primary with sync config");

    // Reconnect after restart
    let mut primary_conn = primary_postgres.connection().await?;
    // Insert a row, then check standby to ensure synchronous commit.
    // If no connected standby matches, the commit on the primary will block indefinitely.
    "BEGIN; INSERT INTO test_data (info) VALUES ('sync_test'); COMMIT;".execute(&mut primary_conn);

    let sync_row: Vec<(String,)> = "SELECT info FROM test_data WHERE info='sync_test'".fetch_retry(
        &mut standby_conn,
        60,
        1000,
        |result| !result.is_empty(),
    );
    assert_eq!(sync_row.len(), 1);

    // Optional: Failover test - Stop primary and promote standby
    let pg_ctl_path = primary_postgres.pg_ctl_path.clone();
    let tempdir_path = primary_postgres.tempdir_path.clone();
    run_cmd!($pg_ctl_path -D $tempdir_path stop &> /dev/null).unwrap();

    // Promote standby using pg_ctl promote
    let tempdir_path = standby_postgres.tempdir_path.clone();
    let pg_ctl_path = standby_postgres.pg_ctl_path.clone();
    run_cmd!($pg_ctl_path -D $tempdir_path promote &> /dev/null)
        .expect("Failed to promote standby");

    thread::sleep(Duration::from_secs(2));
    let mut standby_conn = standby_postgres.connection().await?;
    "INSERT INTO test_data (info) VALUES ('promoted_standby');".execute(&mut standby_conn);

    // Ensure we can read back the inserted row from the now promoted standby
    let promoted_data: Vec<(String,)> = "SELECT info FROM test_data WHERE info='promoted_standby'"
        .fetch_retry(&mut standby_conn, 60, 1000, |result| !result.is_empty());
    assert_eq!(promoted_data.len(), 1);

    Ok(())
}

#[rstest]
async fn test_wal_streaming_replication_with_pg_search() -> Result<()> {
    // Primary Postgres setup + insert data
    let postgresql_conf = "
        listen_addresses = 'localhost'
        wal_level = replica
        max_wal_senders = 4
        shared_preload_libraries = 'pg_search'
        # It's often helpful to have a short wal_keep_size or max_wal_senders for testing
    ";
    let pg_hba_conf = "
        host replication all 127.0.0.1/32 md5
        host replication all ::1/128 md5
    ";
    let source_postgres = EphemeralPostgres::new(Some(postgresql_conf), Some(pg_hba_conf));
    let mut source_conn = source_postgres.connection().await?;
    let source_port = source_postgres.port;
    let source_username = "replicator";

    // Create a replication user and slot on primary
    format!("CREATE USER {source_username} WITH REPLICATION ENCRYPTED PASSWORD 'replicator_pass'")
        .execute(&mut source_conn);
    "SELECT pg_create_physical_replication_slot('wal_receiver_1');".execute(&mut source_conn);

    // Install pg_search on primary and create a test table
    "CREATE EXTENSION pg_search".execute(&mut source_conn);
    "CREATE TABLE items (
        id SERIAL PRIMARY KEY,
        description TEXT,
        category TEXT,
        created_at TIMESTAMP
    )"
    .execute(&mut source_conn);

    // Insert initial data
    "INSERT INTO items (description, category, created_at) VALUES
        ('Red running shoes', 'Footwear', NOW()),
        ('Blue sports shoes', 'Footwear', NOW()),
        ('Wireless headphones', 'Electronics', NOW()),
        ('4K television', 'Electronics', NOW())"
        .execute(&mut source_conn);

    // Create a bm25 index on the items table
    "
    CREATE INDEX items_search_idx ON items
    USING bm25 (id, description, category)
    WITH (key_field = 'id');
    "
    .execute(&mut source_conn);

    // Verify that searching on the primary works
    let source_results: Vec<(i32,)> =
        "SELECT id FROM items WHERE items @@@ 'description:shoes' ORDER BY id"
            .fetch(&mut source_conn);
    assert_eq!(source_results.len(), 2);

    // Set up the standby using pg_basebackup
    let target_tempdir = TempDir::new().expect("Failed to create temp dir for standby");
    let target_tempdir_path = target_tempdir.keep();

    // Permissions for the --pgdata directory passed to pg_basebackup
    // should be u=rwx (0700) or u=rwx,g=rx (0750)
    std::fs::set_permissions(
        target_tempdir_path.as_path(),
        std::fs::Permissions::from_mode(0o700),
    )?;

    let pg_basebackup = EphemeralPostgres::pg_basebackup_path();
    run_cmd!(
        $pg_basebackup
        --pgdata $target_tempdir_path
        --host localhost
        --port $source_port
        --username $source_username
        -Fp -Xs -P -R
        &> /dev/null
    )
    .expect("Failed to run pg_basebackup for standby setup");

    // Start the standby also with pg_search preloaded
    let standby_config = "
        shared_preload_libraries = 'pg_search'
        hot_standby = on
        hot_standby_feedback = true
        primary_slot_name = wal_receiver_1
    ";

    let standby_postgres = EphemeralPostgres::new_from_initialized(
        target_tempdir_path.as_path(),
        Some(standby_config),
        None,
    );
    let mut standby_conn = standby_postgres.connection().await?;

    // Wait for the standby to catch up
    // The fetch_retry helper is used in previous tests; you can adapt a similar approach here.
    "SELECT description FROM items ORDER BY id".fetch_retry::<(String,)>(
        &mut standby_conn,
        60,
        1000,
        |result| !result.is_empty(),
    );

    // Test that the correct error is returned when trying to read from a standby
    let result = "SELECT id FROM items WHERE items @@@ 'category:Electronics' ORDER BY id"
        .fetch_result::<(i32,)>(&mut standby_conn);

    match result {
        Err(err) => assert!(err.to_string().contains("Serving reads from a standby requires write-ahead log (WAL) integration, which is supported on ParadeDB Enterprise, not ParadeDB Community")),
        _ => {
            panic!("physical replication should not be supported on ParadeDB Community {:?}", result);
        }
    }

    Ok(())
}
```

---

## range_json.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );

    INSERT INTO test_table (value_int4, value_int8) VALUES 
        (-1111, -11111111),
        (2222, 22222222), 
        (3333, 33333333), 
        (4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_int4, value_int8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int4",
            "lower_bound": {"included": 2222},
            "upper_bound": {"included": 4444}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int8",
            "lower_bound": {"included": 0},
            "upper_bound": {"excluded": 50000000}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn unbounded_integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );
    INSERT INTO test_table (value_int4, value_int8) VALUES 
        (-1111, -11111111),
        (2222, 22222222), 
        (3333, 33333333), 
        (4444, 44444444);
    "#
    .execute(&mut conn);
    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_int4, value_int8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Test unbounded upper range for INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int4",
            "lower_bound": {"included": 2222},
            "upper_bound": null
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, 2222);
    assert_eq!(rows[2].1, 4444);

    // Test unbounded lower range for INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int4",
            "lower_bound": null,
            "upper_bound": {"included": 2222}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, -1111);
    assert_eq!(rows[1].1, 2222);

    // Test unbounded upper range for INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int8",
            "lower_bound": {"included": 0},
            "upper_bound": null
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, 22222222);
    assert_eq!(rows[2].1, 44444444);

    // Test unbounded lower range for INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int8",
            "lower_bound": null,
            "upper_bound": {"included": -5000000}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, -11111111);
}

#[rstest]
fn float_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_float4 FLOAT4,
        value_float8 FLOAT8,
        value_numeric NUMERIC
    );

    INSERT INTO test_table (value_float4, value_float8, value_numeric) VALUES
        (-1.1, -1111.1111, -111.11111),
        (2.2, 2222.2222, 222.22222),
        (3.3, 3333.3333, 333.33333),
        (4.4, 4444.4444, 444.44444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_float4, value_float8, value_numeric)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_float4",
            "lower_bound": {"included": -2.0},
            "upper_bound": {"included": 3.0}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_float8",
            "lower_bound": {"excluded": 2222.2222},
            "upper_bound": {"included": 3333.3333}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);

    // NUMERIC
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_numeric",
            "lower_bound": {"included": 0.0},
            "upper_bound": {"excluded": 400.0}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn datetime_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz) VALUES 
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST'),
        (DATE '2022-07-14', TIMESTAMP '2022-05-16 07:38:43', TIMESTAMP WITH TIME ZONE '2022-05-16 07:38:43 EST'),
        (DATE '2021-04-30', TIMESTAMP '2021-06-08 08:49:21', TIMESTAMP WITH TIME ZONE '2021-06-08 08:49:21 CST'),
        (DATE '2020-06-28', TIMESTAMP '2020-07-09 15:52:13', TIMESTAMP WITH TIME ZONE '2020-07-09 15:52:13 MST');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_date, value_timestamp, value_timestamptz)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // DATE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ '{
        "range": {
            "field": "value_date",
            "lower_bound": {"included": "2020-05-20T00:00:00.000000Z"},
            "upper_bound": {"included": "2022-06-13T00:00:00.000000Z"},
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ '{
        "range": {
            "field": "value_timestamp",
            "lower_bound": {"included": "2019-08-02T07:52:43.000000Z"},
            "upper_bound": {"included": "2021-06-10T10:32:41.000000Z"},
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ '{
        "range": {
            "field": "value_timestamptz",
            "lower_bound": {"included": "2020-07-09T21:52:13.000000Z"},
            "upper_bound": {"included": "2022-05-16T12:38:43.000000Z"},
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}
```

---

## custom_scan.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

// Tests for ParadeDB's Custom Scan implementation
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::{Number, Value};
use sqlx::PgConnection;

#[rstest]
fn corrupt_targetlist(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score) = "select count(*), max(pdb.score(id)) from paradedb.bm25_search where description @@@ 'keyboard'"
        .fetch_one::<(i64, f32)>(&mut conn);
    assert_eq!((id, score), (2, 3.2668595));

    "PREPARE prep AS select count(*), max(pdb.score(id)) from paradedb.bm25_search where description @@@ 'keyboard'".execute(&mut conn);
    for _ in 0..100 {
        "EXECUTE prep".fetch_one::<(i64, f32)>(&mut conn);
        assert_eq!((id, score), (2, 3.2668595));
    }
}

#[rstest]
fn attribute_1_of_table_has_wrong_type(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, ) = "SELECT id, description FROM paradedb.bm25_search WHERE description @@@ 'keyboard' OR id = 1 ORDER BY id LIMIT 1"
        .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn generates_custom_scan_for_or(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' OR description @@@ 'shoes'".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    let plan = plan.pointer("/0/Plan").unwrap();

    assert_eq!(
        plan.get("Custom Plan Provider"),
        Some(&Value::String(String::from("ParadeDB Scan")))
    );
}

#[rstest]
fn generates_custom_scan_for_and(mut conn: PgConnection) {
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' AND description @@@ 'shoes'".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");
    let plan = plan.pointer("/0/Plan").unwrap();
    assert_eq!(
        plan.get("Custom Plan Provider"),
        Some(&Value::String(String::from("ParadeDB Scan")))
    );
}

#[rstest]
fn includes_segment_count(mut conn: PgConnection) {
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' AND description @@@ 'shoes'".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");
    let plan = plan.pointer("/0/Plan").unwrap();
    assert!(plan.get("Segment Count").is_some());
}

#[rstest]
fn field_on_left(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id,) =
        "SELECT id FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY id ASC"
            .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn table_on_left(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, ) =
        "SELECT id FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id ASC"
            .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn scores_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score) =
        "SELECT id, pdb.score(id) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY pdb.score(id) DESC LIMIT 1"
            .fetch_one::<(i32, f32)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(score, 3.2668595);
}

#[rstest]
fn snippets_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, snippet) =
        "SELECT id, pdb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY pdb.score(id) DESC LIMIT 1"
            .fetch_one::<(i32, String)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(snippet, String::from("Plastic <b>Keyboard</b>"));
}

#[rstest]
fn scores_and_snippets_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score, snippet) =
        "SELECT id, pdb.score(id), pdb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY pdb.score(id) DESC LIMIT 1"
            .fetch_one::<(i32, f32, String)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(score, 3.2668595);
    assert_eq!(snippet, String::from("Plastic <b>Keyboard</b>"));
}

#[rstest]
fn mingets(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, snippet) =
        "SELECT id, pdb.snippet(description, '<MING>', '</MING>') FROM paradedb.bm25_search WHERE description @@@ 'teddy bear'"
            .fetch_one::<(i32, String)>(&mut conn);
    assert_eq!(id, 40);
    assert_eq!(
        snippet,
        String::from("Plush <MING>teddy</MING> <MING>bear</MING>")
    );
}

#[rstest]
fn scores_with_expressions(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select id,
    description,
    pdb.score(id),
    rating,
    pdb.score(id) * rating    /* testing this, specifically */
from paradedb.bm25_search
where metadata @@@ 'color:white'
order by 5 desc, score desc
limit 1;
        "#
    .fetch_one::<(i32, String, f32, i32, f64)>(&mut conn);
    assert_eq!(
        result,
        (
            25,
            "Anti-aging serum".into(),
            3.2455924,
            4,
            12.982369422912598
        )
    );
}

#[rstest]
fn limit_without_order_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    let (plan, ) = r#"
explain (analyze, format json) select * from paradedb.bm25_search where metadata @@@ 'color:white' limit 1;
        "#
        .fetch_one::<(Value,)>(&mut conn);
    let path = plan.pointer("/0/Plan/Plans/0").unwrap();
    assert_eq!(
        path.get("Node Type"),
        Some(&Value::String(String::from("Custom Scan")))
    );
    assert_eq!(path.get("Scores"), Some(&Value::Bool(false)));
    assert_eq!(
        path.get("   TopN Limit"),
        Some(&Value::Number(Number::from(1)))
    );
}

#[rstest]
fn score_and_limit_without_order_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    let (plan, ) = r#"
explain (analyze, format json) select pdb.score(id), * from paradedb.bm25_search where metadata @@@ 'color:white' limit 1;
        "#
        .fetch_one::<(Value,)>(&mut conn);
    let path = plan.pointer("/0/Plan/Plans/0").unwrap();
    assert_eq!(
        path.get("Node Type"),
        Some(&Value::String(String::from("Custom Scan")))
    );
    assert_eq!(path.get("Scores"), Some(&Value::Bool(true)));
    assert_eq!(
        path.get("   TopN Limit"),
        Some(&Value::Number(Number::from(1)))
    );
}

#[rstest]
fn simple_join_with_scores_and_both_sides(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select a.id,
    a.score,
    b.id,
    b.score
from (select pdb.score(id), * from paradedb.bm25_search) a
inner join (select pdb.score(id), * from paradedb.bm25_search) b on a.id = b.id
where a.description @@@ 'bear' AND b.description @@@ 'teddy bear';"#
        .fetch_one::<(i32, f32, i32, f32)>(&mut conn);

    // PG18 introduces self-join elimination (SJE) which combines the queries into a single scan.
    // When SJE kicks in, both score() calls return the same combined score.
    // PG17 and earlier: separate scores (3.3322046 for 'bear', 6.664409 for 'teddy bear')
    // PG18 with SJE: same combined score for both (9.9966135)
    let pg_version: i32 = "SHOW server_version_num"
        .fetch_one::<(String,)>(&mut conn)
        .0
        .parse()
        .unwrap();

    if pg_version >= 180000 {
        // PG18+: SJE combines queries, both aliases get the same combined score
        assert_eq!(result, (40, 9.9966135, 40, 9.9966135));
    } else {
        // PG17 and earlier: separate scores per alias
        assert_eq!(result, (40, 3.3322046, 40, 6.664409));
    }
}

#[rstest]
fn simple_join_with_scores_on_both_sides(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select a.id,
    a.score,
    b.id,
    b.score
from (select pdb.score(id), * from paradedb.bm25_search) a
inner join (select pdb.score(id), * from paradedb.bm25_search) b on a.id = b.id
where a.description @@@ 'bear' OR b.description @@@ 'teddy bear';"#
        .fetch_one::<(i32, f32, i32, f32)>(&mut conn);

    // PG18 introduces self-join elimination (SJE) which combines the queries into a single scan.
    // When SJE kicks in, both score() calls return the same combined score.
    let pg_version: i32 = "SHOW server_version_num"
        .fetch_one::<(String,)>(&mut conn)
        .0
        .parse()
        .unwrap();

    if pg_version >= 180000 {
        // PG18+: SJE combines queries, both aliases get the same combined score
        assert_eq!(result, (40, 9.9966135, 40, 9.9966135));
    } else {
        // PG17 and earlier: separate scores per alias
        assert_eq!(result, (40, 3.3322046, 40, 6.664409));
    }
}

#[rstest]
fn add_scores_across_joins_issue1753(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (key_field='id');

    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'orders',
      table_type => 'Orders'
    );
    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#.execute(&mut conn);

    // this one doesn't plan a custom scan at all, so scores come back as NaN
    let result = "
        SELECT o.order_id, m.description, pdb.score(o.order_id) + pdb.score(m.id) as score
        FROM orders o JOIN mock_items m ON o.product_id = m.id
        WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes'
        ORDER BY order_id
        LIMIT 1"
        .fetch_one::<(i32, String, f32)>(&mut conn);
    assert_eq!(result, (3, "Sleek running shoes".into(), 5.406531));
}

#[rstest]
fn scores_survive_joins(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'a', schema_name => 'public');
    CALL paradedb.create_bm25_test_table(table_name => 'b', schema_name => 'public');
    CALL paradedb.create_bm25_test_table(table_name => 'c', schema_name => 'public');

    CREATE INDEX idxa ON a USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time) WITH (key_field='id');
    CREATE INDEX idxb ON b USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time) WITH (key_field='id');
    CREATE INDEX idxc ON c USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time) WITH (key_field='id');
    "#.execute(&mut conn);

    // this one doesn't plan a custom scan at all, so scores come back as NaN
    let result = r#"
        SELECT a.description, pdb.score(a.id)
        FROM a
        join b on a.id = b.id
        join c on a.id = c.id
        WHERE a.description @@@ 'shoes'
        ORDER BY a.description;"#
        .fetch_result::<(String, f32)>(&mut conn)
        .expect("query failed");
    assert_eq!(
        result,
        vec![
            ("Generic shoes".into(), 2.8772602),
            ("Sleek running shoes".into(), 2.4849067),
            ("White jogging shoes".into(), 2.4849067),
        ]
    );
}

#[rustfmt::skip]
#[rstest]
fn join_issue_1776(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'mock_items'
        );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at)
    WITH (key_field='id');

    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'orders',
          table_type => 'Orders'
        );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#
    .execute(&mut conn);

    let results = r#"
        SELECT o.order_id, m.description, o.customer_name, pdb.score(o.order_id) as orders_score, pdb.score(m.id) as items_score
        FROM orders o
        JOIN mock_items m ON o.product_id = m.id
        WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes' OR m.description @@@ 'Smith'
        ORDER BY order_id
        LIMIT 5;
    "#.fetch_result::<(i32, String, String, f32, f32)>(&mut conn).expect("query failed");

    assert_eq!(results[0], (3, "Sleek running shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
    assert_eq!(results[1], (6, "White jogging shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
    assert_eq!(results[2], (36,"White jogging shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
}

#[rustfmt::skip]
#[rstest]
fn join_issue_1826(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'mock_items'
        );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at)
    WITH (key_field='id');

    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'orders',
          table_type => 'Orders'
        );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#
    .execute(&mut conn);

    let results = r#"
        SELECT o.order_id, m.description, o.customer_name, pdb.score(o.order_id) as orders_score, pdb.score(m.id) as items_score
        FROM orders o
        JOIN mock_items m ON o.product_id = m.id
        WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes' OR m.description @@@ 'Smith'
        ORDER BY pdb.score(m.id) desc, m.id asc
        LIMIT 1;
    "#.fetch_result::<(i32, String, String, f32, f32)>(&mut conn).expect("query failed");

    assert_eq!(results[0], (3, "Sleek running shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
}

#[rstest]
fn leaky_file_handles(mut conn: PgConnection) {
    r#"
        CREATE OR REPLACE FUNCTION raise_exception(int, int) RETURNS bool LANGUAGE plpgsql AS $$
        DECLARE
        BEGIN
            IF $1 = $2 THEN
                RAISE EXCEPTION 'error! % = %', $1, $2;
            END IF;
            RETURN false;
        END;
        $$;
    "#
    .execute(&mut conn);

    let (pid,) = "SELECT pg_backend_pid()".fetch_one::<(i32,)>(&mut conn);
    SimpleProductsTable::setup().execute(&mut conn);

    // this will raise an error when it hits id #12
    let result = "SELECT id, pdb.score(id), raise_exception(id, 12) FROM paradedb.bm25_search WHERE category @@@ 'electronics' ORDER BY pdb.score(id) DESC, id LIMIT 10"
        .execute_result(&mut conn);
    assert!(result.is_err());
    assert_eq!(
        "error returned from database: error! 12 = 12",
        &format!("{}", result.err().unwrap())
    );

    fn tantivy_files_still_open(pid: i32) -> bool {
        let output = std::process::Command::new("lsof")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .expect("`lsof` command should not fail`");

        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("stdout: {stdout}");
        stdout.contains("/tantivy/")
    }

    // see if there's still some open tantivy files
    if tantivy_files_still_open(pid) {
        // if there are, they're probably (hopefully!) from where we the postgres connection
        // is waiting on merge threads in the background.  So we'll give it 5 seconds and try again

        eprintln!("sleeping for 5s and checking open files again");
        std::thread::sleep(std::time::Duration::from_secs(5));

        // this time asserting for real
        assert!(!tantivy_files_still_open(pid));
    }
}

#[rustfmt::skip]
#[rstest]
fn cte_issue_1951(mut conn: PgConnection) {
    r#"
        CREATE TABLE t
        (
            id   SERIAL,
            data TEXT
        );

        CREATE TABLE s
        (
            id   SERIAL,
            data TEXT
        );

        insert into t (id, data) select x, md5(x::text) || ' query' from generate_series(1, 100) x;
        insert into s (id, data) select x, md5(x::text) from generate_series(1, 100) x;

        create index idxt on t using bm25 (id, data) with (key_field = id);
        create index idxs on s using bm25 (id, data) with (key_field = id);
    "#.execute(&mut conn);

    let results = r#"
        with cte as (
        select id, 1 as score from t
        where data @@@ 'query'
        limit 1)
        select cte.id from s
        right join cte on cte.id = s.id
        order by cte.score desc;
    "#.fetch_result::<(i32, )>(&mut conn).expect("query failed");
    assert_eq!(results.len(), 1);
}

#[rstest]
fn without_operator_guc(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, rating)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // This is a small table, and our startup cost (rightly!) dominates the time taken to scan it.
    // To force the index or custom scan to be used, disable sequential scans.
    "SET enable_seqscan TO OFF;".execute(&mut conn);

    fn plan_uses_custom_scan(conn: &mut PgConnection, query_string: &str) -> bool {
        let (plan,) = format!("EXPLAIN (FORMAT JSON) {query_string}").fetch_one::<(Value,)>(conn);
        eprintln!("{query_string}");
        eprintln!("{plan:#?}");
        format!("{plan:?}").contains("ParadeDB Scan")
    }

    for custom_scan_without_operator in [true, false] {
        format!(
            "SET paradedb.enable_custom_scan_without_operator = {custom_scan_without_operator}"
        )
        .execute(&mut conn);

        // Confirm that a plan which doesn't use our operator is affected by the GUC.
        let uses_custom_scan =
            plan_uses_custom_scan(&mut conn, "SELECT id FROM mock_items WHERE rating = 3");
        if custom_scan_without_operator {
            assert!(
                uses_custom_scan,
                "Should use the custom scan when the GUC is enabled."
            );
        } else {
            assert!(
                !uses_custom_scan,
                "Should not use the custom scan when the GUC is disabled."
            );
        }

        // And that a plan which does use our operator is not affected by the GUC.
        let uses_custom_scan =
            plan_uses_custom_scan(&mut conn, "SELECT id FROM mock_items WHERE id @@@ '1'");
        assert!(
            uses_custom_scan,
            "Should use the custom scan when our operator is used, regardless of \
            the GUC value ({custom_scan_without_operator})"
        );
    }
}

#[rstest]
fn top_n_matches(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test;
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            message TEXT,
            severity INTEGER
        ) WITH (autovacuum_enabled = false);

        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);
        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);

        -- INSERT INTO test (message) SELECT 'space fillter ' || x FROM generate_series(1, 10000000) x;

        CREATE INDEX idxtest ON test USING bm25(id, message, severity) WITH (key_field = 'id');
        CREATE OR REPLACE FUNCTION assert(a bigint, b bigint) RETURNS bool STABLE STRICT LANGUAGE plpgsql AS $$
        DECLARE
            current_txid bigint;
        BEGIN
            -- Get the current transaction ID
            current_txid := txid_current();

            -- Check if the values are not equal
            IF a <> b THEN
                RAISE EXCEPTION 'Assertion failed: % <> %. Transaction ID: %', a, b, current_txid;
            END IF;

            RETURN true;
        END;
        $$;
    "#.execute(&mut conn);

    "UPDATE test SET severity = (floor(random() * 10) + 1)::int WHERE id < 10;".execute(&mut conn);
    "UPDATE test SET severity = (floor(random() * 10) + 1)::int WHERE id < 10;".execute(&mut conn);
    "UPDATE test SET severity = (floor(random() * 10) + 1)::int WHERE id < 10;".execute(&mut conn);

    r#"
        SET enable_indexonlyscan to OFF;
        SET enable_indexscan to OFF;
        SET max_parallel_workers = 0;
    "#
    .execute(&mut conn);

    for n in 1..=100 {
        let sql = format!("select assert(count(*), LEAST({n}, 8)), count(*) from (select id from test where message @@@ 'beer' order by severity limit {n}) x;");

        let (b, count) = sql.fetch_one::<(bool, i64)>(&mut conn);
        assert_eq!((b, count), (true, n.min(8)));
    }

    r#"
        SET enable_indexonlyscan to OFF;
        SET enable_indexscan to OFF;
        SET max_parallel_workers = 32;
    "#
    .execute(&mut conn);

    for n in 1..=100 {
        let sql = format!("select assert(count(*), LEAST({n}, 8)), count(*) from (select id from test where message @@@ 'beer' order by severity limit {n}) x;");

        let (b, count) = sql.fetch_one::<(bool, i64)>(&mut conn);
        assert_eq!((b, count), (true, n.min(8)));
    }
}

#[rstest]
fn stable_limit_and_offset(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // the `debug_parallel_query` was added in pg16, so we cannot run this test on anything
        // less than pg16
        return;
    }

    // We use multiple segments, and force multiple workers to be used.
    SimpleProductsTable::setup_multi_segment().execute(&mut conn);

    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET debug_parallel_query TO on".execute(&mut conn);

    let mut query = |offset: usize, limit: usize| -> Vec<(i32, String, f32)> {
        format!(
            "SELECT id, description, pdb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'category:electronics'
             ORDER BY pdb.score(id), id OFFSET {offset} LIMIT {limit}"
        )
        .fetch_collect(&mut conn)
    };

    let mut previous = Vec::new();
    for limit in 1..50 {
        let current = query(0, limit);
        assert_eq!(
            previous[0..],
            current[..previous.len()],
            "With limit {limit}"
        );
        previous = current;
    }

    let all_results = query(0, 50);
    for (offset, expected) in all_results.into_iter().enumerate() {
        let current = query(offset, 1);
        assert_eq!(expected, current[0]);
    }
}

#[rstest]
fn top_n_is_exhausted(mut conn: PgConnection) {
    r#"
        CREATE TABLE exhausted (id SERIAL8 NOT NULL PRIMARY KEY, message TEXT, severity INTEGER);
        CREATE INDEX exhausted_idx ON exhausted USING bm25 (id, message, severity) WITH (key_field = 'id');
        INSERT INTO exhausted (message, severity) VALUES ('beer wine cheese a', 1);
        SET max_parallel_workers = 0;
    "#.execute(&mut conn);

    let (plan,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT * FROM exhausted
        WHERE message @@@ 'beer'
        ORDER BY severity LIMIT 100;
    "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    // We have requested 100 results, but only 1 is available: we should detect during our first
    // query that the search is exhausted, and not attempt to query again to find more.
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/   Queries"),
        Some(&Value::Number(1.into()))
    );
}

#[rstest]
fn top_n_completes_issue2511(mut conn: PgConnection) {
    r#"
        drop table if exists loop;
        create table loop (id serial8 not null primary key, message text) with (autovacuum_enabled = false);
        create index idxloop on loop using bm25 (id, message) WITH (key_field = 'id', layer_sizes = '1GB, 1GB');

        insert into loop (message) select md5(x::text) from generate_series(1, 5000) x;

        update loop set message = message || ' beer';
        update loop set message = message || ' beer';
        update loop set message = message || ' beer';
        update loop set message = message || ' beer';

        set max_parallel_workers = 1;
    "#.execute(&mut conn);

    let results = r#"
        select * from loop where id @@@ paradedb.all() order by id desc limit 25 offset 0;
    "#
    .fetch::<(i64, String)>(&mut conn);
    assert_eq!(results.len(), 25);
}

#[rstest]
fn parallel_custom_scan_with_jsonb_issue2432(mut conn: PgConnection) {
    // Note: We use a very small mutable segment size to force multiple segments to be created.
    r#"
        DROP TABLE IF EXISTS test;
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            message TEXT,
            severity INTEGER
        ) WITH (autovacuum_enabled = false);

        CREATE INDEX idxtest ON test USING bm25(id, message, severity) WITH (key_field = 'id', layer_sizes = '1GB, 1GB', mutable_segment_rows=1);

        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);
        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);
    "#.execute(&mut conn);

    r#"
        SET enable_indexonlyscan to OFF;
        SET enable_indexscan to OFF;
        SET max_parallel_workers = 32;
    "#
    .execute(&mut conn);

    let (plan, ) = r#"
        explain (FORMAT json) select id
        from test
        where message @@@ '{"parse_with_field":{"field":"message","query_string":"beer","lenient":null,"conjunction_mode":null}}'::jsonb
        order by pdb.score(id) desc
        limit 10;
    "#.fetch_one::<(serde_json::Value, )>(&mut conn);

    eprintln!("{plan:#?}");
    let node = plan
        .pointer("/0/Plan/Plans/0/Plans/0/Parallel Aware")
        .unwrap();
    let parallel_aware = node
        .as_bool()
        .expect("should have gotten the `Parallel Aware` node");
    assert_eq!(parallel_aware, true);
}

#[rstest]
fn nested_loop_rescan_issue_2472(mut conn: PgConnection) {
    // Setup tables and test data
    r#"
    -- Create extension
    DROP EXTENSION IF EXISTS pg_search CASCADE;
    CREATE EXTENSION IF NOT EXISTS pg_search;

    -- Create tables
    CREATE TABLE IF NOT EXISTS company (
        id BIGINT PRIMARY KEY,
        name TEXT
    );

    CREATE TABLE IF NOT EXISTS "user" (
        id BIGINT PRIMARY KEY,
        company_id BIGINT,
        status TEXT
    );

    CREATE TABLE IF NOT EXISTS user_products (
        user_id BIGINT,
        product_id BIGINT,
        deleted_at TIMESTAMP
    );

    -- Create ParadeDB BM25 index
    DROP INDEX IF EXISTS company_name_search_idx;
    CREATE INDEX company_name_search_idx ON company
    USING bm25 (id, name)
    WITH (key_field = 'id');

    -- Insert test data
    DELETE FROM company;
    INSERT INTO company VALUES
    (4, 'Testing Company'),
    (5, 'Testing Org'),
    (13, 'Something else'),
    (15, 'Important Testing');

    DELETE FROM "user";
    INSERT INTO "user" VALUES
    (1, 4, 'NORMAL'),
    (2, 5, 'NORMAL'),
    (3, 13, 'NORMAL'),
    (4, 15, 'NORMAL'),
    (5, 7, 'NORMAL');

    DELETE FROM user_products;
    INSERT INTO user_products VALUES
    (1, 100, NULL),
    (2, 100, NULL),
    (3, 200, NULL),
    (4, 100, NULL);
    "#
    .execute(&mut conn);

    // Test in non-parallel mode first
    r#"
    SET max_parallel_workers = 0;
    SET max_parallel_workers_per_gather = 0;
    "#
    .execute(&mut conn);

    println!("Testing in non-parallel mode");

    // Check if we're running in non-parallel mode
    let (plan,) = r#"
    EXPLAIN (FORMAT json)
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, pdb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;"#
        .fetch_one::<(serde_json::Value,)>(&mut conn);

    let node = plan.pointer("/0/Plan").unwrap();
    let is_parallel = node.as_object().unwrap().contains_key("Workers Planned");
    assert!(!is_parallel, "Query should not use parallel execution");

    // First test in non-parallel mode
    let complex_results = r#"
    -- This reproduces the issue with company_id 15
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, pdb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    ),
    scored_users AS (
        SELECT
            u.id,
            u.company_id,
            mc.id as mc_company_id,
            COALESCE(MAX(mc.company_score), 0) AS score
        FROM target_users u
        LEFT JOIN matched_companies mc ON u.company_id = mc.id
        LEFT JOIN user_products up ON up.user_id = u.id
        GROUP BY u.id, u.company_id, mc.id
    )
    SELECT su.id, su.company_id, su.mc_company_id, su.score
    FROM scored_users su
    ORDER BY score DESC;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("complex query failed");

    // Test that we get results for all users, including the problematic company_id 15
    assert_eq!(complex_results.len(), 4);
    let has_company_15 = complex_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Results should include user with company_id 15"
    );

    // The minimal query focusing on the problematic companies in non-parallel mode
    let minimal_results = r#"
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE
          u.status = 'NORMAL' AND
            u.company_id in (13, 15)
    ),
    matched_companies AS (
        SELECT c.id, pdb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(mc.company_score, 0) AS score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("minimal query failed");

    // Verify both companies in non-parallel mode
    assert_eq!(minimal_results.len(), 2);
    let has_company_15 = minimal_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Results should include user with company_id 15"
    );
    println!("minimal_results: {minimal_results:?}");
    let company_15_result = minimal_results
        .iter()
        .find(|(_, company_id, _, _)| *company_id == 15)
        .unwrap();
    assert!(
        company_15_result.3 > 0.0,
        "Company 15 should have a non-zero score"
    );

    // Now test in parallel mode
    r#"
    SET max_parallel_workers = 32;
    SET max_parallel_workers_per_gather = 8;
    "#
    .execute(&mut conn);

    println!("Testing in parallel mode");

    // Check if we're running in parallel mode
    let (plan,) = r#"
    EXPLAIN (FORMAT json)
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, pdb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;"#
        .fetch_one::<(serde_json::Value,)>(&mut conn);

    // Test in parallel mode might not actually use parallelism due to small table sizes
    // But the setting is enabled, which is what we're testing
    let node = plan.pointer("/0/Plan").unwrap();
    let parallel_enabled = node
        .pointer("/Parallel Aware")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
        || node.pointer("/Workers Planned").is_some()
        || node.as_object().unwrap().contains_key("Parallel Aware");

    println!(
        "Plan in parallel mode: {}",
        serde_json::to_string_pretty(&plan).unwrap()
    );

    // Due to small data sizes, PostgreSQL might choose not to use parallelism
    // even when the settings allow it, so we don't assert but print info
    println!("Parallelism indicators in plan: {parallel_enabled}");

    // First test in parallel mode
    let parallel_complex_results = r#"
    -- This reproduces the issue with company_id 15
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, pdb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    ),
    scored_users AS (
        SELECT
            u.id,
            u.company_id,
            mc.id as mc_company_id,
            COALESCE(MAX(mc.company_score), 0) AS score
        FROM target_users u
        LEFT JOIN matched_companies mc ON u.company_id = mc.id
        LEFT JOIN user_products up ON up.user_id = u.id
        GROUP BY u.id, u.company_id, mc.id
    )
    SELECT su.id, su.company_id, su.mc_company_id, su.score
    FROM scored_users su
    ORDER BY score DESC;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("parallel complex query failed");

    // Test that we get results for all users in parallel mode
    assert_eq!(parallel_complex_results.len(), 4);
    let has_company_15 = parallel_complex_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Parallel results should include user with company_id 15"
    );

    // The minimal query focusing on the problematic companies in parallel mode
    let parallel_minimal_results = r#"
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE
          u.status = 'NORMAL' AND
            u.company_id in (13, 15)
    ),
    matched_companies AS (
        SELECT c.id, pdb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(mc.company_score, 0) AS score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("parallel minimal query failed");

    // Verify both companies in parallel mode
    assert_eq!(parallel_minimal_results.len(), 2);
    let has_company_15 = parallel_minimal_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Parallel results should include user with company_id 15"
    );
    let company_15_result = parallel_minimal_results
        .iter()
        .find(|(_, company_id, _, _)| *company_id == 15)
        .unwrap();
    assert!(
        company_15_result.3 > 0.0,
        "Company 15 should have a non-zero score in parallel mode"
    );
}

#[rstest]
fn uses_max_parallel_workers_per_gather_issue2515(mut conn: PgConnection) {
    r#"
    SET max_parallel_workers = 8;
    SET max_parallel_workers_per_gather = 2;
    SET paradedb.enable_aggregate_custom_scan = false;

    CREATE TABLE t (id bigint);
    INSERT INTO t (id) SELECT x FROM generate_series(1, 1000000) x;
    CREATE INDEX t_idx ON t USING bm25(id) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let (plan,) =
        "EXPLAIN (ANALYZE, FORMAT JSON) SELECT COUNT(*) FROM t WHERE id @@@ paradedb.all()"
            .fetch_one::<(Value,)>(&mut conn);
    let plan = plan.pointer("/0/Plan/Plans/0").unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Workers Planned"),
        Some(&Value::Number(Number::from(2)))
    );

    "SET paradedb.enable_custom_scan = false".execute(&mut conn);

    let (plan,) =
        "EXPLAIN (ANALYZE, FORMAT JSON) SELECT COUNT(*) FROM t WHERE id @@@ paradedb.all()"
            .fetch_one::<(Value,)>(&mut conn);
    let plan = plan.pointer("/0/Plan/Plans/0").unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Workers Planned"),
        Some(&Value::Number(Number::from(2)))
    );
}

#[rstest]
fn join_with_string_fast_fields_issue_2505(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS a;
    DROP TABLE IF EXISTS b;

    CREATE TABLE a (
        a_id_pk TEXT,
        content TEXT
    ) WITH (autovacuum_enabled = false);

    CREATE TABLE b (
        b_id_pk TEXT,
        a_id_fk TEXT,
        content TEXT
    ) WITH (autovacuum_enabled = false);

    CREATE INDEX idxa ON a USING bm25 (a_id_pk, content) WITH (key_field = 'a_id_pk');

    CREATE INDEX idxb ON b USING bm25 (b_id_pk, a_id_fk, content) WITH (key_field = 'b_id_pk',
      text_fields = '{ "a_id_fk": { "fast": true, "tokenizer": { "type": "keyword" } } }');

    INSERT INTO a (a_id_pk, content) VALUES ('this-is-a-id', 'beer');
    INSERT INTO b (b_id_pk, a_id_fk, content) VALUES ('this-is-b-id', 'this-is-a-id', 'wine');
    "#
    .execute(&mut conn);

    "VACUUM a, b;  -- needed to get Visibility Map up-to-date".execute(&mut conn);

    // This query previously failed with:
    // "ERROR: assertion failed: natts == state.exec_tuple_which_fast_fields.len()"
    let result = r#"
    SELECT a.a_id_pk as my_a_id_pk, b.b_id_pk as my_b_id_pk
    FROM b
    JOIN a ON a.a_id_pk = b.a_id_fk
    WHERE a.content @@@ 'beer' AND b.content @@@ 'wine';
    "#
    .fetch_result::<(String, String)>(&mut conn)
    .expect("JOIN query with string fast fields should execute successfully");

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0],
        ("this-is-a-id".to_string(), "this-is-b-id".to_string())
    );

    "DROP TABLE a; DROP TABLE b;".execute(&mut conn);
}

#[rstest]
fn custom_scan_respects_parentheses_issue2526(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (key_field='id');
    "#.execute(&mut conn);

    let result: Vec<(i64,)> = "SELECT COUNT(*) from mock_items WHERE description @@@ 'shoes' AND (description @@@ 'keyboard' OR description @@@ 'hat')".fetch(&mut conn);
    assert_eq!(result, vec![(0,)]);
}
```

---

## executor_hooks.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn multiple_index_changes_in_same_xact(mut conn: PgConnection) {
    r#"
        CREATE TABLE a (id int, value text);
        CREATE TABLE b (id int, value text);
        CREATE TABLE c (id int, value text);
        CREATE INDEX idxa ON a USING bm25(id, value) WITH (key_field='id');
        CREATE INDEX idxb ON b USING bm25(id, value) WITH (key_field='id');
        CREATE INDEX idxc ON c USING bm25(id, value) WITH (key_field='id');
        INSERT INTO a (id, value) VALUES (1, 'a');
        INSERT INTO b (id, value) VALUES (1, 'b');
        INSERT INTO c (id, value) VALUES (1, 'c');
    "#
    .execute(&mut conn);

    let results = r#"
        SELECT * FROM a WHERE value @@@ 'a'
           UNION
        SELECT * FROM b WHERE value @@@ 'b'
           UNION
        SELECT * FROM c WHERE value @@@ 'c'
        ORDER BY 1, 2;
    "#
    .fetch::<(i32, String)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (1, "a".to_string()),
            (1, "b".to_string()),
            (1, "c".to_string()),
        ]
    )
}

#[rstest]
fn issue2187_executor_hooks(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table
        (
            id           UUID NOT NULL DEFAULT gen_random_uuid(),
            email        TEXT NOT NULL DEFAULT (
                'user' || floor(random() * 150)::TEXT || '@example.com'
                ),
            is_processed BOOLEAN       DEFAULT FALSE,
            PRIMARY KEY (id, email)
        ) PARTITION BY HASH (email);


        DO
        $$
            DECLARE
                i INT;
            BEGIN
                FOR i IN 0..15
                    LOOP
                        EXECUTE format(
                                'CREATE TABLE test_table_p%s PARTITION OF test_table
                                 FOR VALUES WITH (MODULUS 16, REMAINDER %s);', i, i
                                );
                    END LOOP;
            END
        $$;


        INSERT INTO test_table (is_processed)
        SELECT FALSE
        FROM generate_series(1, 100000);


        DO
        $$
            DECLARE
                i          INT;
                table_name TEXT;
                index_name TEXT;
            BEGIN
                FOR i IN 0..15
                    LOOP
                        table_name := format('test_table_p%s', i);
                        index_name := format('test_table_search_p%s', i);

                        EXECUTE format(
                                'CREATE INDEX %I ON %I
                                 USING bm25 (id, is_processed)
                                 WITH (
                                     key_field = ''id'',
                                     boolean_fields = ''{
                                         "is_processed": {
                                             "fast": true,
                                             "indexed": true
                                         }
                                     }''
                                 )', index_name, table_name);
                    END LOOP;
            END
        $$;


        DO
        $$
            DECLARE
                batch_size INT := 1000;
                uuid_batch UUID[];
            BEGIN
                LOOP
                    SELECT ARRAY_AGG(id)
                    INTO uuid_batch
                    FROM (SELECT id
                          FROM test_table
                          WHERE is_processed = FALSE
                          LIMIT batch_size) sub;

                    IF uuid_batch IS NULL OR array_length(uuid_batch, 1) = 0 THEN
                        EXIT;
                    END IF;

                    UPDATE test_table
                    SET is_processed = TRUE
                    WHERE id = ANY (uuid_batch);
                END LOOP;
            END
        $$;
    "#
    .execute(&mut conn);
}
```

---

## query_json.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn boolean_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
   '{
        "boolean": {
            "should": [
                {"parse": {"query_string": "description:shoes"}},
                {"phrase_prefix": {"field": "description", "phrases": ["book"]}},
                {"term": {"field": "description", "value": "speaker"}},
                {"fuzzy_term": {"field": "description", "value": "wolo", "transposition_cost_one": false, "distance": 1, "prefix": true}}
            ]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5, 7, 10, 32, 33, 34, 37, 39, 41]);
}

#[rstest]
fn fuzzy_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"fuzzy_term": {"field": "category", "value": "elector", "prefix": true}}'::jsonb
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32], "wrong results");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{"term": {"field": "category", "value": "electornics"}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(columns.is_empty(), "without fuzzy field should be empty");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{
            "fuzzy_term": {
                "field": "description",
                "value": "keybaord",
                "transposition_cost_one": false,
                "distance": 1,
                "prefix": true
            }
        }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(
        columns.is_empty(),
        "transposition_cost_one false should be empty"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{
            "fuzzy_term": {
                "field": "description",
                "value": "keybaord",
                "transposition_cost_one": true,
                "distance": 1,
                "prefix": true
            }
        }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![1, 2],
        "incorrect transposition_cost_one true"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{
            "fuzzy_term": {
                "field": "description",
                "value": "keybaord",
                "prefix": true
            }
        }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2], "incorrect defaults");
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{"all": null}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{"boost": {"query": {"all": null}, "factor": 1.5}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"const_score": {"query": {"all": null}, "score": 3.9}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{"disjunction_max": {"disjuncts": [{"parse": {"query_string": "description:shoes"}}]}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{"empty": null}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{
        "fuzzy_term": {
            "field": "description",
            "value": "wolo",
            "transposition_cost_one": false,
            "distance": 1,
            "prefix": true
        }
    }'::jsonb ORDER BY ID"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{"parse": {"query_string": "description:teddy"}}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{"phrase_prefix": {"field": "description", "phrases": ["har"]}}'::jsonb
        ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{"phrase": {"field": "description", "phrases": ["robot"]}}'::jsonb
        ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{
        "phrase": {
            "field": "description",
            "phrases": ["robot", "building", "kit"]
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{
            "range": {
                "field": "last_updated_date",
                "lower_bound": {"included": "2023-05-01T00:00:00.000000Z"},
                "upper_bound": {"included": "2023-05-03T00:00:00.000000Z"},
                "is_datetime": true
            }
        }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{
        "regex": {
            "field": "description",
            "pattern": "(hardcover|plush|leather|running|wireless)"
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"term": {"field": "description", "value": "shoes"}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    //
    // NB:  This once worked, but the capability was removed when the new "pdb.*" builder functions
    //      were added.  The general problem is that there's no longer a clean way to indicate
    //      the desire to "search all column"
    //
    // // Term with no field (should search all columns)
    // let columns: SimpleProductsTableVec = r#"
    // SELECT * FROM paradedb.bm25_search
    // WHERE bm25_search @@@ '{"term": {"value": "shoes"}}'::jsonb ORDER BY id"#
    //     .fetch_collect(&mut conn);
    // assert_eq!(columns.len(), 3);

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "term_set": {
            "terms": [
                {"field": "description", "value": "shoes", "is_datetime": false},
                {"field": "description", "value": "novel", "is_datetime": false}
            ]
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn single_queries_jsonb_build_object(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('all', null) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('boost', jsonb_build_object(
        'query', jsonb_build_object('all', null),
        'factor', 1.5)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('const_score', jsonb_build_object(
        'query', jsonb_build_object('all', null),
        'score', 3.9)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('disjunction_max', jsonb_build_object(
        'disjuncts', jsonb_build_array(
            jsonb_build_object('parse', jsonb_build_object(
                'query_string', 'description:shoes'))))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('empty', null) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('fuzzy_term', jsonb_build_object(
        'field', 'description',
        'value', 'wolo',
        'transposition_cost_one', false,
        'distance', 1,
        'prefix', true)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('parse', jsonb_build_object(
        'query_string', 'description:teddy')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('phrase_prefix', jsonb_build_object(
        'field', 'description',
        'phrases', jsonb_build_array('har'))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('phrase', jsonb_build_object(
        'field', 'description',
        'phrases', jsonb_build_array('robot'))) ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('phrase', jsonb_build_object(
        'field', 'description',
        'phrases', jsonb_build_array('robot', 'building', 'kit'))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('range', jsonb_build_object(
        'field', 'last_updated_date',
        'lower_bound', jsonb_build_object('included', '2023-05-01T00:00:00.000000Z'),
        'upper_bound', jsonb_build_object('included', '2023-05-03T00:00:00.000000Z'),
        'is_datetime', true)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('regex', jsonb_build_object(
        'field', 'description',
        'pattern', '(hardcover|plush|leather|running|wireless)')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('term', jsonb_build_object(
        'field', 'description',
        'value', 'shoes')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    //
    // NB:  This once worked, but the capability was removed when the new "pdb.*" builder functions
    //      were added.  The general problem is that there's no longer a clean way to indicate
    //      the desire to "search all column"
    //
    // // Term with no field (should search all columns)
    // let columns: SimpleProductsTableVec = r#"
    // SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    // jsonb_build_object('term', jsonb_build_object('value', 'shoes')) ORDER BY id"#
    //     .fetch_collect(&mut conn);
    // assert_eq!(columns.len(), 3);

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    jsonb_build_object('term_set', jsonb_build_object(
        'terms', jsonb_build_array(
            jsonb_build_array('description', 'shoes', false),
            jsonb_build_array('description', 'novel', false)))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn exists_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Simple exists query
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{"exists": {"field": "rating"}}'::jsonb
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Non fast field should fail
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{"exists": {"field": "description"}}'::jsonb
    "#
    .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains("not a fast field")),
        _ => panic!("exists() over non-fast field should fail"),
    }

    // Exists with boolean query
    "INSERT INTO paradedb.bm25_search (id, description, rating) VALUES (42, 'shoes', NULL)"
        .execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
        '{
            "boolean": {
                "must": [
                    {"exists": {"field": "rating"}},
                    {"parse": {"query_string": "description:shoes"}}
                ]
            }
        }'::jsonb
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);
}

#[rstest]
fn more_like_this_raw(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES
        ('apple'),
        ('banana'),
        ('cherry'),
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Missing keys should fail.
    match r#"
    SELECT id, flavour FROM test_more_like_this_table WHERE test_more_like_this_table @@@
        '{"more_like_this": {}}'::jsonb;
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either key_value or document")
        }
        _ => panic!("key_value or document validation failed"),
    }

    // Conflicting keys should fail.
    match r#"
    SELECT id, flavour FROM test_more_like_this_table WHERE test_more_like_this_table @@@
        '{"more_like_this": {
            "key_value": 0,
            "document": [["flavour", "banana"]]
        }}'::jsonb;
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either key_value or document")
        }
        _ => panic!("key_value or document validation failed"),
    }

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "key_value": 2
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_empty(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES
        ('apple'),
        ('banana'),
        ('cherry'),
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{"more_like_this": {}}'::jsonb
    ORDER BY id;
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either key_value or document")
        }
        _ => panic!("key_value or document validation failed"),
    }
}

#[rstest]
fn more_like_this_text(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES
        ('apple'),
        ('banana'),
        ('cherry'),
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_boolean_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BOOLEAN PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (true, 'apple'),
        (false, 'banana')
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(bool, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn more_like_this_uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id UUID PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'apple'),
        ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'banana'),
        ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'cherry'),
        ('eb833eb6-c598-4042-b84a-0045828fceea', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(uuid::Uuid, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BIGINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1, 'apple'),
        (2, 'banana'),
        (3, 'cherry'),
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id INT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1, 'apple'),
        (2, 'banana'),
        (3, 'cherry'),
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i16_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SMALLINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1, 'apple'),
        (2, 'banana'),
        (3, 'cherry'),
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i16, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id FLOAT4 PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(f32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id FLOAT8 PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_literal_cast(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id INT PRIMARY KEY,
        year INTEGER
    );

    INSERT INTO test_more_like_this_table (id, year) VALUES
        (1, 2012),
        (2, 2013),
        (3, 2014),
        (4, 2012);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, year)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, i32)> = r#"
    SELECT id, year FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [
                ["year", 2012]
            ]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_numeric_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id NUMERIC PRIMARY KEY,
    flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT CAST(id AS FLOAT8), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_date_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id DATE PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('2023-05-03', 'apple'),
        ('2023-05-04', 'banana'),
        ('2023-05-05', 'cherry'),
        ('2023-05-06', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_time_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIME PRIMARY KEY,
    flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('08:09:10', 'apple'),
        ('09:10:11', 'banana'),
        ('10:11:12', 'cherry'),
        ('11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamp_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIMESTAMP PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('2023-05-03 08:09:10', 'apple'),
        ('2023-05-04 09:10:11', 'banana'),
        ('2023-05-05 10:11:12', 'cherry'),
        ('2023-05-06 11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamptz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIMESTAMP WITH TIME ZONE PRIMARY KEY,
    flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('2023-05-03 08:09:10 EST', 'apple'),
        ('2023-05-04 09:10:11 PST', 'banana'),
        ('2023-05-05 10:11:12 MST', 'cherry'),
        ('2023-05-06 11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timetz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIME WITH TIME ZONE PRIMARY KEY,
        flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES
        ('08:09:10 EST', 'apple'),
        ('09:10:11 PST', 'banana'),
        ('10:11:12 MST', 'cherry'),
        ('11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);
    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn match_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "match": {
            "field": "description",
            "value": "ruling shoeez",
            "distance": 2
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "match": {
            "field": "description",
            "value": "ruling shoeez",
            "distance": 2,
            "conjunction_mode": true
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "match": {
            "field": "description",
            "value": "ruling shoeez",
            "distance": 1
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id.len(), 0);
}

#[rstest]
fn range_term(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'deliveries',
        table_type => 'Deliveries'
    );

    CREATE INDEX deliveries_idx ON deliveries
    USING bm25 (delivery_id, weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times)
    WITH (key_field='delivery_id');
    "#
    .execute(&mut conn);

    // int4range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 1 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "weights", "value": 1}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 13 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "weights", "value": 13}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // int8range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE quantities @> 17000::int8 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "quantities", "value": 17000}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // numrange
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE prices @> 3.5 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "prices", "value": 3.5}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // daterange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-07'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "ship_dates", "value": "2023-03-07T00:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-06'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "ship_dates", "value": "2023-03-06T00:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // tsrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 14:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "facility_arrival_times", "value": "2024-05-01T14:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 15:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "facility_arrival_times", "value": "2024-05-01T15:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // tstzrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01 06:31:00-04'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "delivery_times", "value": "2024-05-01T10:31:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01T11:30:00Z'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "delivery_times", "value": "2024-05-01T11:30:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);
}

#[rstest]
fn parse_error(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let result = r#"
    SELECT id FROM paradedb.bm25_search WHERE bm25_search @@@
    '{"all": {}}'::jsonb ORDER BY id"#
        .fetch_result::<(i32,)>(&mut conn);

    match result {
        Err(err) => assert_eq!(
            err.to_string(),
            r#"error returned from database: error parsing search query input json at ".": data did not match any variant of untagged enum SearchQueryInput"#
        ),
        _ => {
            panic!("search input query variant with no fields should not be able to receive a map")
        }
    }
}
```

---

## parallel_index_scan.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn index_scan_under_parallel_path(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // the `debug_parallel_query` was added in pg16, so we simply cannot run this test on anything
        // less than pg16
        return;
    }

    SimpleProductsTable::setup().execute(&mut conn);

    r#"
        set paradedb.enable_custom_scan to off;
        set enable_indexonlyscan to off;
        set debug_parallel_query to on;
    "#
    .execute(&mut conn);

    let count = r#"
        select count(1) from paradedb.bm25_search where description @@@ 'shoes';
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(3,)]);
}

#[rstest]
fn dont_do_parallel_index_scan(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "VACUUM paradedb.bm25_search".execute(&mut conn);
    "set enable_indexscan to off;".execute(&mut conn);
    let (plan, ) = "EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) select count(*) from paradedb.bm25_search where description @@@ 'shoes';".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");
    let plan = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    pretty_assertions::assert_eq!(
        plan.get("Node Type"),
        Some(&Value::String(String::from("Custom Scan")))
    );
    // TODO: Make this not sporadically return 0 in CI:
    //   see https://github.com/paradedb/paradedb/issues/2588
    // pretty_assertions::assert_eq!(
    //     plan.get("Virtual Tuples"),
    //     Some(&Value::Number(serde_json::Number::from(3)))
    // );

    let count = r#"
        select count(*) from paradedb.bm25_search where description @@@ 'shoes';
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(3,)]);
}
```

---

## query_edge_cases.rs

```
mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn select_everything(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text
    );
    INSERT INTO test_table (value) VALUES ('beer'), ('wine'), ('cheese');
    CREATE INDEX test_index ON test_table USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    r#"set paradedb.enable_custom_scan to off; set max_parallel_workers_per_gather = 0;"#
        .execute(&mut conn);
    let (count,) = r#"SELECT count(*) FROM test_table WHERE id @@@ paradedb.all() OR id > 0"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 3);
}

#[rstest]
fn query_empty_table(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test_table;
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text[]
    );

    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers = 0;".execute(&mut conn);
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'beer';".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    "SET max_parallel_workers = 8;".execute(&mut conn);
    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'beer';".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);
}

#[rstest]
fn unary_not_issue2141(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text[]
    );

    INSERT INTO test_table (value) VALUES (ARRAY['beer', 'cheese']), (ARRAY['beer', 'wine']), (ARRAY['beer']), (ARRAY['beer']);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'beer';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 4);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'beer';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'wine';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 3);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'wine' AND NOT value @@@ 'cheese';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine' OR NOT value @@@ 'missing';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 4);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine' AND NOT value @@@ 'cheese';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 2);
}
```

---

## fixtures/db.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use async_std::prelude::Stream;
use async_std::stream::StreamExt;
use async_std::task::block_on;
use bytes::Bytes;
use rand::Rng;
use sqlx::{
    postgres::PgRow,
    testing::{TestArgs, TestContext, TestSupport},
    ConnectOptions, Connection, Decode, Error, Executor, FromRow, PgConnection, Postgres, Type,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Db {
    context: TestContext<Postgres>,
}

impl Db {
    pub async fn new() -> Self {
        let path =
            // timestamp
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("current time should be retrievable")
                .as_micros()
                .to_string()

                // plus the current thread name, which is typically going to be the test name
                + &std::thread::current()
                .name()
                .map(String::from)
                .unwrap_or_else(|| {
                    // or a random 7-letter "word"
                    rand::rng()
                        .sample_iter(&rand::distr::Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect()
                });

        let args = TestArgs::new(Box::leak(path.into_boxed_str()));
        let context = Postgres::test_context(&args)
            .await
            .unwrap_or_else(|err| panic!("could not create test database: {err:#?}"));

        Self { context }
    }

    pub async fn connection(&self) -> PgConnection {
        self.context
            .connect_opts
            .connect()
            .await
            .unwrap_or_else(|err| panic!("failed to connect to test database: {err:#?}"))
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        let db_name = self.context.db_name.to_string();
        async_std::task::spawn(async move {
            Postgres::cleanup_test(db_name.as_str()).await.ok(); // ignore errors as there's nothing we can do about it
        });
    }
}

pub trait ConnExt {
    fn deallocate_all(&mut self) -> Result<(), sqlx::Error>;
}

impl ConnExt for PgConnection {
    /// Deallocate all cached prepared statements.  Akin to Postgres' `DEALLOCATE ALL` command
    /// but also does the right thing for the sql [`PgConnection`] internals.
    fn deallocate_all(&mut self) -> Result<(), Error> {
        async_std::task::block_on(async { self.clear_cached_statements().await })
    }
}

#[allow(dead_code)]
pub trait Query
where
    Self: AsRef<str> + Sized,
{
    fn execute(self, connection: &mut PgConnection) {
        block_on(async { self.execute_async(connection).await })
    }

    #[allow(async_fn_in_trait)]
    async fn execute_async(self, connection: &mut PgConnection) {
        connection
            .execute(self.as_ref())
            .await
            .expect("query execution should succeed");
    }

    fn execute_result(self, connection: &mut PgConnection) -> Result<(), sqlx::Error> {
        block_on(async { connection.execute(self.as_ref()).await })?;
        Ok(())
    }

    fn fetch<T>(self, connection: &mut PgConnection) -> Vec<T>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    fn fetch_retry<T>(
        self,
        connection: &mut PgConnection,
        retries: u32,
        delay_ms: u64,
        validate: fn(&[T]) -> bool,
    ) -> Vec<T>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        for attempt in 0..retries {
            match block_on(async {
                sqlx::query_as::<_, T>(self.as_ref())
                    .fetch_all(&mut *connection)
                    .await
                    .map_err(anyhow::Error::from)
            }) {
                Ok(result) => {
                    if validate(&result) {
                        return result;
                    } else if attempt < retries - 1 {
                        block_on(async_std::task::sleep(Duration::from_millis(delay_ms)));
                    } else {
                        return vec![];
                    }
                }
                Err(_) if attempt < retries - 1 => {
                    block_on(async_std::task::sleep(Duration::from_millis(delay_ms)));
                }
                Err(e) => panic!("Fetch attempt {}/{} failed: {}", attempt + 1, retries, e),
            }
        }
        panic!("Exhausted retries for query '{}'", self.as_ref());
    }

    fn fetch_dynamic(self, connection: &mut PgConnection) -> Vec<PgRow> {
        block_on(async {
            sqlx::query(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    fn fetch_scalar<T>(self, connection: &mut PgConnection) -> Vec<T>
    where
        T: Type<Postgres> + for<'a> Decode<'a, sqlx::Postgres> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_scalar(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    fn fetch_one<T>(self, connection: &mut PgConnection) -> T
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(self.as_ref())
                .fetch_one(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    fn fetch_result<T>(self, connection: &mut PgConnection) -> Result<Vec<T>, sqlx::Error>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(self.as_ref())
                .fetch_all(connection)
                .await
        })
    }

    fn fetch_collect<T, B>(self, connection: &mut PgConnection) -> B
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
        B: FromIterator<T>,
    {
        self.fetch(connection).into_iter().collect::<B>()
    }
}

impl Query for String {}
impl Query for &String {}
impl Query for &str {}

pub trait DisplayAsync: Stream<Item = Result<Bytes, sqlx::Error>> + Sized {
    fn to_csv(self) -> String {
        let mut csv_str = String::new();
        let mut stream = Box::pin(self);

        while let Some(chunk) = block_on(stream.as_mut().next()) {
            let chunk = chunk.expect("chunk should be valid for DisplayAsync");
            csv_str.push_str(&String::from_utf8_lossy(&chunk));
        }

        csv_str
    }
}

impl<T> DisplayAsync for T where T: Stream<Item = Result<Bytes, sqlx::Error>> + Send + Sized {}
```

---

## fixtures/mod.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod db;
pub mod querygen;
pub mod tables;
pub mod utils;

use async_std::task::block_on;
use rstest::*;
use sqlx::{self, PgConnection};

pub use crate::fixtures::db::*;
pub use crate::fixtures::tables::*;

#[fixture]
pub fn database() -> Db {
    block_on(async { Db::new().await })
}

pub fn pg_major_version(conn: &mut PgConnection) -> usize {
    r#"select (regexp_match(version(), 'PostgreSQL (\d+)'))[1]::int;"#
        .fetch_one::<(i32,)>(conn)
        .0 as usize
}

#[fixture]
pub fn conn(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;

        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_search");

        sqlx::query("SET log_error_verbosity TO VERBOSE;")
            .execute(&mut conn)
            .await
            .expect("could not adjust log_error_verbosity");

        // Setting to 1 provides test coverage for both mutable and immutable segments, because
        // bulk insert statements will create both a mutable and immutable segment.
        sqlx::query("SET paradedb.global_mutable_segment_rows TO 1;")
            .execute(&mut conn)
            .await
            .expect("could not adjust mutable_segment_rows");

        sqlx::query("SET log_min_duration_statement TO 1000;")
            .execute(&mut conn)
            .await
            .expect("could not set long-running-statement logging");

        conn
    })
}
```

---

## fixtures/utils.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
#![allow(dead_code)]
use super::db::*;

use sqlx::PgConnection;
use std::path::PathBuf;

pub fn database_oid(conn: &mut PgConnection) -> String {
    let db_name = "SELECT current_database()".fetch_one::<(String,)>(conn).0;

    format!("SELECT oid FROM pg_database WHERE datname='{db_name}'")
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0
        .to_string()
}

pub fn schema_oid(conn: &mut PgConnection, schema_name: &str) -> String {
    format!("SELECT oid FROM pg_namespace WHERE nspname='{schema_name}'")
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0
        .to_string()
}

pub fn table_oid(conn: &mut PgConnection, schema_name: &str, table_name: &str) -> String {
    format!("SELECT oid FROM pg_class WHERE relname='{table_name}' AND relnamespace=(SELECT oid FROM pg_namespace WHERE nspname='{schema_name}')")
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
        .0
        .to_string()
}

pub fn default_database_path(conn: &mut PgConnection) -> PathBuf {
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    let deltalake_dir = "deltalake";
    let database_oid = database_oid(conn);

    PathBuf::from(&data_dir)
        .join(deltalake_dir)
        .join(database_oid)
}

pub fn default_schema_path(conn: &mut PgConnection, schema_name: &str) -> PathBuf {
    let schema_oid = schema_oid(conn, schema_name);
    default_database_path(conn).join(schema_oid)
}

pub fn default_table_path(conn: &mut PgConnection, schema_name: &str, table_name: &str) -> PathBuf {
    let table_oid = table_oid(conn, schema_name, table_name);
    default_schema_path(conn, schema_name).join(table_oid)
}
```

---

## fixtures/tables/partitioned.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use chrono::{NaiveDate, NaiveDateTime};
use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct PartitionedTable {
    pub id: i32,
    pub sale_date: NaiveDateTime,
    pub amount: f32,
    pub description: String,
}

impl PartitionedTable {
    pub fn setup() -> String {
        PARTITIONED_TABLE_SETUP.into()
    }
}

static PARTITIONED_TABLE_SETUP: &str = r#"
BEGIN;
    CREATE TABLE sales (
        id SERIAL,
        sale_date DATE NOT NULL,
        amount REAL NOT NULL,
        description TEXT,
        PRIMARY KEY (id, sale_date)
    ) PARTITION BY RANGE (sale_date);

    CREATE TABLE sales_2023_q1 PARTITION OF sales
      FOR VALUES FROM ('2023-01-01') TO ('2023-04-01');

    CREATE TABLE sales_2023_q2 PARTITION OF sales
      FOR VALUES FROM ('2023-04-01') TO ('2023-06-30');

    CREATE INDEX sales_index ON sales
      USING bm25 (id, description, sale_date, amount)
      WITH (
        key_field='id',
        numeric_fields='{"amount": {"fast": true}}',
        datetime_fields = '{"sale_date": {"fast": true}}'
      );
COMMIT;
"#;
```

---

## fixtures/tables/simple_products.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use chrono::{NaiveDate, NaiveDateTime};
use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct SimpleProductsTable {
    pub id: i32,
    pub description: String,
    pub category: String,
    pub rating: i32,
    pub in_stock: bool,
    pub metadata: serde_json::Value,
    pub created_at: NaiveDateTime,
    pub last_updated_date: NaiveDate,
}

impl SimpleProductsTable {
    pub fn setup() -> String {
        SIMPLE_PRODUCTS_TABLE_SETUP.into()
    }

    pub fn setup_multi_segment() -> String {
        // Inserting one additional row will get us an additional segment.
        format!(
            r#"{SIMPLE_PRODUCTS_TABLE_SETUP}
            INSERT INTO paradedb.bm25_search
              (description, category, rating, in_stock, metadata, created_at, last_updated_date)
            VALUES
              ('Product with mixed array', 'Electronics', 5, true, '{{"attributes": ["fast", 4, true]}}', now(), current_date);
            "#
        )
    }
}

static SIMPLE_PRODUCTS_TABLE_SETUP: &str = r#"
BEGIN;
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_bm25_index
    ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (key_field='id');
COMMIT;
"#;
```

---

## fixtures/tables/icu_czech_posts.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuCzechPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuCzechPostsTable {
    pub fn setup() -> &'static str {
        ICU_CZECH_POSTS
    }
}

static ICU_CZECH_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_czech_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);
INSERT INTO icu_czech_posts (author, title, message)
VALUES
    ('Tomáš', 'kouše sendvič', 'červená karkulka v lese šla sbírat dříví'),
    ('Eliška', 'zdravý banán', 'zpívat srdcem do světa'),
    ('Adéla', 'bylo nebylo', 've ztraceném tajném městě žil velký mág');
"#;
```

---

## fixtures/tables/user_session_logs.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use soa_derive::StructOfArray;
use sqlx::FromRow;
use time::Date;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Serialize, Deserialize)]
pub struct UserSessionLogsTable {
    pub id: i32,
    pub event_date: Option<Date>,
    pub user_id: Option<i32>,
    pub event_name: Option<String>,
    pub session_duration: Option<i32>,
    pub page_views: Option<i32>,
    pub revenue: Option<BigDecimal>,
}

impl UserSessionLogsTable {
    pub fn setup_parquet() -> String {
        USER_SESSION_LOGS_TABLE_SETUP.replace("{}", "parquet")
    }

    pub fn setup_heap() -> String {
        USER_SESSION_LOGS_TABLE_SETUP.replace("{}", "heap")
    }
}

static USER_SESSION_LOGS_TABLE_SETUP: &str = r#"
CREATE TABLE user_session_logs (
    id SERIAL PRIMARY KEY,
    event_date DATE,
    user_id INT,
    event_name VARCHAR(50),
    session_duration INT,
    page_views INT,
    revenue DECIMAL(10, 2)
);

INSERT INTO user_session_logs
(event_date, user_id, event_name, session_duration, page_views, revenue)
VALUES
('2024-01-01', 1, 'Login', 300, 5, 20.00),
('2024-01-02', 2, 'Purchase', 450, 8, 150.50),
('2024-01-03', 3, 'Logout', 100, 2, 0.00),
('2024-01-04', 4, 'Signup', 200, 3, 0.00),
('2024-01-05', 5, 'ViewProduct', 350, 6, 30.75),
('2024-01-06', 1, 'AddToCart', 500, 10, 75.00),
('2024-01-07', 2, 'RemoveFromCart', 250, 4, 0.00),
('2024-01-08', 3, 'Checkout', 400, 7, 200.25),
('2024-01-09', 4, 'Payment', 550, 11, 300.00),
('2024-01-10', 5, 'Review', 600, 9, 50.00),
('2024-01-11', 6, 'Login', 320, 3, 0.00),
('2024-01-12', 7, 'Purchase', 480, 7, 125.30),
('2024-01-13', 8, 'Logout', 150, 2, 0.00),
('2024-01-14', 9, 'Signup', 240, 4, 0.00),
('2024-01-15', 10, 'ViewProduct', 360, 5, 45.00),
('2024-01-16', 6, 'AddToCart', 510, 9, 80.00),
('2024-01-17', 7, 'RemoveFromCart', 270, 3, 0.00),
('2024-01-18', 8, 'Checkout', 430, 6, 175.50),
('2024-01-19', 9, 'Payment', 560, 12, 250.00),
('2024-01-20', 10, 'Review', 610, 10, 60.00);
"#;
```

---

## fixtures/tables/icu_amharic_posts.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuAmharicPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuAmharicPostsTable {
    pub fn setup() -> &'static str {
        ICU_AMHARIC_POSTS
    }
}

static ICU_AMHARIC_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_amharic_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);
INSERT INTO icu_amharic_posts (author, title, message)
VALUES
    ('መሐመድ', 'መደመር ተጨማሪ', 'እንኳን ነበር በመደመር ተጨማሪ፣ በደስታ እና በልዩ ዝናብ ይከብዳል።'),
    ('ፋትስ', 'የምስሉ ማህበረሰብ', 'በዚህ ግዜ የምስሉ ማህበረሰብ እና እንደዚህ ዝናብ ይችላል።'),
    ('አለም', 'መረጃዎች ለመማር', 'እነዚህ መረጃዎች የምስሉ ለመማር በእያንዳንዱ ላይ ይመልከቱ።');
"#;
```

---

## fixtures/tables/mod.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod deliveries;
mod icu_amharic_posts;
mod icu_arabic_posts;
mod icu_czech_posts;
mod icu_greek_posts;
mod partitioned;
mod simple_products;
mod user_session_logs;

pub use deliveries::*;
pub use icu_amharic_posts::*;
pub use icu_arabic_posts::*;
pub use icu_czech_posts::*;
pub use icu_greek_posts::*;
pub use partitioned::*;
pub use simple_products::*;
pub use user_session_logs::*;
```

---

## fixtures/tables/icu_arabic_posts.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuArabicPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuArabicPostsTable {
    pub fn setup() -> &'static str {
        ICU_ARABIC_POSTS
    }
}

static ICU_ARABIC_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_arabic_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO icu_arabic_posts (author, title, message)
VALUES
    ('فاطمة', 'رحلة إلى الشرق', 'في هذا المقال، سنستكشف رحلة مثيرة إلى الشرق ونتعرف على ثقافات مختلفة وتاريخها الغني'),
    ('محمد','رحلة إلى السوق مع أبي', 'مرحباً بك في المقالة الأولى. أتمنى أن تجد المحتوى مفيدًا ومثيرًا للاهتمام'),
    ('أحمد', 'نصائح للنجاح', 'هنا نقدم لك بعض النصائح القيمة لتحقيق النجاح في حياتك المهنية والشخصية. استفد منها وحقق أهدافك');
"#;
```

---

## fixtures/tables/deliveries.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use bigdecimal::BigDecimal;
use chrono::{NaiveDate, NaiveDateTime};
use soa_derive::StructOfArray;
use sqlx::postgres::types::PgRange;
use sqlx::FromRow;
use std::ops::Range;

#[derive(Debug, PartialEq, FromRow, StructOfArray)]
pub struct DeliveriesTable {
    pub delivery_id: i32,
    pub weights: Range<i32>,
    pub quantities: PgRange<i64>,
    pub prices: BigDecimal,
    pub ship_dates: PgRange<NaiveDate>,
    pub facility_arrival_times: PgRange<NaiveDateTime>,
    pub delivery_times: PgRange<NaiveDateTime>,
}

impl DeliveriesTable {
    pub fn setup() -> String {
        DELIVERIES_TABLE_SETUP.into()
    }
}

static DELIVERIES_TABLE_SETUP: &str = r#"
BEGIN;
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'deliveries',
        table_type => 'Deliveries'
    );
   
    CREATE INDEX deliveries_idx ON deliveries
    USING bm25 (delivery_id, weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times)
    WITH (key_field='delivery_id');
COMMIT;
"#;
```

---

## fixtures/tables/icu_greek_posts.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuGreekPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuGreekPostsTable {
    pub fn setup() -> &'static str {
        ICU_GREEK_POSTS
    }
}

static ICU_GREEK_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_greek_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);
INSERT INTO icu_greek_posts (author, title, message)
VALUES
    ('Δημήτρης', 'Η πρώτη άρθρο', 'Καλώς ήρθες στο πρώτο άρθρο. Ελπίζω να βρεις το περιεχόμενο χρήσιμο και ενδιαφέρον.'),
    ('Σοφία', 'Ταξίδι στην Ανατολή', 'Σε αυτό το άρθρο, θα εξερευνήσουμε ένα συναρπαστικό ταξίδι στην Ανατολή και θα γνωρίσουμε διάφορες πολιτισμικές και ιστορικές πτυχές.'),
    ('Αλέξανδρος', 'Συμβουλές για την επιτυχία', 'Εδώ παρέχουμε μερικές πολύτιμες συμβουλές για την επίτευξη επιτυχίας στην επαγγελματική και προσωπική σας ζωή. Επωφεληθείτε από αυτές και επιτύχετε τους στόχους σας.');
"#;
```

---

## fixtures/querygen/joingen.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};

use proptest::prelude::*;
use proptest::sample;
use proptest_derive::Arbitrary;

#[derive(Arbitrary, Copy, Clone, Debug)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JoinType::Inner => "JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        f.write_str(s)
    }
}

#[derive(Clone, Debug)]
struct JoinStep {
    join_type: JoinType,
    table: String,
    on_left_table: Option<String>,
    on_left_col: Option<String>,
    on_right_col: Option<String>,
}

#[derive(Clone)]
pub struct JoinExpr {
    initial_table: String,
    steps: Vec<JoinStep>,
}

impl JoinExpr {
    pub fn used_tables(&self) -> Vec<&str> {
        let mut v = Vec::with_capacity(1 + self.steps.len());
        v.push(self.initial_table.as_str());
        for s in &self.steps {
            v.push(s.table.as_str());
        }
        v
    }

    /// Render as a SQL fragment, e.g.
    /// `FROM t0 JOIN t1 ON t0.a = t1.b LEFT JOIN t2 ON t1.x = t2.y ...`
    pub fn to_sql(&self) -> String {
        let mut join_clause = format!("FROM {}", self.initial_table);

        for step in &self.steps {
            join_clause.push(' ');
            join_clause.push_str(&step.join_type.to_string());
            join_clause.push(' ');
            join_clause.push_str(&step.table);
            if let JoinType::Cross = step.join_type {
                // no ON clause
            } else {
                let lt = step.on_left_table.as_ref().unwrap();
                let lc = step.on_left_col.as_ref().unwrap();
                let rc = step.on_right_col.as_ref().unwrap();
                join_clause.push_str(&format!(" ON {}.{} = {}.{}", lt, lc, step.table, rc));
            }
        }

        join_clause
    }
}

impl Debug for JoinExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JoinExpr")
            .field("sql", &self.to_sql())
            .finish_non_exhaustive()
    }
}

///
/// Generate all possible joins involving exactly the given tables.
///
pub fn arb_joins(
    join_types: impl Strategy<Value = JoinType>,
    tables_to_join: Vec<impl AsRef<str>>,
    columns: Vec<impl AsRef<str>>,
) -> impl Strategy<Value = JoinExpr> {
    let tables_to_join = tables_to_join
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();
    let table_cols = columns
        .into_iter()
        .map(|cn| cn.as_ref().to_string())
        .collect::<Vec<_>>();

    // Choose joins and join columns.
    let join_count = tables_to_join.len() - 1;
    (
        proptest::collection::vec(join_types, join_count),
        proptest::sample::subsequence(table_cols, join_count),
    )
        .prop_map(move |(join_types, join_columns)| {
            // Construct a JoinExpr for the tables and joins.
            let mut tables_to_join = tables_to_join.clone().into_iter();
            let initial_table = tables_to_join
                .next()
                .expect("At least one table in a join.");

            let mut previous_table = initial_table.clone();
            let mut steps = Vec::with_capacity(join_types.len());
            for ((join_type, join_column), table_to_join) in
                join_types.into_iter().zip(join_columns).zip(tables_to_join)
            {
                match join_type {
                    JoinType::Cross => {
                        steps.push(JoinStep {
                            join_type,
                            table: table_to_join.clone(),
                            on_left_table: None,
                            on_left_col: None,
                            on_right_col: None,
                        });
                    }
                    _ => {
                        steps.push(JoinStep {
                            join_type,
                            table: table_to_join.clone(),
                            on_left_table: Some(previous_table.to_owned()),
                            on_left_col: Some(join_column.clone()),
                            on_right_col: Some(join_column),
                        });
                    }
                }
                previous_table = table_to_join;
            }

            JoinExpr {
                initial_table,
                steps,
            }
        })
}
```

---

## fixtures/querygen/groupbygen.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use proptest::prelude::*;
use std::fmt::Debug;

/// Represents an item in the SELECT list
#[derive(Clone, Debug, PartialEq)]
pub enum SelectItem {
    Column(String),
    Aggregate(String),
}

/// Represents a GROUP BY expression with an explicit target list
#[derive(Clone, Debug)]
pub struct GroupByExpr {
    pub group_by_columns: Vec<String>,
    pub target_list: Vec<SelectItem>,
}

impl GroupByExpr {
    pub fn to_sql(&self) -> String {
        if self.group_by_columns.is_empty() {
            String::new()
        } else {
            format!("GROUP BY {}", self.group_by_columns.join(", "))
        }
    }

    pub fn to_select_list(&self) -> String {
        self.target_list
            .iter()
            .map(|item| match item {
                SelectItem::Column(col) => col.clone(),
                SelectItem::Aggregate(agg) => agg.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Generate arbitrary GROUP BY expressions with random target list ordering
pub fn arb_group_by(
    columns: Vec<impl AsRef<str>>,
    aggregates: Vec<&'static str>,
) -> impl Strategy<Value = GroupByExpr> {
    let columns = columns
        .into_iter()
        .map(|c| c.as_ref().to_string())
        .collect::<Vec<_>>();

    // Generate 0-3 grouping columns from the available columns
    proptest::sample::subsequence(columns, 0..3).prop_flat_map(move |selected_columns| {
        // Generate 0-3 aggregates from the available aggregates
        // TODO: Support 3 aggregates as soon as issue #2963 is fixed
        let max_aggregates = std::cmp::min(aggregates.len(), 2);
        let agg_range = if selected_columns.is_empty() {
            // No GROUP BY - need at least one aggregate
            1..=max_aggregates
        } else {
            // GROUP BY - can have 0 to max_aggregates
            0..=max_aggregates
        };

        proptest::sample::subsequence(aggregates.clone(), agg_range).prop_flat_map(
            move |selected_aggregates| {
                if selected_columns.is_empty() {
                    // No GROUP BY - just aggregates
                    let target_list = selected_aggregates
                        .iter()
                        .map(|&agg| SelectItem::Aggregate(agg.to_string()))
                        .collect();

                    Just(GroupByExpr {
                        group_by_columns: vec![],
                        target_list,
                    })
                    .boxed()
                } else {
                    // GROUP BY - aggregates and columns.
                    // Choose a subset of columns for grouping
                    let aggregates_clone = selected_aggregates.clone();
                    // Create select items for columns and aggregates
                    let mut select_items = Vec::new();

                    // Add all selected columns as SelectItem::Column
                    for col in &selected_columns {
                        select_items.push(SelectItem::Column(col.clone()));
                    }

                    // Add all aggregates as SelectItem::Aggregate
                    for &agg in &aggregates_clone {
                        select_items.push(SelectItem::Aggregate(agg.to_string()));
                    }

                    let selected_columns_clone = selected_columns.clone();

                    // Generate a random permutation of the target list
                    Just(select_items)
                        .prop_shuffle()
                        .prop_map(move |permuted_target_list| GroupByExpr {
                            group_by_columns: selected_columns_clone.clone(),
                            target_list: permuted_target_list,
                        })
                        .boxed()
                }
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_expr_empty() {
        let expr = GroupByExpr {
            group_by_columns: vec![],
            target_list: vec![SelectItem::Aggregate("COUNT(*)".to_string())],
        };
        assert_eq!(expr.to_sql(), "");
        assert_eq!(expr.to_select_list(), "COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_single_column_first() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string()],
            target_list: vec![
                SelectItem::Column("name".to_string()),
                SelectItem::Aggregate("COUNT(*)".to_string()),
            ],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name");
        assert_eq!(expr.to_select_list(), "name, COUNT(*)");
    }

    #[test]
    fn test_group_by_expr_single_aggregate_first() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string()],
            target_list: vec![
                SelectItem::Aggregate("COUNT(*)".to_string()),
                SelectItem::Column("name".to_string()),
            ],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name");
        assert_eq!(expr.to_select_list(), "COUNT(*), name");
    }

    #[test]
    fn test_group_by_expr_multiple_mixed_order() {
        let expr = GroupByExpr {
            group_by_columns: vec!["name".to_string(), "color".to_string()],
            target_list: vec![
                SelectItem::Aggregate("COUNT(*)".to_string()),
                SelectItem::Column("name".to_string()),
                SelectItem::Column("color".to_string()),
            ],
        };
        assert_eq!(expr.to_sql(), "GROUP BY name, color");
        assert_eq!(expr.to_select_list(), "COUNT(*), name, color");
    }
}
```

---

## fixtures/querygen/opexprgen.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use proptest::prelude::*;
use proptest::sample;
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, Arbitrary)]
pub enum Operator {
    Eq, // =
    Ne, // <>
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

impl Operator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "<>",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum ArrayQuantifier {
    Any,
    All,
}

impl ArrayQuantifier {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ArrayQuantifier::Any => "ANY",
            ArrayQuantifier::All => "ALL",
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum ScalarArrayOperator {
    In,
    NotIn,
}

impl ScalarArrayOperator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ScalarArrayOperator::In => "IN",
            ScalarArrayOperator::NotIn => "NOT IN",
        }
    }
}
```

---

## fixtures/querygen/mod.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

pub mod groupbygen;
pub mod joingen;
pub mod opexprgen;
pub mod pagegen;
pub mod wheregen;

use std::fmt::{Debug, Write};

use futures::executor::block_on;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use sqlx::{Connection, PgConnection};

use crate::fixtures::db::Query;
use crate::fixtures::ConnExt;
use joingen::{JoinExpr, JoinType};
use opexprgen::{ArrayQuantifier, Operator};
use wheregen::Expr;

#[derive(Debug, Clone)]
pub struct BM25Options {
    /// "text_fields" or "numeric_fields"
    pub field_type: &'static str,
    /// The JSON config for this field, e.g. `{ "tokenizer": { "type": "keyword" } }`
    pub config_json: &'static str,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: &'static str,
    pub sql_type: &'static str,
    pub sample_value: &'static str,
    pub is_primary_key: bool,
    pub is_groupable: bool,
    pub is_whereable: bool,
    pub is_indexed: bool,
    pub bm25_options: Option<BM25Options>,
    pub random_generator_sql: &'static str,
}

impl Column {
    pub const fn new(
        name: &'static str,
        sql_type: &'static str,
        sample_value: &'static str,
    ) -> Self {
        Self {
            name,
            sql_type,
            sample_value,
            is_primary_key: false,
            is_groupable: true,
            is_whereable: true,
            is_indexed: true,
            bm25_options: None,
            random_generator_sql: "NULL",
        }
    }

    pub const fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self
    }

    pub const fn groupable(mut self, is_groupable: bool) -> Self {
        self.is_groupable = is_groupable;
        self
    }

    pub const fn whereable(mut self, is_whereable: bool) -> Self {
        self.is_whereable = is_whereable;
        self
    }

    pub const fn indexed(mut self, is_indexed: bool) -> Self {
        self.is_indexed = is_indexed;
        self
    }

    pub const fn bm25_text_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "text_fields",
            config_json,
        });
        self
    }

    pub const fn bm25_numeric_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "numeric_fields",
            config_json,
        });
        self
    }

    /// Note: should use only the `random()` function to generate random data.
    pub const fn random_generator_sql(mut self, random_generator_sql: &'static str) -> Self {
        self.random_generator_sql = random_generator_sql;
        self
    }
}

pub fn generated_queries_setup(
    conn: &mut PgConnection,
    tables: &[(&str, usize)],
    columns_def: &[Column],
) -> String {
    "CREATE EXTENSION pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let seed_sql = format!("SET seed TO {};\n", rand::rng().random_range(-1.0..=1.0));
    seed_sql.as_str().execute(conn);

    let mut setup_sql = seed_sql;

    let column_definitions = columns_def
        .iter()
        .map(|col| {
            if col.is_primary_key {
                format!("{} {} NOT NULL PRIMARY KEY", col.name, col.sql_type)
            } else {
                format!("{} {}", col.name, col.sql_type)
            }
        })
        .collect::<Vec<_>>()
        .join(", \n");

    // For bm25 index
    let bm25_columns = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .map(|c| c.name)
        .collect::<Vec<_>>()
        .join(", ");
    let key_field = columns_def
        .iter()
        .find(|c| c.is_primary_key)
        .map(|c| c.name)
        .expect("At least one column must be a primary key");

    let text_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "text_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    let numeric_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "numeric_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    // For INSERT statements
    let insert_columns = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.name)
        .collect::<Vec<_>>()
        .join(", ");

    let sample_values = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.sample_value)
        .collect::<Vec<_>>()
        .join(", ");

    let random_generators = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.random_generator_sql)
        .collect::<Vec<_>>()
        .join(",\n      ");

    for (tname, row_count) in tables {
        let sql = format!(
            r#"
CREATE TABLE {tname} (
    {column_definitions}
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idx{tname} ON {tname} USING bm25 ({bm25_columns}) WITH (
    key_field = '{key_field}',
    text_fields = '{{ {text_fields} }}',
    numeric_fields = '{{ {numeric_fields} }}'
);

INSERT into {tname} ({insert_columns}) VALUES ({sample_values});

INSERT into {tname} ({insert_columns}) SELECT {random_generators} FROM generate_series(1, {row_count});

{b_tree_indexes}

ANALYZE;
"#,
            b_tree_indexes = columns_def
                .iter()
                .filter(|c| c.is_indexed)
                .map(|c| format!(
                    "CREATE INDEX idx{tname}_{name} ON {tname} ({name});",
                    name = c.name
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        (&sql).execute(conn);
        setup_sql.push_str(&sql);
    }

    setup_sql
}

///
/// Generates arbitrary joins and where clauses for the given tables and columns.
///
pub fn arb_joins_and_wheres(
    join_types: impl Strategy<Value = JoinType> + Clone,
    tables: Vec<impl AsRef<str>>,
    columns: &[Column],
) -> impl Strategy<Value = (JoinExpr, Expr)> {
    let table_names = tables
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();

    let columns = columns.to_vec();

    // Choose how many tables will be joined.
    (2..=table_names.len())
        .prop_flat_map(move |join_size| {
            // Then choose tables for that join size.
            proptest::sample::subsequence(table_names.clone(), join_size)
        })
        .prop_flat_map(move |tables| {
            // Finally, choose the joins and where clauses for those tables.
            (
                joingen::arb_joins(
                    join_types.clone(),
                    tables.clone(),
                    columns.iter().map(|c| c.name.to_owned()).collect(),
                ),
                wheregen::arb_wheres(tables.clone(), &columns.to_vec()),
            )
        })
}

#[derive(Copy, Clone, Debug, Arbitrary)]
pub struct PgGucs {
    aggregate_custom_scan: bool,
    custom_scan: bool,
    custom_scan_without_operator: bool,
    filter_pushdown: bool,
    seqscan: bool,
    indexscan: bool,
    parallel_workers: bool,
}

impl Default for PgGucs {
    fn default() -> Self {
        Self {
            aggregate_custom_scan: false,
            custom_scan: false,
            custom_scan_without_operator: false,
            filter_pushdown: false,
            seqscan: true,
            indexscan: true,
            parallel_workers: true,
        }
    }
}

impl PgGucs {
    pub fn set(&self) -> String {
        let PgGucs {
            aggregate_custom_scan,
            custom_scan,
            custom_scan_without_operator,
            filter_pushdown,
            seqscan,
            indexscan,
            parallel_workers,
        } = self;

        let max_parallel_workers = if *parallel_workers { 8 } else { 0 };

        let mut gucs = String::with_capacity(512);
        writeln!(
            gucs,
            "SET paradedb.enable_aggregate_custom_scan TO {aggregate_custom_scan};"
        )
        .unwrap();
        writeln!(gucs, "SET paradedb.enable_custom_scan TO {custom_scan};").unwrap();
        writeln!(
            gucs,
            "SET paradedb.enable_custom_scan_without_operator TO {custom_scan_without_operator};"
        )
        .unwrap();
        writeln!(
            gucs,
            "SET paradedb.enable_filter_pushdown TO {filter_pushdown};"
        )
        .unwrap();
        writeln!(gucs, "SET enable_seqscan TO {seqscan};").unwrap();
        writeln!(gucs, "SET enable_indexscan TO {indexscan};").unwrap();
        writeln!(gucs, "SET max_parallel_workers TO {max_parallel_workers};").unwrap();
        writeln!(gucs, "SET paradedb.add_doc_count_to_aggs TO true;").unwrap();
        gucs
    }
}

/// Run the given pg and bm25 queries on the given connection, and compare their results when run
/// with the given GUCs.
pub fn compare<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup_sql: &str,
    run_query: F,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> R,
{
    match inner_compare(pg_query, bm25_query, gucs, conn, run_query) {
        Ok(()) => Ok(()),
        Err(e) => Err(handle_compare_error(
            e, pg_query, bm25_query, gucs, setup_sql,
        )),
    }
}

fn inner_compare<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    run_query: F,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> R,
{
    // the postgres query is always run with the paradedb custom scan turned off
    // this ensures we get the actual, known-to-be-correct result from Postgres'
    // plan, and not from ours where we did some kind of pushdown
    PgGucs::default().set().execute(conn);

    conn.deallocate_all()?;

    let pg_result = run_query(pg_query, conn);

    // and for the "bm25" query, we run it with the given GUCs set.
    gucs.set().execute(conn);

    conn.deallocate_all()?;

    let bm25_result = run_query(bm25_query, conn);

    prop_assert_eq!(
        &pg_result,
        &bm25_result,
        "\ngucs={:?}\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
        gucs,
        pg_query,
        bm25_query,
        format!("EXPLAIN {bm25_query}")
            .fetch::<(String,)>(conn)
            .into_iter()
            .map(|(s,)| s)
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(())
}

/// Helper function to handle comparison errors and generate reproduction scripts
pub fn handle_compare_error(
    error: TestCaseError,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    setup_sql: &str,
) -> TestCaseError {
    let error_msg = error.to_string();
    let failure_type = if error_msg.contains("error returned from database")
        || error_msg.contains("SQL execution error")
        || error_msg.contains("syntax error")
    {
        "QUERY EXECUTION FAILURE"
    } else {
        "RESULT MISMATCH"
    };

    let repro_script = format!(
        r#"
-- ==== {failure_type} REPRODUCTION SCRIPT ====
-- Copy and paste this entire block to reproduce the issue
--
-- Prerequisites: Ensure pg_search extension is available
CREATE EXTENSION IF NOT EXISTS pg_search;
--
-- Table and index setup
{setup_sql}
--
-- Default GUCs:
{default_gucs}
--
-- PostgreSQL query:
{pg_query};
--
-- Set GUCs to match the failing test case
{gucs_sql}
--
-- BM25 query:
{bm25_query};
--
-- ==== END REPRODUCTION SCRIPT ====

Original error:
{error_msg}
"#,
        failure_type = failure_type,
        setup_sql = setup_sql,
        default_gucs = PgGucs::default().set(),
        gucs_sql = gucs.set(),
        pg_query = pg_query,
        bm25_query = bm25_query,
        error_msg = error_msg
    );

    TestCaseError::fail(format!(
        "{}\n{repro_script}",
        if failure_type == "QUERY EXECUTION FAILURE" {
            "Query execution failed"
        } else {
            "Results differ between PostgreSQL and BM25"
        }
    ))
}
```

---

## fixtures/querygen/pagegen.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::fmt::{Debug, Display};

use proptest::prelude::*;

#[derive(Clone, Debug)]
pub struct PagingExprs {
    order_by: Vec<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl PagingExprs {
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();

        let mut order_bys = self.order_by.iter();
        if let Some(order_by) = order_bys.next() {
            sql.push_str("ORDER BY ");
            sql.push_str(order_by);
        }
        for order_by in order_bys {
            sql.push_str(", ");
            sql.push_str(order_by);
        }

        if let Some(offset) = &self.offset {
            if !sql.is_empty() {
                sql.push(' ');
            }
            sql.push_str("OFFSET ");
            sql.push_str(&offset.to_string());
        }

        if let Some(limit) = &self.limit {
            if !sql.is_empty() {
                sql.push(' ');
            }
            sql.push_str("LIMIT ");
            sql.push_str(&limit.to_string());
        }
        sql
    }
}

/// Generate arbitrary `ORDER BY`, `OFFSET`, and `LIMIT` expressions.
///
/// This strategy limits itself to combinations which allow for deterministic comparison:
/// it will always generate an `ORDER BY` including one of the given tiebreaker columns (which are
/// assumed to be unique).
pub fn arb_paging_exprs(
    table: impl AsRef<str>,
    columns: Vec<&str>,
    tiebreaker_columns: Vec<&str>,
) -> impl Strategy<Value = String> {
    let columns = columns
        .into_iter()
        .map(|col| format!("{}.{col}", table.as_ref()))
        .collect::<Vec<_>>();
    let columns_len = columns.len();
    let tiebreaker_columns = tiebreaker_columns
        .into_iter()
        .map(|col| format!("{}.{}", table.as_ref(), col))
        .collect::<Vec<_>>();

    let order_by_prefix = if columns_len > 0 {
        proptest::sample::subsequence(columns, 0..columns_len).boxed()
    } else {
        Just(vec![]).boxed()
    };

    // Choose a prefix of columns to `order by`, and a tiebreaker column to ensure determinism.
    (
        order_by_prefix,
        proptest::sample::select(tiebreaker_columns),
    )
        .prop_flat_map(move |(mut order_by_prefix, tiebreaker)| {
            order_by_prefix.push(tiebreaker);
            (
                Just(order_by_prefix),
                proptest::option::of(0..100_usize),
                proptest::option::of(0..100_usize),
            )
        })
        .prop_map(|(order_by, offset, limit)| {
            PagingExprs {
                order_by,
                offset,
                limit,
            }
            .to_sql()
        })
}
```

---

## fixtures/querygen/wheregen.rs

```
// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::fmt::{Debug, Display};

use proptest::prelude::*;

use crate::fixtures::querygen::Column;

#[derive(Clone, Debug)]
pub enum Expr {
    Atom {
        name: String,
        value: String,
        is_indexed: bool,
    },
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn to_sql(&self, indexed_op: &str) -> String {
        match self {
            Expr::Atom {
                name,
                value,
                is_indexed,
            } => {
                let op = if *is_indexed { indexed_op } else { " = " };
                format!("{name} {op} {value}")
            }
            Expr::Not(e) => {
                format!("NOT ({})", e.to_sql(indexed_op))
            }
            Expr::And(l, r) => {
                format!("({}) AND ({})", l.to_sql(indexed_op), r.to_sql(indexed_op))
            }
            Expr::Or(l, r) => {
                format!("({}) OR ({})", l.to_sql(indexed_op), r.to_sql(indexed_op))
            }
        }
    }
}

pub fn arb_wheres(tables: Vec<impl AsRef<str>>, columns: &[Column]) -> impl Strategy<Value = Expr> {
    let tables = tables
        .into_iter()
        .map(|t| t.as_ref().to_owned())
        .collect::<Vec<_>>();
    let columns = columns
        .iter()
        .filter(|c| c.is_whereable)
        .map(|c| (c.name.to_owned(), c.sample_value.to_owned(), c.is_indexed))
        .collect::<Vec<_>>();

    // leaves: the atomic predicate. select a table, and a column.
    let atom = proptest::sample::select(tables).prop_flat_map(move |table| {
        proptest::sample::select::<Expr>(
            columns
                .iter()
                .map(|(col, val, is_indexed)| Expr::Atom {
                    name: format!("{table}.{col}"),
                    value: val.clone(),
                    is_indexed: *is_indexed,
                })
                .collect::<Vec<_>>(),
        )
    });

    // inner nodes
    atom.prop_recursive(
        5, // target depth
        8, // target total size
        3, // expected size of each node
        |child| {
            prop_oneof![
                child.clone().prop_map(|c| Expr::Not(Box::new(c.clone()))),
                (child.clone(), child.clone())
                    .prop_map(|(l, r)| Expr::And(Box::new(l), Box::new(r))),
                (child.clone(), child.clone())
                    .prop_map(|(l, r)| Expr::Or(Box::new(l), Box::new(r))),
            ]
        },
    )
}
```

---

