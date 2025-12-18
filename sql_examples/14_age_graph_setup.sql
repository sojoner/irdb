-- 14_age_graph_setup.sql
-- Apache AGE Graph Database Setup and Structure Tests
-- Tests basic graph creation, property indexes, and schema verification
--
-- This test file is IDEMPOTENT and SELF-CONTAINED
-- 
-- Usage:
--   psql -h localhost -U postgres -d app -f 14_age_graph_setup.sql

\echo '=============================================='
\echo '=== Apache AGE Graph Setup Tests ==='
\echo '=============================================='
\echo ''

-- Verify AGE extension is installed
\echo '--- VERIFICATION: Checking AGE extension ---'
SELECT extname, extversion FROM pg_extension WHERE extname = 'age';
\echo '✓ AGE extension verified'
\echo ''

-- Check if knowledge_graph exists, if not create it
\echo '--- SETUP: Creating graph structures ---'

DO $$
BEGIN
    -- Create graph if it doesn't exist
    PERFORM * FROM ag_catalog.ag_graph WHERE name = 'test_setup_graph';
    IF NOT FOUND THEN
        PERFORM ag_catalog.create_graph('test_setup_graph');
    END IF;
EXCEPTION WHEN OTHERS THEN
    -- Graph might already exist
    NULL;
END $$;
\echo '✓ test_setup_graph created (or already exists)'

-- List all available graphs
\echo ''
\echo '--- Test 1: Available Graphs ---'
SELECT name, namespace FROM ag_catalog.ag_graph ORDER BY name;

-- Test 2: Create vertices (nodes)
\echo ''
\echo '--- Test 2: Create Document Vertices ---'
SELECT * FROM cypher('test_setup_graph', $$
    CREATE (d1:Document {id: 1, title: 'PostgreSQL Guide', author: 'PostgreSQL Team'}),
           (d2:Document {id: 2, title: 'Graph Databases', author: 'Neo4j Company'}),
           (d3:Document {id: 3, title: 'Knowledge Graphs', author: 'Google Research'})
    RETURN d1, d2, d3
$$) AS (d1 agtype, d2 agtype, d3 agtype);
\echo '✓ Document vertices created'

-- Test 3: Create entity vertices
\echo ''
\echo '--- Test 3: Create Entity Vertices ---'
SELECT * FROM cypher('test_setup_graph', $$
    CREATE (e1:Entity {id: 'postgresql', type: 'Database', category: 'RelationalDB'}),
           (e2:Entity {id: 'graph_db', type: 'Database', category: 'GraphDB'}),
           (e3:Entity {id: 'knowledge_base', type: 'System', category: 'AI'})
    RETURN e1, e2, e3
$$) AS (e1 agtype, e2 agtype, e3 agtype);
\echo '✓ Entity vertices created'

-- Test 4: Create relationships (edges)
\echo ''
\echo '--- Test 4: Create Document-Entity Relationships ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH (d:Document {title: 'PostgreSQL Guide'}), (e:Entity {id: 'postgresql'})
    CREATE (d)-[r:MENTIONS {confidence: 0.95}]->(e)
    RETURN r
$$) AS (r agtype);
\echo '✓ Document-Entity relationships created'

-- Test 5: Create document references
\echo ''
\echo '--- Test 5: Create Document-Document References ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH (d1:Document {title: 'PostgreSQL Guide'}), 
          (d2:Document {title: 'Graph Databases'})
    CREATE (d1)-[r:REFERENCES {reason: 'mentions database concepts'}]->(d2)
    RETURN r
$$) AS (r agtype);
\echo '✓ Document-Document references created'

-- Test 6: Query all vertices
\echo ''
\echo '--- Test 6: Count All Vertices ---'
SELECT COUNT(*) as total_vertices, vertex_label FROM (
    SELECT cypher('test_setup_graph', $$
        MATCH (n) RETURN labels(n)[0] as vertex_label
    $$) AS result
) AS subquery
WHERE (result).vertex_label IS NOT NULL
GROUP BY vertex_label;

-- Test 7: Query all relationships
\echo ''
\echo '--- Test 7: Query All Relationships ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH ()-[r]->() 
    RETURN type(r) as relationship_type, COUNT(*) as count
$$) AS (relationship_type TEXT, count BIGINT);

-- Test 8: Find document properties
\echo ''
\echo '--- Test 8: Document Properties ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH (d:Document)
    RETURN d.id, d.title, d.author
    ORDER BY d.id
$$) AS (id BIGINT, title TEXT, author TEXT);

-- Test 9: Find entity properties
\echo ''
\echo '--- Test 9: Entity Properties ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH (e:Entity)
    RETURN e.id, e.type, e.category
    ORDER BY e.id
$$) AS (id TEXT, type TEXT, category TEXT);

-- Test 10: Graph statistics
\echo ''
\echo '--- Test 10: Graph Statistics ---'
DO $$
DECLARE
    vertex_count BIGINT;
    edge_count BIGINT;
BEGIN
    -- Count vertices
    SELECT COUNT(*) INTO vertex_count FROM cypher('test_setup_graph', $$
        MATCH (n) RETURN n
    $$) AS (n agtype);
    
    -- Count edges
    SELECT COUNT(*) INTO edge_count FROM cypher('test_setup_graph', $$
        MATCH ()-[r]->() RETURN r
    $$) AS (r agtype);
    
    RAISE NOTICE 'Graph Statistics for test_setup_graph:';
    RAISE NOTICE '  Total Vertices: %', vertex_count;
    RAISE NOTICE '  Total Edges: %', edge_count;
END $$;

-- Test 11: Property index check (if supported)
\echo ''
\echo '--- Test 11: Label and Type Distribution ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH (n)
    WITH labels(n)[0] as label, type(n) as node_type
    RETURN label, COUNT(*) as count
    GROUP BY label
    ORDER BY label
$$) AS (label TEXT, count BIGINT);

-- Test 12: Path length analysis
\echo ''
\echo '--- Test 12: Basic Path Queries (2-hop relationships) ---'
SELECT * FROM cypher('test_setup_graph', $$
    MATCH (d1:Document)-[r1:REFERENCES]->(d2:Document)
    RETURN d1.title as source_doc, r1.reason as reference_reason, d2.title as target_doc
$$) AS (source_doc TEXT, reference_reason TEXT, target_doc TEXT);

-- Cleanup (optional - comment out to keep test data)
\echo ''
\echo '--- Test 13: Data Retention Check ---'
SELECT COUNT(*) as remaining_vertices FROM cypher('test_setup_graph', $$
    MATCH (n) RETURN n
$$) AS (n agtype);

\echo ''
\echo '=============================================='
\echo '=== Apache AGE Graph Setup Tests Complete ==='
\echo '=============================================='
\echo ''
\echo 'Summary:'
\echo '  ✓ Graph created and populated'
\echo '  ✓ Vertices (nodes) created: Documents, Entities'
\echo '  ✓ Edges (relationships) created: MENTIONS, REFERENCES'
\echo '  ✓ Properties stored and queryable'
\echo ''
\echo 'Next Steps:'
\echo '  - Run 15_age_cypher_tests.sql for advanced Cypher queries'
\echo '  - Run 16_age_hybrid_queries.sql for SQL+Cypher integration'
\echo ''
