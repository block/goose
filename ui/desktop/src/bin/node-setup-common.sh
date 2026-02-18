#!/bin/bash

# Common setup script for node and npx
# This script sets up hermit and node.js environment

# Enable strict mode to exit on errors and unset variables
set -euo pipefail

# Set log file
LOG_FILE="/tmp/mcp.log"

# Clear the log file at the start
> "${LOG_FILE}"

# Function for logging
log() {
    local MESSAGE="${1}"
    echo "$(date +'%Y-%m-%d %H:%M:%S') - ${MESSAGE}" | tee -a "${LOG_FILE}" >&2
}

# Trap errors and log them before exiting
trap 'log "An error occurred. Exiting with status $?."' ERR

log "Starting node setup (common)."

if [ -n "${GOOSE_CONFIG_DIR:-}" ]; then
    RESOLVED_GOOSE_CONFIG_DIR="${GOOSE_CONFIG_DIR}"
elif [ -n "${GOOSE_PATH_ROOT:-}" ]; then
    RESOLVED_GOOSE_CONFIG_DIR="${GOOSE_PATH_ROOT}/config"
else
    RESOLVED_GOOSE_CONFIG_DIR="${HOME}/.config/goose"
fi
MCP_HERMIT_DIR="${RESOLVED_GOOSE_CONFIG_DIR}/mcp-hermit"

# One-time cleanup for existing Linux users to fix locking issues
CLEANUP_MARKER="${RESOLVED_GOOSE_CONFIG_DIR}/.mcp-hermit-cleanup-v1"
if [[ "$(uname -s)" == "Linux" ]] && [ ! -f "${CLEANUP_MARKER}" ]; then
    log "Performing one-time cleanup of old mcp-hermit directory to fix locking issues."
    if [ -d "${MCP_HERMIT_DIR}" ]; then
        rm -rf "${MCP_HERMIT_DIR}"
        log "Removed old mcp-hermit directory."
    fi
    mkdir -p "${RESOLVED_GOOSE_CONFIG_DIR}"
    touch "${CLEANUP_MARKER}"
    log "Cleanup completed. Marker file created."
fi

# Ensure mcp-hermit/bin exists
log "Creating directory ${MCP_HERMIT_DIR}/bin if it does not exist."
mkdir -p "${MCP_HERMIT_DIR}/bin"

# Change to the mcp-hermit directory
log "Changing to directory ${MCP_HERMIT_DIR}."
cd "${MCP_HERMIT_DIR}"


# Check if hermit binary exists and download if not
if [ ! -f "${MCP_HERMIT_DIR}/bin/hermit" ]; then
    log "Hermit binary not found. Downloading hermit binary."
    curl -fsSL "https://github.com/cashapp/hermit/releases/download/stable/hermit-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/amd64/' | sed 's/aarch64/arm64/').gz" \
        | gzip -dc > "${MCP_HERMIT_DIR}/bin/hermit" && chmod +x "${MCP_HERMIT_DIR}/bin/hermit"
    log "Hermit binary downloaded and made executable."
else
    log "Hermit binary already exists. Skipping download."
fi


log "setting hermit cache to be local for MCP servers"
mkdir -p "${MCP_HERMIT_DIR}/cache"
export HERMIT_STATE_DIR="${MCP_HERMIT_DIR}/cache"


# Update PATH
export PATH="${MCP_HERMIT_DIR}/bin:${PATH}"
log "Updated PATH to include ${MCP_HERMIT_DIR}/bin."


# Verify hermit installation
log "Checking for hermit in PATH."
which hermit >> "${LOG_FILE}"

# Check if hermit environment is already initialized (only run init on first setup)
if [ ! -f "bin/activate-hermit" ]; then
    log "Hermit environment not yet initialized. Setting up hermit."

    # Fix hermit self-update lock issues on Linux by using temp binary for init only
    if [[ "$(uname -s)" == "Linux" ]]; then
        log "Creating temp dir with bin subdirectory for hermit copy to avoid self-update locks."
        HERMIT_TMP_DIR="/tmp/hermit_tmp_$$/bin"
        mkdir -p "${HERMIT_TMP_DIR}"
        cp "${MCP_HERMIT_DIR}/bin/hermit" "${HERMIT_TMP_DIR}/hermit"
        chmod +x "${HERMIT_TMP_DIR}/hermit"
        export PATH="${HERMIT_TMP_DIR}:${PATH}"
        HERMIT_CLEANUP_DIR="/tmp/hermit_tmp_$$"
    fi

    # Initialize hermit
    log "Initializing hermit."
    hermit init >> "${LOG_FILE}"

    # Clean up temp dir if it was created
    if [[ -n "${HERMIT_CLEANUP_DIR:-}" ]]; then
        log "Cleaning up temporary hermit binary directory."
        rm -rf "${HERMIT_CLEANUP_DIR}"
    fi
else
    log "Hermit environment already initialized. Skipping init."
fi

# Activate the environment with output redirected to log
if [[ "$(uname -s)" == "Linux" ]]; then
    log "Activating hermit environment."
    { . "bin/activate-hermit"; } >> "${LOG_FILE}" 2>&1
fi

# Install Node.js using hermit
log "Installing Node.js with hermit."
hermit install node >> "${LOG_FILE}"

# Verify installations
log "Verifying installation locations:"
log "hermit: $(which hermit)"
log "node: $(which node)"
log "npx: $(which npx)"


log "Checking for GOOSE_NPM_REGISTRY and GOOSE_NPM_CERT environment variables for custom npm registry setup..."
# Check if GOOSE_NPM_REGISTRY is set and accessible
if [ -n "${GOOSE_NPM_REGISTRY:-}" ] && curl -s --head --fail "${GOOSE_NPM_REGISTRY}" > /dev/null; then
    log "Checking custom goose registry availability: ${GOOSE_NPM_REGISTRY}"
    log "${GOOSE_NPM_REGISTRY} is accessible. Using it for npm registry."
    export NPM_CONFIG_REGISTRY="${GOOSE_NPM_REGISTRY}"

    # Check if GOOSE_NPM_CERT is set and accessible
    if [ -n "${GOOSE_NPM_CERT:-}" ] && curl -s --head --fail "${GOOSE_NPM_CERT}" > /dev/null; then
        log "Downloading certificate from: ${GOOSE_NPM_CERT}"
        curl -sSL -o "${MCP_HERMIT_DIR}/cert.pem" "${GOOSE_NPM_CERT}"
        if [ $? -eq 0 ]; then
            log "Certificate downloaded successfully."
            export NODE_EXTRA_CA_CERTS="${MCP_HERMIT_DIR}/cert.pem"
        else
            log "Unable to download the certificate. Skipping certificate setup."
        fi
    else
        log "GOOSE_NPM_CERT is either not set or not accessible. Skipping certificate setup."
    fi

else
    log "GOOSE_NPM_REGISTRY is either not set or not accessible. Falling back to default npm registry."
    export NPM_CONFIG_REGISTRY="https://registry.npmjs.org/"
fi

log "Node setup (common) completed successfully."
