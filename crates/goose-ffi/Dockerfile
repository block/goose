FROM --platform=linux/amd64 ubuntu:22.04 as builder

# Install build tools
RUN apt-get update && apt-get install -y \
    curl \
    pkg-config \
    build-essential \
    perl \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# OpenSSL configuration
ENV OPENSSL_DIR=/usr
ENV OPENSSL_STATIC=1

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p crates

# Copy workspace crates
COPY crates/goose crates/goose/
COPY crates/goose-ffi/Cargo.toml crates/goose-ffi/
COPY crates/goose-ffi/src crates/goose-ffi/src/
COPY crates/mcp-client crates/mcp-client/
COPY crates/mcp-core crates/mcp-core/
COPY crates/mcp-macros crates/mcp-macros/
COPY crates/mcp-server crates/mcp-server/

# Build the cdylib
RUN cargo build --release --package goose-ffi --lib && \
    test -f target/release/libgoose_ffi.so && \
    ls -lh target/release/

# --- Final image stage ---
FROM alpine:latest
WORKDIR /app

# Copy only the final .so
COPY --from=builder /build/target/release/libgoose_ffi.so .

CMD ["cp", "/app/libgoose_ffi.so", "/output/"]