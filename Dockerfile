# Build stage - contains all build tools and dependencies
FROM postgres:17.5-bookworm AS builder

# Configure APT to avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    build-essential \
    git \
    libssl-dev \
    pkg-config \
    libclang-dev \
    postgresql-server-dev-17 \
    bison \
    flex \
    libreadline-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    /root/.cargo/bin/cargo install cargo-pgrx --version 0.15.0 --locked

# Build ParadeDB pgsearch extension
RUN git clone --branch v0.17.2 https://github.com/paradedb/paradedb.git /tmp/paradedb && \
    cd /tmp/paradedb/pg_search && \
    /root/.cargo/bin/cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config && \
    /root/.cargo/bin/cargo pgrx install --release

# Runtime stage - minimal PostgreSQL image with only necessary components
FROM postgres:17.5-bookworm AS runtime

# Configure APT to avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    postgresql-contrib \
    postgresql-17-pgvector \
    && rm -rf /var/lib/apt/lists/*

# Copy built ParadeDB extension from builder stage
COPY --from=builder /usr/lib/postgresql/17/lib/pg_search.so /usr/lib/postgresql/17/lib/
COPY --from=builder /usr/share/postgresql/17/extension/pg_search* /usr/share/postgresql/17/extension/

# Copy PostgreSQL configuration
COPY postgresql.conf /etc/postgresql/postgresql.conf

# Copy initialization scripts
COPY docker-entrypoint-initdb.d/ /docker-entrypoint-initdb.d/

# Extensions are created via the mounted initialization scripts

# Set up user
USER postgres
