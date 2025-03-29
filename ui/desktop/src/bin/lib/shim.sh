#!/usr/bin/env bash

# Enable strict mode to exit on errors and unset variables
set -euo pipefail

# Set log file
LOG_FILE="/tmp/mcp.log"

# Clear the log file at the start
> "$LOG_FILE"

# Function for logging
log() {
    local MESSAGE="$1"
    echo "$(date +'%Y-%m-%d %H:%M:%S') - $MESSAGE" | tee -a "$LOG_FILE"
}

# Trap errors and log them before exiting
trap 'log "An error occurred. Exiting with status $?."' ERR

# Common directory setup function
setup_directories() {
    # Ensure ~/.config/goose/mcp-hermit/bin exists
    log "Creating directory ~/.config/goose/mcp-hermit/bin if it does not exist."
    mkdir -p ~/.config/goose/mcp-hermit/bin

    # Change to the ~/.config/goose/mcp-hermit directory
    log "Changing to directory ~/.config/goose/mcp-hermit."
    cd ~/.config/goose/mcp-hermit

    # Setup hermit cache
    log "setting hermit cache to be local for MCP servers"
    mkdir -p ~/.config/goose/mcp-hermit/cache
    export HERMIT_STATE_DIR=~/.config/goose/mcp-hermit/cache
}

# Common hermit setup function
setup_hermit() {
    # Check if hermit binary exists and download if not
    if [ ! -f ~/.config/goose/mcp-hermit/bin/hermit ]; then
        log "Hermit binary not found. Downloading hermit binary."
        curl -fsSL "https://github.com/cashapp/hermit/releases/download/stable/hermit-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/amd64/' | sed 's/aarch64/arm64/').gz" \
            | gzip -dc > ~/.config/goose/mcp-hermit/bin/hermit && chmod +x ~/.config/goose/mcp-hermit/bin/hermit
        log "Hermit binary downloaded and made executable."
    else
        log "Hermit binary already exists. Skipping download."
    fi

    # Update PATH
    export PATH=~/.config/goose/mcp-hermit/bin:$PATH
    log "Updated PATH to include ~/.config/goose/mcp-hermit/bin."

    # Initialize hermit
    log "Initializing hermit."
    hermit init >> "$LOG_FILE"
}

# Function to verify a URL is accessible
check_url_accessible() {
    local URL="$1"
    curl -s --head --fail "$URL" > /dev/null
}

# Function to verify binary locations
verify_binary() {
    local BINARY_NAME="$1"
    log "$BINARY_NAME: $(which $BINARY_NAME)"
}
