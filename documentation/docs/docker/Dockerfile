# Build stage
FROM rust:bullseye AS builder

SHELL ["/bin/bash", "-c"]

# Install Node.js and any missing dependencies
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - && \
    apt-get update && apt-get install -y \
    nodejs \
    libdbus-1-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new directory for the app
WORKDIR /usr/src/goose

# Copy the entire project
COPY . .

RUN apt-get update && apt-get install -y protobuf-compiler

# Build the project
RUN cargo build --release

# Runtime stage
FROM ubuntu:22.04

# Configure non-interactive installation
ENV DEBIAN_FRONTEND=noninteractive

# Install runtime libraries with DBus and keyring support
RUN apt-get update && apt-get install -y \
    # Runtime dependencies
    ca-certificates \
    curl \
    gnupg \
    # DBus and keyring
    dbus \
    dbus-x11 \
    gnome-keyring \
    libsecret-1-0 \
    libsecret-tools \
    # Other dependencies
    libssl3 \
    libxcb1 \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - && \
    apt-get update && apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Install common development tools
RUN apt-get update && apt-get install -y \
    # Version control
    git \
    # Text processing and search
    ripgrep \
    fd-find \
    fzf \
    # File manipulation
    jq \
    # Network tools
    wget \
    # Process management
    htop \
    # System utilities
    sudo \
    # Text editors
    nano \
    vim \
    # Archive tools
    zip \
    unzip \
    # Build essentials
    build-essential \
    # Python
    python3 \
    python3-pip \
    # Java
    openjdk-17-jdk \
    # Additional tools
    tree \
    tmux \
    && rm -rf /var/lib/apt/lists/*

# Install uv using curl
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install JBang using curl
RUN curl -Ls https://sh.jbang.dev | bash -s - app setup

# Copy the built binaries
COPY --from=builder /usr/src/goose/target/release/goose /usr/local/bin/

# Create a wrapper script to initialize DBus and keyring
RUN echo '#!/bin/bash\n\
# Start DBus session daemon if not running\n\
if [ -z "$DBUS_SESSION_BUS_ADDRESS" ]; then\n\
  eval $(dbus-launch --sh-syntax)\n\
  export DBUS_SESSION_BUS_ADDRESS\n\
fi\n\
\n\
# Initialize keyring if needed\n\
if [ -z "$GNOME_KEYRING_CONTROL" ]; then\n\
  eval $(gnome-keyring-daemon --start)\n\
  export GNOME_KEYRING_CONTROL SSH_AUTH_SOCK\n\
fi\n\
\n\
' > /usr/local/bin/entrypoint.sh && \
chmod +x /usr/local/bin/entrypoint.sh

# Set up some basic git config
RUN git config --global init.defaultBranch main && \
    git config --global core.editor "vim"

# Add some helpful aliases
RUN echo 'alias ll="ls -la"' >> ~/.bashrc && \
    echo 'alias fd=fdfind' >> ~/.bashrc

# Add JBang to PATH
RUN echo 'export PATH="$HOME/.jbang/bin:$PATH"' >> ~/.bashrc
