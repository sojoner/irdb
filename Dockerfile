# Use a Debian-based approach for better extension compatibility
FROM postgres:17.5-bookworm

# Configure APT to avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install required packages including lsb-release for repository identification
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    lsb-release \
    build-essential \ 
    git \ 
    libssl-dev \ 
    pkg-config \ 
    libclang-dev \
    postgresql-server-dev-17 \
    && rm -rf /var/lib/apt/lists/*

# Copy PostgreSQL configuration
COPY postgresql.conf /etc/postgresql/postgresql.conf

# Copy initialization scripts
COPY docker-entrypoint-initdb.d/ /docker-entrypoint-initdb.d/

# Install standard PostgreSQL extensions
RUN apt-get update && apt-get install -y \
    postgresql-contrib \
    && rm -rf /var/lib/apt/lists/*

# Install pgvector extension for PostgreSQL 17
RUN apt-get update && apt-get install -y postgresql-17-pgvector

# Install dependencies for cargo pgrx1
RUN apt-get update && apt-get install -y \
    bison \
    flex \
    libreadline-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    /root/.cargo/bin/cargo install cargo-pgrx --version 0.15.0 --locked

# Install ParadeDB pgsearch extension    
RUN git clone --branch v0.17.2 https://github.com/paradedb/paradedb.git /tmp/paradedb && \
    cd /tmp/paradedb/pg_search && \
    /root/.cargo/bin/cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config && \
    /root/.cargo/bin/cargo pgrx install --release && \
    rm -rf /tmp/paradedb


# Set up user
USER postgres
