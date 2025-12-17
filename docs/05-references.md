# References & Resources

This document contains all upstream documentation, research papers, Git repositories, and learning resources used in the IRDB project.

## Core Technologies

### PostgreSQL
- **Website**: https://www.postgresql.org/
- **Documentation**: https://www.postgresql.org/docs/current/
- **Full-Text Search**: https://www.postgresql.org/docs/current/textsearch.html
- **Version Used**: 17.5
- **Key Features**: ACID compliance, extensibility, advanced indexing

### ParadeDB pg_search
- **Website**: https://www.paradedb.com/
- **Documentation**: https://docs.paradedb.com/search/overview
- **GitHub**: https://github.com/paradedb/paradedb
- **Version Used**: 0.20.2
- **Key Features**:
  - BM25 ranking algorithm
  - Custom search operators (`|||`, `&&&`)
  - Inverted index with fast keyword matching
  - Built on Apache Lucene algorithms

**Relevant Documentation Pages**:
- [BM25 Search](https://docs.paradedb.com/search/bm25)
- [Search Operators](https://docs.paradedb.com/search/full_text/operators)
- [Index Configuration](https://docs.paradedb.com/search/indexing)

### pgvector
- **GitHub**: https://github.com/pgvector/pgvector
- **Documentation**: https://github.com/pgvector/pgvector#readme
- **Version Used**: 0.8.0
- **Key Features**:
  - Vector similarity search
  - HNSW and IVFFlat indexes
  - Cosine distance, L2 distance, inner product
  - 1536-dimension embeddings support

**Relevant Documentation**:
- [HNSW Indexing](https://github.com/pgvector/pgvector#hnsw)
- [Distance Functions](https://github.com/pgvector/pgvector#distance)
- [Performance Tuning](https://github.com/pgvector/pgvector#performance)

## Rust Ecosystem

### Leptos
- **Website**: https://leptos.dev/
- **GitHub**: https://github.com/leptos-rs/leptos
- **Documentation**: https://leptos.dev/guide/
- **Version Used**: 0.7+
- **Key Features**:
  - Fine-grained reactivity
  - Server-side rendering
  - WebAssembly compilation
  - Type-safe server functions

**Relevant Guides**:
- [Server Functions](https://leptos.dev/guide/server_functions.html)
- [Components](https://leptos.dev/guide/components.html)
- [Routing](https://leptos.dev/guide/routing.html)
- [Deployment](https://leptos.dev/deployment/)

### sqlx
- **GitHub**: https://github.com/launchbadge/sqlx
- **Documentation**: https://docs.rs/sqlx/
- **Version Used**: 0.7
- **Key Features**:
  - Compile-time query checking
  - Async PostgreSQL driver
  - Connection pooling
  - Type-safe query macros

**Relevant Documentation**:
- [Getting Started](https://github.com/launchbadge/sqlx#getting-started)
- [Query Macros](https://docs.rs/sqlx/latest/sqlx/macro.query.html)
- [PostgreSQL Types](https://docs.rs/sqlx/latest/sqlx/postgres/types/index.html)

### Actix-web
- **Website**: https://actix.rs/
- **GitHub**: https://github.com/actix/actix-web
- **Documentation**: https://actix.rs/docs/
- **Version Used**: 4.x
- **Key Features**:
  - High-performance HTTP server
  - Middleware support
  - WebSocket support
  - Type-safe extractors

## Kubernetes & Cloud Native

### CloudNativePG
- **Website**: https://cloudnative-pg.io/
- **Documentation**: https://cloudnative-pg.io/documentation/
- **GitHub**: https://github.com/cloudnative-pg/cloudnative-pg
- **Version Used**: Latest (via Helm)
- **Key Features**:
  - PostgreSQL operator for Kubernetes
  - Automated failover and high availability
  - Continuous backup and PITR
  - Declarative configuration

**Relevant Documentation**:
- [Architecture](https://cloudnative-pg.io/documentation/1.24/architecture/)
- [Backup & Recovery](https://cloudnative-pg.io/documentation/1.24/backup_recovery/)
- [High Availability](https://cloudnative-pg.io/documentation/1.24/replication/)
- [Monitoring](https://cloudnative-pg.io/documentation/1.24/monitoring/)

### Helm
- **Website**: https://helm.sh/
- **Documentation**: https://helm.sh/docs/
- **GitHub**: https://github.com/helm/helm
- **Version Used**: 3.12+
- **Key Features**:
  - Kubernetes package manager
  - Template engine
  - Release management
  - Chart dependencies

## Academic Papers & Research

### BM25 Ranking Algorithm
- **Paper**: [The Probabilistic Relevance Framework: BM25 and Beyond](https://www.staff.city.ac.uk/~sbrp622/papers/foundations_bm25_review.pdf)
- **Authors**: Stephen Robertson, Hugo Zaragoza
- **Year**: 2009
- **Summary**: Comprehensive overview of BM25 algorithm, its theoretical foundation, and practical applications in information retrieval.

**Key Insights**:
- BM25 considers term frequency (TF), document frequency (IDF), and document length normalization
- Parameters k1 (term saturation) and b (length normalization) are tunable
- Industry standard for text search (used in Elasticsearch, Solr, Lucene)

### HNSW (Hierarchical Navigable Small World)
- **Paper**: [Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs](https://arxiv.org/abs/1603.09320)
- **Authors**: Yu. A. Malkov, D. A. Yashunin
- **Year**: 2016
- **arXiv**: https://arxiv.org/abs/1603.09320
- **Summary**: Graph-based algorithm for approximate nearest neighbor (ANN) search with logarithmic complexity.

**Key Insights**:
- Multi-layer graph structure for hierarchical navigation
- Parameters: M (connections per node), ef_construction (build quality), ef_search (query quality)
- O(log N) query time with high recall (>95% at proper settings)
- Used in production by major companies (Spotify, Pinterest, etc.)

### Dense Passage Retrieval
- **Paper**: [Dense Passage Retrieval for Open-Domain Question Answering](https://arxiv.org/abs/2004.04906)
- **Authors**: Vladimir Karpukhin et al. (Facebook AI)
- **Year**: 2020
- **arXiv**: https://arxiv.org/abs/2004.04906
- **Summary**: Neural retrieval approach using dense embeddings for semantic search.

**Key Insights**:
- Pre-trained language models (BERT) for encoding questions and passages
- Outperforms BM25 on semantic similarity tasks
- Complementary to lexical methods (basis for hybrid search)

### Hybrid Search Approaches
- **Paper**: [Combining Lexical and Semantic Retrieval for Question Answering](https://arxiv.org/abs/2104.00445)
- **Year**: 2021
- **Summary**: Explores weighted combination of BM25 and dense retrieval.

**Key Insights**:
- Linear combination of scores is effective and efficient
- Optimal weights vary by dataset (typically 20-40% lexical, 60-80% semantic)
- Late fusion (score combination) outperforms early fusion (query rewriting)

## Related Projects & Inspiration

### Vespa
- **Website**: https://vespa.ai/
- **GitHub**: https://github.com/vespa-engine/vespa
- **Hybrid Search**: https://docs.vespa.ai/en/ranking.html#hybrid-ranking
- **Description**: Open-source big data serving engine with native hybrid search support

### Elasticsearch
- **Website**: https://www.elastic.co/elasticsearch/
- **Learning to Rank**: https://elasticsearch-learning-to-rank.readthedocs.io/
- **Description**: Distributed search and analytics engine with plugin-based extensibility

### Weaviate
- **Website**: https://weaviate.io/
- **Hybrid Search**: https://weaviate.io/developers/weaviate/search/hybrid
- **GitHub**: https://github.com/weaviate/weaviate
- **Description**: Vector database with native hybrid search capabilities

### Qdrant
- **Website**: https://qdrant.tech/
- **GitHub**: https://github.com/qdrant/qdrant
- **Hybrid Search**: https://qdrant.tech/documentation/concepts/hybrid-search/
- **Description**: Vector similarity search engine written in Rust

## Learning Resources

### Vector Search Fundamentals
- [Pinecone Learning Center](https://www.pinecone.io/learn/)
- [Approximate Nearest Neighbor Oh Yeah (ANN Benchmarks)](http://ann-benchmarks.com/)
- [Understanding HNSW](https://www.pinecone.io/learn/series/faiss/hnsw/) - Pinecone tutorial

### Information Retrieval
- [Stanford IR Textbook](https://nlp.stanford.edu/IR-book/) - Free online textbook
- [Modern Information Retrieval](http://grupoweb.upf.es/mir2ed/) - Comprehensive textbook

### Rust WebAssembly
- [WASM Bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [Rust WASM Book](https://rustwasm.github.io/docs/book/)

### PostgreSQL Extensions
- [Writing PostgreSQL Extensions](https://www.postgresql.org/docs/current/extend.html)
- [pgrx (Rust Extensions)](https://github.com/pgcentralfoundation/pgrx)

## Embeddings & Language Models

### OpenAI Embeddings
- **API**: https://platform.openai.com/docs/guides/embeddings
- **Model Used**: text-embedding-ada-002 (1536 dimensions)
- **Alternatives**: text-embedding-3-small, text-embedding-3-large

### Open Source Alternatives
- **Sentence Transformers**: https://www.sbert.net/
  - Models: all-MiniLM-L6-v2, all-mpnet-base-v2
  - GitHub: https://github.com/UKPLab/sentence-transformers

- **FastEmbed**: https://github.com/qdrant/fastembed
  - Lightweight embedding library in Rust
  - Multiple model support

- **Ollama**: https://ollama.ai/
  - Local LLM and embedding models
  - Easy deployment

## Tools & Infrastructure

### Docker
- **Website**: https://www.docker.com/
- **Documentation**: https://docs.docker.com/
- **Multi-stage Builds**: https://docs.docker.com/build/building/multi-stage/

### Kubernetes
- **Website**: https://kubernetes.io/
- **Documentation**: https://kubernetes.io/docs/
- **kind (Kubernetes in Docker)**: https://kind.sigs.k8s.io/

### ArgoCD
- **Website**: https://argo-cd.readthedocs.io/
- **GitHub**: https://github.com/argoproj/argo-cd
- **Getting Started**: https://argo-cd.readthedocs.io/en/stable/getting_started/

## Monitoring & Observability

### Prometheus
- **Website**: https://prometheus.io/
- **PostgreSQL Exporter**: https://github.com/prometheus-community/postgres_exporter

### Grafana
- **Website**: https://grafana.com/
- **CloudNativePG Dashboard**: https://grafana.com/grafana/dashboards/20417-cloudnativepg/

## Community & Support

### Forums & Discussion
- **PostgreSQL Mailing Lists**: https://www.postgresql.org/list/
- **Rust Users Forum**: https://users.rust-lang.org/
- **Leptos Discord**: https://discord.gg/leptos
- **ParadeDB Discord**: https://discord.gg/paradedb

### Stack Overflow Tags
- [postgresql](https://stackoverflow.com/questions/tagged/postgresql)
- [rust](https://stackoverflow.com/questions/tagged/rust)
- [pgvector](https://stackoverflow.com/questions/tagged/pgvector)
- [leptos](https://stackoverflow.com/questions/tagged/leptos)

## Benchmarks & Comparisons

### Vector Database Benchmarks
- [ANN Benchmarks](http://ann-benchmarks.com/) - Comprehensive ANN algorithm comparison
- [VectorDBBench](https://github.com/zilliztech/VectorDBBench) - Vector database performance comparison

### PostgreSQL Performance
- [PGBench](https://www.postgresql.org/docs/current/pgbench.html) - Built-in benchmarking tool
- [pg_stat_statements](https://www.postgresql.org/docs/current/pgstatstatements.html) - Query statistics

## License & Legal

### Licenses Used in This Project
- **IRDB**: Apache License 2.0
- **PostgreSQL**: PostgreSQL License (similar to MIT)
- **ParadeDB**: Apache License 2.0 / MIT
- **pgvector**: PostgreSQL License
- **Rust**: MIT / Apache 2.0 dual license
- **Leptos**: MIT License

### Apache License 2.0
- **Text**: https://www.apache.org/licenses/LICENSE-2.0
- **FAQ**: https://www.apache.org/foundation/license-faq.html
- **Summary**: Permissive license allowing commercial use, modification, distribution, patent use

## Credits & Acknowledgments

This project builds upon the work of many open-source contributors:

- **PostgreSQL Global Development Group** - Core database
- **ParadeDB Team** - BM25 search extension
- **pgvector Contributors** - Vector similarity search
- **Leptos Team** - Rust web framework
- **CloudNativePG Team** - Kubernetes operator
- **Rust Community** - Language and ecosystem

## Contributing

To contribute to IRDB or report issues:
- **GitHub**: https://github.com/yourusername/irdb
- **Issues**: https://github.com/yourusername/irdb/issues
- **Pull Requests**: https://github.com/yourusername/irdb/pulls

## Stay Updated

### Relevant Blogs & Newsletters
- [PostgreSQL Weekly](https://postgresweekly.com/)
- [Rust Weekly](https://this-week-in-rust.org/)
- [This Week in Databases](https://dbweekly.com/)

### Conferences
- **PGConf** - PostgreSQL Conference (various locations)
- **RustConf** - Annual Rust conference
- **KubeCon** - Cloud Native Computing Foundation conference

---

Last Updated: 2025-12-17
