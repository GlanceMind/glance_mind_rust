# ===== Chef Stage - Prepare dependency recipe =====
FROM rust:latest AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ===== Planner Stage - Analyze dependencies =====
FROM chef AS planner
# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# Generate recipe.json with all dependency info
RUN cargo chef prepare --recipe-path recipe.json

# ===== Builder Stage - Build application =====
FROM chef AS builder
# Copy dependency recipe
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (this layer is cached unless Cargo.toml/lock changes)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy workspace source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build application (only compiles app code, dependencies are cached)
RUN cargo build --release -p glance_mind_api && \
    cp /app/target/release/glance_mind_api /app/glance_mind_api

# ===== Runtime Stage - Final image =====
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libpq-dev \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /app/glance_mind_api /usr/local/bin/glance_mind_api

# Set permissions
RUN chmod +x /usr/local/bin/glance_mind_api

# Expose port
EXPOSE 8000

# Run application
CMD ["glance_mind_api"]
