# Makefile for IR DB - AI-Enhanced PostgreSQL Platform
# Provides granular tasks for Docker Compose and Kubernetes deployments

# Variables
CLUSTER_NAME ?= irdb-cluster
NAMESPACE ?= irdb
OPERATOR_NAMESPACE ?= cnpg-system
IMAGE_NAME ?= sojoner/database
IMAGE_TAG_CURRENT ?= 0.0.7
IMAGE_TAG_PG_VERSION ?= 17
KUSTOMIZE_OVERLAY ?= overlays/dev
DB_PASSWORD ?= custom_secure_password_123
DB_USER ?= postgres
DB_NAME ?= database
DB_PORT ?= 5432

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
RED := \033[0;31m
YELLOW := \033[1;33m
NC := \033[0m # No Color

.PHONY: help
help: ## Show this help message
	@echo '$(CYAN)IR DB Makefile - Available targets:$(NC)'
	@echo ''
	@echo '$(YELLOW)Prerequisites:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(check|install)-' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Kind Cluster Management:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(create|delete|verify|list)-cluster' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Operator Management:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(add-helm|install-operator|uninstall-operator|verify-operator)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Helm Chart Management:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(helm-)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Database Deployment:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(retag|load|deploy|upgrade|undeploy|verify-db)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Validation & Testing:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E 'validate-' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Access & Connectivity:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(port-forward|connect|logs)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Docker Compose:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E 'compose-' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Cleanup:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E 'clean-' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'
	@echo ''
	@echo '$(YELLOW)Complete Workflows:$(NC)'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(setup-all|test-all)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-30s$(NC) %s\n", $$1, $$2}'

##@ Prerequisites

.PHONY: check-prereqs
check-prereqs: ## Check all required tools are installed
	@echo "$(CYAN)Checking prerequisites...$(NC)"
	@command -v docker >/dev/null 2>&1 || (echo "$(RED)✗ docker not found$(NC)" && exit 1)
	@echo "$(GREEN)✓ docker found$(NC)"
	@command -v kind >/dev/null 2>&1 || (echo "$(RED)✗ kind not found - run 'make install-kind'$(NC)" && exit 1)
	@echo "$(GREEN)✓ kind found$(NC)"
	@command -v kubectl >/dev/null 2>&1 || (echo "$(RED)✗ kubectl not found - run 'make install-kubectl'$(NC)" && exit 1)
	@echo "$(GREEN)✓ kubectl found$(NC)"
	@command -v helm >/dev/null 2>&1 || (echo "$(RED)✗ helm not found - run 'make install-helm'$(NC)" && exit 1)
	@echo "$(GREEN)✓ helm found$(NC)"
	@echo "$(GREEN)All prerequisites satisfied!$(NC)"

.PHONY: install-kind
install-kind: ## Install kind (Kubernetes in Docker)
	@echo "$(CYAN)Installing kind...$(NC)"
	@if [ "$$(uname)" = "Darwin" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			brew install kind; \
		else \
			echo "$(YELLOW)Homebrew not found. Installing via curl...$(NC)"; \
			curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-darwin-amd64; \
			chmod +x ./kind; \
			sudo mv ./kind /usr/local/bin/kind; \
		fi \
	elif [ "$$(uname)" = "Linux" ]; then \
		curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-linux-amd64; \
		chmod +x ./kind; \
		sudo mv ./kind /usr/local/bin/kind; \
	fi
	@kind version && echo "$(GREEN)✓ kind installed successfully$(NC)" || echo "$(RED)✗ kind installation failed$(NC)"

.PHONY: install-kubectl
install-kubectl: ## Install kubectl
	@echo "$(CYAN)Installing kubectl...$(NC)"
	@if [ "$$(uname)" = "Darwin" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			brew install kubectl; \
		else \
			curl -LO "https://dl.k8s.io/release/$$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/darwin/amd64/kubectl"; \
			chmod +x ./kubectl; \
			sudo mv ./kubectl /usr/local/bin/kubectl; \
		fi \
	elif [ "$$(uname)" = "Linux" ]; then \
		curl -LO "https://dl.k8s.io/release/$$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"; \
		chmod +x ./kubectl; \
		sudo mv ./kubectl /usr/local/bin/kubectl; \
	fi
	@kubectl version --client && echo "$(GREEN)✓ kubectl installed successfully$(NC)" || echo "$(RED)✗ kubectl installation failed$(NC)"

.PHONY: install-helm
install-helm: ## Install Helm package manager
	@echo "$(CYAN)Installing helm...$(NC)"
	@if [ "$$(uname)" = "Darwin" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			brew install helm; \
		else \
			curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash; \
		fi \
	elif [ "$$(uname)" = "Linux" ]; then \
		curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash; \
	fi
	@helm version && echo "$(GREEN)✓ helm installed successfully$(NC)" || echo "$(RED)✗ helm installation failed$(NC)"

##@ Kind Cluster Management

.PHONY: create-cluster
create-cluster: check-prereqs ## Create kind cluster
	@echo "$(CYAN)Creating kind cluster: $(CLUSTER_NAME)...$(NC)"
	@if kind get clusters 2>/dev/null | grep -q "^$(CLUSTER_NAME)$$"; then \
		echo "$(YELLOW)Cluster $(CLUSTER_NAME) already exists$(NC)"; \
		exit 1; \
	fi
	@kind create cluster --config kind-config.yaml
	@echo "$(GREEN)✓ Cluster created successfully$(NC)"

.PHONY: delete-cluster
delete-cluster: ## Delete kind cluster
	@echo "$(CYAN)Deleting kind cluster: $(CLUSTER_NAME)...$(NC)"
	@if ! kind get clusters 2>/dev/null | grep -q "^$(CLUSTER_NAME)$$"; then \
		echo "$(YELLOW)Cluster $(CLUSTER_NAME) does not exist$(NC)"; \
		exit 0; \
	fi
	@kind delete cluster --name $(CLUSTER_NAME)
	@echo "$(GREEN)✓ Cluster deleted successfully$(NC)"

.PHONY: verify-cluster
verify-cluster: ## Verify kind cluster is ready
	@echo "$(CYAN)Verifying cluster status...$(NC)"
	@if ! kind get clusters 2>/dev/null | grep -q "^$(CLUSTER_NAME)$$"; then \
		echo "$(RED)✗ Cluster $(CLUSTER_NAME) not found$(NC)"; \
		exit 1; \
	fi
	@kubectl cluster-info --context kind-$(CLUSTER_NAME) >/dev/null 2>&1 || (echo "$(RED)✗ Cluster not accessible$(NC)" && exit 1)
	@kubectl wait --for=condition=Ready nodes --all --timeout=60s >/dev/null 2>&1 || (echo "$(RED)✗ Nodes not ready$(NC)" && exit 1)
	@echo "$(GREEN)✓ Cluster is ready$(NC)"
	@kubectl get nodes

.PHONY: list-clusters
list-clusters: ## List all kind clusters
	@echo "$(CYAN)Kind clusters:$(NC)"
	@kind get clusters 2>/dev/null || echo "No clusters found"

##@ Operator Management

.PHONY: add-helm-repo
add-helm-repo: ## Add CloudNativePG Helm repository
	@echo "$(CYAN)Adding CloudNativePG Helm repository...$(NC)"
	@helm repo add cnpg https://cloudnative-pg.github.io/charts 2>/dev/null || \
		(echo "$(YELLOW)Repository already exists, updating...$(NC)" && helm repo add cnpg https://cloudnative-pg.github.io/charts --force-update)
	@helm repo update
	@echo "$(GREEN)✓ Helm repository added$(NC)"

.PHONY: verify-helm-repo
verify-helm-repo: ## Verify CloudNativePG Helm repository
	@echo "$(CYAN)Verifying Helm repository...$(NC)"
	@helm repo list | grep -q cnpg || (echo "$(RED)✗ cnpg repository not found$(NC)" && exit 1)
	@helm search repo cnpg/cloudnative-pg >/dev/null 2>&1 || (echo "$(RED)✗ cnpg chart not found$(NC)" && exit 1)
	@echo "$(GREEN)✓ Helm repository verified$(NC)"
	@helm search repo cnpg/cloudnative-pg

.PHONY: install-operator
install-operator: verify-cluster add-helm-repo ## Install CloudNativePG operator
	@echo "$(CYAN)Installing CloudNativePG operator...$(NC)"
	@if helm list -n $(OPERATOR_NAMESPACE) 2>/dev/null | grep -q cnpg; then \
		echo "$(YELLOW)Operator already installed$(NC)"; \
		exit 1; \
	fi
	@helm install cnpg \
		--namespace $(OPERATOR_NAMESPACE) \
		--create-namespace \
		cnpg/cloudnative-pg
	@echo "$(GREEN)✓ Operator installation initiated$(NC)"

.PHONY: uninstall-operator
uninstall-operator: ## Uninstall CloudNativePG operator
	@echo "$(CYAN)Uninstalling CloudNativePG operator...$(NC)"
	@if ! helm list -n $(OPERATOR_NAMESPACE) 2>/dev/null | grep -q cnpg; then \
		echo "$(YELLOW)Operator not installed$(NC)"; \
		exit 0; \
	fi
	@helm uninstall cnpg -n $(OPERATOR_NAMESPACE)
	@kubectl delete namespace $(OPERATOR_NAMESPACE) --ignore-not-found=true
	@echo "$(GREEN)✓ Operator uninstalled$(NC)"

.PHONY: verify-operator
verify-operator: ## Verify CloudNativePG operator is running
	@echo "$(CYAN)Verifying operator status...$(NC)"
	@if ! kubectl get namespace $(OPERATOR_NAMESPACE) >/dev/null 2>&1; then \
		echo "$(RED)✗ Operator namespace not found$(NC)"; \
		exit 1; \
	fi
	@kubectl wait --for=condition=Available \
		--timeout=300s \
		-n $(OPERATOR_NAMESPACE) \
		deployment/cnpg-cloudnative-pg >/dev/null 2>&1 || (echo "$(RED)✗ Operator not ready$(NC)" && exit 1)
	@echo "$(GREEN)✓ Operator is ready$(NC)"
	@kubectl get deployment -n $(OPERATOR_NAMESPACE)

##@ Database Deployment

.PHONY: retag-image
retag-image: ## Retag Docker image with PostgreSQL version
	@echo "$(CYAN)Retagging Docker image...$(NC)"
	@if ! docker image inspect $(IMAGE_NAME):$(IMAGE_TAG_CURRENT) >/dev/null 2>&1; then \
		echo "$(RED)✗ Source image $(IMAGE_NAME):$(IMAGE_TAG_CURRENT) not found$(NC)"; \
		echo "$(YELLOW)Build it first with: docker build -t $(IMAGE_NAME):$(IMAGE_TAG_CURRENT) .$(NC)"; \
		exit 1; \
	fi
	@docker tag $(IMAGE_NAME):$(IMAGE_TAG_CURRENT) $(IMAGE_NAME):$(IMAGE_TAG_PG_VERSION)
	@echo "$(GREEN)✓ Image retagged as $(IMAGE_NAME):$(IMAGE_TAG_PG_VERSION)$(NC)"

.PHONY: verify-image
verify-image: ## Verify Docker image exists
	@echo "$(CYAN)Verifying Docker image...$(NC)"
	@docker image inspect $(IMAGE_NAME):$(IMAGE_TAG_PG_VERSION) >/dev/null 2>&1 || \
		(echo "$(RED)✗ Image $(IMAGE_NAME):$(IMAGE_TAG_PG_VERSION) not found - run 'make retag-image'$(NC)" && exit 1)
	@echo "$(GREEN)✓ Image found$(NC)"
	@docker images $(IMAGE_NAME):$(IMAGE_TAG_PG_VERSION)

.PHONY: load-image
load-image: verify-cluster verify-image ## Load Docker image into kind cluster
	@echo "$(CYAN)Loading image into kind cluster...$(NC)"
	@kind load docker-image $(IMAGE_NAME):$(IMAGE_TAG_PG_VERSION) --name $(CLUSTER_NAME)
	@echo "$(GREEN)✓ Image loaded into cluster$(NC)"

.PHONY: helm-dependency-update
helm-dependency-update: ## Update Helm chart dependencies
	@echo "$(CYAN)Updating Helm chart dependencies...$(NC)"
	@cd k8s && helm dependency update
	@echo "$(GREEN)✓ Helm dependencies updated$(NC)"

.PHONY: helm-lint
helm-lint: ## Lint Helm chart
	@echo "$(CYAN)Linting Helm chart...$(NC)"
	@helm lint k8s/
	@echo "$(GREEN)✓ Helm chart validated$(NC)"

.PHONY: helm-render
helm-render: helm-dependency-update ## Render Helm chart with default values
	@echo "$(CYAN)Rendering Helm chart with default values...$(NC)"
	@helm template irdb-postgres k8s/ \
		--namespace $(NAMESPACE)

.PHONY: helm-render-dev
helm-render-dev: helm-dependency-update ## Render Helm chart with development values
	@echo "$(CYAN)Rendering Helm chart with development values...$(NC)"
	@helm template irdb-postgres k8s/ \
		--namespace $(NAMESPACE) \
		-f k8s/values-dev.yaml

.PHONY: helm-render-prod
helm-render-prod: helm-dependency-update ## Render Helm chart with production values
	@echo "$(CYAN)Rendering Helm chart with production values...$(NC)"
	@helm template irdb-postgres k8s/ \
		--namespace $(NAMESPACE) \
		-f k8s/values-prod.yaml

.PHONY: helm-diff
helm-diff: ## Show diff between current deployment and chart
	@echo "$(CYAN)Showing differences...$(NC)"
	@if ! helm list -n $(NAMESPACE) 2>/dev/null | grep -q irdb-postgres; then \
		echo "$(YELLOW)Chart not installed yet$(NC)"; \
		exit 1; \
	fi
	@helm diff upgrade irdb-postgres k8s/ \
		--namespace $(NAMESPACE) \
		-f k8s/values-dev.yaml 2>/dev/null || \
		echo "$(YELLOW)helm diff plugin not installed. Install with: helm plugin install https://github.com/databus23/helm-diff$(NC)"

.PHONY: deploy-db
deploy-db: verify-operator helm-dependency-update ## Deploy IR DB instance using Helm
	@echo "$(CYAN)Deploying IR DB instance with Helm...$(NC)"
	@if helm list -n $(NAMESPACE) 2>/dev/null | grep -q irdb-postgres; then \
		echo "$(RED)✗ Database cluster already deployed$(NC)"; \
		echo "$(YELLOW)Use 'make upgrade-db' to upgrade or 'make undeploy-db' to remove first$(NC)"; \
		exit 1; \
	fi
	@helm install irdb-postgres k8s/ \
		--namespace $(NAMESPACE) \
		--create-namespace \
		-f k8s/values-dev.yaml
	@echo "$(GREEN)✓ Database deployment initiated$(NC)"

.PHONY: deploy-db-prod
deploy-db-prod: verify-operator helm-dependency-update ## Deploy IR DB instance for production
	@echo "$(CYAN)Deploying IR DB instance (production) with Helm...$(NC)"
	@if helm list -n $(NAMESPACE) 2>/dev/null | grep -q irdb-postgres; then \
		echo "$(RED)✗ Database cluster already deployed$(NC)"; \
		exit 1; \
	fi
	@helm install irdb-postgres k8s/ \
		--namespace $(NAMESPACE) \
		--create-namespace \
		-f k8s/values-prod.yaml
	@echo "$(GREEN)✓ Database deployment initiated$(NC)"

.PHONY: upgrade-db
upgrade-db: verify-operator helm-dependency-update ## Upgrade existing IR DB deployment
	@echo "$(CYAN)Upgrading IR DB instance...$(NC)"
	@if ! helm list -n $(NAMESPACE) 2>/dev/null | grep -q irdb-postgres; then \
		echo "$(RED)✗ Database cluster not deployed$(NC)"; \
		echo "$(YELLOW)Use 'make deploy-db' to deploy first$(NC)"; \
		exit 1; \
	fi
	@helm upgrade irdb-postgres k8s/ \
		--namespace $(NAMESPACE) \
		-f k8s/values-dev.yaml
	@echo "$(GREEN)✓ Database upgrade initiated$(NC)"

.PHONY: undeploy-db
undeploy-db: ## Remove IR DB instance
	@echo "$(CYAN)Removing IR DB instance...$(NC)"
	@if ! helm list -n $(NAMESPACE) 2>/dev/null | grep -q irdb-postgres; then \
		echo "$(YELLOW)Database cluster not deployed$(NC)"; \
		exit 0; \
	fi
	@helm uninstall irdb-postgres -n $(NAMESPACE)
	@echo "$(YELLOW)Waiting for resources to be deleted...$(NC)"
	@sleep 5
	@echo "$(GREEN)✓ Database removed$(NC)"
	@echo "$(YELLOW)Note: PVCs are retained. To delete them: kubectl delete pvc -n $(NAMESPACE) -l cnpg.io/cluster=postgres$(NC)"

.PHONY: verify-db
verify-db: ## Verify IR DB is running
	@echo "$(CYAN)Verifying database status...$(NC)"
	@if ! kubectl get namespace $(NAMESPACE) >/dev/null 2>&1; then \
		echo "$(RED)✗ Namespace $(NAMESPACE) not found$(NC)"; \
		exit 1; \
	fi
	@if ! kubectl get cluster -n $(NAMESPACE) irdb-postgres >/dev/null 2>&1; then \
		echo "$(RED)✗ Cluster resource not found$(NC)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)Waiting for database pods to be ready (this may take a few minutes)...$(NC)"
	@kubectl wait --for=condition=Ready \
		--timeout=600s \
		-n $(NAMESPACE) \
		pod -l cnpg.io/cluster=irdb-postgres 2>/dev/null || (echo "$(RED)✗ Database pods not ready$(NC)" && exit 1)
	@echo "$(GREEN)✓ Database is ready$(NC)"
	@kubectl get cluster -n $(NAMESPACE)
	@kubectl get pods -n $(NAMESPACE)

##@ Validation & Testing

.PHONY: validate-extensions
validate-extensions: verify-db ## Validate extensions are installed
	@echo "$(CYAN)Validating extensions...$(NC)"
	@./k8s/verify-extensions.sh
	@echo "$(GREEN)✓ Extensions validated$(NC)"

.PHONY: validate-bm25
validate-bm25: verify-db ## Test BM25 full-text search
	@echo "$(CYAN)Testing BM25 search...$(NC)"
	@POD=$$(kubectl get pod -n $(NAMESPACE) -l cnpg.io/cluster=irdb-postgres,role=primary -o jsonpath='{.items[0].metadata.name}'); \
	kubectl exec -n $(NAMESPACE) $$POD -- psql -U $(DB_USER) -d $(DB_NAME) -c "\
		INSERT INTO ai_data.documents (title, content, embedding) VALUES \
		('PostgreSQL Guide', 'PostgreSQL is a powerful open source database', ai_data.generate_random_vector(1536)), \
		('ParadeDB Tutorial', 'ParadeDB extends PostgreSQL with search capabilities', ai_data.generate_random_vector(1536)), \
		('Vector Search', 'Using pgvector for similarity search', ai_data.generate_random_vector(1536)) \
		ON CONFLICT DO NOTHING;" && \
	kubectl exec -n $(NAMESPACE) $$POD -- psql -U $(DB_USER) -d $(DB_NAME) -c "\
		SELECT id, title, \
		ts_rank(to_tsvector('english', title || ' ' || content), to_tsquery('english', 'PostgreSQL')) as score \
		FROM ai_data.documents \
		WHERE to_tsvector('english', title || ' ' || content) @@ to_tsquery('english', 'PostgreSQL') \
		ORDER BY score DESC \
		LIMIT 5;"
	@echo "$(GREEN)✓ BM25 search test passed$(NC)"

.PHONY: validate-vector
validate-vector: verify-db ## Test vector similarity search
	@echo "$(CYAN)Testing vector search...$(NC)"
	@POD=$$(kubectl get pod -n $(NAMESPACE) -l cnpg.io/cluster=irdb-postgres,role=primary -o jsonpath='{.items[0].metadata.name}'); \
	kubectl exec -n $(NAMESPACE) $$POD -- psql -U $(DB_USER) -d $(DB_NAME) -c "\
		WITH query_vector AS ( \
			SELECT ai_data.generate_random_vector(1536) as qv \
		) \
		SELECT d.id, d.title, \
		1 - (d.embedding <=> query_vector.qv) as similarity \
		FROM ai_data.documents d, query_vector \
		ORDER BY d.embedding <=> query_vector.qv \
		LIMIT 5;"
	@echo "$(GREEN)✓ Vector search test passed$(NC)"

.PHONY: validate-hybrid
validate-hybrid: verify-db ## Test hybrid search (vector + BM25)
	@echo "$(CYAN)Testing hybrid search...$(NC)"
	@POD=$$(kubectl get pod -n $(NAMESPACE) -l cnpg.io/cluster=irdb-postgres,role=primary -o jsonpath='{.items[0].metadata.name}'); \
	kubectl exec -n $(NAMESPACE) $$POD -- psql -U $(DB_USER) -d $(DB_NAME) -c "\
		SELECT * FROM ai_data.hybrid_search( \
			query_text => 'PostgreSQL database', \
			query_embedding => (SELECT ai_data.generate_random_vector(1536)), \
			similarity_threshold => 0.0, \
			limit_count => 5 \
		);"
	@echo "$(GREEN)✓ Hybrid search test passed$(NC)"

.PHONY: validate-all
validate-all: validate-extensions validate-bm25 validate-vector validate-hybrid ## Run all validation tests
	@echo "$(GREEN)✓ All validation tests passed!$(NC)"

##@ Access & Connectivity

.PHONY: port-forward
port-forward: verify-db ## Setup port-forward to database (Ctrl+C to stop)
	@echo "$(CYAN)Setting up port-forward to database...$(NC)"
	@echo "$(YELLOW)Connect with: psql -h localhost -U $(DB_USER) -d $(DB_NAME) -p $(DB_PORT)$(NC)"
	@echo "$(YELLOW)Password: $(DB_PASSWORD)$(NC)"
	@kubectl port-forward -n $(NAMESPACE) svc/irdb-postgres-rw $(DB_PORT):$(DB_PORT)

.PHONY: connect
connect: ## Connect to database using psql (requires port-forward or NodePort)
	@echo "$(CYAN)Connecting to database...$(NC)"
	@PGPASSWORD=$(DB_PASSWORD) psql -h localhost -U $(DB_USER) -d $(DB_NAME) -p $(DB_PORT)

.PHONY: logs
logs: ## View database logs
	@echo "$(CYAN)Viewing database logs...$(NC)"
	@kubectl logs -n $(NAMESPACE) -l cnpg.io/cluster=irdb-postgres --tail=100 -f

.PHONY: status
status: ## Show cluster and pod status
	@echo "$(CYAN)Cluster status:$(NC)"
	@kubectl get cluster -n $(NAMESPACE) 2>/dev/null || echo "No clusters found"
	@echo ""
	@echo "$(CYAN)Pod status:$(NC)"
	@kubectl get pods -n $(NAMESPACE) 2>/dev/null || echo "No pods found"

##@ Docker Compose

.PHONY: compose-build
compose-build: ## Build Docker image
	@echo "$(CYAN)Building Docker image...$(NC)"
	@docker build -t $(IMAGE_NAME):$(IMAGE_TAG_CURRENT) .
	@echo "$(GREEN)✓ Image built successfully$(NC)"

.PHONY: compose-up
compose-up: ## Start Docker Compose services
	@echo "$(CYAN)Starting Docker Compose services...$(NC)"
	@docker-compose up -d
	@echo "$(GREEN)✓ Services started$(NC)"
	@echo "$(YELLOW)PostgreSQL: localhost:5432$(NC)"
	@echo "$(YELLOW)pgAdmin: http://localhost:5433$(NC)"

.PHONY: compose-down
compose-down: ## Stop Docker Compose services
	@echo "$(CYAN)Stopping Docker Compose services...$(NC)"
	@docker-compose down
	@echo "$(GREEN)✓ Services stopped$(NC)"

.PHONY: compose-clean
compose-clean: ## Stop services and remove volumes
	@echo "$(CYAN)Cleaning Docker Compose setup...$(NC)"
	@docker-compose down -v
	@echo "$(GREEN)✓ Services stopped and volumes removed$(NC)"

.PHONY: compose-logs
compose-logs: ## View Docker Compose logs
	@docker-compose logs -f

.PHONY: compose-restart
compose-restart: compose-down compose-up ## Restart Docker Compose services

##@ Cleanup

.PHONY: clean-db
clean-db: undeploy-db ## Remove database deployment only

.PHONY: clean-operator
clean-operator: uninstall-operator ## Remove operator only

.PHONY: clean-cluster
clean-cluster: delete-cluster ## Remove kind cluster only

.PHONY: clean-all
clean-all: ## Remove everything (cluster, operator, database)
	@echo "$(CYAN)Cleaning all resources...$(NC)"
	@$(MAKE) undeploy-db || true
	@$(MAKE) uninstall-operator || true
	@$(MAKE) delete-cluster || true
	@echo "$(GREEN)✓ All resources cleaned$(NC)"

##@ Complete Workflows

.PHONY: setup-all
setup-all: create-cluster verify-cluster install-operator verify-operator retag-image load-image deploy-db verify-db ## Complete setup from scratch
	@echo "$(GREEN)✓✓✓ Complete setup finished! ✓✓✓$(NC)"
	@echo ""
	@echo "$(CYAN)Next steps:$(NC)"
	@echo "  1. Run validation: $(YELLOW)make validate-all$(NC)"
	@echo "  2. Connect to DB: $(YELLOW)make connect$(NC) (or use NodePort on localhost:5432)"
	@echo "  3. View logs: $(YELLOW)make logs$(NC)"
	@echo ""
	@echo "$(CYAN)Connection details:$(NC)"
	@echo "  Host: localhost"
	@echo "  Port: $(DB_PORT)"
	@echo "  Database: $(DB_NAME)"
	@echo "  Username: $(DB_USER)"
	@echo "  Password: $(DB_PASSWORD)"

.PHONY: test-all
test-all: verify-cluster verify-operator verify-db validate-all ## Verify everything is working
	@echo "$(GREEN)✓✓✓ All tests passed! ✓✓✓$(NC)"
