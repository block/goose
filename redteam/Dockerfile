# syntax=docker/dockerfile:1.4
# Red Team Goose Distro
# Based on Kali Linux with custom security tools

# Build stage - same as original goose
FROM rust:1.82-bookworm AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libdbus-1-dev \
    protobuf-compiler \
    libprotobuf-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /build

# Copy source code (from root context)
COPY . .

# Build release binaries with optimizations
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ENV CARGO_PROFILE_RELEASE_LTO=true
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=z
ENV CARGO_PROFILE_RELEASE_STRIP=true
RUN cargo build --release --package goose-cli

# Runtime stage - Kali Linux Rolling
FROM kalilinux/kali-rolling

# Install security tools and dependencies
# This layer will be large, but it contains the core value of the distro
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    python3-full \
    python3-pip \
    pipx \
    nmap \
    tshark \
    metasploit-framework \
    exploitdb \
    burpsuite \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install python mcp library
# Use --break-system-packages because on Kali/Debian 12+ pip is managed externally usually,
# but for a container, it's fine to install system-wide or use venv.
# Given it's a dedicated container, system-wide is acceptable for simplicity.
RUN pip3 install mcp --break-system-packages

# Copy goose binary from builder
COPY --from=builder /build/target/release/goose /usr/local/bin/goose

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash goose && \
    mkdir -p /home/goose/.config/goose && \
    chown -R goose:goose /home/goose

# Create directory for redteam tools
RUN mkdir -p /opt/redteam
COPY redteam/tools/security_tools.py /opt/redteam/security_tools.py
COPY redteam/recipe.yaml /home/goose/recipe.yaml

# Ensure tools are executable/readable
RUN chmod +x /opt/redteam/security_tools.py && \
    chown -R goose:goose /opt/redteam && \
    chown goose:goose /home/goose/recipe.yaml

# Set up environment
ENV PATH="/usr/local/bin:${PATH}"
ENV HOME="/home/goose"

# Switch to non-root user
USER goose
WORKDIR /home/goose

# Default to running the redteam recipe
ENTRYPOINT ["/usr/local/bin/goose"]
CMD ["run", "--recipe", "/home/goose/recipe.yaml"]

# Labels
LABEL org.opencontainers.image.title="goose-redteam"
LABEL org.opencontainers.image.description="Red Team focused Goose Distribution"
