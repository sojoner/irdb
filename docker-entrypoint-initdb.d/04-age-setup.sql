-- docker-entrypoint-initdb.d/04-age-setup.sql
-- Apache AGE Extension Setup
-- Initializes Apache AGE graph database extension

\echo 'Setting up Apache AGE (A Graph Extension)...'

-- Create AGE extension
CREATE EXTENSION IF NOT EXISTS age;
\echo '✓ age extension created'

-- Create schema for graph data
CREATE SCHEMA IF NOT EXISTS agens;
\echo '✓ agens schema created'

-- Grant permissions on schema to postgres user (superuser)
GRANT USAGE ON SCHEMA agens TO postgres;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA agens TO postgres;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA agens TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA agens GRANT ALL ON TABLES TO postgres;
ALTER DEFAULT PRIVILEGES IN SCHEMA agens GRANT ALL ON SEQUENCES TO postgres;
\echo '✓ Permissions granted to postgres user'

-- Grant permissions to app user
GRANT USAGE ON SCHEMA agens TO app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA agens TO app;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA agens TO app;
ALTER DEFAULT PRIVILEGES IN SCHEMA agens GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app;
ALTER DEFAULT PRIVILEGES IN SCHEMA agens GRANT ALL ON SEQUENCES TO app;
\echo '✓ Permissions granted to app user'

-- Create example knowledge graph for testing
-- This graph will be used by test suites
SELECT * FROM ag_catalog.create_graph('knowledge_graph');
\echo '✓ knowledge_graph created'

-- Create another graph for hybrid search testing
SELECT * FROM ag_catalog.create_graph('test_graph');
\echo '✓ test_graph created'

\echo ''
\echo '✓ Apache AGE setup complete!'
\echo ''
\echo 'Available Graphs:'
SELECT * FROM ag_catalog.ag_graph;

\echo ''
\echo 'Extension Status:'
SELECT extname, extversion FROM pg_extension WHERE extname = 'age';
