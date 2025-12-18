// web_app/api/queries.rs - Database query implementations
//
// This module implements the three search modes (BM25, Vector, Hybrid)
// using ParadeDB pg_search and pgvector extensions.
//
// Philosophy: Pure functions that take a pool and parameters,
// return typed results. No side effects, easy to test.

use sqlx::{PgPool, Row};
use crate::web_app::model::*;

/// Helper struct for mapping SQL rows to SearchResult
/// This is an intermediate representation that sqlx can map to
#[derive(Clone, sqlx::FromRow)]
struct SearchResultRow {
    // Product fields
    id: i32,
    name: String,
    description: String,
    brand: String,
    category: String,
    subcategory: Option<String>,
    tags: Vec<String>,
    price: rust_decimal::Decimal,
    rating: rust_decimal::Decimal,
    review_count: i32,
    stock_quantity: i32,
    in_stock: bool,
    featured: bool,
    attributes: Option<serde_json::Value>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,

    // Score fields
    bm25_score: Option<f64>,
    vector_score: Option<f64>,
    combined_score: f64,
    snippet: Option<String>,
}

impl From<SearchResultRow> for SearchResult {
    fn from(row: SearchResultRow) -> Self {
        SearchResult {
            product: Product {
                id: row.id,
                name: row.name,
                description: row.description,
                brand: row.brand,
                category: row.category,
                subcategory: row.subcategory,
                tags: row.tags,
                price: row.price,
                rating: row.rating,
                review_count: row.review_count,
                stock_quantity: row.stock_quantity,
                in_stock: row.in_stock,
                featured: row.featured,
                attributes: row.attributes,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            bm25_score: row.bm25_score,
            vector_score: row.vector_score,
            combined_score: row.combined_score,
            snippet: row.snippet,
        }
    }
}

/// BM25 full-text search using ParadeDB operators
pub async fn search_bm25(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error> {
    search_bm25_with_schema(pool, query, filters, "products").await
}

pub async fn search_bm25_with_schema(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
    schema: &str,
) -> Result<SearchResults, sqlx::Error> {
    // Treat "*" as empty query (match all)
    let query = if query.trim() == "*" { "" } else { query };

    let offset = (filters.page * filters.page_size) as i64;
    let limit = filters.page_size as i64;

    // Build category filter
    let category_clause = if filters.categories.is_empty() {
        "TRUE".to_string()
    } else {
        "category = ANY($4)".to_string()
    };

    let sort_clause = match filters.sort_by {
        SortOption::Relevance => "pdb.score(p.id) DESC",
        SortOption::PriceAsc => "p.price ASC",
        SortOption::PriceDesc => "p.price DESC",
        SortOption::RatingDesc => "p.rating DESC",
        SortOption::Newest => "p.created_at DESC",
    };

    let sql = format!(r#"
        SELECT
            p.id, p.name, p.description, p.brand, p.category,
            p.subcategory, p.tags, p.price::numeric as price,
            p.rating::numeric as rating, p.review_count,
            p.stock_quantity, p.in_stock, p.featured, p.attributes,
            p.created_at, p.updated_at,
            pdb.score(p.id)::float8 as bm25_score,
            NULL::float8 as vector_score,
            COALESCE(pdb.score(p.id), 0.0)::float8 as combined_score,
            NULL::text as snippet
        FROM {}.items p
        WHERE ($1 = '' OR p.description ||| $1 OR p.name ||| $1 OR p.brand ||| $1)
          AND ($2::float8 IS NULL OR p.price >= $2)
          AND ($3::float8 IS NULL OR p.price <= $3)
          AND {}
          AND ($5::float8 IS NULL OR p.rating >= $5)
          AND ($6::bool IS FALSE OR p.in_stock = TRUE)
        ORDER BY {}
        LIMIT $7 OFFSET $8
    "#, schema, category_clause, sort_clause);

    let results = sqlx::query_as::<_, SearchResultRow>(&sql)
        .bind(query)
        .bind(filters.price_min)
        .bind(filters.price_max)
        .bind(&filters.categories)
        .bind(filters.min_rating)
        .bind(filters.in_stock_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    // Get total count
    let count_sql = format!(r#"
        SELECT COUNT(*)
        FROM {}.items
        WHERE ($1 = '' OR description ||| $1 OR name ||| $1 OR brand ||| $1)
          AND ($2::float8 IS NULL OR price >= $2)
          AND ($3::float8 IS NULL OR price <= $3)
          AND ($4::float8 IS NULL OR rating >= $4)
          AND ($5::bool IS FALSE OR in_stock = TRUE)
    "#, schema);

    let total_row = sqlx::query(&count_sql)
        .bind(query)
        .bind(filters.price_min)
        .bind(filters.price_max)
        .bind(filters.min_rating)
        .bind(filters.in_stock_only)
        .fetch_one(pool)
        .await?;

    let total_count: i64 = total_row.get(0);

    // Get facets
    let category_facets = get_category_facets_with_schema(pool, query, schema).await?;
    let brand_facets = get_brand_facets_with_schema(pool, query, schema).await?;
    let price_histogram = get_price_histogram_with_schema(pool, query, schema).await?;

    Ok(SearchResults {
        results: results.into_iter().map(|r| r.into()).collect(),
        total_count,
        category_facets,
        brand_facets,
        price_histogram,
        avg_price: 0.0,
        avg_rating: 0.0,
    })
}

/// Vector similarity search using pgvector
pub async fn search_vector(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error> {
    search_vector_with_schema(pool, query, filters, "products").await
}

pub async fn search_vector_with_schema(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
    schema: &str,
) -> Result<SearchResults, sqlx::Error> {
    // Treat "*" as empty query (match all)
    let query = if query.trim() == "*" { "" } else { query };

    // Generate query embedding (MVP: random vector)
    let query_embedding = generate_query_embedding(query);
    let offset = (filters.page * filters.page_size) as i64;
    let limit = filters.page_size as i64;

    let sql = format!(r#"
        SELECT
            p.id, p.name, p.description, p.brand, p.category,
            p.subcategory, p.tags, p.price::numeric as price,
            p.rating::numeric as rating, p.review_count,
            p.stock_quantity, p.in_stock, p.featured, p.attributes,
            p.created_at, p.updated_at,
            NULL::float8 as bm25_score,
            (1 - (p.description_embedding <=> $1::vector(1536)))::float8 as vector_score,
            (1 - (p.description_embedding <=> $1::vector(1536)))::float8 as combined_score,
            NULL::text as snippet
        FROM {}.items p
        WHERE ($2::float8 IS NULL OR p.price >= $2)
          AND ($3::float8 IS NULL OR p.price <= $3)
          AND ($4::text[] IS NULL OR $4 = '{{}}' OR p.category = ANY($4))
          AND ($5::float8 IS NULL OR p.rating >= $5)
          AND ($6::bool IS FALSE OR p.in_stock = TRUE)
        ORDER BY p.description_embedding <=> $1::vector(1536)
        LIMIT $7 OFFSET $8
    "#, schema);

    let results = sqlx::query_as::<_, SearchResultRow>(&sql)
        .bind(&query_embedding)
        .bind(filters.price_min)
        .bind(filters.price_max)
        .bind(&filters.categories)
        .bind(filters.min_rating)
        .bind(filters.in_stock_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    // Get approximate total count
    let count_sql = format!("SELECT COUNT(*) FROM {}.items", schema);
    let total_row = sqlx::query(&count_sql)
        .fetch_one(pool)
        .await?;
    let total_count: i64 = total_row.get(0);

    Ok(SearchResults {
        results: results.into_iter().map(|r| r.into()).collect(),
        total_count,
        category_facets: vec![],
        brand_facets: vec![],
        price_histogram: vec![],
        avg_price: 0.0,
        avg_rating: 0.0,
    })
}

/// Hybrid search combining BM25 (30%) and Vector (70%)
pub async fn search_hybrid(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
) -> Result<SearchResults, sqlx::Error> {
    search_hybrid_with_schema(pool, query, filters, "products").await
}

pub async fn search_hybrid_with_schema(
    pool: &PgPool,
    query: &str,
    filters: &SearchFilters,
    schema: &str,
) -> Result<SearchResults, sqlx::Error> {
    // Treat "*" as empty query (match all)
    let query = if query.trim() == "*" { "" } else { query };

    let query_embedding = generate_query_embedding(query);
    let offset = (filters.page * filters.page_size) as i64;
    let limit = filters.page_size as i64;

    let sql = format!(r#"
        WITH bm25_results AS (
            SELECT id, pdb.score(id) AS bm25_score
            FROM {}.items
            WHERE description ||| $1 OR name ||| $1 OR brand ||| $1 OR $1 = ''
            ORDER BY pdb.score(id) DESC
            LIMIT 100
        ),
        vector_results AS (
            SELECT
                id,
                (1 - (description_embedding <=> $2::vector(1536)))::float8 AS vector_score
            FROM {}.items
            ORDER BY description_embedding <=> $2::vector(1536)
            LIMIT 100
        ),
        combined AS (
            SELECT
                COALESCE(b.id, v.id) AS id,
                COALESCE(b.bm25_score, 0)::float8 AS bm25_score,
                COALESCE(v.vector_score, 0)::float8 AS vector_score,
                (COALESCE(b.bm25_score, 0) * 0.3 + COALESCE(v.vector_score, 0) * 0.7)::float8 AS combined_score
            FROM bm25_results b
            FULL OUTER JOIN vector_results v ON b.id = v.id
        )
        SELECT
            p.id, p.name, p.description, p.brand, p.category,
            p.subcategory, p.tags, p.price::numeric as price,
            p.rating::numeric as rating, p.review_count,
            p.stock_quantity, p.in_stock, p.featured, p.attributes,
            p.created_at, p.updated_at,
            c.bm25_score,
            c.vector_score,
            c.combined_score,
            NULL::text as snippet
        FROM combined c
        JOIN {}.items p ON p.id = c.id
        WHERE ($3::float8 IS NULL OR p.price >= $3)
          AND ($4::float8 IS NULL OR p.price <= $4)
          AND ($5::text[] IS NULL OR $5 = '{{}}' OR p.category = ANY($5))
          AND ($6::float8 IS NULL OR p.rating >= $6)
          AND ($7::bool IS FALSE OR p.in_stock = TRUE)
        ORDER BY c.combined_score DESC
        LIMIT $8 OFFSET $9
    "#, schema, schema, schema);

    let results = sqlx::query_as::<_, SearchResultRow>(&sql)
        .bind(query)
        .bind(&query_embedding)
        .bind(filters.price_min)
        .bind(filters.price_max)
        .bind(&filters.categories)
        .bind(filters.min_rating)
        .bind(filters.in_stock_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    // Get facets
    let category_facets = get_category_facets_with_schema(pool, query, schema).await?;
    let brand_facets = get_brand_facets_with_schema(pool, query, schema).await?;
    let price_histogram = get_price_histogram_with_schema(pool, query, schema).await?;

    // Calculate total count before moving results
    let total_count = results.len() as i64;
    let search_results: Vec<SearchResult> = results.into_iter().map(|r| r.into()).collect();

    Ok(SearchResults {
        results: search_results,
        total_count,
        category_facets,
        brand_facets,
        price_histogram,
        avg_price: 0.0,
        avg_rating: 0.0,
    })
}

// Helper functions

async fn get_category_facets_with_schema(pool: &PgPool, query: &str, schema: &str) -> Result<Vec<FacetCount>, sqlx::Error> {
    let sql = format!(r#"
        SELECT category as value, COUNT(*) as count
        FROM {}.items
        WHERE description ||| $1 OR name ||| $1 OR brand ||| $1 OR $1 = ''
        GROUP BY category
        ORDER BY count DESC
    "#, schema);

    let rows = sqlx::query(&sql)
        .bind(query)
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|row| FacetCount {
        value: row.get("value"),
        count: row.get("count"),
    }).collect())
}

async fn get_brand_facets_with_schema(pool: &PgPool, query: &str, schema: &str) -> Result<Vec<FacetCount>, sqlx::Error> {
    let sql = format!(r#"
        SELECT brand as value, COUNT(*) as count
        FROM {}.items
        WHERE description ||| $1 OR name ||| $1 OR brand ||| $1 OR $1 = ''
        GROUP BY brand
        ORDER BY count DESC
        LIMIT 20
    "#, schema);

    let rows = sqlx::query(&sql)
        .bind(query)
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|row| FacetCount {
        value: row.get("value"),
        count: row.get("count"),
    }).collect())
}

async fn get_price_histogram_with_schema(pool: &PgPool, query: &str, schema: &str) -> Result<Vec<PriceBucket>, sqlx::Error> {
    let sql = format!(r#"
        SELECT
            FLOOR(price::float8 / 50) * 50 as min,
            FLOOR(price::float8 / 50) * 50 + 50 as max,
            COUNT(*) as count
        FROM {}.items
        WHERE description ||| $1 OR name ||| $1 OR brand ||| $1 OR $1 = ''
        GROUP BY FLOOR(price::float8 / 50)
        ORDER BY min
    "#, schema);

    let rows = sqlx::query(&sql)
        .bind(query)
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|row| PriceBucket {
        min: row.get("min"),
        max: row.get("max"),
        count: row.get("count"),
    }).collect())
}

/// Generate query embedding
fn generate_query_embedding(_query: &str) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();
    format!("[{}]", vec.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_query_embedding_format() {
        let embedding = generate_query_embedding("test query");
        assert!(embedding.starts_with('['));
        assert!(embedding.ends_with(']'));
        let comma_count = embedding.matches(',').count();
        assert_eq!(comma_count, 1535);
    }

    #[test]
    fn test_generate_query_embedding_values() {
        let embedding = generate_query_embedding("test");
        let first_value_str = embedding.trim_start_matches('[').split(',').next().unwrap();
        let first_value: f32 = first_value_str.parse().unwrap();
        assert!(first_value >= -1.0 && first_value <= 1.0);
    }

    #[test]
    fn test_search_result_row_to_search_result() {
        use rust_decimal::Decimal;
        let row = SearchResultRow {
            id: 1,
            name: "Test Product".to_string(),
            description: "A test product".to_string(),
            brand: "TestBrand".to_string(),
            category: "Electronics".to_string(),
            subcategory: None,
            tags: vec!["tag1".to_string()],
            price: Decimal::new(9999, 2),
            rating: Decimal::new(45, 1),
            review_count: 100,
            stock_quantity: 50,
            in_stock: true,
            featured: false,
            attributes: None,
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            bm25_score: Some(0.85),
            vector_score: Some(0.92),
            combined_score: 0.90,
            snippet: Some("test snippet".to_string()),
        };
        let result: SearchResult = row.into();
        assert_eq!(result.product.id, 1);
        assert_eq!(result.product.name, "Test Product");
    }
}
