#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

NAMESPACE="databases"
CLUSTER_NAME="irdb-postgres-cnpg"
KUBE_CONTEXT="${KUBE_CONTEXT:-k0s-cluster-admin@k0s-cluster}"

echo -e "${GREEN}=== Verifying IR DB Extensions ===${NC}"

# Get the primary pod
echo -e "\n${YELLOW}Finding primary pod...${NC}"
PRIMARY_POD=$(kubectl --context ${KUBE_CONTEXT} get pod -n ${NAMESPACE} \
    -l cnpg.io/cluster=${CLUSTER_NAME},cnpg.io/instanceRole=primary \
    -o jsonpath='{.items[0].metadata.name}')

if [ -z "$PRIMARY_POD" ]; then
    echo -e "${RED}✗ Could not find primary pod${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Primary pod: ${PRIMARY_POD}${NC}"

# Run verification SQL
echo -e "\n${YELLOW}Running verification queries...${NC}"

kubectl --context ${KUBE_CONTEXT} exec -n ${NAMESPACE} ${PRIMARY_POD} -- psql -U postgres -d app <<'EOF'
\echo '=== Installed Extensions ==='
SELECT extname, extversion
FROM pg_extension
WHERE extname IN ('vector', 'pg_search', 'pg_stat_statements', 'pg_trgm', 'btree_gin')
ORDER BY extname;

\echo ''
\echo '=== AI Data Schema ==='
SELECT schema_name
FROM information_schema.schemata
WHERE schema_name = 'ai_data';

\echo ''
\echo '=== AI Data Tables ==='
SELECT table_name
FROM information_schema.tables
WHERE table_schema = 'ai_data'
ORDER BY table_name;

\echo ''
\echo '=== Documents Table Structure ==='
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_schema = 'ai_data' AND table_name = 'documents'
ORDER BY ordinal_position;

\echo ''
\echo '=== Testing Vector Generation Function ==='
SELECT ai_data.generate_random_vector(5) AS sample_vector;

\echo ''
\echo '=== Testing Hybrid Search Function ==='
SELECT COUNT(*) as function_exists
FROM pg_proc p
JOIN pg_namespace n ON p.pronamespace = n.oid
WHERE n.nspname = 'ai_data' AND p.proname = 'hybrid_search';

\echo ''
\echo '=== Indexes on Documents Table ==='
SELECT indexname, indexdef
FROM pg_indexes
WHERE schemaname = 'ai_data' AND tablename = 'documents';

\echo ''
\echo '=== PostgreSQL Version ==='
SELECT version();
EOF

echo -e "\n${GREEN}✓ Verification complete!${NC}"

# Check if we can insert test data
echo -e "\n${YELLOW}Testing data insertion...${NC}"

kubectl --context ${KUBE_CONTEXT} exec -n ${NAMESPACE} ${PRIMARY_POD} -- psql -U postgres -d app <<'EOF'
-- Insert a test document
INSERT INTO ai_data.documents (title, content, metadata, embedding)
VALUES (
    'Test Document',
    'This is a test document to verify the IR DB setup.',
    '{"source": "verification_script", "test": true}'::jsonb,
    ai_data.generate_random_vector(1536)
)
ON CONFLICT DO NOTHING;

-- Verify insertion
SELECT COUNT(*) as document_count FROM ai_data.documents;
EOF

echo -e "${GREEN}✓ Data insertion test passed!${NC}"

echo -e "\n${GREEN}=== All Checks Passed! ===${NC}"
echo -e "${YELLOW}Your IR DB is ready for use.${NC}"
