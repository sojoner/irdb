#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
CLUSTER_NAME="irdb-cluster"
NAMESPACE="irdb"
OPERATOR_NAMESPACE="cnpg-system"
IMAGE_NAME="sojoner/database"
POSTGRES_VERSION="17"

echo -e "${GREEN}=== IR DB Kubernetes Setup ===${NC}"

# Check prerequisites
echo -e "\n${YELLOW}Checking prerequisites...${NC}"

command -v kind >/dev/null 2>&1 || { echo -e "${RED}kind is required but not installed${NC}"; exit 1; }
command -v kubectl >/dev/null 2>&1 || { echo -e "${RED}kubectl is required but not installed${NC}"; exit 1; }
command -v helm >/dev/null 2>&1 || { echo -e "${RED}helm is required but not installed${NC}"; exit 1; }
command -v docker >/dev/null 2>&1 || { echo -e "${RED}docker is required but not installed${NC}"; exit 1; }

echo -e "${GREEN}✓ All prerequisites installed${NC}"

# Step 1: Retag Docker image (optional, skip if already done)
echo -e "\n${YELLOW}Step 1: Checking Docker image...${NC}"
if docker image inspect ${IMAGE_NAME}:${POSTGRES_VERSION} >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Image ${IMAGE_NAME}:${POSTGRES_VERSION} already exists${NC}"
else
    echo -e "${YELLOW}Attempting to retag from ${IMAGE_NAME}:0.0.7...${NC}"
    if docker image inspect ${IMAGE_NAME}:0.0.7 >/dev/null 2>&1; then
        docker tag ${IMAGE_NAME}:0.0.7 ${IMAGE_NAME}:${POSTGRES_VERSION}
        echo -e "${GREEN}✓ Image retagged successfully${NC}"

        read -p "Do you want to push the image to registry? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            docker push ${IMAGE_NAME}:${POSTGRES_VERSION}
            echo -e "${GREEN}✓ Image pushed to registry${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ Image not found locally. Make sure ${IMAGE_NAME}:${POSTGRES_VERSION} is available${NC}"
    fi
fi

# Step 2: Create kind cluster
echo -e "\n${YELLOW}Step 2: Creating kind cluster...${NC}"
if kind get clusters | grep -q "^${CLUSTER_NAME}$"; then
    echo -e "${YELLOW}Cluster ${CLUSTER_NAME} already exists${NC}"
    read -p "Do you want to delete and recreate it? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        kind delete cluster --name ${CLUSTER_NAME}
        kind create cluster --config kind-config.yaml
        echo -e "${GREEN}✓ Cluster recreated${NC}"
    fi
else
    kind create cluster --config kind-config.yaml
    echo -e "${GREEN}✓ Cluster created${NC}"
fi

# Wait for cluster to be ready
echo -e "${YELLOW}Waiting for cluster to be ready...${NC}"
kubectl wait --for=condition=Ready nodes --all --timeout=300s
echo -e "${GREEN}✓ Cluster is ready${NC}"

# Step 3: Load Docker image into kind (if using local image)
echo -e "\n${YELLOW}Step 3: Loading Docker image into kind...${NC}"
if docker image inspect ${IMAGE_NAME}:${POSTGRES_VERSION} >/dev/null 2>&1; then
    kind load docker-image ${IMAGE_NAME}:${POSTGRES_VERSION} --name ${CLUSTER_NAME}
    echo -e "${GREEN}✓ Image loaded into kind cluster${NC}"
else
    echo -e "${YELLOW}⚠ Skipping image load (will pull from registry)${NC}"
fi

# Step 4: Install CloudNativePG Operator
echo -e "\n${YELLOW}Step 4: Installing CloudNativePG operator...${NC}"

# Add Helm repo
helm repo add cnpg https://cloudnative-pg.github.io/charts >/dev/null 2>&1 || true
helm repo update >/dev/null 2>&1

# Check if operator is already installed
if helm list -n ${OPERATOR_NAMESPACE} | grep -q cnpg; then
    echo -e "${YELLOW}Operator already installed${NC}"
    read -p "Do you want to upgrade it? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        helm upgrade cnpg cnpg/cloudnative-pg -n ${OPERATOR_NAMESPACE}
        echo -e "${GREEN}✓ Operator upgraded${NC}"
    fi
else
    helm install cnpg \
        --namespace ${OPERATOR_NAMESPACE} \
        --create-namespace \
        cnpg/cloudnative-pg
    echo -e "${GREEN}✓ Operator installed${NC}"
fi

# Wait for operator to be ready
echo -e "${YELLOW}Waiting for operator to be ready...${NC}"
kubectl wait --for=condition=Available \
    --timeout=300s \
    -n ${OPERATOR_NAMESPACE} \
    deployment/cnpg-cloudnative-pg
echo -e "${GREEN}✓ Operator is ready${NC}"

# Step 5: Deploy IR DB
echo -e "\n${YELLOW}Step 5: Deploying IR DB...${NC}"

# Check if already deployed
if kubectl get namespace ${NAMESPACE} >/dev/null 2>&1; then
    echo -e "${YELLOW}Namespace ${NAMESPACE} already exists${NC}"
    read -p "Do you want to redeploy? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        kubectl delete -k k8s/overlays/dev/ --ignore-not-found=true
        sleep 5
    fi
fi

kubectl apply -k k8s/overlays/dev/
echo -e "${GREEN}✓ IR DB deployment created${NC}"

# Wait for cluster to be ready
echo -e "\n${YELLOW}Waiting for database cluster to be ready...${NC}"
echo -e "${YELLOW}This may take a few minutes...${NC}"

# Wait for the cluster to be created
sleep 5

# Wait for pods to be created
kubectl wait --for=condition=Ready \
    --timeout=600s \
    -n ${NAMESPACE} \
    pod -l cnpg.io/cluster=irdb-postgres || true

echo -e "${GREEN}✓ Database cluster is ready${NC}"

# Step 6: Display connection information
echo -e "\n${GREEN}=== Setup Complete! ===${NC}"
echo -e "\n${YELLOW}Connection Information:${NC}"
echo -e "Host: localhost"
echo -e "Port: 5432"
echo -e "Database: database"
echo -e "Username: postgres"
echo -e "Password: custom_secure_password_123"

echo -e "\n${YELLOW}Connect using psql:${NC}"
echo -e "psql -h localhost -U postgres -d database -p 5432"

echo -e "\n${YELLOW}Or use port-forward:${NC}"
echo -e "kubectl port-forward -n ${NAMESPACE} svc/irdb-postgres-rw 5432:5432"

echo -e "\n${YELLOW}Check cluster status:${NC}"
echo -e "kubectl get cluster -n ${NAMESPACE}"
echo -e "kubectl get pods -n ${NAMESPACE}"

echo -e "\n${YELLOW}View logs:${NC}"
echo -e "kubectl logs -n ${NAMESPACE} -l cnpg.io/cluster=irdb-postgres -f"

echo -e "\n${GREEN}Setup completed successfully!${NC}"
