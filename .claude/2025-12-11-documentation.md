# documentation

## overview.mdx

```
---
title: Overview
description: Introducing ParadeDB's v2 API
canonical: https://docs.paradedb.com/documentation/overview
---

![ParadeDB Banner](/images/paradedb_v2_banner.png)

## Welcome to `v2`

ParadeDB is undergoing a revamp to make its SQL interface more intuitive and ORM-friendly.
This new experience is being introduced as `v2` of the API.

To move back to the v1 API at any time click `Documentation` in the left navigation bar.

`v1` will remain fully supported — there’s no need to change existing integrations.
`v2` focuses on improving developer experience across three key areas:

### Declarative Schema Configuration

No more complex JSON strings to configure tokenizers, fast fields, etc.

### Transparent Search Operators

More intuitive syntax and more transparent behavior, making it easier to understand exactly what's happening just by reading the SQL query.

### ORM-Friendly Query Builders

Queries will support structured column references instead of relying on string literals, improving compatibility with query builders and reducing runtime errors.

## Rollout Timeline

The `v2` API will be rolled out iteratively, with new features and improvements added to this section of the documentation as they land.
During this period, users are encouraged to explore the new features and provide feedback as we refine the interface.

`v2` is targeted to be released with full coverage of the `v1` API by the end of October 2025.
```

---

## filtering.mdx

```
---
title: Filtering
description: Filter search results based on metadata from other fields
canonical: https://docs.paradedb.com/documentation/filtering
---

Adding filters to text search is as simple as using PostgreSQL's built-in `WHERE` clauses and operators.
For instance, the following query filters out results that do not meet `rating > 2`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes' AND rating > 2;
```

## Filter Pushdown

### Non-Text Fields

While not required, filtering performance over non-text columns can be improved by including them in the BM25 index.
When these columns are part of the index, `WHERE` clauses that reference them can be pushed down into the index scan itself.
This can result in faster query execution over large datasets.

For example, if `rating` and `created_at` are frequently used in filters, they can be added to the BM25 index during index creation:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25(id, description, rating, created_at)
WITH (key_field = 'id');
```

Filter pushdown is currently supported for the following combinations of types and operators:

| Operator                                   | Left Operand Type | Right Operand Type | Example                    |
| ------------------------------------------ | ----------------- | ------------------ | -------------------------- |
| `=`, `<`, `>`, `<=`, `>=`, `<>`, `BETWEEN` | `int2`            | `int2`             | `WHERE rating = 2`         |
|                                            | `int4`            | `int4`             |
|                                            | `int8`            | `int8`             |
|                                            | `int2`            | `int4`             |
|                                            | `int2`            | `int8`             |
|                                            | `int4`            | `int8`             |
|                                            | `float4`          | `float4`           |
|                                            | `float8`          | `float8`           |
|                                            | `float4`          | `float8`           |
|                                            | `date`            | `date`             |
|                                            | `time`            | `time`             |
|                                            | `timetz`          | `timetz`           |
|                                            | `timestamp`       | `timestamp`        |
|                                            | `timestamptz`     | `timestamptz`      |
|                                            | `uuid`            | `uuid`             |
| `=`                                        | `bool`            | `bool`             | `WHERE in_stock = true`    |
| `IN`, `ANY`, `ALL`                         | `bool`            | `bool[]`           | `WHERE rating IN (1,2,3)`  |
|                                            | `int2`            | `int2[]`           |
|                                            | `int4`            | `int4[]`           |
|                                            | `int8`            | `int8[]`           |
|                                            | `int2`            | `int4[]`           |
|                                            | `int2`            | `int8[]`           |
|                                            | `int4`            | `int8[]`           |
|                                            | `float4`          | `float4[]`         |
|                                            | `float8`          | `float8[]`         |
|                                            | `float4`          | `float8[]`         |
|                                            | `date`            | `date[]`           |
|                                            | `timetz`          | `timetz[]`         |
|                                            | `timestamp`       | `timestamp[]`      |
|                                            | `timestamptz`     | `timestamptz[]`    |
|                                            | `uuid`            | `uuid[]`           |
| `IS`, `IS NOT`                             | `bool`            | `bool`             | `WHERE in_stock IS true`   |
| `IS NULL`, `IS NOT NULL`                   | `bool`            |                    | `WHERE rating IS NOT NULL` |
|                                            | `int2`            |                    |
|                                            | `int4`            |                    |
|                                            | `int8`            |                    |
|                                            | `int2`            |                    |
|                                            | `int2`            |                    |
|                                            | `int4`            |                    |
|                                            | `float4`          |                    |
|                                            | `float8`          |                    |
|                                            | `float4`          |                    |
|                                            | `date`            |                    |
|                                            | `time`            |                    |
|                                            | `timetz`          |                    |
|                                            | `timestamp`       |                    |
|                                            | `timestamptz`     |                    |
|                                            | `uuid`            |                    |

### Text Fields

Suppose we have a text filter that looks for an exact string match like `category = 'Footwear'`:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ 'shoes' AND category = 'Footwear';
```

To push down the `category = 'Footwear'` filter, `category` must be indexed using the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25(id, description, (category::pdb.literal))
WITH (key_field = 'id');
```

Pushdown of set filters over text fields also requires the literal tokenizer:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ 'shoes' AND category IN ('Footwear', 'Apparel');
```
```

---

## tokenizers/overview.mdx

```
---
title: How Tokenizers Work
description: Tokenizers split large chunks of text into small, searchable units called tokens
canonical: https://docs.paradedb.com/documentation/tokenizers/overview
---

Before text is indexed, it is first split into searchable units called tokens.

The default tokenizer in ParadeDB is the [unicode tokenizer](/documentation/tokenizers/available-tokenizers/unicode). It splits text according to word boundaries defined by the Unicode Standard Annex #29 rules. All characters are lowercased by default. To visualize how this tokenizer works, you can cast a text string to the tokenizer type, and then to `text[]`:

```sql
SELECT 'Hello world!'::pdb.simple::text[];
```

```ini Expected Response
     text
---------------
 {hello,world}
(1 row)
```

On the other hand, the [ngrams](/documentation/tokenizers/available-tokenizers/ngrams) tokenizer splits text into "grams" of size `n`. In this example, `n = 3`:

```sql
SELECT 'Hello world!'::pdb.ngram(3,3)::text[];
```

{/* * codespell:ignore-begin * */}

```ini Expected Response
                      text
-------------------------------------------------
 {hel,ell,llo,"lo ","o w"," wo",wor,orl,rld,ld!}
(1 row)
```

{/* * codespell:ignore-end * */}

Choosing the right tokenizer is crucial to getting the search results you want. For instance, the simple tokenizer works best for whole-word matching like "hello" or "world", while the ngram tokenizer enables partial matching.

To configure a tokenizer for a column in the index, simply cast it to the desired tokenizer type:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.ngram(3,3)))
WITH (key_field='id');
```
```

---

## tokenizers/multiple-per-field.mdx

```
---
title: Multiple Tokenizers Per Field
description: Apply different token configurations to the same field
canonical: https://docs.paradedb.com/documentation/tokenizers/multiple-per-field
---

In many cases, a text field needs to be tokenized multiple ways. For instance, using the [unicode](/documentation/tokenizers/available-tokenizers/unicode)
tokenizer for search, and the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer for [Top N ordering](/documentation/sorting/topn).

To tokenize a field in more than one way, append an `alias=<alias_name>` argument to the additional tokenizer configurations.
The alias name can be any string you like. For instance, the following statement tokenizes `description` using both the simple and literal tokenizers.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (description::pdb.literal),
  (description::pdb.simple('alias=description_simple'))
) WITH (key_field='id');
```

Under the hood, two distinct fields are created in the index: a field called `description`, which uses the literal tokenizer,
and an aliased field called `description_simple`, which uses the simple tokenizer.

To query against the aliased field, cast it to `pdb.alias('alias_name')`:

```sql
-- Query against `description_simple`
SELECT description, rating, category
FROM mock_items
WHERE description::pdb.alias('description_simple') ||| 'Sleek running shoes';

-- Query against `description`
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'Sleek running shoes';
```

<Note>
If a text field uses multiple tokenizers and one of them is [literal](/documentation/tokenizers/available-tokenizers/literal), we recommend aliasing
the other tokenizers and leaving the literal tokenizer un-aliased. This is so queries that `GROUP BY`, `ORDER BY`, or aggregate the
text field can reference the field directly:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (description::pdb.literal),
  (description::pdb.simple('alias=description_simple'))
) WITH (key_field='id');

SELECT description, rating, category
FROM mock_items
WHERE description @@@ 'shoes'
ORDER BY description
LIMIT 5;
```

</Note>
```

---

## tokenizers/available-tokenizers/jieba.mdx

```
---
title: Jieba
description: The most advanced Chinese tokenizer that leverages both a dictionary and statistical models
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/jieba
---

The Jieba tokenizer is a tokenizer for Chinese text that leverages both a dictionary and statistical models. It is generally considered to be better at identifying ambiguous Chinese word boundaries
compared to the [Chinese Lindera](/documentation/tokenizers/available-tokenizers/lindera) and [Chinese compatible](/documentation/tokenizers/available-tokenizers/chinese-compatible) tokenizers, but
the tradeoff is that it is slower.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.jieba))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Hello world! 你好!'::pdb.jieba::text[];
```

```ini Expected Response
              text
--------------------------------
 {hello," ",world,!," ",你好,!}
(1 row)
```
```

---

## tokenizers/available-tokenizers/chinese-compatible.mdx

```
---
title: Chinese Compatible
description: A simple tokenizer for Chinese, Japanese, and Korean characters
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/chinese-compatible
---

The Chinese compatible tokenizer is like the [simple](/documentation/tokenizers/available-tokenizers/simple) tokenizer -- it lowercases non-CJK characters and splits on
any non-alphanumeric character. Additionally, it treats each CJK character as its own token.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.chinese_compatible))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Hello world! 你好!'::pdb.chinese_compatible::text[];
```

```ini Expected Response
        text
---------------------
 {hello,world,你,好}
(1 row)
```
```

---

## tokenizers/available-tokenizers/lindera.mdx

```
---
title: Lindera
description: Uses prebuilt dictionaries to tokenize Chinese, Japanese, and Korean text
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/lindera
---

The Lindera tokenizer is a more advanced CJK tokenizer that uses prebuilt Chinese, Japanese, or Korean dictionaries to break text into meaningful tokens (words or phrases) rather than on individual characters.
Chinese Lindera uses the CC-CEDICT dictionary, Korean Lindera uses the KoDic dictionary, and Japanese Lindera uses the IPADIC dictionary.

By default, non-CJK text is lowercased, but punctuation and whitespace are not ignored.

<CodeGroup>
```sql Chinese Lindera
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.lindera(chinese)))
WITH (key_field='id');
```

```sql Korean Lindera
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.lindera(korean)))
WITH (key_field='id');
```

```sql Japanese Lindera
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.lindera(japanese)))
WITH (key_field='id');
```

</CodeGroup>

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Hello world! 你好!'::pdb.lindera(chinese)::text[];
```

```ini Expected Response
              text
--------------------------------
 {hello," ",world,!," ",你好,!}
(1 row)
```
```

---

## tokenizers/available-tokenizers/regex.mdx

```
---
title: Regex Patterns
description: Tokenizes text using a regular expression
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/regex
---

The `regex_pattern` tokenizer tokenizes text using a regular expression. The regular expression can be specified with the pattern parameter.
For instance, the following tokenizer creates tokens only for words starting with the letter `h`:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.regex_pattern('(?i)\bh\w*')))
WITH (key_field='id');
```

The regex tokenizer uses the Rust [regex](https://docs.rs/regex/latest/regex/) crate, which supports all regex constructs with the following
exceptions:

1. Lazy quantifiers such as `+?`
2. Word boundaries such as `\b`

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Hello world!'::pdb.regex_pattern('(?i)\bh\w*')::text[];
```

```ini Expected Response
  text
---------
 {hello}
(1 row)
```
```

---

## tokenizers/available-tokenizers/literal.mdx

```
---
title: Literal
description: Indexes the text in its raw form, without any splitting or processing
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/literal
---

<Note>
  The literal tokenizer is not ideal for text search queries like
  [match](/documentation/full-text/match) or
  [phrase](/documentation/full-text/phrase). If you need to do text search over
  a field that is literal tokenized, consider using [multiple
  tokenizers](/documentation/tokenizers/multiple-per-field).
</Note>

<Note>
  Because the literal tokenizer preserves the source text exactly, [token
  filters](/documentation/token-filters/overview) cannot be configured for this
  tokenizer.
</Note>

The literal tokenizer applies no tokenization to the text, preserving it as-is. It is the default for `uuid` fields (since
exact UUID matching is a common use case), and is useful for doing exact string matching over text fields.

It is also required if the text field is used as a sort field in a [Top N](/documentation/sorting/topn) query,
or as part of an [aggregate](/documentation/aggregates/overview).

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.literal))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Tokenize me!'::pdb.literal::text[];
```

```ini Expected Response
       text
------------------
 {"Tokenize me!"}
(1 row)
```
```

---

## tokenizers/available-tokenizers/unicode.mdx

```
---
title: Unicode
description: The default text tokenizer in ParadeDB
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/unicode
---

The unicode tokenizer splits text according to word boundaries defined by the [Unicode Standard Annex #29](https://www.unicode.org/reports/tr29/)
rules. All characters are [lowercased](/documentation/token-filters/lowercase) by default.

This tokenizer is the default text tokenizer. If no tokenizer is specified for a text field, the unicode tokenizer will be used
(unless the text field is the [key field](/documentation/indexing/create-index#choosing-a-key-field), in which case the text is not tokenized).

```sql
-- The following two configurations are equivalent
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description)
WITH (key_field='id');

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.unicode_words))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Tokenize me!'::pdb.unicode_words::text[];
```

```ini Expected Response
     text
---------------
 {tokenize,me}
(1 row)
```
```

---

## tokenizers/available-tokenizers/source-code.mdx

```
---
title: Source Code
description: Tokenizes text that is actually code
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/source-code
---

The source code tokenizer is intended for tokenizing code. In addition to splitting on whitespace,
punctuation, and symbols, it also splits on common casing conventions like camel case and snake case. For instance, text like
`my_variable` or `myVariable` would get split into `my` and `variable`.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.source_code))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'let my_variable = 2;'::pdb.source_code::text[];
```

```ini Expected Response
        text
---------------------
 {let,my,variable,2}
(1 row)
```
```

---

## tokenizers/available-tokenizers/whitespace.mdx

```
---
title: Whitespace
description: Tokenizes text by splitting on whitespace
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/whitespace
---

The whitespace tokenizer splits only on whitespace. It also [lowercases](/documentation/token-filters/lowercase) characters by default.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.whitespace))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Tokenize me!'::pdb.whitespace::text[];
```

```ini Expected Response
      text
----------------
 {tokenize,me!}
(1 row)
```
```

---

## tokenizers/available-tokenizers/ngrams.mdx

```
---
title: Ngram
description: Splits text into small chunks called grams, useful for partial matching
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/ngrams
---

The ngram tokenizer splits text into "grams," where each "gram" is of a certain length.

The tokenizer takes two arguments. The first is the minimum character length of a "gram," and the second is the maximum character length. Grams will be generated for all sizes between
the minimum and maximum gram size, inclusive. For example, `pdb.ngram(2,5)` will generate tokens of size `2`, `3`, `4`, and `5`.

To generate grams of a single fixed length, set the minimum and maximum gram size equal to each other.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.ngram(3,3)))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Tokenize me!'::pdb.ngram(3,3)::text[];
```

```ini Expected Response
                      text
-------------------------------------------------
 {tok,oke,ken,eni,niz,ize,"ze ","e m"," me",me!}
(1 row)
```
```

---

## tokenizers/available-tokenizers/literal-normalized.mdx

```
---
title: Literal Normalized
description: Like the literal tokenizer, but allows for token filters
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/literal-normalized
---

The literal normalized tokenizer is similar to the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer in that it does not split the source text.
All text is treated as a single token, regardless of how many words are contained.

However, unlike the literal tokenizer, this tokenizer allows [token filters](/documentation/token-filters/overview) to be applied. By default, the literal normalized tokenizer
also [lowercases](/documentation/token-filters/lowercase) the text.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.literal_normalized))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Tokenize me!'::pdb.literal_normalized::text[];
```

```ini Expected Response
       text
------------------
 {"tokenize me!"}
(1 row)
```
```

---

## tokenizers/available-tokenizers/simple.mdx

```
---
title: Simple
description: Splits on any non-alphanumeric character
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/simple
---

The simple tokenizer splits on any non-alphanumeric character (e.g. whitespace, punctuation, symbols). All characters are
[lowercased](/documentation/token-filters/lowercase) by default.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Tokenize me!'::pdb.simple::text[];
```

```ini Expected Response
     text
---------------
 {tokenize,me}
(1 row)
```
```

---

## tokenizers/available-tokenizers/icu.mdx

```
---
title: ICU
description: Splits text according to the Unicode standard
canonical: https://docs.paradedb.com/documentation/tokenizers/available-tokenizers/icu
---

The ICU (International Components for Unicode) tokenizer breaks down text according to the Unicode standard. It can be used to tokenize most languages and recognizes the nuances in word boundaries across different languages.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.icu))
WITH (key_field='id');
```

To get a feel for this tokenizer, run the following command and replace the text with your own:

```sql
SELECT 'Hello world! 你好!'::pdb.icu::text[];
```

```ini Expected Response
        text
--------------------
 {hello,world,你好}
(1 row)
```
```

---

## performance-tuning/overview.mdx

```
---
title: How to Tune ParadeDB
description: Settings for better read and write performance
canonical: https://docs.paradedb.com/documentation/performance-tuning/overview
---

ParadeDB uses Postgres' settings, which can be found in the `postgresql.conf` file. To find your `postgresql.conf` file, use `SHOW`.

```sql
SHOW config_file;
```

These settings can be changed in several ways:

1. By editing the `postgresql.conf` file and restarting Postgres. This makes the setting permanent for all sessions. `postgresql.conf`
   accepts ParadeDB's custom `paradedb.*` settings.
2. By running `SET`. This temporarily changes the setting for the current session. Note that Postgres does not allow all `postgresql.conf` settings to be changed with `SET`.

```sql
SET maintenance_work_mem = '8GB'
```

If ParadeDB is deployed with [CloudNativePG](/deploy/self-hosted/kubernetes), these settings should be set in your
`.tfvars` file.

```hcl .tfvars
postgresql = {
    parameters = {
      max_worker_processes                   = 76
      max_parallel_workers                   = 64
      # Note that paradedb.* settings must be wrapped in double quotes
      "paradedb.global_mutable_segment_rows" = 1000
    }
}
```
```

---

## performance-tuning/writes.mdx

```
---
title: Write Throughput
description: Settings to improve write performance
canonical: https://docs.paradedb.com/documentation/performance-tuning/writes
---

These actions can improve the throughput of `INSERT`/`UPDATE`/`COPY` statements to the BM25 index.

## Ensure Merging Happens in the Background

During every `INSERT`/`UPDATE`/`COPY`/`VACUUM`, the BM25 index runs a compaction process that looks for opportunities to merge segments
together. The goal is to consolidate smaller segments into larger ones, reducing the total number of segments and improving query performance.

Segments become candidates for merging if their combined size meets or exceeds one of several **configurable layer thresholds**. These thresholds define target
segment sizes — such as `10KB`, `100KB`, `1MB`, etc. For each layer, the compactor checks if there are enough smaller segments whose total size adds up to the threshold.

The default layer sizes are `100KB`, `1MB`, `100MB`, `1GB`, and `10GB` but can be configured.

```sql
ALTER INDEX search_idx SET (background_layer_sizes = '100MB, 1GB');
```

By default, merging happens in the background so that writes are not blocked. The `layer_sizes` option allows merging to happen in the foreground.
This is not typically recommended because it slows down writes, but can be used to apply back pressure to writes if segments are being created faster
than they can be merged down.

```sql
ALTER INDEX search_idx SET (layer_sizes = '100KB, 1MB');
```

Setting `layer_sizes` to `0` disables foreground merging, and setting `background_layer_sizes` to `0` disables background merging.

## Increase Work Memory for Bulk Updates

`work_mem` controls how much memory to allocate to a single `INSERT`/`UPDATE`/`COPY` statement. Each statement that writes to a BM25 index is required to have at least `15MB` memory. If
`work_mem` is below `15MB`, it will be ignored and `15MB` will be used.

If your typical update patterns are large, bulk updates (not single-row updates) a larger value may be better.

```sql
SET work_mem = 64MB;
```

Since many write operations can be running concurrently, this value should be raised more conservatively than `maintenance_work_mem`.

## Increase Mutable Segment Size

The `mutable_segment_rows` setting enables use of mutable segments, which buffer new rows in order to amortize the cost of indexing them.
By default, it is set to `1000`, which means that 1000 writes are buffered before being flushed.

```sql
ALTER INDEX search_idx SET (mutable_segment_rows = 1000);
```

A higher value generally improves write throughput at the expense of read performance,
since the mutable data structure is slower to search. Additionally, the mutable data structure is read into
memory, so higher values cause reads to consume more RAM.

Alternatively, this setting can be set to apply to all indexes in the database:

```sql
SET paradedb.global_mutable_segment_rows = 1000
```

If both a per-index setting and global setting exist, the global `paradedb.global_mutable_segment_rows` will be used.
To ignore the global setting, set `paradedb.global_mutable_segment_rows` to `-1` (this is the default).

```sql
SET paradedb.global_mutable_segment_rows = -1
```
```

---

## performance-tuning/reads.mdx

```
---
title: Read Throughput
description: Settings to improve read performance
canonical: https://docs.paradedb.com/documentation/performance-tuning/reads
---

As a general rule of thumb, the performance of expensive search queries can be greatly improved
if they are able to access more parallel Postgres workers and more shared buffer memory.

## Raise Parallel Workers

There are three settings that control how many parallel workers ultimately get assigned to a query.

First, `max_worker_processes` is a global limit for the number of workers.
Next, `max_parallel_workers` is a subset of `max_worker_processes`, and sets the limit for workers used in
parallel queries. Finally, `max_parallel_workers_per_gather` limits how many workers a _single query_ can receive.

```init postgresql.conf
max_worker_processes = 72
max_parallel_workers = 64;
max_parallel_workers_per_gather = 4;
```

In the above example, the maximum number of workers that a single query can receive is set to `4`. The `max_parallel_workers` pool
is set to `64`, which means that `16` queries can execute simultaneously with `4` workers each. Finally, `max_worker_processes` is
set to `72` to give headroom for other workers like autovacuum and replication.

In practice, we recommend experimenting with different settings, as the best configuration depends on the underlying hardware,
query patterns, and volume of data.

<Note>
  If all `max_parallel_workers` are in use, Postgres will still execute
  additional queries, but those queries will run without parallelism. This means
  that queries do not fail — they just may run slower due to lack of
  parallelism.
</Note>

## Raise Shared Buffers

`shared_buffers` controls how much memory is available to the Postgres buffer cache. We recommend allocating no more than 40% of total memory
to `shared_buffers`.

```bash postgresql.conf
shared_buffers = 8GB
```

The `pg_prewarm` extension can be used to load the BM25 index into the buffer cache after Postgres restarts. A higher `shared_buffers` value allows more of the index to be
stored in the buffer cache.

```sql
CREATE EXTENSION pg_prewarm;
SELECT pg_prewarm('search_idx');
```

## Configure Autovacuum

If an index experiences frequent writes, the search performance of some queries like [sorting](/documentation/sorting/score) or
[aggregates](/documentation/aggregates/overview) can degrade if `VACUUM` has not been recently run. This is because writes can cause parts of Postgres' visibility map
to go out of date, and `VACUUM` updates the visibility map.

To determine if search performance is degraded by lack of `VACUUM`, run `EXPLAIN ANALYZE` over a query. A `Parallel Custom Scan`
in the query plan with a large number of `Heap Fetches` typically means that `VACUUM` should be run.

Postgres can be configured to automatically vacuum a table when a certain number of rows have been updated. Autovacuum settings
can be set globally in `postgresql.conf` or for a specific table.

```sql
ALTER TABLE mock_items SET (autovacuum_vacuum_threshold = 500);
```

There are several [autovacuum settings](https://www.postgresql.org/docs/current/routine-vacuuming.html#AUTOVACUUM), but the important ones to
note are:

1. `autovacuum_vacuum_scale_factor` triggers an autovacuum if a certain percentage of rows in a table have been updated.
2. `autovacuum_vacuum_threshold` triggers an autovacuum if an absolute number of rows have been updated.
3. `autovacuum_naptime` ensures that vacuum does not run too frequently.

This means that setting `autovacuum_vacuum_scale_factor` to `0` and `autovacuum_vacuum_threshold` to `100000` will trigger an autovacuum
for every `100000` row updates. As a general rule of thumb, we recommend autovacuuming at least once every `100000` single-row updates.

## Adjust Target Segment Count

By default, `CREATE INDEX`/`REINDEX` will create as many segments as there are CPUs on the host machine. This can be changed using the
`target_segment_count` index option.

```sql
CREATE INDEX search_idx ON mock_items USING bm25 (id, description, rating) WITH (key_field = 'id', target_segment_count = 32, ...);
```

This property is attached to the index so that during `REINDEX`, the same value will be used.

It can be changed with ALTER INDEX, like so:

```sql
ALTER INDEX search_idx SET (target_segment_count = 8);
```

However, a `REINDEX` is required to rebalance the index to that segment count.

For optimal performance, the segment count should equal the number of parallel workers that a query can receive, which is controlled by
[`max_parallel_workers_per_gather`](/documentation/performance-tuning/reads#raise-parallel-workers). If `max_parallel_workers_per_gather` is greater than the number of CPUs on the host machine, then increasing the target segment count to match `max_parallel_workers_per_gather` can improve query
performance.

<Note>
`target_segment_count` is merely a suggestion.

While `pg_search` will endeavor to ensure the created index will have exactly this many segments, it is possible for it
to have less or more. Mostly this depends on the distribution of work across parallel builder processes, memory
constraints, and table size.

</Note>
```

---

## performance-tuning/joins.mdx

```
---
title: Joins
description: Optimize JOIN queries in ParadeDB
canonical: https://docs.paradedb.com/documentation/performance-tuning/joins
---

ParadeDB supports all PostgreSQL JOIN types and extends them with BM25-powered full-text search. This guide explains how JOINs behave with search, how to identify sub-optimal query plans, and offers strategies to keep queries fast.

## Supported JOIN Types

ParadeDB supports all PostgreSQL JOINs:

- `INNER JOIN`
- `LEFT / RIGHT / FULL OUTER JOIN`
- `CROSS JOIN`
- `LATERAL`
- Semi and Anti JOINs

For the most part you can mix search and relational queries without changing your SQL.

## Scoring in JOINs

When using `paradedb.score()` or `paradedb.snippet()` inside JOINs:

- Scores and snippets are computed **before the JOIN** at the base table level.
- JOIN conditions never change the score, they only determine which rows are combined.

This design keeps scores predictable and consistent across queries.

## Performance Characteristics

### Fast Cases

Queries are efficient when search filters can be applied directly to the underlying tables.
In these cases, PostgreSQL can push down the `|||` operators so that each table does its own filtered index scan before the JOIN runs.

That means:

- Each table only contributes rows that already match the search condition.
- The JOIN operates on much smaller intermediate sets.

In this query, both `a.bio` and `b.content` are filtered independently.
The planner runs efficient index scans on each table and then joins the results.

```sql
SELECT a.name, b.title, paradedb.score(a.id)
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE
    a.bio ||| 'science fiction'
    AND b.content ||| 'space travel';
```

The plan will have this shape:

```
Gather
  -> Parallel Hash Join
       Hash Cond: (b.id = a.id)
       -> Parallel ParadeDB Scan on authors a
       -> Parallel Hash
            -> Parallel ParadeDB Scan on books b
```

### Slower Cases

Queries become slower when search conditions span multiple tables in a way that prevents PostgreSQL from pushing them down. The most common example is an `OR` across different tables:

```sql
SELECT a.name, b.title
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE
    a.bio ||| 'science'
    OR b.content ||| 'artificial';
```

Because the condition references both `a` and `b`, PostgreSQL cannot apply it until after the join. As a result, both tables must be scanned in full, joined, and only then filtered.

The plan will have this shape:

```
Gather
  -> Parallel Hash Join
       Hash Cond: (a.id = b.author_id)
       Join Filter: (a.bio ||| (...) OR b.content ||| (...))
       -> Parallel Seq Scan on authors a
       -> Parallel Hash
            -> Parallel Seq Scan on books b
```

Note that the `|||` query is in the _Join Filter_, not in the scan.

## Diagnosing Performance

Use `EXPLAIN` to check the query plan:

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT a.name, b.title, paradedb.score(a.id)
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE a.bio ||| 'science'
   OR b.content ||| 'artificial';
```

Watch for:

- `Custom Scan` nodes with large row counts
- ParadeDB operators inside JOIN conditions
- `Tantivy Query: all` (full index scan)

## Writing Faster JOIN Queries

### Replace Cross-Table OR with UNION

If you don’t need scores/snippets and have a simple JOIN, express the OR as a UNION of two separately filtered joins. This lets PostgreSQL push each search predicate down to a Custom Index Scan and avoid a join-time filter.

```sql
SELECT a.name, b.title
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE a.bio ||| 'science'
UNION
SELECT a.name, b.title
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE b.content ||| 'artificial';
```

### Use CTEs for Complex Queries

Use common table expressions (CTEs) to pre-filter each table with its own search condition, then join the smaller result sets together.
If possible, add a `LIMIT` to each CTE to keep the result sets small.

```sql
WITH matching_authors AS (
  SELECT id, name, paradedb.score(id) AS author_score
  FROM authors
  WHERE bio ||| 'science'
  LIMIT 100
),
matching_books AS (
  SELECT id, title, author_id, paradedb.score(id) AS book_score
  FROM books
  WHERE content ||| 'artificial'
  LIMIT 100
)
SELECT
  COALESCE(ma.name, a.name) AS name,
  COALESCE(mb.title, b.title) AS title,
  ma.author_score,
  mb.book_score
FROM matching_authors ma
FULL JOIN matching_books mb ON ma.id = mb.author_id
LEFT JOIN authors a ON mb.author_id = a.id AND ma.id IS NULL
LEFT JOIN books b ON ma.id = b.author_id AND mb.id IS NULL;
```

BM25 scores should not be added, if you want to combine scores then consider using [reciprocal rank fusion (RRF)](https://www.paradedb.com/learn/search-concepts/reciprocal-rank-fusion).

## Roadmap

We really want to remove the need to think about the way to do `JOIN`s in ParadeDB. At the moment we are actively working on:

- A `CustomScan Join API` for native join handling
- Smarter cost estimation for the PostgreSQL planner
```

---

## performance-tuning/create-index.mdx

```
---
title: Index Creation
description: Settings to make index creation faster
canonical: https://docs.paradedb.com/documentation/performance-tuning/create-index
---

These actions can improve the performance and memory consumption of `CREATE INDEX` and `REINDEX` statements.

### Raise Parallel Indexing Workers

ParadeDB uses Postgres' `max_parallel_maintenance_workers` setting to determine the degree of parallelism during `CREATE INDEX`/`REINDEX`. Postgres' default is `2`, which may be too low for large tables.

```sql
SET max_parallel_maintenance_workers = 8;
```

In order for `max_parallel_maintenance_workers` to take effect, it must be less than or equal to both `max_parallel_workers` and `max_worker_processes`.

### Configure Indexing Memory

The default Postgres `maintenance_work_mem` value of `64MB` is quite conservative and can slow down parallel index builds. We recommend at least `64MB` per
[parallel indexing worker](#raise-parallel-indexing-workers).

```sql
SET maintenance_work_mem = '2GB';
```

<Note>
  Each worker is required to have at least `15MB` memory. If
  `maintenance_work_mem` is set too low, an error will be returned.
</Note>

### Defer Index Creation

If possible, creating the BM25 index should be deferred until **after** a table has been populated. To illustrate:

```sql
-- This is preferred
CREATE TABLE test (id SERIAL, data text);
INSERT INTO test (data) VALUES ('hello world'), ('many more values');
CREATE INDEX ON test USING bm25 (id, data) WITH (key_field = 'id');

-- ...to this
CREATE TABLE test (id SERIAL, data text);
CREATE INDEX ON test USING bm25 (id, data) WITH (key_field = 'id');
INSERT INTO test (data) VALUES ('hello world'), ('many more values');
```

This allows the BM25 index to create a more tightly packed, efficient representation on disk and will lead to faster build times.
```

---

## aggregates/facets.mdx

```
---
title: Facets
description: Compute a Top N and aggregate in one query
canonical: https://docs.paradedb.com/documentation/aggregates/facets
---

A common pattern in search is to query for both an aggregate and a set of search results. For example, "find the top 10
results, and also count the total number of results."

Instead of issuing two separate queries -- one for the search results, and another for the aggregate -- `pdb.agg` allows for
these results to be returned in a single "faceted" query. This can significantly improve read throughput, since issuing a single
query uses less CPU and disk I/O.

For example, this query returns the top 3 search results alongside the total number of results found.

```sql
SELECT
  id, description, rating,
  pdb.agg('{"value_count": {"field": "id"}}') OVER ()
FROM mock_items
WHERE category === 'electronics'
ORDER BY rating DESC
LIMIT 3;
```

```ini Expected Response
 id |         description         | rating |      agg
----+-----------------------------+--------+----------------
 12 | Innovative wireless earbuds |      5 | {"value": 5.0}
  1 | Ergonomic metal keyboard    |      4 | {"value": 5.0}
  2 | Plastic Keyboard            |      4 | {"value": 5.0}
(3 rows)
```

<Note>
  Faceted queries require that `pdb.agg` be used as a window function:
  `pdb.agg() OVER ()`.
</Note>

### Faceted Performance Optimization

On every query, ParadeDB runs checks to ensure that deleted or updated-away rows are not factored into the result set.

If your table is not frequently updated or you can tolerate an approximate result, the performance of faceted queries can be improved by disabling these visibility checks.
To do so, set the second argument of `pdb.agg` to `false`.

```sql
SELECT
     description, rating, category,
     pdb.agg('{"value_count": {"field": "id"}}', false) OVER ()
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY rating
LIMIT 5;
```

Disabling this check can improve query times by 2-4x in some cases (at the expense of correctness).
```

---

## aggregates/overview.mdx

```
---
title: Aggregate Syntax
description: Accelerate aggregates with the ParadeDB index
canonical: https://docs.paradedb.com/documentation/aggregates/overview
---

The `pdb.agg` function accepts an Elasticsearch-compatible JSON aggregate query string. It executes the aggregate using the
[columnar](/welcome/architecture#columnar-index) portion of the ParadeDB index, which can significantly accelerate performance compared to vanilla Postgres.

For example, the following query counts the total number of results for a search query.

```sql
SELECT pdb.agg('{"value_count": {"field": "id"}}')
FROM mock_items
WHERE category === 'electronics';
```

```ini Expected Response
      agg
----------------
 {"value": 5.0}
(1 row)
```

This query counts the number of results for every distinct group:

```sql
SELECT rating, pdb.agg('{"value_count": {"field": "id"}}')
FROM mock_items
WHERE category === 'electronics'
GROUP BY rating
ORDER BY rating
LIMIT 5;
```

```ini Expected Response
 rating |      agg
--------+----------------
      3 | {"value": 1.0}
      4 | {"value": 3.0}
      5 | {"value": 1.0}
(3 rows)
```

## Multiple Aggregations

To compute multiple aggregations at once, simply include multiple `pdb.agg` functions in the target list:

```sql
SELECT
  pdb.agg('{"avg": {"field": "rating"}}') AS avg_rating,
  pdb.agg('{"value_count": {"field": "id"}}') AS count
FROM mock_items
WHERE category === 'electronics';
```

```ini Expected Response
   avg_rating   |     count
----------------+----------------
 {"value": 4.0} | {"value": 5.0}
(1 row)
```

## JSON Fields

If `metadata` is a JSON field with key `color`, use `metadata.color` as the field name:

```sql
SELECT pdb.agg('{"terms": {"field": "metadata.color"}}')
FROM mock_items
WHERE id @@@ pdb.all();
```

<Note>
If a text or JSON field is used inside `pdb.agg`, it must use the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.
</Note>
```

---

## aggregates/tantivy.mdx

```
---
title: Tantivy Aggregates
description: Run Tantivy JSON aggregations over ParadeDB BM25 indexes
noindex: true
---

In addition to plain SQL aggregates, ParadeDB also has the ability to compute aggregates over a single BM25 index by accepting JSON query strings.

These aggregates can be more performant than plain SQL aggregates over some datasets.

## Syntax

`paradedb.aggregate` accepts three arguments: the name of the BM25 index, a full text search query builder function,
and a Tantivy aggregate JSON.

```sql
SELECT * FROM paradedb.aggregate(
    '<index_name>',
    <search_query>,
    '<aggregate_query>'
);
```

<ParamField body="index_name" required>
  The name of the BM25 index as a string.
</ParamField>
<ParamField body="search_query" required>
  A full text search query builder function. The aggregate will be computed over
  the results of this function.
</ParamField>
<ParamField body="aggregate_query" required>
  A Tantivy aggregate JSON string. See the sections below for how to construct
  these JSONs.
</ParamField>

## Count

A count aggregation tallies the number of values for the specified field across all documents.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "rating_total": {
            "value_count": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the count on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Average

An average aggregation calculates the mean of the specified numeric field values across all documents.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "avg_rating": {
            "avg": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the average on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Sum

A sum aggregation computes the total sum of the specified numeric field values across all documents.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "rating_total": {
            "sum": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the sum on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Min

A min aggregation finds the smallest value for the specified numeric field across all documents.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "min_rating": {
            "min": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the minimum on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Max

A max aggregation finds the largest value for the specified numeric field across all documents.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "max_rating": {
            "max": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the maximum on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Stats

A stats aggregation provides a collection of statistical metrics for the specified numeric field, including count, sum, average, min, and max.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "rating_stats": {
            "stats": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the stats on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Percentiles

The percentiles aggregation calculates the values below which given percentages of the data fall, providing insights into the distribution of a dataset.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "rating_percentiles": {
            "percentiles": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the percentiles on.
</ParamField>
<ParamField body="percents" default={[1.0, 5.0, 25.0, 50.0, 75.0, 95.0, 99.0]}>
  The percentiles to compute.
</ParamField>
<ParamField body="keyed" default={false}>
  Whether to return the percentiles as a hash map.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Cardinality

A cardinality aggregation estimates the number of unique values in the specified field using the HyperLogLog++ algorithm.
This is useful for understanding the uniqueness of values in a large dataset.

<Note>
  The cardinality aggregation provides an approximate count, which is accurate
  within a small error range. This trade-off allows for efficient computation
  even on very large datasets.
</Note>

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "unique_users": {
            "cardinality": {"field": "user_id", "missing": "unknown"}
        }
    }'
);
```

<ParamField body="field" required>
  The field name to compute the cardinality on.
</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field. By default, missing values
  are ignored.
</ParamField>

## Histogram

Histogram is a bucket aggregation where buckets are created dynamically based on a specified interval. Each document value is rounded down to its bucket. For example, if you have a price of 18 and an interval of 5, the document will fall into the bucket with the key 15. The formula used for this is: `((val - offset) / interval).floor() * interval + offset`.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "rating_histogram": {
            "histogram": {"field": "rating", "interval": 1}
        }
    }'
);
```

<ParamField body="field" required>
  The field to aggregate on.
</ParamField>
<ParamField body="interval" required>
  The interval to chunk your data range. Each bucket spans a value range of
  [0..interval). Must be a positive value.
</ParamField>
<ParamField body="offset" default={0.0}>
  Shift the grid of buckets by the specified offset.
</ParamField>
<ParamField body="min_doc_count" default={0}>
  The minimum number of documents in a bucket to be returned.
</ParamField>
<ParamField body="hard_bounds">
  Limits the data range to [min, max] closed interval.
</ParamField>
<ParamField body="extended_bounds">
  Extends the value range of the buckets.
</ParamField>
<ParamField body="keyed" default={false}>
  Whether to return the buckets as a hash map.
</ParamField>
<ParamField body="is_normalized_to_ns" default={false}>
  Whether the values are normalized to ns for date time values.
</ParamField>

## Date Histogram

Similar to histogram, but can only be used with datetime types. Currently, only fixed time intervals are supported.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "created_at_histogram": {
            "date_histogram": {"field": "created_at", "fixed_interval": "1h"}
        }
    }'
);
```

<ParamField body="field" required>
  The field to aggregate on.
</ParamField>
<ParamField body="fixed_interval" required>
  The interval to chunk your data range. Each bucket spans a value range of
  [0..fixed_interval). Accepted values should end in `ms`, `s`, `m`, `h`, or
  `d`.
</ParamField>
<ParamField body="offset" default={0}>
  Shift the grid of buckets by the specified offset.
</ParamField>
<ParamField body="min_doc_count" default={0}>
  The minimum number of documents in a bucket to be returned.
</ParamField>
<ParamField body="hard_bounds">
  Limits the data range to [min, max] closed interval.
</ParamField>
<ParamField body="extended_bounds">
  Extends the value range of the buckets.
</ParamField>
<ParamField body="keyed" default={false}>
  Whether to return the buckets as a hash map.
</ParamField>

## Range

Range allows you to define custom buckets for specific ranges.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "ranges": {
            "range": {"field": "rating", "ranges": [
                { "to": 3.0 },
                { "from": 3.0, "to": 7.0 },
                { "from": 7.0, "to": 20.0 },
                { "from": 20.0 }
            ]}
        }
    }'
);
```

<ParamField body="field" required>
  The field to aggregate on.
</ParamField>
<ParamField body="ranges" required>
  A list of ranges to aggregate on.
</ParamField>
<ParamField body="keyed" default={false}>
  Whether to return the buckets as a hash map.
</ParamField>

## Terms

Terms creates a bucket for every unique term and counts the number of occurrences.

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "rating_terms": {
            "terms": {"field": "rating"}
        }
    }'
);
```

<ParamField body="field" required>
  The field to aggregate on.
</ParamField>
<ParamField body="size" default={10}>
  The number of terms to return.
</ParamField>
<ParamField body="segment_size" default={100}>
  The number of terms to fetch from each segment.
</ParamField>
<ParamField body="show_term_doc_count_error" default={false}>
  Whether to include the document count error.
</ParamField>
<ParamField body="min_doc_count" default={1}>
  The minimum number of documents in a term to be returned.
</ParamField>
<ParamField body="order">The order in which to return the terms.</ParamField>
<ParamField body="missing">
  The value to use for documents missing the field.
</ParamField>

## Nested Aggregations

Buckets can contain sub-aggregations. For example, creating buckets with the range aggregation and then calculating the average on each bucket:

```sql
SELECT * FROM paradedb.aggregate(
    'search_idx',
    paradedb.all(),
    '{
        "range_rating": {
            "range": {
            "field": "rating",
            "ranges": [
                { "from": 1, "to": 3 },
                { "from": 3, "to": 5 }
            ]
            },
            "aggs": {
            "average_in_range": { "avg": { "field": "rating"} }
            }
        }
    }'
);
```
```

---

## aggregates/limitations.mdx

```
---
title: Limitations
description: Caveats for aggregate support
canonical: https://docs.paradedb.com/documentation/aggregates/limitations
---

## ParadeDB Operator

In order for ParadeDB to push down an aggregate, a ParadeDB text search operator must be present in the query.

```sql
-- Not pushed down
SELECT COUNT(*) FROM mock_items
WHERE rating = 5;

-- Pushed down
SELECT COUNT(*) FROM mock_items
WHERE rating = 5
AND id @@@ pdb.all();
```

If your query does not contain a ParadeDB operator, a way to "force" aggregate pushdown is to append the [all query](/documentation/query-builder/compound/all) to the query's
`WHERE` clause.

## Join Support

ParadeDB is currently only able to push down aggregates over a single table. JOINs are not yet pushed down but are on the [roadmap](/welcome/roadmap).
```

---

## aggregates/tuning.mdx

```
---
title: Performance Tuning
description: Several settings can be tuned to improve the performance of aggregates in ParadeDB
canonical: https://docs.paradedb.com/documentation/aggregates/tuning
---

### Configure Parallel Workers

ParadeDB uses Postgres parallel workers. By default, Postgres allows two workers per parallel query.
Increasing the number of [parallel workers](/documentation/performance-tuning/reads) allows parallel queries to use all of the available hardware on the host machine and can deliver significant
speedups.

### Run `VACUUM`

`VACUUM` updates the table's [visibility map](https://www.postgresql.org/docs/current/storage-vm.html),
which speeds up Postgres' visibility checks.

```sql
VACUUM mock_items;
```

If the table experiences frequent updates, we recommend configuring [autovacuum](https://www.postgresql.org/docs/current/routine-vacuuming.html).

### Run `pg_prewarm`

The `pg_prewarm` extension can be used to preload data from the index into the Postgres buffer cache, which
improves the response times of "cold" queries (i.e. the first search query after Postgres has restarted).

```sql
CREATE EXTENSION pg_prewarm;
SELECT pg_prewarm('search_idx');
```
```

---

## aggregates/metrics/average.mdx

```
---
title: Average
description: Compute the average value of a field
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/average
---

The following query computes the average value over a specific field:

```sql
SELECT pdb.agg('{"avg": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
              agg
-------------------------------
 {"value": 3.8536585365853657}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.AverageAggregation.html) for all available options.

## SQL Average Syntax

SQL's `AVERAGE` syntax is supported in beta. To enable it, first run

```sql
SET paradedb.enable_aggregate_custom_scan TO on;
```

With this feature enabled, the following query is equivalent to the above and is executed in the same way.

```sql
SELECT AVG(rating) FROM mock_items
WHERE id @@@ pdb.all();
```

By default, `AVG` ignores null values. Use `COALESCE` to include them in the final average:

```sql
SELECT AVG(COALESCE(rating, 0)) FROM mock_items
WHERE id @@@ pdb.all();
```
```

---

## aggregates/metrics/cardinality.mdx

```
---
title: Cardinality
description: Compute the number of distinct values in a field
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/cardinality
---

The cardinality aggregation estimates the number of distinct values in a field.

```sql
SELECT pdb.agg('{"cardinality": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
      agg
----------------
 {"value": 5.0}
(1 row)
```

Unlike SQL's `DISTINCT` clause, which returns an exact value but is very computationally expensive, the cardinality aggregation uses the HyperLogLog++ algorithm to
closely approximate the number of distinct values.

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CardinalityAggregationReq.html) for all available options.
```

---

## aggregates/metrics/minmax.mdx

```
---
title: Min/Max
description: Compute the min/max value of a field
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/minmax
---

`min` and `max` return the smallest and largest values of a column, respectively.

<CodeGroup>
```sql Min
SELECT pdb.agg('{"min": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```sql Max
SELECT pdb.agg('{"max": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

</CodeGroup>

<CodeGroup>
```ini Expected Response (Min)
      agg
----------------
 {"value": 1.0}
(1 row)
```

```ini Expected Response (Max)
      agg
----------------
 {"value": 5.0}
(1 row)
```

</CodeGroup>

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.MinAggregation.html) for all available options.

## SQL Min/Max Syntax

SQL's `MIN`/`MAX` syntax is supported in beta. To enable it, first run

```sql
SET paradedb.enable_aggregate_custom_scan TO on;
```

With this feature enabled, the following query is equivalent to the above and is executed in the same way.

<CodeGroup>
```sql Min
SELECT MIN(rating) FROM mock_items
WHERE id @@@ pdb.all();
```

```sql Max
SELECT MAX(rating) FROM mock_items
WHERE id @@@ pdb.all();
```

</CodeGroup>

By default, `MIN`/`MAX` ignore null values. Use `COALESCE` to include them in the final sum:

```sql
SELECT MIN(COALESCE(rating, 0)) FROM mock_items
WHERE id @@@ pdb.all();
```
```

---

## aggregates/metrics/percentiles.mdx

```
---
title: Percentiles
description: Analyze the distribution of a field
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/percentiles
---

The percentiles aggregation computes the values below which a given percentage of the data falls.
In this example, the aggregation will return the 50th and 95th percentiles for `rating`.

```sql
SELECT pdb.agg('{"percentiles": {"field": "rating", "percents": [50, 95]}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
                                 agg
---------------------------------------------------------------------
 {"values": {"50.0": 4.014835333028612, "95.0": 5.0028295751107414}}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.PercentilesAggregationReq.html) for all available options.
```

---

## aggregates/metrics/tophits.mdx

```
---
title: Top Hits
description: Compute the top hits for each bucket in a terms aggregation
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/tophits
---

The top hits aggregation is meant to be used in conjunction with the [terms](/documentation/aggregates/bucket/terms)
aggregation. It returns the top documents for each bucket of a terms aggregation.

For example, the following query answers "what are top 3 results sorted by `created_at` for each
`rating` category?"

```sql
SELECT pdb.agg('{"top_hits": {"size": 3, "sort": [{"created_at": "desc"}], "docvalue_fields": ["id", "created_at"]}}')
FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY rating;
```

```ini Expected Response
      agg
---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
 {"hits": [{"sort": [10907000251854775808], "docvalue_fields": {"id": [25], "created_at": ["2023-05-09T10:30:15Z"]}}, {"sort": [10906844884854775808], "docvalue_fields": {"id": [26], "created_at": ["2023-05-07T15:20:48Z"]}}, {"sort": [10906666358854775808], "docvalue_fields": {"id": [13], "created_at": ["2023-05-05T13:45:22Z"]}}]}
 {"hits": [{"sort": [10906756363854775808], "docvalue_fields": {"id": [24], "created_at": ["2023-05-06T14:45:27Z"]}}, {"sort": [10906385295854775808], "docvalue_fields": {"id": [28], "created_at": ["2023-05-02T07:40:59Z"]}}, {"sort": [10906236353854775808], "docvalue_fields": {"id": [29], "created_at": ["2023-04-30T14:18:37Z"]}}]}
 {"hits": [{"sort": [10906480573854775808], "docvalue_fields": {"id": [17], "created_at": ["2023-05-03T10:08:57Z"]}}, {"sort": [10906315942854775808], "docvalue_fields": {"id": [20], "created_at": ["2023-05-01T12:25:06Z"]}}, {"sort": [10906218361854775808], "docvalue_fields": {"id": [8], "created_at": ["2023-04-30T09:18:45Z"]}}]}
 {"hits": [{"sort": [10906573359854775808], "docvalue_fields": {"id": [27], "created_at": ["2023-05-04T11:55:23Z"]}}, {"sort": [10905961160854775808], "docvalue_fields": {"id": [15], "created_at": ["2023-04-27T09:52:04Z"]}}, {"sort": [10905202003854775808], "docvalue_fields": {"id": [7], "created_at": ["2023-04-18T14:59:27Z"]}}]}
 {"hits": [{"sort": [10906586188854775808], "docvalue_fields": {"id": [10], "created_at": ["2023-05-04T15:29:12Z"]}}]}
(5 rows)
```

The `sort` value returned by the aggregation is Tantivy's internal sort ID and should be ignored.
To get the actual fields, pass a list of fields to `docvalue_fields`.

If a text or JSON field is passed to `docvalue_fields`, it must be indexed with the [literal](/documentation/tokenizers/available-tokenizers/literal)
tokenizer.

To specify an offset, use `from`:

```sql
SELECT pdb.agg('{"top_hits": {"size": 3, "from": 1, "sort": [{"created_at": "desc"}], "docvalue_fields": ["id", "created_at"]}}')
FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY rating;
```

If multiple fields are passed into `sort`, the additional fields are used as tiebreakers:

```sql
SELECT pdb.agg('{"top_hits": {"size": 3, "sort": [{"created_at": "desc"}, {"id": "asc"}], "docvalue_fields": ["id", "created_at"]}}')
FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY rating;
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.TopHitsAggregationReq.html) for all available options.
```

---

## aggregates/metrics/sum.mdx

```
---
title: Sum
description: Compute the sum of a field
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/sum
---

The sum aggregation computes the sum of a field.

```sql
SELECT pdb.agg('{"sum": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
       agg
------------------
 {"value": 158.0}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.SumAggregation.html) for all available options.

## SQL Sum Syntax

SQL's `SUM` syntax is supported in beta. To enable it, first run

```sql
SET paradedb.enable_aggregate_custom_scan TO on;
```

With this feature enabled, the following query is equivalent to the above and is executed in the same way.

```sql
SELECT SUM(rating) FROM mock_items
WHERE id @@@ pdb.all();
```

By default, `SUM` ignores null values. Use `COALESCE` to include them in the final sum:

```sql
SELECT SUM(COALESCE(rating, 0)) FROM mock_items
WHERE id @@@ pdb.all();
```
```

---

## aggregates/metrics/count.mdx

```
---
title: Count
description: Count the number of values in a field
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/count
---

The following query counts the number of values in a field:

```sql
SELECT pdb.agg('{"value_count": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
       agg
-----------------
 {"value": 41.0}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html) for all available options.

## SQL Count Syntax

SQL's `COUNT` syntax is supported in beta. To enable it, first run

```sql
SET paradedb.enable_aggregate_custom_scan TO on;
```

With this feature enabled, the following query is equivalent to the above and is executed in the same way.

```sql
SELECT COUNT(rating) FROM mock_items
WHERE id @@@ pdb.all();
```

To count all rows, including rows with null values, use `COUNT(*)`:

```sql
SELECT COUNT(*) FROM mock_items
WHERE id @@@ pdb.all();
```
```

---

## aggregates/metrics/stats.mdx

```
---
title: Stats
description: Compute several metrics at once
canonical: https://docs.paradedb.com/documentation/aggregates/metrics/stats
---

The stats aggregation returns the count, sum, min, max, and average all at once.

```sql
SELECT pdb.agg('{"stats": {"field": "rating"}}') FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
                                      agg
--------------------------------------------------------------------------------
 {"avg": 3.8536585365853657, "max": 5.0, "min": 1.0, "sum": 158.0, "count": 41}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.StatsAggregation.html) for all available options.
```

---

## aggregates/bucket/histogram.mdx

```
---
title: Histogram
description: Count the number of occurrences over some interval
canonical: https://docs.paradedb.com/documentation/aggregates/bucket/histogram
---

The histogram aggregation dynamically creates buckets for a given `interval` and counts the number of occurrences
in each bucket.

Each value is rounded down to its bucket. For instance, a rating of `18` with an interval of `5` rounds down to a bucket
with key `15`.

```sql
SELECT pdb.agg('{"histogram": {"field": "rating", "interval": "1"}}')
FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
                                                                                  agg
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------
 {"buckets": [{"key": 1.0, "doc_count": 1}, {"key": 2.0, "doc_count": 3}, {"key": 3.0, "doc_count": 9}, {"key": 4.0, "doc_count": 16}, {"key": 5.0, "doc_count": 12}]}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/bucket/struct.HistogramAggregation.html)
for all available options.
```

---

## aggregates/bucket/terms.mdx

```
---
title: Terms
description: Count the number of occurrences for each value in a result set
canonical: https://docs.paradedb.com/documentation/aggregates/bucket/terms
---

<Note>
  If a text or JSON field is in the `GROUP BY` or `ORDER BY` clause, it must use
  the [literal](/documentation/tokenizers/available-tokenizers/literal)
  tokenizer.
</Note>

A terms aggregation counts the number of occurrences for every unique value in a field. For example, the following query
groups the `mock_items` table by `rating`, and calculates the number of items for each unique `rating`.

```sql
SELECT rating, pdb.agg('{"value_count": {"field": "id"}}') FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY rating
LIMIT 10;
```

```ini Expected Response
 rating |       agg
--------+-----------------
      4 | {"value": 16.0}
      5 | {"value": 12.0}
      3 | {"value": 9.0}
      2 | {"value": 3.0}
      1 | {"value": 1.0}
(5 rows)
```

Ordering by the bucketing field is supported:

```sql
SELECT rating, pdb.agg('{"value_count": {"field": "id"}}') FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY rating
ORDER BY rating
LIMIT 10;
```

<Note>Ordering by the aggregate value is not yet supported.</Note>

For performance reasons, we strongly recommend adding a `LIMIT` to the `GROUP BY`. Terms aggregations without a `LIMIT` consume more memory and
are slower to execute. If a query does not have a limit and more than `65000` unique values are found in a field, an error will be returned.
```

---

## aggregates/bucket/filters.mdx

```
---
title: Filters
description: Compute aggregations over multiple filters in one query
canonical: https://docs.paradedb.com/documentation/aggregates/bucket/filters
---

The filters aggregation allows a single query to return aggregations for multiple search queries at a time.
To use this aggregation, pass `pdb.agg` to the left-hand side of `FILTER` and a search query to the right-hand side.
For example:

```sql
SELECT
    pdb.agg('{"value_count": {"field": "id"}}')
    FILTER (WHERE category === 'electronics') AS electronics_count,
    pdb.agg('{"value_count": {"field": "id"}}')
    FILTER (WHERE category === 'footwear') AS footwear_count
FROM mock_items;
```

```ini Expected Response
 electronics_count | footwear_count
-------------------+----------------
 {"value": 5.0}    | {"value": 6.0}
(1 row)
```
```

---

## aggregates/bucket/range.mdx

```
---
title: Range
description: Count the number of occurrences over user-defined buckets
canonical: https://docs.paradedb.com/documentation/aggregates/bucket/range
---

The range aggregation counts the number of occurrences over user-defined buckets. The buckets must be continuous and cannot overlap.

```sql
SELECT pdb.agg('{"range": {"field": "rating", "ranges": [{"to": 3.0 }, {"from": 3.0, "to": 6.0} ]}}')
FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response
                                                                              agg
----------------------------------------------------------------------------------------------------------------------------------------------------------------
 {"buckets": [{"to": 3.0, "key": "*-3", "doc_count": 4}, {"to": 6.0, "key": "3-6", "from": 3.0, "doc_count": 37}, {"key": "6-*", "from": 6.0, "doc_count": 0}]}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/bucket/struct.RangeAggregation.html)
for all available options.
```

---

## aggregates/bucket/datehistogram.mdx

```
---
title: Date Histogram
description: Count the number of occurrences over fixed time intervals
canonical: https://docs.paradedb.com/documentation/aggregates/bucket/datehistogram
---

The date histogram aggregation constructs a histogram for date fields.

```sql
SELECT pdb.agg('{"date_histogram": {"field": "created_at", "fixed_interval": "30d"}}')
FROM mock_items
WHERE id @@@ pdb.all();
```

```ini Expected Response

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
 {"buckets": [{"key": 1679616000000.0, "doc_count": 14, "key_as_string": "2023-03-24T00:00:00Z"}, {"key": 1682208000000.0, "doc_count": 27, "key_as_string": "2023-04-23T00:00:00Z"}]}
(1 row)
```

See the [Tantivy documentation](https://docs.rs/tantivy/latest/tantivy/aggregation/bucket/struct.DateHistogramAggregationReq.html)
for all available options.
```

---

## getting-started/install.mdx

```
---
title: Install ParadeDB
description: How to run the ParadeDB Docker image
canonical: https://docs.paradedb.com/documentation/getting-started/install
---

The fastest way to install ParadeDB is by pulling the ParadeDB Docker image and running it locally. If
your primary Postgres is in a virtual private cloud (VPC), we recommend deploying ParadeDB on a compute
instance within your VPC to avoid exposing public IP addresses and needing to provision traffic routing
rules.

**Note**: ParadeDB supports Postgres 14+, and the `latest` tag ships with Postgres 17. To specify a different Postgres version, please refer to the available tags on [Docker Hub](https://hub.docker.com/r/paradedb/paradedb/tags).

```bash
docker run \
  --name paradedb \
  -e POSTGRES_USER=myuser \
  -e POSTGRES_PASSWORD=mypassword \
  -e POSTGRES_DB=mydatabase \
  -v paradedb_data:/var/lib/postgresql/ \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

You may replace `myuser`, `mypassword`, and `mydatabase` with whatever values you want. These will be your database
connection credentials.

To connect to ParadeDB, install the `psql` client and run

```bash
docker exec -it paradedb psql -U myuser -d mydatabase -W
```

To see all the ways in which you can install ParadeDB, please refer to our [deployment documentation](/deploy/overview).

That's it! Next, let's [run a few queries](/documentation/getting-started/quickstart) over mock data with ParadeDB.
```

---

## getting-started/load.mdx

```
---
title: Load Data from Postgres
description: Dump data from an existing Postgres and load into ParadeDB
canonical: https://docs.paradedb.com/documentation/getting-started/load
---

The easiest way to copy data from another Postgres into ParadeDB is with the `pg_dump` and `pg_restore` utilities. These are
installed by default when you install `psql`.

This approach is ideal for quickly testing ParadeDB. See the [deployment guide](/deploy/overview) for how to deploy ParadeDB into production.

## Create a Dump

Run `pg_dump` to create a copy of your database. The `pg_dump` version needs be greater than or equal to that of your Postgres database. You can check the version with `pg_dump --version`.

Below, we use the "custom" format (`-Fc`) for both `pg_dump` and `pg_restore`. Please review the [Postgres `pg_dump` documentation](https://www.postgresql.org/docs/current/app-pgdump.html) for other options that may be more appropriate for your environment.

<Note>
  Replace `host`, `username`, and `dbname` with your existing Postgres database
  credentials. If you deployed ParadeDB within your VPC, the `host` will be the
  private IP address of your existing Postgres database.
</Note>

```bash
pg_dump -Fc --no-acl --no-owner \
    -h <host> \
    -U <username> \
    <dbname> > old_db.dump
```

If your database is large, this can take some time. You can speed this up by dumping specific tables.

```bash
pg_dump -Fc --no-acl --no-owner \
    -h <host> \
    -U <username> \
    -t <table_name_1> -t <table_name_2> \
    <dbname> > old_db.dump
```

## Restore the Dump

Run `pg_restore` to load this data into ParadeDB. The `pg_restore` version needs be greater than or equal to that of your `pg_dump`. You can check the version with `pg_restore --version`.

<Note>
  Replace `host`, `username`, and `dbname` with your ParadeDB credentials.
</Note>

```bash
pg_restore --verbose --clean --no-acl --no-owner \
    -h <host> \
    -U <username> \
    -d <dbname> \
    -Fc \
    old_db.dump
```

Congratulations! You are now ready to run real queries over your data. To get started, refer to our [full text search documentation](https://docs.paradedb.com/documentation/full-text/overview).
```

---

## getting-started/quickstart.mdx

```
---
title: Quickstart
description: Get started with ParadeDB in five minutes
canonical: https://docs.paradedb.com/documentation/getting-started/quickstart
---

This guide will walk you through a few queries to give you a feel for ParadeDB.

## Create Example Table

ParadeDB comes with a helpful procedure that creates a table populated with mock data to help
you get started. Once connected with `psql`, run the following commands to create and inspect
this table.

```sql
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

SELECT description, rating, category
FROM mock_items
LIMIT 3;
```

```ini Expected Response
       description        | rating |  category
--------------------------+--------+-------------
 Ergonomic metal keyboard |      4 | Electronics
 Plastic Keyboard         |      4 | Electronics
 Sleek running shoes      |      5 | Footwear
(3 rows)
```

Next, let's create a BM25 index called `search_idx` on this table. A BM25 index is a covering index, which means that multiple columns can be included in the same index.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id');
```

<Note>
  As a general rule of thumb, any columns that you want to filter, `GROUP BY`,
  `ORDER BY`, or aggregate as part of a full text query should be added to the
  index for faster performance.
</Note>

<Note>
  Note the mandatory `key_field` option. See [choosing a key
  field](/documentation/indexing/create-index#choosing-a-key-field) for more details.
</Note>

## Match Query

We're now ready to execute a basic text search query. We'll look for matches where `description` matches `running shoes` where `rating` is greater than `2`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes' AND rating > 2
ORDER BY rating
LIMIT 5;
```

```ini Expected Response
     description     | rating | category
---------------------+--------+----------
 White jogging shoes |      3 | Footwear
 Generic shoes       |      4 | Footwear
 Sleek running shoes |      5 | Footwear
(3 rows)
```

`|||` is ParadeDB's custom [match disjunction](/documentation/full-text/match#disjunction) operator, which means "find me all documents containing
`running OR shoes`.

If we want all documents containing `running AND shoes`, we can use ParadeDB's `&&&` [match conjunction](/documentation/full-text/match#conjunction) operator.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description &&& 'running shoes' AND rating > 2
ORDER BY rating
LIMIT 5;
```

```ini Expected Response
     description     | rating | category
---------------------+--------+----------
 Sleek running shoes |      5 | Footwear
(1 row)
```

## BM25 Scoring

Next, let's add [BM25 scoring](/documentation/sorting/score) to the results, which sorts matches by relevance. To do this, we'll use `pdb.score`.

```sql
SELECT description, pdb.score(id)
FROM mock_items
WHERE description ||| 'running shoes' AND rating > 2
ORDER BY score DESC
LIMIT 5;
```

```ini Expected Response
     description     |   score
---------------------+-----------
 Sleek running shoes |  6.817111
 Generic shoes       | 3.8772602
 White jogging shoes | 3.4849067
(3 rows)
```

## Highlighting

Finally, let's also [highlight](/documentation/full-text/highlight) the relevant portions of the documents that were matched.
To do this, we'll use `pdb.snippet`.

```sql
SELECT description, pdb.snippet(description), pdb.score(id)
FROM mock_items
WHERE description ||| 'running shoes' AND rating > 2
ORDER BY score DESC
LIMIT 5;
```

```ini Expected Response
     description     |              snippet              |   score
---------------------+-----------------------------------+-----------
 Sleek running shoes | Sleek <b>running</b> <b>shoes</b> |  6.817111
 Generic shoes       | Generic <b>shoes</b>              | 3.8772602
 White jogging shoes | White jogging <b>shoes</b>        | 3.4849067
(3 rows)
```

## Top N

ParadeDB is highly optimized for quickly returning the [Top N](/documentation/sorting/topn) results out of the index. In SQL, this means queries that contain an `ORDER BY...LIMIT`:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY rating
LIMIT 5;
```

```ini Expected Response
     description     | rating | category
---------------------+--------+----------
 White jogging shoes |      3 | Footwear
 Generic shoes       |      4 | Footwear
 Sleek running shoes |      5 | Footwear
(3 rows)
```

## Facets

[Faceted queries](/documentation/aggregates/facets) allow a single query to return both the Top N results and an aggregate value,
which is more CPU-efficient than issuing two separate queries.

For example, the following query returns the top 3 results as well as the total number of results matched.

```sql
SELECT
     description, rating, category,
     pdb.agg('{"value_count": {"field": "id"}}') OVER ()
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY rating
LIMIT 5;
```

```ini Expected Response
     description     | rating | category |      agg
---------------------+--------+----------+----------------
 White jogging shoes |      3 | Footwear | {"value": 3.0}
 Generic shoes       |      4 | Footwear | {"value": 3.0}
 Sleek running shoes |      5 | Footwear | {"value": 3.0}
(3 rows)
```

That's it! Next, let's [load your data](/documentation/getting-started/load) to start running real queries.
```

---

## sorting/topn.mdx

```
---
title: Top N
description: ParadeDB is optimized for quickly finding the "Top N" results in a table
canonical: https://docs.paradedb.com/documentation/sorting/topn
---

ParadeDB is highly optimized for quickly returning the Top N results out of the index. In SQL, this means queries that contain an `ORDER BY...LIMIT`:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY rating
LIMIT 5;
```

In order for a Top N query to be executed by ParadeDB vs. vanilla Postgres, all of the following conditions must be met:

1. All `ORDER BY` fields must be indexed. If they are text fields, they [must use the literal tokenizer](#sorting-by-text).
2. At least one ParadeDB text search operator must be present at the same level as the `ORDER BY...LIMIT`.
3. The query must have a `LIMIT`.
4. With the exception of `lower`, ordering by expressions is not supported -- only the raw fields themselves.

To verify that ParadeDB is executing the Top N, look for a `Custom Scan` with a `TopNScanExecState` in the `EXPLAIN` output:

```sql
EXPLAIN SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY rating
LIMIT 5;
```

<Accordion title = "Expected Response">
```csv
                                                                                                   QUERY PLAN
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
 Limit  (cost=10.00..10.02 rows=3 width=552)
   ->  Custom Scan (ParadeDB Scan) on mock_items  (cost=10.00..10.02 rows=3 width=552)
         Table: mock_items
         Index: search_idx
         Segment Count: 1
         Exec Method: TopNScanExecState
         Scores: false
            TopN Order By: rating asc
            TopN Limit: 5
         Tantivy Query: {"with_index":{"query":{"match":{"field":"description","value":"running shoes","tokenizer":null,"distance":null,"transposition_cost_one":null,"prefix":null,"conjunction_mode":false}}}}
(10 rows)
```
</Accordion>

If any of the above conditions are not met, the query cannot be fully optimized and you will not see a `TopNScanExecState` in the `EXPLAIN` output.

## Tiebreaker Sorting

To guarantee stable sorting in the event of a tie, additional columns can be provided to `ORDER BY`:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY rating, id
LIMIT 5;
```

<Note>
  ParadeDB is currently able to handle 3 `ORDER BY` columns. If there are more
  than 3 columns, the `ORDER BY` will not be efficiently executed by ParadeDB.
</Note>

## Sorting by Text

If a text field is present in the `ORDER BY` clause, it must be indexed with the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.
The reason is that the literal tokenizer preserves the original text, which is necessary for accurate sorting.

Sorting by lowercase text using `lower(<text_field>)` is also supported. To enable this, first ensure that `lower(<text_field>)` is indexed with the literal tokenizer.
See [indexing expressions](/documentation/indexing/indexing-expressions) for more information.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (lower(description)::pdb.literal))
WITH (key_field='id');
```

This allows sorting by lowercase to be optimized.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY lower(description)
LIMIT 5;
```
```

---

## sorting/boost.mdx

```
---
title: Relevance Tuning
description: Tune the BM25 score by adjusting the weights of individual queries
canonical: https://docs.paradedb.com/documentation/sorting/boost
---

## Boosting

ParadeDB offers several ways to tune a document's [BM25 score](/documentation/sorting/score).
The first is boosting, which increases or decreases the impact of a specific query by multiplying its contribution to the overall BM25 score.

To boost a query, cast the query to the `boost` type. In this example, the `shoes` query is weighted twice as heavily as the `footwear` query.

```sql
SELECT id, pdb.score(id), description, category
FROM mock_items
WHERE description ||| 'shoes'::pdb.boost(2) OR category ||| 'footwear'
ORDER BY score DESC
LIMIT 5;
```

`boost` takes a numeric value, which is the multiplicative boost factor. It can be any floating point number between `-2048` and `2048`.

[Query builder functions](/documentation/query-builder/overview) can also be boosted:

```sql
SELECT id, description, category, pdb.score(id)
FROM mock_items
WHERE description @@@ pdb.regex('key.*')::pdb.boost(2)
ORDER BY score DESC
LIMIT 5;
```

Boost can be used in conjunction with other type casts, like [fuzzy](/documentation/full-text/fuzzy):

```sql
SELECT id, description, category, pdb.score(id)
FROM mock_items
WHERE description ||| 'shose'::pdb.fuzzy(2)::pdb.boost(2)
ORDER BY score DESC
LIMIT 5;
```

## Constant Scoring

Constant scoring assigns the same score to all documents that match a query. To apply a constant score, cast the query to the `const` type with a
numeric value.

For instance, the following query assigns a score of `1` to all documents matching the query `shoes`.

```sql
SELECT id, pdb.score(id), description, category
FROM mock_items
WHERE description ||| 'shoes'::pdb.const(1)
ORDER BY score DESC
LIMIT 5;
```
```

---

## sorting/score.mdx

```
---
title: BM25 Scoring
description: BM25 scores sort the result set by relevance
canonical: https://docs.paradedb.com/documentation/sorting/score
---

BM25 scores measure how relevant a score is for a given query. Higher scores indicate higher relevance.

## Basic Usage

The `pdb.score(<key_field>)` function produces a BM25 score and can be added to any query where any of the ParadeDB operators are present.

```sql
SELECT id, pdb.score(id)
FROM mock_items
WHERE description ||| 'shoes'
ORDER BY pdb.score(id)
LIMIT 5;
```

In order for a field to be factored into the BM25 score, it must be present in the BM25 index. For instance,
consider this query:

```sql
SELECT id, pdb.score(id)
FROM mock_items
WHERE description ||| 'keyboard' OR rating < 2
ORDER BY pdb.score(id)
LIMIT 5;
```

While BM25 scores will be returned as long as `description` is indexed, including `rating` in the BM25 index definition will allow results matching
`rating < 2` to rank higher than those that do not match.

## Joined Scores

First, let's create a second table called `orders` that can be joined with `mock_items`:

```sql
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
USING bm25 (order_id, product_id, order_quantity, order_total, customer_name)
WITH (key_field = 'order_id');
```

Next, let's compute a "combined BM25 score" over a join across both tables.

```sql
SELECT o.order_id, o.customer_name, m.description, pdb.score(o.order_id) + pdb.score(m.id) as score
FROM orders o
JOIN mock_items m ON o.product_id = m.id
WHERE o.customer_name ||| 'Johnson' AND m.description ||| 'running shoes'
ORDER BY score DESC, o.order_id
LIMIT 5;
```

## Score Refresh

The scores generated by the BM25 index may be influenced by dead rows that have not been cleaned up by the `VACUUM` process.

Running `VACUUM` on the underlying table will remove all dead rows from the index and ensures that only rows visible to the current
transaction are factored into the BM25 score.

```sql
VACUUM mock_items;
```

This can be automated with [autovacuum](/documentation/performance-tuning/overview).
```

---

## token-filters/trim.mdx

```
---
title: Trim
description: Remove trailing and leading whitespace from a token
canonical: https://docs.paradedb.com/documentation/token-filters/trim
---

The trim filter removes leading and trailing whitespace from a token (but not whitespace in the middle). If a token consists
entirely of whitespace, the token is eliminated entirely.

This filter is useful for tokenizers that don't already split on whitespace, like the [literal normalized](/documentation/tokenizers/available-tokenizers/literal-normalized)
tokenizer or certain language-specific tokenizers.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.literal_normalized('trim=true')))
WITH (key_field='id');
```

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  '    token with whitespace   '::pdb.literal_normalized::text[],
  '    token with whitespace   '::pdb.literal_normalized('trim=true')::text[];
```

```ini Expected Response
               text               |           text
----------------------------------+---------------------------
 {"    token with whitespace   "} | {"token with whitespace"}
(1 row)
```
```

---

## token-filters/lowercase.mdx

```
---
title: Lowercase
description: Converts all characters to lowercase
canonical: https://docs.paradedb.com/documentation/token-filters/lowercase
---

The lowercase filter converts all characters to lowercase, allowing for case-insensitive queries. It is enabled by default but can be
configured for all tokenizers besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.

To disable, append `lowercase=false` to the tokenizer's arguments:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('lowercase=false')))
WITH (key_field='id');
```

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  'Tokenize me!'::pdb.simple::text[],
  'Tokenize me!'::pdb.simple('lowercase=false')::text[];
```

```ini Expected Response
     text      |     text
---------------+---------------
 {tokenize,me} | {Tokenize,me}
(1 row)
```
```

---

## token-filters/overview.mdx

```
---
title: How Token Filters Work
description: Token filters apply additional processing to tokens like lowercasing or stemming
canonical: https://docs.paradedb.com/documentation/token-filters/overview
---

After a [tokenizer](/documentation/tokenizers/overview) splits up text into tokens, token filters
apply additional processing to each token. Common examples include [stemming](/documentation/token-filters/stemming)
to reduce words to their root form, or [ASCII folding](/documentation/token-filters/ascii-folding) to remove accents.

Token filters can be added to any tokenizer besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer, which by definition
must preserve the source text exactly.

To add a token filter to a tokenizer, append a configuration string to the argument list:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('stemmer=english', 'ascii_folding=true')))
WITH (key_field='id');
```
```

---

## token-filters/stemming.mdx

```
---
title: Stemmer
description: Reduces words to their root form for a given language
canonical: https://docs.paradedb.com/documentation/token-filters/stemming
---

Stemming is the process of reducing words to their root form. In English, for example, the root form of "running" and "runs" is "run".
Stemming can be configured for any tokenizer besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.

To set a stemmer, append `stemmer=<language>` to the tokenizer's arguments.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('stemmer=english')))
WITH (key_field='id');
```

Valid languages are `arabic`, `danish`, `dutch`, `english`, `finnish`, `french`, `german`, `greek`, `hungarian`, `italian`, `norwegian`, `polish`, `portuguese`, `romanian`, `russian`, `spanish`, `swedish`, `tamil`, and `turkish`.

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  'I am running'::pdb.simple::text[],
  'I am running'::pdb.simple('stemmer=english')::text[];
```

```ini Expected Response
      text      |    text
----------------+------------
 {i,am,running} | {i,am,run}
(1 row)
```
```

---

## token-filters/token-length.mdx

```
---
title: Token Length
description: Remove tokens that are above or below a certain byte length from the index
canonical: https://docs.paradedb.com/documentation/token-filters/token-length
---

The token length filter automatically removes tokens that are above or below a certain length in bytes.
To remove all tokens longer than a certain length, append a `remove_long` configuration to the tokenizer:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('remove_long=100')))
WITH (key_field='id');
```

To remove all tokens shorter than a length, use `remove_short`:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('remove_short=3')))
WITH (key_field='id');
```

All tokenizers besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer accept these configurations.

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  'A supersupersuperlong token'::pdb.simple::text[],
  'A supersupersuperlong token'::pdb.simple('remove_short=2', 'remove_long=10')::text[];
```

```ini Expected Response
             text              |  text
-------------------------------+---------
 {a,supersupersuperlong,token} | {token}
(1 row)
```
```

---

## token-filters/ascii-folding.mdx

```
---
title: ASCII Folding
description: Strips away diacritical marks like accents
canonical: https://docs.paradedb.com/documentation/token-filters/ascii-folding
---

The ASCII folding filter strips away diacritical marks (accents, umlauts, tildes, etc.) while leaving the base character intact.
It is supported for all tokenizers besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.

To enable, append `ascii_folding=true` to the tokenizer's arguments.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('ascii_folding=true')))
WITH (key_field='id');
```

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  'Café naïve coöperate'::pdb.simple::text[],
  'Café naïve coöperate'::pdb.simple('ascii_folding=true')::text[];
```

```ini Expected Response
          text          |          text
------------------------+------------------------
 {café,naïve,coöperate} | {cafe,naive,cooperate}
(1 row)
```
```

---

## token-filters/stopwords.mdx

```
---
title: Remove Stopwords
description: Remove language-specific stopwords from the index
canonical: https://docs.paradedb.com/documentation/token-filters/stopwords
---

Stopwords are words that are so common or semantically insignificant in most contexts that they can be ignored during indexing.
In English, for example, stopwords include "a", "and", "or", etc.

All tokenizers besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer can be configured to automatically remove stopwords
for a given language.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('stopwords_language=english')))
WITH (key_field='id');
```

Valid languages are `danish`, `dutch`, `english`, `finnish`, `french`, `german`, `hungarian`, `italian`, `norwegian`,`polish`, `portuguese`, `russian`, `spanish`, `swedish`.

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  'The cat in the hat'::pdb.simple::text[],
  'The cat in the hat'::pdb.simple('stopwords_language=english')::text[];
```

```ini Expected Response
         text         |   text
----------------------+-----------
 {the,cat,in,the,hat} | {cat,hat}
(1 row)
```
```

---

## token-filters/alphanumeric.mdx

```
---
title: Alpha Numeric Only
description: Removes any tokens that contain characters that are not ASCII letters
canonical: https://docs.paradedb.com/documentation/token-filters/alphanumeric
---

The alpha numeric only filter removes any tokens that contain characters that are not ASCII letters (i.e. `a` to `z` and `A` to `Z`) or digits
(i.e. `0` to `9`). It is supported for all tokenizers besides the [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.

To enable, append `alpha_num_only=true` to the tokenizer's arguments.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('alpha_num_only=true')))
WITH (key_field='id');
```

To demonstrate this token filter, let's compare the output of the following two statements:

```sql
SELECT
  'The café at 9pm!'::pdb.simple::text[],
  'The café at 9pm!'::pdb.simple('alpha_num_only=true')::text[];
```

```ini Expected Response
       text        |     text
-------------------+--------------
 {the,café,at,9pm} | {the,at,9pm}
(1 row)
```
```

---

## full-text/highlight.mdx

```
---
title: Highlighting
description: Generate snippets for portions of the source text that match the query string
canonical: https://docs.paradedb.com/documentation/full-text/highlight
---

<Note>
  Highlighting is an expensive process and can slow down query times. We
  recommend passing a `LIMIT` to any query where `pdb.snippet` or `pdb.snippets`
  is called to restrict the number of snippets that need to be generated.
</Note>

<Note>Highlighting is not supported for fuzzy search.</Note>

Highlighting refers to the practice of visually emphasizing the portions of a document that match a user's
search query.

## Basic Usage

`pdb.snippet(<column>)` can be added to any query where an `@@@` operator is present. `pdb.snippet` returns the single best snippet, sorted by relevance score.
The following query generates highlighted snippets against the `description` field.

```sql
SELECT id, pdb.snippet(description)
FROM mock_items
WHERE description ||| 'shoes'
LIMIT 5;
```

<ParamField body="start_tag" default="<b>">
  The leading indicator around the highlighted region.
</ParamField>
<ParamField body="end_tag" default="</b>">
  The trailing indicator around the highlighted region.
</ParamField>
<ParamField body="max_num_chars" default={150}>
  Max number of characters for a highlighted snippet. A snippet may contain
  multiple matches if they are close to each other.
</ParamField>

By default, `<b></b>` encloses the snippet. This can be configured with `start_tag` and `end_tag`:

```sql
SELECT id, pdb.snippet(description, start_tag => '<i>', end_tag => '</i>')
FROM mock_items
WHERE description ||| 'shoes'
LIMIT 5;
```

## Multiple Snippets

`pdb.snippets(<column>)` returns an array of snippets, allowing you to retrieve multiple highlighted matches from a document. This is particularly useful when a document has several relevant matches spread throughout its content.

```sql
SELECT id, pdb.snippets(description, max_num_chars => 15)
FROM mock_items
WHERE description ||| 'artistic vase'
LIMIT 5;
```

```ini Expected Response
 id |                snippets
----+-----------------------------------------
 19 | {<b>Artistic</b>,"ceramic <b>vase</b>"}
(1 row)

```

<ParamField body="start_tag" default="<b>">
  The leading indicator around the highlighted region.
</ParamField>
<ParamField body="end_tag" default="</b>">
  The trailing indicator around the highlighted region.
</ParamField>
<ParamField body="max_num_chars" default={150}>
  Max number of characters for a highlighted snippet. When `max_num_chars` is
  small, multiple snippets may be generated for a single document.
</ParamField>
<ParamField body="limit" default={5}>
  The maximum number of snippets to return per document.
</ParamField>
<ParamField body="offset" default={0}>
  The number of snippets to skip before returning results. Use with `limit` for
  pagination.
</ParamField>
<ParamField body="sort_by" default="score">
  The order in which to sort the snippets. Can be `'score'` (default, sorts by
  relevance) or `'position'` (sorts by appearance in the document).
</ParamField>

### Limiting and Offsetting Snippets

You can control the number and order of snippets returned using the `limit`, `offset`, and `sort_by` parameters.

For example, to get only the first snippet:

```sql
SELECT id, pdb.snippets(description, max_num_chars => 15, "limit" => 1)
FROM mock_items
WHERE description ||| 'running'
LIMIT 5;
```

To get the second snippet (by skipping the first one):

```sql
SELECT id, pdb.snippets(description, max_num_chars => 15, "limit" => 1, "offset" => 1)
FROM mock_items
WHERE description ||| 'running'
LIMIT 5;
```

### Sorting Snippets

Snippets can be sorted either by their relevance score (`'score'`) or their position within the document (`'position'`).

To sort snippets by their appearance in the document:

```sql
SELECT id, pdb.snippets(description, max_num_chars => 15, sort_by => 'position')
FROM mock_items
WHERE description ||| 'artistic vase'
LIMIT 5;
```

## Byte Offsets

`pdb.snippet_positions(<column>)` returns the byte offsets in the original text where the snippets would appear. It returns an array of
tuples, where the the first element of the tuple is the byte index of the first byte of the highlighted region, and the second element is the byte index after the last byte of the region.

```sql
SELECT id, pdb.snippet(description), pdb.snippet_positions(description)
FROM mock_items
WHERE description ||| 'shoes'
LIMIT 5;
```

```ini Expected Response
 id |          snippet           | snippet_positions
----+----------------------------+-------------------
  3 | Sleek running <b>shoes</b> | {"{14,19}"}
  4 | White jogging <b>shoes</b> | {"{14,19}"}
  5 | Generic <b>shoes</b>       | {"{8,13}"}
(3 rows)
```
```

---

## full-text/term.mdx

```
---
title: Term
description: Look for exact token matches in the source document, without any further processing of the query string
canonical: https://docs.paradedb.com/documentation/full-text/term
---

Term queries look for exact token matches. A term query is like an exact string match, but at the
token level.

Unlike [match](/documentation/full-text/match) or [phrase](/documentation/full-text/phrase) queries, term queries treat the query
string as a **finalized token**. This means that the query string is taken as-is, without any further tokenization or filtering.

Term queries use the `===` operator. To understand exactly how it works, let's consider the following two term queries:

```sql
-- Term query 1
SELECT description, rating, category
FROM mock_items
WHERE description === 'running';

-- Term query 2
SELECT description, rating, category
FROM mock_items
WHERE description === 'RUNNING';
```

The first query returns:

```csv
     description     | rating | category
---------------------+--------+----------
 Sleek running shoes |      5 | Footwear
(1 row)
```

However, the second query returns no results. This is because term queries look for exact matches, which includes
case sensitivity, and there are no documents in the example dataset containing the token `RUNNING`.

<Note>
  All tokenizers besides the literal tokenizer
  [lowercase](/documentation/token-filters/lowercase) tokens by default. Make
  sure to account for this when searching for a term.
</Note>

<Note>
  If you are using `===` to do an exact string match on the original text, make
  sure that the text uses the
  [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer.
</Note>

## How It Works

Under the hood, `===` simply finds all documents where any of their tokens are an exact string match against the query token.
A document's tokens are determined by the field's tokenizer and token filters, configured at index creation time.

## Examples

Let’s consider a few more hypothetical documents to see whether they would be returned by the term query.
These examples assume that index uses the default tokenizer and token filters, and that the term query is
`running`.

| Original Text       | Tokens                    | Match | Reason                                | Related                                             |
| ------------------- | ------------------------- | ----- | ------------------------------------- | --------------------------------------------------- |
| Sleek running shoes | `sleek` `running` `shoes` | ✅    | Contains the token `running`.         |
| Running shoes sleek | `sleek` `running` `shoes` | ✅    | Contains the token `running`.         |
| SLeeK RUNNING ShOeS | `sleek` `running` `shoes` | ✅    | Contains the token `running`.         | [Lowercasing](/documentation/indexing/create-index) |
| Sleek run shoe      | `sleek` `run` `shoe`      | ❌    | Does not contain the token `running`. | [Stemming](/documentation/indexing/create-index)    |
| Sleke ruining shoez | `sleke` `ruining` `shoez` | ❌    | Does not contain the token `running`. | [Fuzzy](/documentation/full-text/fuzzy)             |
| White jogging shoes | `white` `jogging` `shoes` | ❌    | Does not contain the token `running`. |

## Term Set

Passing a text array to the right-hand side of `===` means "find all documents containing any one of these tokens."

```sql
SELECT description, rating, category
FROM mock_items
WHERE description === ARRAY['shoes', 'running'];
```
```

---

## full-text/overview.mdx

```
---
title: How Text Search Works
description: Understand how ParadeDB uses token matching to efficiently search large corpuses of text
canonical: https://docs.paradedb.com/documentation/full-text/overview
---

Text search in ParadeDB, like Elasticsearch and most search engines, is centered around the concept of **token matching**.

Token matching consists of two steps. First, at indexing time, text is processed by a tokenizer, which breaks input into discrete units called **tokens** or
**terms**. For example, the [default](/documentation/indexing/create-index) tokenizer splits the text `Sleek running shoes` into the tokens `sleek`, `running`, and `shoes`.

Second, at query time, the query engine looks for token matches based on the specified query and query type. Some common query types include:

- [Match](/documentation/full-text/match): Matches documents containing any or all query tokens
- [Phrase](/documentation/full-text/phrase): Matches documents where all tokens appear in the same order as the query
- [Term](/documentation/full-text/term): Matches documents containing an exact token
- ...and many more [advanced](/documentation/query-builder/overview) query types

## Not Substring Matching

While ParadeDB supports substring matching via [regex](/documentation/query-builder/term/regex) queries, it's important to note that token matching is **not** the
same as substring matching.

Token matching is a much more versatile and powerful technique. It enables relevance scoring, language-specific analysis, typo tolerance, and more expressive query types — capabilities that go far beyond simply looking for a sequence of characters.

## Similarity Search

Text search is different than similarity search, also known as vector search. Whereas text search matches based on token matches, similarity search
matches based on semantic meaning.

ParadeDB currently does not build its own extensions for similarity search. Most ParadeDB users install [pgvector](https://github.com/pgvector/pgvector), the
Postgres extension for vector search, for this use case.

We have tentative long-term plans in our [roadmap](/welcome/roadmap#vector-search-improvements) to make improvements to Postgres' vector search.
If this is useful to you, please [reach out](mailto:support@paradedb.com).
```

---

## full-text/phrase.mdx

```
---
title: Phrase
description: Phrase queries are like match queries, but with order and position of matching tokens enforced
canonical: https://docs.paradedb.com/documentation/full-text/phrase
---

Phrase queries work exactly like [match conjunction](/documentation/full-text/match#match-conjunction), but are more strict in that they require the
order and position of tokens to be the same.

Suppose our query is `running shoes`, and we want to omit results like
`running sleek shoes` or `shoes running` — these results contain the right tokens, but not in the exact order and position
that the query specifies.

Enter the `###` phrase operator:

```sql
INSERT INTO mock_items (description, rating, category) VALUES
('running sleek shoes', 5, 'Footwear'),
('shoes running', 5, 'Footwear');

SELECT description, rating, category
FROM mock_items
WHERE description ### 'running shoes';
```

This query returns:

```csv
     description     | rating | category
---------------------+--------+----------
 Sleek running shoes |      5 | Footwear
(1 row)
```

Note that `running sleek shoes` and `shoes running` did not match the phrase `running shoes` despite having the tokens `running` and
`shoes` because they appear in the wrong order or with other words in between.

## How It Works

Let's look at what happens under the hood for the above phrase query:

1. Retrieves the tokenizer configuration of the `description` column. In this example,
   let's assume `description` uses the [unicode](/documentation/tokenizers/available-tokenizers/unicode) tokenizer.
2. Tokenizes the query string with the same tokenizer. This means `running shoes` becomes two tokens: `running` and `shoes`.
3. Finds all rows where `description` contains `running` immediately followed by `shoes`.

## Examples

Let’s consider a few more hypothetical documents to see whether they would be returned by the phrase query.
These examples assume that index uses the default tokenizer and token filters, and that the query is
`running shoes`.

| Original Text       | Tokens                    | Match | Reason                                         | Related                                                               |
| ------------------- | ------------------------- | ----- | ---------------------------------------------- | --------------------------------------------------------------------- |
| Sleek running shoes | `sleek` `running` `shoes` | ✅    | Contains `running` and `shoes`, in that order. |
| Running shoes sleek | `sleek` `running` `shoes` | ❌    | `running` and `shoes` not in the right order.  | [Match conjunction](/documentation/full-text/match#match-conjunction) |
| SLeeK RUNNING ShOeS | `sleek` `running` `shoes` | ✅    | Contains `running` and `shoes`, in that order. | [Lowercasing](/documentation/indexing/create-index)                   |
| Sleek run shoe      | `sleek` `run` `shoe`      | ❌    | Does not contain both `running` and `shoes`.   | [Stemming](/documentation/indexing/create-index)                      |
| Sleke ruining shoez | `sleke` `ruining` `shoez` | ❌    | Does not contain both `running` and `shoes`.   |
| White jogging shoes | `white` `jogging` `shoes` | ❌    | Does not contain both `running` and `shoes`.   |

## Adding Slop

Slop allows the token ordering requirement of phrase queries to be relaxed. It specifies how many changes — like extra words in between or transposed word positions — are allowed while still considering the phrase a match:

- An extra word in between (e.g. `sleek shoes` vs. `sleek running shoes`) has a slop of `1`
- A transposition (e.g. `running shoes` vs. `shoes running`) has a slop of `2`

To apply slop to a phrase query, cast the query to `slop(n)`, where `n` is the maximum allowed slop.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ### 'shoes running'::pdb.slop(2);
```

## Using a Custom Tokenizer

The phrase query supports custom query tokenization.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ### 'running shoes'::pdb.whitespace;
```

## Using Pretokenized Text

The phrase operator also accepts a text array as the right-hand side argument. If a text array is provided, each element of the array is treated as an exact token,
which means that no further processing is done.

The following query matches documents containing the token `shoes` immediately followed by `running`:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ### ARRAY['running', 'shoes'];
```

Adding slop is supported:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ### ARRAY['shoes', 'running']::pdb.slop(2);
```
```

---

## full-text/fuzzy.mdx

```
---
title: Fuzzy
description: Allow for typos in the query string
canonical: https://docs.paradedb.com/documentation/full-text/fuzzy
---

Fuzziness allows for tokens to be considered a match even if they are not identical, allowing for typos
in the query string.

## Overview

To add fuzziness to a query, cast it to the `fuzzy(n)` type, where `n` is the [edit distance](#how-it-works).
Fuzziness is supported for [match](/documentation/full-text/match) and [term](/documentation/full-text/term) queries.

```sql
-- Fuzzy match disjunction
SELECT id, description
FROM mock_items
WHERE description ||| 'runing shose'::pdb.fuzzy(2)
LIMIT 5;

-- Fuzzy match conjunction
SELECT id, description
FROM mock_items
WHERE description &&& 'runing shose'::pdb.fuzzy(2)
LIMIT 5;

-- Fuzzy Term
SELECT id, description
FROM mock_items
WHERE description === 'shose'::pdb.fuzzy(2)
LIMIT 5;
```

## How It Works

By default, the [match](/documentation/full-text/match) and [term](/documentation/full-text/term) queries require exact token matches between the query and indexed text. When a query is cast to `fuzzy(n)`, this requirement is relaxed -- tokens are matched if their Levenshtein distance, or edit distance, is less than or equal to `n`.

Edit distance is a measure of how many single-character operations are needed to turn one string into another. The allowed operations are:

- **Insertion** adds a character e.g., "shoe" → "shoes" (insert "s") has an edit distance of `1`
- **Deletion** removes a character e.g. "runnning" → "running" (delete one "n") has an edit distance of `1`
- **Transposition** replaces on character with another e.g., "shose" → "shoes" (transpose "s" → "e") has an edit distance of `2`

<Note>For performance reasons, the maximum allowed edit distance is `2`.</Note>

<Note>Casting a query to `fuzzy(0)` is the same as an exact token match.</Note>

## Fuzzy Prefix

`fuzzy` also supports prefix matching.
For instance, "runn" is a prefix of "running" because it matches the beginning of the token exactly. "rann" is a **fuzzy prefix** of "running" because it matches the
beginning within an edit distance of `1`.

To treat the query string as a prefix, set the second argument of `fuzzy` to either `t` or `"true"`:

```sql
SELECT id, description
FROM mock_items
WHERE description === 'rann'::pdb.fuzzy(1, t)
LIMIT 5;
```

<Note>
  Postgres requires that `true` be double-quoted, i.e. `fuzzy(1, "true")`.
</Note>

When used with [match](/documentation/full-text/match) queries, fuzzy prefix treats all tokens in the query string as prefixes.
For instance, the following query means "find all documents containing the fuzzy prefix `rann` AND the fuzzy prefix `slee`":

```sql
SELECT id, description
FROM mock_items
WHERE description &&& 'slee rann'::pdb.fuzzy(1, t)
LIMIT 5;
```

## Transposition Cost

By default, the cost of a transposition (i.e. "shose" → "shoes") is `2`. Setting the third argument of `fuzzy` to `t` lowers the
cost of a transposition to `1`:

```sql
SELECT id, description
FROM mock_items
WHERE description === 'shose'::pdb.fuzzy(1, f, t)
LIMIT 5;
```

<Note>
  The default value for the second and third arguments of `fuzzy` is `f`, which
  means `fuzzy(1)` is equivalent to `fuzzy(1, f, f)`.
</Note>
```

---

## full-text/match.mdx

```
---
title: Match
description: Returns documents that match the provided query string, which is tokenized before matching
canonical: https://docs.paradedb.com/documentation/full-text/match
---

Match queries are the go-to query type for text search in ParadeDB. There are two types of match queries:
[match disjunction](#match-disjunction) and [match conjunction](#match-conjunction).

## Match Disjunction

Match disjunction uses the `|||` operator and means "find all documents that contain one or more of the terms tokenized from this text input."

To understand what this looks like in practice, let's consider the following query:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes';
```

This query returns:

```csv
     description     | rating | category
---------------------+--------+----------
 Sleek running shoes |      5 | Footwear
 White jogging shoes |      3 | Footwear
 Generic shoes       |      4 | Footwear
(3 rows)
```

### How It Works

Let's look at what the `|||` operator does:

1. Retrieves the tokenizer configuration of the `description` column. In this example,
   let's assume `description` uses the [unicode](/documentation/tokenizers/available-tokenizers/unicode) tokenizer.
2. Tokenizes the query string with the same tokenizer. This means `running shoes` becomes two tokens: `running` and `shoes`.
3. Finds all rows where `description` contains **any one** of the tokens, `running` or `shoes`.

This is why all results have either `running` or `shoes` tokens in `description`.

### Examples

Let's consider a few more hypothetical documents to see whether they would be returned by match disjunction.
These examples assume that the index uses the default tokenizer and token filters, and that the query is
`running shoes`.

| Original Text       | Tokens                    | Match | Reason                                  | Related                                                               |
| ------------------- | ------------------------- | ----- | --------------------------------------- | --------------------------------------------------------------------- |
| Sleek running shoes | `sleek` `running` `shoes` | ✅    | Contains both `running` and `shoes`.    |
| Running shoes sleek | `sleek` `running` `shoes` | ✅    | Contains both `running` and `shoes`.    | [Phrase](/documentation/full-text/phrase)                             |
| SLeeK RUNNING ShOeS | `sleek` `running` `shoes` | ✅    | Contains both `running` and `shoes`.    | [Lowercasing](/documentation/indexing/create-index)                   |
| Sleek run shoe      | `sleek` `run` `shoe`      | ❌    | Contains neither `running` nor `shoes`. | [Stemming](/documentation/indexing/create-index)                      |
| Sleke ruining shoez | `sleke` `ruining` `shoez` | ❌    | Contains neither `running` nor `shoes`. | [Fuzzy](/documentation/full-text/fuzzy)                               |
| White jogging shoes | `white` `jogging` `shoes` | ✅    | Contains `shoes`.                       | [Match conjunction](/documentation/full-text/match#match-conjunction) |

## Match Conjunction

Suppose we want to find rows that contain both `running` **and** `shoes`. This is where the `&&&` match conjunction operator comes in.
`&&&` means "find all documents that contain all terms tokenized from this text input."

```sql
SELECT description, rating, category
FROM mock_items
WHERE description &&& 'running shoes';
```

This query returns:

```csv
     description     | rating | category
---------------------+--------+----------
 Sleek running shoes |      5 | Footwear
(1 row)
```

Note that `White jogging shoes` and `Generic shoes` are no longer returned because they do not have the token `running`.

### How It Works

Match conjunction works exactly like match disjunction, except for one key distinction. Instead of finding documents containing
at least one matching token from the query, it finds documents where **all tokens** from the query are a match.

### Examples

Let’s consider a few more hypothetical documents to see whether they would be returned by match conjunction.
These examples assume that the index uses the default tokenizer and token filters, and that the query is
`running shoes`.

| Original Text       | Tokens                    | Match | Reason                                       | Related                                                               |
| ------------------- | ------------------------- | ----- | -------------------------------------------- | --------------------------------------------------------------------- |
| Sleek running shoes | `sleek` `running` `shoes` | ✅    | Contains both `running` and `shoes`.         |
| Running shoes sleek | `sleek` `running` `shoes` | ✅    | Contains both `running` and `shoes`.         | [Phrase](/documentation/full-text/phrase)                             |
| SLeeK RUNNING ShOeS | `sleek` `running` `shoes` | ✅    | Contains both `running` and `shoes`.         | [Lowercasing](/documentation/indexing/create-index)                   |
| Sleek run shoe      | `sleek` `run` `shoe`      | ❌    | Does not contain both `running` and `shoes`. | [Stemming](/documentation/indexing/create-index)                      |
| Sleke ruining shoez | `sleke` `ruining` `shoez` | ❌    | Does not contain both `running` and `shoes`. | [Fuzzy](/documentation/full-text/fuzzy)                               |
| White jogging shoes | `white` `jogging` `shoes` | ❌    | Does not contain both `running` and `shoes`. | [Match conjunction](/documentation/full-text/match#match-conjunction) |

<Note>
If the query string only contains one token, then `|||` and `&&&` are effectively the same:

```sql
-- These two queries produce the same results
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'shoes';

SELECT description, rating, category
FROM mock_items
WHERE description &&& 'shoes';
```

</Note>

## Using a Custom Tokenizer

By default, the match query automatically tokenizes the query string with the same tokenizer used by the field it's being searched against.
This behavior can be overridden by explicitly casting the query to a different tokenizer.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description ||| 'running shoes'::pdb.whitespace;
```

## Using Pretokenized Text

The match operators also accept text arrays. If a text array is provided, each element of the array is treated as an exact token,
which means that no further processing is done.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description &&& ARRAY['running', 'shoes'];
```
```

---

## full-text/proximity.mdx

```
---
title: Proximity
description: Match documents based on token proximity within the source document
canonical: https://docs.paradedb.com/documentation/full-text/proximity
---

Proximity queries are used to match documents containing tokens that are within a certain token distance of one another.

## Overview

The following query finds all documents where the token `sleek` is at most `1` token away from `shoes`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ ('sleek' ## 1 ## 'shoes');
```

<Note>
  Like the [term](/documentation/full-text/term) query, the query string in a
  proximity query is treated as a finalized token.
</Note>

`##` does not care about order -- the term on the left-hand side may appear before or after the term on the right-hand side.
To ensure that the left-hand term appears before the right-hand term, use `##>`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ ('sleek' ##> 1 ##> 'shoes');
```

## Proximity Regex

In addition to exact tokens, proximity queries can also match against regex expressions.

The following query finds all documents where any token matching the regex query `sl.*` is at most `1` token away
from the token `shoes`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ (pdb.prox_regex('sl.*') ## 1 ## 'shoes');
```

By default, `pdb.prox_regex` will expand to the first `50` regex matches in each document. This limit can be overridden
by providing a second argument:

```sql
-- Expand up to 100 regex matches
SELECT description, rating, category
FROM mock_items
WHERE description @@@ (pdb.prox_regex('sl.*', 100) ## 1 ## 'shoes');
```

## Proximity Array

`pdb.prox_array` matches against an array of tokens instead of a single token. For example, the following query finds all
documents where any of the tokens `sleek` or `white` is within `1` token of `shoes`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ (pdb.prox_array('sleek', 'white') ## 1 ## 'shoes');
```

`pdb.prox_array` can also take regex:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ (pdb.prox_array(pdb.prox_regex('sl.*'), 'white') ## 1 ## 'shoes');
```
```

---

## indexing/indexing-expressions.mdx

```
---
title: Indexing Expressions
description: Add Postgres expressions to the index
canonical: https://docs.paradedb.com/documentation/indexing/indexing-expressions
---

In addition to indexing columns, Postgres expressions can also be indexed.

## Indexing Text/JSON Expressions

The following statement indexes an expression which concatenates `description` and `category`, which are both text fields:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, ((description || ' ' || category)::pdb.simple('alias=description_concat')))
WITH (key_field='id');
```

To index a text/JSON expression:

1. Add the expression to the column list. In this example, the expression is `description || ' ' || category`.
2. Cast it to a [tokenizer](/documentation/tokenizers/overview), in this example `pdb.simple`.
3. ParadeDB will try and infer a field name based on the field used in the expression. However,
   if the field name cannot be inferred (e.g. because the expression involves more than one field), you will be required
   to add an `alias=<alias_name>` to the tokenizer.

Querying against the expression is the same as querying a regular field:

```sql
SELECT description, rating, category
FROM mock_items
WHERE (description || ' ' || category) &&& 'running shoes';
```

<Note>
  The expression on the left-hand side of the operator must exactly match the
  expression that was indexed.
</Note>

## Indexing Non-Text Expressions

To index a non-text expression, cast the expression to `pdb.alias`. For example, the following statement indexes
the expression `rating + 1`, which returns an integer:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, ((rating + 1)::pdb.alias('rating')))
WITH (key_field='id');
```

With the expression indexed, queries containing the expression can be pushed down to the ParadeDB index:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description &&& 'running shoes'
AND rating + 1 > 3;
```

<Note>
  A current limitation is that the alias name must be the same as the field
  inside the expression in order for the index to be used at query time. For
  example, the statement above uses `rating` as the alias name.
</Note>
```

---

## indexing/indexing-arrays.mdx

```
---
title: Indexing Text Arrays
description: Add text arrays to the index
canonical: https://docs.paradedb.com/documentation/indexing/indexing-arrays
---

The BM25 index accepts arrays of type `text[]` or `varchar[]`.

```sql
CREATE TABLE array_demo (id SERIAL PRIMARY KEY, categories TEXT[]);
INSERT INTO array_demo (categories) VALUES
    ('{"food","groceries and produce"}'),
    ('{"electronics","computers"}'),
    ('{"books","fiction","mystery"}');

CREATE INDEX ON array_demo USING bm25 (id, categories)
WITH (key_field = 'id');
```

Under the hood, each element in the array is indexed as a separate entry. This means that an array is considered a
match if **any** of its entries is a match.

```sql
SELECT * FROM array_demo WHERE categories === 'food';
```

```ini Expected Response
 id |           categories
----+--------------------------------
  1 | {food,"groceries and produce"}
(1 row)
```

Text arrays can be [tokenized](/documentation/tokenizers/overview) and [filtered](/documentation/token-filters/overview) in the same way as text fields:

```sql
CREATE INDEX ON array_demo USING bm25 (id, (categories::pdb.literal))
WITH (key_field = 'id');
```
```

---

## indexing/indexing-json.mdx

```
---
title: Indexing JSON
description: Add JSON and JSONB types to the index
canonical: https://docs.paradedb.com/documentation/indexing/indexing-json
---

When indexing JSON, ParadeDB automatically indexes all sub-fields of the JSON object. The type of each sub-field is also inferred automatically.
For example, consider the following statement where `metadata` is `JSONB`:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, metadata)
WITH (key_field='id');
```

A single `metadata` JSON may look like:

```json
{ "color": "Silver", "location": "United States" }
```

ParadeDB will automatically index both `metadata.color` and `metadata.location` as text.

By default, all text sub-fields of a JSON object use the same tokenizer. The tokenizer can be configured the same way as text fields:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (metadata::pdb.ngram(2,3)))
WITH (key_field='id');
```

Instead of indexing the entire JSON, sub-fields of the JSON can be indexed individually. This allows for configuring separate tokenizers
within a larger JSON:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, ((metadata->>'color')::pdb.ngram(2,3)))
WITH (key_field='id');
```
```

---

## indexing/reindexing.mdx

```
---
title: Reindexing
description: Reindex an existing index
canonical: https://docs.paradedb.com/documentation/indexing/reindexing
---

Reindexing is necessary to change the index's schema. This includes adding, removing, or renaming fields, or changing a field's tokenizer
configuration.

The basic syntax for `REINDEX` is:

```sql
REINDEX INDEX search_idx;
```

This operation takes an exclusive lock on the table, which blocks incoming writes (but not reads) while the new index is being built.

To allow for concurrent writes during a reindex, use `REINDEX CONCURRENTLY`:

```sql
REINDEX INDEX CONCURRENTLY search_idx;
```

The tradeoff is that `REINDEX CONCURRENTLY` is slower than a plain `REINDEX`. Generally speaking, `REINDEX CONCURRENTLY` is recommended for
production systems that cannot tolerate temporarily blocked writes.

<Note>
  In order for `REINDEX CONCURRENTLY` to succeed, Postgres requires that the
  session that is executing the command remain open. If the session is closed,
  Postgres will cancel the reindex. This is relevant if you are using a
  connection pooler like `pgbouncer`, which can be configured to terminate
  sessions after a certain idle timeout is reached.
</Note>

<Note>
If `REINDEX CONCURRENTLY` fails or is cancelled, an invalid transient index will be left behind that must be dropped manually.
To check for invalid indexes in `psql`, run `\d <table_name>` and look for indexes suffixed by `_ccnew`.
</Note>
```

---

## indexing/create-index.mdx

```
---
title: Create an Index
description: Index a Postgres table for full text search
canonical: https://docs.paradedb.com/documentation/indexing/create-index
---

Before a table can be searched, it must be indexed. ParadeDB uses a custom index type called the BM25 index.
The following code block creates a BM25 index over several columns in the `mock_items` table.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, category)
WITH (key_field='id');
```

By default, text columns are tokenized using the [unicode](/documentation/tokenizers/available-tokenizers/unicode) tokenizer, which splits text according to the
Unicode segmentation standard. Because index creation is a time-consuming operation, we recommend experimenting with the [available tokenizers](/documentation/tokenizers/overview)
to find the most suitable one before running `CREATE INDEX`.

For instance, if a column contains multiple languages, the ICU tokenizer may be more appropriate.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.icu), category)
WITH (key_field='id');
```

Only one BM25 index can exist per table. We recommend indexing all columns in a table that may be present in a search query,
including columns used for sorting, grouping, filtering, and aggregations.

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id');
```

Most Postgres types, including text, JSON, numeric, timestamp, range, boolean, and arrays, can be indexed.

## Track Create Index Progress

To monitor the progress of a long-running `CREATE INDEX`, open a separate Postgres connection and query `pg_stat_progress_create_index`:

```sql
SELECT pid, phase, blocks_done, blocks_total
FROM pg_stat_progress_create_index;
```

Comparing `blocks_done` to `blocks_total` will provide a good approximation of the progress so far. If `blocks_done` equals
`blocks_total`, that means that all rows have been indexed and the index is being flushed to disk.

## Choosing a Key Field

In the `CREATE INDEX` statement above, note the mandatory `key_field` option.
Every BM25 index needs a `key_field`, which is the name of a column that will function as a row’s unique identifier within the index.

The `key_field` must:

1. Have a `UNIQUE` constraint. Usually this means the table's `PRIMARY KEY`.
2. Be the first column in the column list.
3. Be untokenized, if it is a text field.

## Token Filters

After tokens are created, [token filters](/documentation/token-filters/overview) can be configured to apply further processing like lowercasing, stemming, or unaccenting.
For example, the following code block adds English stemming to `description`:

```sql
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('stemmer=english')), category)
WITH (key_field='id');
```
```

---

## query-builder/overview.mdx

```
---
title: How Advanced Query Functions Work
description: ParadeDB's query builder functions provide advanced query types
canonical: https://docs.paradedb.com/documentation/query-builder/overview
---

In addition to basic [match](/documentation/full-text/match), [phrase](/documentation/full-text/phrase), and
[term](/documentation/full-text/term) queries, additional advanced query types are exposed as query builder functions.

Query builder functions use the `@@@` operator. `@@@` takes a column on the left-hand side and a query builder function on the
right-hand side. It means "find all rows where the column matches the given query."

For example:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.regex('key.*rd');
```

```ini Expected Response
       description        | rating |  category
--------------------------+--------+-------------
 Ergonomic metal keyboard |      4 | Electronics
 Plastic Keyboard         |      4 | Electronics
(2 rows)
```

This uses the [regex](/documentation/query-builder/term/regex) builder function to match all rows where `description` matches the regex expression `key.*rd`.
```

---

## query-builder/json.mdx

```
---
title: JSON Query Syntax
description: How to write text search queries as JSON
canonical: https://docs.paradedb.com/documentation/query-builder/json
---

ParadeDB also supports writing query builder functions as JSON objects.
This is useful for programmatic generation or client applications that construct search queries as structured JSON.

To write a query as JSON, first call `SELECT` on the desired query builder function, which returns its JSON
representation:

```sql
SELECT pdb.regex('key.*');
```

<Accordion title="Expected Response">
```csv
             regex
-------------------------------
 {"regex":{"pattern":"key.*"}}
(1 row)
```
</Accordion>

Next, paste this JSON string into the right-hand side of the `@@@` operator and cast it to `pdb.query`:

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@
'{
    "regex": {
        "pattern": "key.*"
    }
}'::pdb.query;
```

<Note>
  The JSON query object must be explicitly cast to `pdb.query` using
  `::pdb.query`.
</Note>
```

---

## query-builder/match.mdx

```
---
title: Match
description: The query builder version of the match query
canonical: https://docs.paradedb.com/documentation/query-builder/match
---

<Note>
  Highlighting is not supported for `pdb.match` if `distance` is greater than
  zero.
</Note>

<Note>
  For most use cases, we recommend using the
  [match](/documentation/full-text/match) query instead.
</Note>

Under the hood, the [match](/documentation/full-text/match) conjunction and disjunction operators get rewritten to this query builder function.
This function exposes a few advanced configuration options like the ability to use a custom tokenizer.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.match('running shoes');
```

<div className="mt-8" />

<ParamField body="value" required>
  The query to match against. This query is automatically tokenized in the same
  way as the field on the left-hand side of `@@@`.
</ParamField>
<ParamField body="distance" default={0}>
  If greater than zero, fuzzy matching is applied. Configures the maximum
  Levenshtein distance (i.e. single character edits) allowed to consider a term
  in the index as a match for the query term. Maximum value is `2`.
</ParamField>
<ParamField body="transposition_cost_one" default={true}>
  When set to `true` and fuzzy matching is enabled, transpositions (swapping two
  adjacent characters) as a single edit in the Levenshtein distance calculation,
  while `false` considers it two separate edits (a deletion and an insertion).
</ParamField>
<ParamField body="prefix" default={false}>
  When set to `true` and fuzzy matching is enabled, the initial substring
  (prefix) of the query term is exempted from the fuzzy edit distance
  calculation, while false includes the entire string in the calculation.
</ParamField>
<ParamField body="conjunction_mode" default={false}>
  When set to `true`, **all** tokens of the query have to match in order for a
  document to be considered a match. For instance, the query `running shoes` is
  by default executed as `running OR shoes`, but setting `conjunction_mode` to
  `true` executes it as `running AND shoes`.
</ParamField>

## Fuzzy Matching

When `distance` is set to a positive integer, fuzzy matching is applied. This allows `match` to tolerate typos in the query string.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.match('ruining shoez', distance => 2);
```

## Conjunction Mode

By default, `match` constructs an `OR` boolean query from the query string's tokens. For instance, the query `running shoes` is executed as `running OR shoes`.

When set to `true`, `conjunction_mode` constructs an `AND` boolean query instead.

```sql Function Syntax
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.match('running shoes', conjunction_mode => true);
```
```

---

## query-builder/phrase/phrase.mdx

```
---
title: Phrase
description: The query builder version of the phrase query
canonical: https://docs.paradedb.com/documentation/query-builder/phrase/phrase
---

Under the hood, the [phrase operator](/documentation/full-text/phrase) gets rewritten to this query builder function.
By default we recommend using the [phrase operator](/documentation/full-text/term) instead of this function.

## Basic Usage

Searches for documents containing a phrase.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.phrase(ARRAY['running', 'shoes']);
```

<div className="mt-8" />

<ParamField body="phrases" required>
  An `ARRAY` of tokens that form the search phrase. These tokens must appear in
  the specified order within the document for a match to occur, although some
  flexibility is allowed based on the `slop` parameter. Because these are
  tokens, they are not processed further.
</ParamField>
<ParamField body="slop" default={0}>
  A slop of `0` requires the terms to appear exactly as they are in the phrase
  and adjacent to each other. Higher slop values allow for more distance between
  the terms.
</ParamField>

Setting slop equal to `n` allows `n` terms to come in between the terms in the phrase as well
as term transpositions.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.phrase(ARRAY['running', 'shoes'], 1);
```
```

---

## query-builder/phrase/regex-phrase.mdx

```
---
title: Regex Phrase
description: Matches a specific sequence of regex queries
canonical: https://docs.paradedb.com/documentation/query-builder/phrase/regex-phrase
---

Regex phrase matches a specific sequence of regex queries. Think of it like a conjunction of [regex](/documentation/query-builder/term/regex)
queries, with positions and ordering of tokens enforced.

For example, the regex phrase query for `ru.* shoes` will match `running shoes`, but will not match `shoes running`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.regex_phrase(ARRAY['ru.*', 'shoes']);
```

<div className="mt-8" />

<ParamField body="phrases" required>
  An `ARRAY` of expressions that form the search phrase. These expressions must
  appear in the specified order within the document for a match to occur,
  although some flexibility is allowed based on the `slop` parameter. Please see
  [regex](/documentation/query-builder/term/regex) for allowed regex constructs.
</ParamField>
<ParamField body="slop" default={0}>
  A slop of `0` requires the terms to appear exactly as they are in the phrase
  and adjacent to each other. Higher slop values allow for transpositions and
  distance between terms.
</ParamField>
<ParamField body="max_expansions" default={16384}>
  Limits total number of terms that the regex phrase query can expand to. If
  this number is exceeded, an error will be returned.
</ParamField>
```

---

## query-builder/phrase/phrase-prefix.mdx

```
---
title: Phrase Prefix
description: Finds documents containing a phrase followed by a term prefix
canonical: https://docs.paradedb.com/documentation/query-builder/phrase/phrase-prefix
---

Phrase prefix identifies documents containing a phrase followed by a term prefix.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.phrase_prefix(ARRAY['running', 'sh']);
```

<div className="mt-8" />

<ParamField body="phrases" required>
  An `ARRAY` of tokens that the search is looking to match, followed by a term
  prefix rather than a complete term.
</ParamField>
<ParamField body="max_expansions" default={50}>
  Limits the number of term variations that the prefix can expand to during the
  search. This helps in controlling the breadth of the search by setting a cap
  on how many different terms the prefix can match.
</ParamField>

## Performance Considerations

Expanding a prefix might lead to thousands of matching terms, which impacts search times.

With `max_expansions`, the prefix term is expanded to at most `max_expansions` terms
in lexicographic order. For instance, if `sh` matches `shall`, `share`, `shoe`, and `shore` but `max_expansions` is set to 3,
`sh` will only be expanded to `shall`, `share`, and `shoe`.
```

---

## query-builder/term/fuzzy-term.mdx

```
---
title: Fuzzy Term
description: Finds results that approximately match the query token, allowing for a certain edit distance
canonical: https://docs.paradedb.com/documentation/query-builder/term/fuzzy-term
---

<Note>Highlighting is not supported for `pdb.fuzzy_term`.</Note>
<Note>
  For most use cases, we recommend using the [fuzzy query
  string](/documentation/full-text/fuzzy) syntax instead.
</Note>

`fuzzy_term` finds results that approximately match the query term, allowing for minor typos in the input.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.fuzzy_term('shoez')
LIMIT 5;
```

<div className="mt-8" />

<ParamField body="value" required>
  Defines the term you are searching for within the specified field, using fuzzy
  logic based on Levenshtein distance to find similar terms.
</ParamField>
<ParamField body="distance" default={2}>
  The maximum Levenshtein distance (i.e. single character edits) allowed to
  consider a term in the index as a match for the query term. Maximum value is
  `2`.
</ParamField>
<ParamField body="transposition_cost_one" default={true}>
  When set to `true`, transpositions (swapping two adjacent characters) as a
  single edit in the Levenshtein distance calculation, while `false` considers
  it two separate edits (a deletion and an insertion).
</ParamField>
<ParamField body="prefix" default={false}>
  When set to `true`, the initial substring (prefix) of the query term is
  exempted from the fuzzy edit distance calculation, while false includes the
  entire string in the calculation.
</ParamField>
```

---

## query-builder/term/term.mdx

```
---
title: Term
description: The query builder equivalent of the term query
canonical: https://docs.paradedb.com/documentation/query-builder/term/term
---

<Note>
  For most use cases, we recommend using the
  [term](/documentation/full-text/term) query instead.
</Note>

A term query treats the query as a single token. Because it does not apply any additional tokenization or processing
to the query, it is useful when looking for **exact** matches.

Under the hood, the [term operator](/documentation/full-text/term) gets rewritten to this query builder function.
By default we recommend using the [term operator](/documentation/full-text/term) instead of this function.

## Basic Usage

Matches documents containing a specified term.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.term('shoes');
```

Numeric, boolean, or datetime fields can also be passed into the `term` query. Doing so is equivalent to using the
`=` equality operator.

```sql
SELECT description, rating, category
FROM mock_items
WHERE rating @@@ pdb.term(4);
```

## Enumerated Types

`term` can be used to filter over custom Postgres [enums](/documentation/indexing/create-index#enumerated-types)
if the query term is explicitly cast to the enum type.

In this example, `color` is a custom enum:

```sql
SELECT description, rating, category
FROM mock_items
WHERE color @@@ pdb.term('red'::color);
```
```

---

## query-builder/term/term-set.mdx

```
---
title: Term Set
description: The query builder equivalent of the term set query
canonical: https://docs.paradedb.com/documentation/query-builder/term/term-set
---

<Note>
  BM25 scoring is not enabled for term set queries. As a result, the scores
  returned may not be accurate.
</Note>

<Note>
  For most use cases, we recommend using the [term
  set](/documentation/full-text/term#term-set) query instead.
</Note>

Matches documents containing any term from a specified set.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.term_set(ARRAY['shoes', 'keyboard']);
```

`pdb.term_set` is equivalent to `OR`ing together multiple `pdb.term` queries.
```

---

## query-builder/term/regex.mdx

```
---
title: Regex
description: Searches for terms that match a regex pattern
canonical: https://docs.paradedb.com/documentation/query-builder/term/regex
---

Regex queries search for terms that follow a pattern. For example, the wildcard pattern `key.*` finds all terms that start with `key`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE description @@@ pdb.regex('key.*');
```

ParadeDB supports all regex constructs of the Rust [regex](https://docs.rs/regex/latest/regex/) crate, with the following exceptions:

1. Lazy quantifiers such as `+?`
2. Word boundaries such as `\b`

Otherwise, the full syntax of the [regex](https://docs.rs/regex/latest/regex/) crate is supported, including all Unicode support and relevant flags.

A list of regex flags and grouping options can be [found here](https://docs.rs/regex/latest/regex/#grouping-and-flags), which includes:

- named and numbered capture groups
- case insensitivty flag (`i`)
- multi-line mode (`m`)

<Note>
  Regex queries operate at the token level. To execute regex over the original
  text, use the keyword tokenizer.
</Note>

## Performance Considerations

During a regex query, ParadeDB doesn't scan through every single word. Instead, it uses a highly optimized structure called a [finite state transducer (FST)](https://en.wikipedia.org/wiki/Finite-state_transducer) that makes it possible to jump straight to the matching terms.
Even if the index contains millions of words, the regex query only looks at the ones that have a chance of matching, skipping everything else.

This is why the certain regex constructs are not supported -- they are difficult to implement efficiently.
```

---

## query-builder/term/range-term.mdx

```
---
title: Range Term
description: Filters over Postgres range types
canonical: https://docs.paradedb.com/documentation/query-builder/term/range-term
---

`range_term` is the equivalent of Postgres' operators over [range types](https://www.postgresql.org/docs/current/rangetypes.html).
It supports operations like range containment, overlap, and intersection.

## Term Within

In this example, `weight_range` is an `int4range` type.
The following query finds all rows where `weight_range` contains `1`:

```sql
SELECT id, weight_range FROM mock_items
WHERE weight_range @@@ pdb.range_term(1);
```

## Range Intersects

The following query finds all ranges that share at least one common
point with the query range:

```sql
SELECT id, weight_range FROM mock_items
WHERE weight_range @@@ pdb.range_term('(10, 12]'::int4range, 'Intersects');
```

## Range Contains

The following query finds all ranges that are contained by the query range:

```sql
SELECT id, weight_range FROM mock_items
WHERE weight_range @@@ pdb.range_term('(3, 9]'::int4range, 'Contains');
```

## Range Within

The following query finds all ranges that contain the query range:

```sql
SELECT id, weight_range FROM mock_items
WHERE weight_range @@@ pdb.range_term('(2, 11]'::int4range, 'Within');
```
```

---

## query-builder/term/range.mdx

```
---
title: Range
description: The equivalent of Postgres' comparison operators (less than, greater than)
canonical: https://docs.paradedb.com/documentation/query-builder/term/range
---

<Note>
  For most use cases, we recommend using Postgres' native [range
  syntax](/documentation/filtering) instead.
</Note>

Finds documents containing a term that falls within a specified range of values. This produces the same results as using
Postgres' `<`, `>`, etc. range operators.

```sql
SELECT description, rating, category
FROM mock_items
WHERE rating @@@ pdb.range(int4range(1, 3, '[)'));
```

<div className="mt-8" />

<ParamField body="range" required>
  A Postgres range specifying the range of values to match the field against.
  Range types include `int4range`, `int8range`, `daterange`, `tsrange`, and
  `tstzrange`.
</ParamField>

## Inclusive vs. Exclusive Range

`pdb.range`accepts a Postgres [range type](https://www.postgresql.org/docs/current/rangetypes.html).
An inclusive lower bound is represented by `[` while an exclusive lower bound is represented by `(`. Likewise, an inclusive upper bound is represented by `]`, while an exclusive upper bound is represented by `)`.
For instance, the following query selects ratings between `1` and `3`, inclusive.

```sql
-- 1 to 3 inclusive
int4range(1, 3, '[]')

-- 1 to 3 exclusive
int4range(1, 3, '()')
```

## Unbounded Range

Passing `NULL` into either the upper or lower bound causes Postgres to treat the upper/lower bounds as
positive/negative infinity.

```sql
int4range(1, NULL, '[)')
```
```

---

## query-builder/term/exists.mdx

```
---
title: Exists
description: Checks that a field is not null
canonical: https://docs.paradedb.com/documentation/query-builder/term/exists
---

<Note>
  For most use cases, we recommend using `IS NOT NULL` instead of `pdb.exists`.
</Note>

<Note>
  Text fields must use the
  [literal](/documentation/tokenizers/available-tokenizers/literal) tokenizer in
  order for `pdb.exists` to work.
</Note>

Matches all documents with a non-null value in the specified field. All matched documents get a BM25 score of `1.0`.
This query produces the same results as Postgres' `IS NOT NULL`.

```sql
SELECT description, rating, category
FROM mock_items
WHERE rating @@@ pdb.exists()
LIMIT 5;
```
```

---

## query-builder/specialized/more-like-this.mdx

```
---
title: More Like This
description: Finds documents that are "like" another document.
canonical: https://docs.paradedb.com/documentation/query-builder/specialized/more-like-this
---

The more like this (MLT) query finds documents that are "like" another document.
To use this query, pass the [key field](/documentation/indexing/create-index#choosing-a-key-field) value of the input document
to `pdb.more_like_this`.

For instance, the following query finds documents that are "like" a document with an `id` of `3`:

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3)
ORDER BY id;
```

```ini Expected Response
 id |     description      | rating | category
----+----------------------+--------+----------
  3 | Sleek running shoes  |      5 | Footwear
  4 | White jogging shoes  |      3 | Footwear
  5 | Generic shoes        |      4 | Footwear
 13 | Sturdy hiking boots  |      4 | Footwear
 23 | Comfortable slippers |      3 | Footwear
 33 | Winter woolen socks  |      5 | Footwear
(6 rows)
```

In the output above, notice that documents matching any of the indexed fields, `description`, `rating`, and `category`, were returned.
This is because, by default, all fields present in the index are considered for matching.

<Note>
  The only exception is JSON fields, which are not yet supported and are ignored
  by the more like this query.
</Note>

To find only documents that match on specific fields, provide an array of field names as the second argument:

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3, ARRAY['description'])
ORDER BY id;
```

```ini Expected Response
 id |     description     | rating | category
----+---------------------+--------+----------
  3 | Sleek running shoes |      5 | Footwear
  4 | White jogging shoes |      3 | Footwear
  5 | Generic shoes       |      4 | Footwear
(3 rows)
```

<Note>
  Because JSON fields are not yet supported for MLT, an error will be returned
  if a JSON field is passed into the array.
</Note>

## How It Works

Let's look at how the MLT query works under the hood:

1. Stored values for the input document's fields are retrieved. If they are text fields, they are tokenized and filtered in the same way
   as the field was during [index creation](/documentation/indexing/create-index).
2. A set of representative terms is created from the input document. For example, in the statement above, these terms would be
   `sleek`, `running`, and `shoes` for the `description` field; `5` for the `rating` field; `footwear` for the `category` field.
3. Documents with at least one term match across any of the fields are considered a match.

## Using a Custom Input Document

In addition to providing a key field value, a custom document can also be provided as JSON.
The JSON keys are field names and must correspond to field names in the index.

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this('{"description": "Sleek running shoes", "category": "footwear"}')
ORDER BY id;
```

## Configuration Options

### Term Frequency

`min_term_frequency` excludes terms that appear fewer than a certain number of times in the input document,
while `max_term_frequency` excludes terms that appear more than that many times. By default, no terms are excluded
based on term frequency.

For instance, the following query returns no results because no term appears twice in the input document.

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3, min_term_frequency => 2)
ORDER BY id;
```

### Document Frequency

`min_doc_frequency` excludes terms that appear in fewer than a certain number of documents across the entire index,
while `max_doc_frequency` excludes terms that appear in more than that many documents. By default, no terms are excluded
based on document frequency.

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3, min_doc_frequency => 3)
ORDER BY id;
```

### Max Query Terms

By default, only the top 25 terms across all fields are considered for matching. Terms are scored using a combination of inverse document
frequency and term frequency (TF-IDF) -- this means that terms that appear frequently in the input document and are rare across the index
score the highest.

This can be configured with `max_query_terms`:

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3, max_query_terms => 10)
ORDER BY id;
```

### Term Length

`min_word_length` and `max_word_length` can be used to exclude terms that are too short or too long, respectively. By default, no terms
are excluded based on length.

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3, min_word_length => 5)
ORDER BY id;
```

### Custom Stopwords

To exclude terms from being considered, provide a text array to `stopwords`:

```sql
SELECT id, description, rating, category
FROM mock_items
WHERE id @@@ pdb.more_like_this(3, stopwords => ARRAY['the', 'a'])
ORDER BY id;
```
```

---

## query-builder/compound/all.mdx

```
---
title: All
description: Search all rows in the index
canonical: https://docs.paradedb.com/documentation/query-builder/compound/all
---

The all query means "search all rows in the index."

The primary use case for the all query is to force the query to be executed by the ParadeDB index instead of Postgres' other execution methods.
Because ParadeDB executes a query only when a ParadeDB operator is present in the query, the all query injects an operator into the query
without changing the query's meaning.

To use it, pass the [key field](/documentation/indexing/create-index#choosing-a-key-field) to the left-hand side of `@@@` and `pdb.all()` to the right-hand side.

```sql
-- Top N executed by standard Postgres
SELECT * FROM mock_items
WHERE rating IS NOT NULL
ORDER BY rating
LIMIT 5;

-- Top N executed by ParadeDB
SELECT * FROM mock_items
WHERE rating IS NOT NULL AND id @@@ pdb.all()
ORDER BY rating
LIMIT 5;
```

This is useful for cases where queries that don't contain a ParadeDB operator can be more efficiently executed by ParadeDB vs. standard Postgres,
like [Top N](/documentation/sorting/topn) or [aggregate](/documentation/aggregates/overview) queries.
```

---

## query-builder/compound/query-parser.mdx

```
---
title: Query Parser
description: Accept raw user-provided query strings
canonical: https://docs.paradedb.com/documentation/query-builder/compound/query-parser
---

The parse query accepts a [Tantivy query string](https://docs.rs/tantivy/latest/tantivy/query/struct.QueryParser.html).
The intended use case is for accepting raw query strings provided by the end user.

To use it, pass the [key field](/documentation/indexing/create-index#choosing-a-key-field) to the left-hand side of `@@@` and `pdb.parse('<query>')` to the right-hand side.

```sql
SELECT description, rating, category FROM mock_items
WHERE id @@@ pdb.parse('description:(sleek shoes) AND rating:>3');
```

Please refer to the [Tantivy docs](https://docs.rs/tantivy/latest/tantivy/query/struct.QueryParser.html) for an overview of
the query string language.

## Lenient Parsing

By default, strict syntax parsing is used. This means that if any part of the query does not conform to Tantivy’s query string syntax, the query fails. For instance, a valid field name must be provided before every query (i.e. `category:footwear`).
By setting `lenient` to `true`, the query is executed on a best-effort basis. For example, if no field names are provided, the query is executed over all fields in the index.

```sql
SELECT description, rating, category FROM mock_items
WHERE id @@@ pdb.parse('description:(sleek shoes) AND rating:>3', lenient => true);
```

## Conjunction Mode

By default, terms in the query string are `OR`ed together. With `conjunction_mode` set to `true`, they are instead `AND`ed together.
For instance, the following query returns documents containing both `sleek` and `shoes`.

```sql
SELECT description, rating, category FROM mock_items
WHERE id @@@ pdb.parse('description:(sleek shoes)', conjunction_mode => true);
```
```

---

