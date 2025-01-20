#!/usr/bin/env bash
set -euo pipefail

##############################################################################
# Goose CLI Install Script
#
# This script downloads the latest 'goose' CLI binary from GitHub releases
# and installs it to your system.
#
# Supported OS: macOS (darwin), Linux
# Supported Architectures: x86_64, arm64
#
# Usage:
#   curl -H 'Accept: application/vnd.github.v3.raw' "https://api.github.com/repos/block/goose/contents/download_cli.sh?ref=v1.0" | bash
#
# Environment variables:
#   GOOSE_BIN_DIR  - Directory to which Goose will be installed (default: $HOME/.local/bin)
#   GOOSE_PROVIDER - Optional: provider for goose (passed to "goose configure")
#   GOOSE_MODEL    - Optional: model for goose (passed to "goose configure")
##############################################################################

# --- 1) Check for curl ---
if ! command -v curl >/dev/null 2>&1; then
  echo "Error: 'curl' is required to download Goose. Please install curl and try again."
  exit 1
fi

# --- 2) Variables ---
REPO="block/goose"
OUT_FILE="goose"
GITHUB_API_ENDPOINT="api.github.com"
GOOSE_BIN_DIR="${GOOSE_BIN_DIR:-"$HOME/.local/bin"}"

# Helper function to fetch JSON from GitHub
gh_curl() {
  curl -sL -H "Accept: application/vnd.github.v3.raw" "$@"
}

# --- 3) Detect OS/Architecture ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux|darwin) ;;
  *) 
    echo "Error: Unsupported OS '$OS'. Goose only supports Linux and macOS."
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

# Build the filename we expect in the release assets
if [ "$OS" = "darwin" ]; then
  FILE="goose-$ARCH-apple-darwin.tar.bz2"
else
  FILE="goose-$ARCH-unknown-linux-gnu.tar.bz2"
fi

# --- 4) Fetch GitHub Releases and locate the correct asset ID ---
echo "Looking up the most recent goose binary release..."
RELEASES=$(gh_curl https://$GITHUB_API_ENDPOINT/repos/$REPO/releases)

# Parse JSON to find the asset ID
ASSET_ID=$(echo "$RELEASES" | awk -v file="$FILE" '
  BEGIN { found_asset = 0; }
  /"assets"/ { in_assets = 1; next }
  in_assets && /"id":/ {
    match($0, /[0-9]+/);
    current_id = substr($0, RSTART, RLENGTH);
    next
  }
  in_assets && /"name":/ && $0 ~ file {
    print current_id;
    exit;
  }
')

if [ -z "$ASSET_ID" ]; then
  echo "Error: Could not find a release asset named '$FILE' in the latest releases."
  exit 1
fi

# --- 5) Download & extract 'goose' binary ---
echo "Downloading $FILE..."
curl -sL --header 'Accept: application/octet-stream' \
  "https://$GITHUB_API_ENDPOINT/repos/$REPO/releases/assets/$ASSET_ID" \
  --output "$FILE"

echo "Extracting $FILE..."
tar -xjf "$FILE"
rm "$FILE" # clean up the downloaded tarball

# Make binaries executable
chmod +x goose

# --- 6) Install to $GOOSE_BIN_DIR ---
if [ ! -d "$GOOSE_BIN_DIR" ]; then
  echo "Creating directory: $GOOSE_BIN_DIR"
  mkdir -p "$GOOSE_BIN_DIR"
fi

echo "Moving goose to $GOOSE_BIN_DIR/$OUT_FILE"
mv goose "$GOOSE_BIN_DIR/$OUT_FILE"


# --- 7) Check PATH and give instructions if needed ---
if [[ ":$PATH:" != *":$GOOSE_BIN_DIR:"* ]]; then
  echo ""
  echo "Warning: $GOOSE_BIN_DIR is not in your PATH."
  echo "Add it to your PATH by editing ~/.bashrc, ~/.zshrc, or similar:"
  echo "    export PATH=\"$GOOSE_BIN_DIR:\$PATH\""
  echo "Then reload your shell (e.g. 'source ~/.bashrc', 'source ~/.zshrc') to apply changes."
  echo ""
fi

# --- 8) Auto-configure Goose (Optional) ---
CONFIG_ARGS=""
if [ -n "${GOOSE_PROVIDER:-}" ]; then
  CONFIG_ARGS="$CONFIG_ARGS -p $GOOSE_PROVIDER"
fi
if [ -n "${GOOSE_MODEL:-}" ]; then
  CONFIG_ARGS="$CONFIG_ARGS -m $GOOSE_MODEL"
fi

# Print a different message based on whether CONFIG_ARGS is set
echo ""
if [ -n "$CONFIG_ARGS" ]; then
  echo "Configuring Goose with: '$CONFIG_ARGS'"
else
  echo "Configuring Goose"
fi
echo ""
"$GOOSE_BIN_DIR/$OUT_FILE" configure $CONFIG_ARGS

echo ""
echo "Goose installed successfully! Run '$OUT_FILE session' to get started."
echo ""
