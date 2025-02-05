# Build stage
FROM rust:1.84 AS builder

WORKDIR /usr/src/goose

# Copy only the files needed for building
COPY Cargo.toml ./
COPY crates ./crates

# Install minimal build dependenciesd
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libdbus-1-dev && \
    rm -rf /var/lib/apt/lists/*

# Build the binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates libxcb1 libdbus-1-3 && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/goose/target/release/goose /app/goose

ENV GOOSE_KEYRING_BACKEND=file

CMD ["/app/goose"]