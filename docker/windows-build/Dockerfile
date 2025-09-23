# Reusable image for cross-compiling to x86_64-pc-windows-gnu
FROM rust:latest

# Install cross toolchain and build deps
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    mingw-w64 \
    protobuf-compiler \
    cmake \
    pkg-config \
    build-essential \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Install the Rust target so the image is ready to run builds immediately
RUN rustup target add x86_64-pc-windows-gnu || true

# Set a sensible workdir
WORKDIR /usr/src/myapp

# Default command is a shell so scripts can run commands inside the container
CMD ["sh"]
