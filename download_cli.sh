#!/usr/bin/env bash
set -eu

##############################################################################
# Goose CLI Install Script
#
# This script downloads the latest stable 'goose' CLI binary from GitHub releases
# and installs it to your system.
#
# Supported OS: macOS (darwin), Linux
# Supported Architectures: x86_64, arm64
#
# Usage:
#   curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
#
# Environment variables:
#   GOOSE_BIN_DIR  - Directory to which Goose will be installed (default: $HOME/.local/bin)
#   GOOSE_PROVIDER - Optional: provider for goose
#   GOOSE_MODEL    - Optional: model for goose
#   CANARY         - Optional: if set to "true", downloads from canary release instead of stable
#   CONFIGURE      - Optional: if set to "false", disables running goose configure interactively
#   ** other provider specific environment variables (eg. DATABRICKS_HOST)
##############################################################################

# --- 1) Check for dependencies ---
# Check for curl
if ! command -v curl >/dev/null 2>&1; then
  echo "Error: 'curl' is required to download Goose. Please install curl and try again."
  exit 1
fi

# Check for tar
if ! command -v tar >/dev/null 2>&1; then
  echo "Error: 'tar' is required to download Goose. Please install tar and try again."
  exit 1
fi

# Check for bzip2 if tar needs it
check_bzip2_needed() {
  tar_info="$(tar --version 2>&1)"

  if echo "$tar_info" | grep -qi 'bsdtar'; then
    # tar has native bzip2 support (uses libbz2 internally). 
    return 0
  elif echo "$tar_info" | grep -qi 'GNU tar'; then
    # bzip2 is required 
    if ! command -v bzip2 >/dev/null 2>&1; then
      # bzip2 needs to be installed  
      return 1
    else
      # bzip is already installed
      return 0
    fi
  else
    # Fallback: check if tar is linked to libbz2
    if command -v ldd >/dev/null 2>&1; then
      if ldd "$(command -v tar)" 2>/dev/null | grep -q 'libbz2'; then
        # tar is linked with libbz2 (has native bzip2 support)
        return 0
      fi
    elif command -v otool >/dev/null 2>&1; then
      if otool -L "$(command -v tar)" | grep -q 'libbz2'; then
        # tar is linked with libbz2 (has native bzip2 support)
        return 0
      fi
    fi
    # Could not determine if tar has native bzip2 support
    if ! command -v bzip2 >/dev/null 2>&1; then
      # bzip2 MAY be required, and is not installed
      return 2
    else
      # bzip2 is already installed
      return 0
    fi
  fi
}

set +e
check_bzip2_needed
bzip_status=$?
set -e

case $bzip_status in
  0)
    # All good — nothing to do
    ;;
  1)
    echo "Error: 'bzip2' is required to download Goose. Please install bzip2 and try again."
    exit 1
    ;;
  2)
    echo "Warning: Could not determine if 'bzip2' is required. You MAY encounter extraction issues. Proceeding anyway..."
    ;;
  *)
    echo "Error: Unexpected return code from function check_bzip2_needed: $bzip_status"
    exit 1
    ;;
esac

# --- 2) Variables ---
REPO="block/goose"
OUT_FILE="goose"
GOOSE_BIN_DIR="${GOOSE_BIN_DIR:-"$HOME/.local/bin"}"
RELEASE="${CANARY:-false}"
RELEASE_TAG="$([[ "$RELEASE" == "true" ]] && echo "canary" || echo "stable")"
CONFIGURE="${CONFIGURE:-true}"

# --- 3) Detect OS/Architecture ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux|darwin) ;;
  *)
    echo "Error: Unsupported OS '$OS'. Goose currently only supports Linux and macOS."
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64)
    ARCH="x86_64"
    ;;
  arm64|aarch64)
    # Some systems use 'arm64' and some 'aarch64' – standardize to 'aarch64'
    ARCH="aarch64"
    ;;
  *)
    echo "Error: Unsupported architecture '$ARCH'."
    exit 1
    ;;
esac

# Build the filename and URL for the stable release
if [ "$OS" = "darwin" ]; then
  FILE="goose-$ARCH-apple-darwin.tar.bz2"
else
  FILE="goose-$ARCH-unknown-linux-gnu.tar.bz2"
fi

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$RELEASE_TAG/$FILE"

# --- 4) Download & extract 'goose' binary ---
echo "Downloading $RELEASE_TAG release: $FILE..."
if ! curl -sLf "$DOWNLOAD_URL" --output "$FILE"; then
  echo "Error: Failed to download $DOWNLOAD_URL"
  exit 1
fi

# Create a temporary directory for extraction
TMP_DIR="/tmp/goose_install_$RANDOM"
if ! mkdir -p "$TMP_DIR"; then
  echo "Error: Could not create temporary extraction directory"
  exit 1
fi
# Clean up temporary directory
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Extracting $FILE to temporary directory..."
tar -xjf "$FILE" -C "$TMP_DIR"
rm "$FILE" # clean up the downloaded tarball

# Make binary executable
chmod +x "$TMP_DIR/goose"

# --- 5) Install to $GOOSE_BIN_DIR ---
if [ ! -d "$GOOSE_BIN_DIR" ]; then
  echo "Creating directory: $GOOSE_BIN_DIR"
  mkdir -p "$GOOSE_BIN_DIR"
fi

echo "Moving goose to $GOOSE_BIN_DIR/$OUT_FILE"
mv "$TMP_DIR/goose" "$GOOSE_BIN_DIR/$OUT_FILE"

# skip configuration for non-interactive installs e.g. automation, docker
if [ "$CONFIGURE" = true ]; then
  # --- 6) Configure Goose (Optional) ---
  echo ""
  echo "Configuring Goose"
  echo ""
  "$GOOSE_BIN_DIR/$OUT_FILE" configure
else
  echo "Skipping 'goose configure', you may need to run this manually later"
fi

# --- 7) Check PATH and give instructions if needed ---
if [[ ":$PATH:" != *":$GOOSE_BIN_DIR:"* ]]; then
  echo ""
  echo "Warning: Goose installed, but $GOOSE_BIN_DIR is not in your PATH."
  echo "Add it to your PATH by editing ~/.bashrc, ~/.zshrc, or similar:"
  echo "    export PATH=\"$GOOSE_BIN_DIR:\$PATH\""
  echo "Then reload your shell (e.g. 'source ~/.bashrc', 'source ~/.zshrc') to apply changes."
  echo ""
fi
