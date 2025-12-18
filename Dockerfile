# Version variables for easier maintenance and updates
ARG POSTGRES_VERSION=18.1
ARG POSTGRES_VARIANT=bookworm
ARG PG_MAJOR_VERSION=18
ARG CARGO_PGRX_VERSION=0.16.1
ARG PARADEDB_VERSION=0.21.2
ARG AGE_VERSION=1.7.0

# Build stage - contains all build tools and dependencies
FROM postgres:${POSTGRES_VERSION}-${POSTGRES_VARIANT} AS builder

# Configure APT to avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive
ENV PG_MAJOR_VERSION=18
ENV CARGO_PGRX_VERSION=0.16.1
ENV PARADEDB_VERSION=0.21.2
ENV AGE_VERSION=1.7.0

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    build-essential \
    git \
    libssl-dev \
    pkg-config \
    libclang-dev \
    postgresql-server-dev-${PG_MAJOR_VERSION} \
    bison \
    flex \
    libreadline-dev \
    zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain (cached layer)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Install cargo-pgrx separately to cache this layer
RUN /root/.cargo/bin/cargo install cargo-pgrx --version ${CARGO_PGRX_VERSION} --locked

# Set Rust build environment for optimized compilation
ENV CARGO_BUILD_JOBS=8
ENV RUSTFLAGS="-C opt-level=3"

# Clone ParadeDB (separate layer for better caching)
RUN git clone --branch v${PARADEDB_VERSION} https://github.com/paradedb/paradedb.git /tmp/paradedb

# Build ParadeDB pgsearch extension with cache mounts
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/tmp/paradedb/pg_search/target \
    cd /tmp/paradedb/pg_search && \
    /root/.cargo/bin/cargo pgrx init --pg${PG_MAJOR_VERSION}=/usr/lib/postgresql/${PG_MAJOR_VERSION}/bin/pg_config && \
    /root/.cargo/bin/cargo pgrx install --release

# Clone Apache AGE (separate layer for better caching)
RUN git clone --branch PG${PG_MAJOR_VERSION}/v${AGE_VERSION}-rc0 https://github.com/apache/age.git /tmp/age

# Build Apache AGE extension using PostgreSQL module system
RUN cd /tmp/age && \
    make && \
    make install

# Runtime stage - minimal PostgreSQL image with only necessary components
FROM postgres:${POSTGRES_VERSION}-${POSTGRES_VARIANT} AS runtime

# Configure APT to avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive
ENV PG_MAJOR_VERSION=18

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    postgresql-contrib \
    postgresql-${PG_MAJOR_VERSION}-pgvector \
    && rm -rf /var/lib/apt/lists/*

# Copy built ParadeDB extension from builder stage
COPY --from=builder /usr/lib/postgresql/${PG_MAJOR_VERSION}/lib/pg_search.so /usr/lib/postgresql/${PG_MAJOR_VERSION}/lib/
COPY --from=builder /usr/share/postgresql/${PG_MAJOR_VERSION}/extension/pg_search* /usr/share/postgresql/${PG_MAJOR_VERSION}/extension/

# Copy built Apache AGE extension from builder stage
COPY --from=builder /usr/lib/postgresql/${PG_MAJOR_VERSION}/lib/age.so /usr/lib/postgresql/${PG_MAJOR_VERSION}/lib/
COPY --from=builder /usr/share/postgresql/${PG_MAJOR_VERSION}/extension/age* /usr/share/postgresql/${PG_MAJOR_VERSION}/extension/

# Copy PostgreSQL configuration
COPY postgresql.conf /etc/postgresql/postgresql.conf

# Copy initialization scripts
COPY docker-entrypoint-initdb.d/ /docker-entrypoint-initdb.d/

# Extensions are created via the mounted initialization scripts

# Use postgres user for running the database
USER postgres

# Use default postgres entrypoint with custom config
CMD ["postgres", "-c", "config_file=/etc/postgresql/postgresql.conf"]
