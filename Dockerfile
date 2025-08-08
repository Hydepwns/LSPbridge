# Build stage
FROM rust:1.75-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/lspbridge

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY benches ./benches
COPY tests ./tests
COPY resources ./resources

# Build release binary
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 lspbridge

# Copy binary from builder
COPY --from=builder /usr/src/lspbridge/target/release/lspbridge /usr/local/bin/lspbridge

# Copy default configuration
COPY --from=builder /usr/src/lspbridge/resources/default.lspbridge.toml /etc/lspbridge/default.toml

# Set ownership
RUN chown -R lspbridge:lspbridge /etc/lspbridge

# Switch to non-root user
USER lspbridge

# Set working directory
WORKDIR /workspace

# Expose default port for API (if applicable)
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV LSP_BRIDGE_CONFIG=/etc/lspbridge/default.toml

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD lspbridge config validate || exit 1

# Default command
ENTRYPOINT ["lspbridge"]
CMD ["--help"]