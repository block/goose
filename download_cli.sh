#!/usr/bin/env sh
set -euo pipefail

REPO="block/goose"
FILE="goose"
OUT_FILE="goose"
GITHUB_API_ENDPOINT="api.github.com"

function gh_curl() {
  curl -sL -H "Accept: application/vnd.github.v3.raw" $@
}

# Find the goose binary asset id without using jq
echo "Looking up the most recent goose binary release..."
echo ""
RELEASES=$(gh_curl https://$GITHUB_API_ENDPOINT/repos/$REPO/releases)

# Use awk to find the asset ID
# This script looks for the first prerelease and within it finds the asset with matching name
ASSET_ID=$(echo "$RELEASES" | awk -v file="$FILE" '
  BEGIN { found_prerelease = 0; found_asset = 0; }
  /"prerelease"/ && /true/ { found_prerelease = 1; next }
  found_prerelease && /"assets"/ { in_assets = 1; next }
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
  echo "ERROR: $FILE asset not found"
  exit 1
fi

# Download the goose binary
echo "Downloading goose..."
echo ""
curl -sL --header 'Accept: application/octet-stream' https://$GITHUB_API_ENDPOINT/repos/$REPO/releases/assets/$ASSET_ID > $OUT_FILE
chmod +x $OUT_FILE

LOCAL_BIN="$HOME/.local/bin"
if [ ! -d "$LOCAL_BIN" ]; then
  echo "Directory $LOCAL_BIN does not exist. Creating it now..."
  mkdir -p "$LOCAL_BIN"
  echo "Directory $LOCAL_BIN created successfully."
  echo ""
fi

echo "Sending goose to $LOCAL_BIN/$OUT_FILE"
echo ""
chmod +x $OUT_FILE
mv $OUT_FILE $LOCAL_BIN/$OUT_FILE

# Check if the directory is in the PATH
if [[ ":$PATH:" != *":$LOCAL_BIN:"* ]]; then
  echo "The directory $LOCAL_BIN is not in your PATH."
  echo "To add it, append the following line to your shell configuration file (e.g., ~/.bashrc or ~/.zshrc):"
  echo ""
  echo "    export PATH=\"$LOCAL_BIN:\$PATH\""
  echo ""
  echo "Then reload your shell configuration file by running:"
  echo ""
  echo "    source ~/.bashrc    # or source ~/.zshrc\n"
fi

# Initialize config args with the default name
CONFIG_ARGS="-n default"

# Check for GOOSE_PROVIDER environment variable
if [ -n "${GOOSE_PROVIDER:-}" ]; then
    CONFIG_ARGS="$CONFIG_ARGS -p $GOOSE_PROVIDER"
fi

# Check for GOOSE_MODEL environment variable
if [ -n "${GOOSE_MODEL:-}" ]; then
    CONFIG_ARGS="$CONFIG_ARGS -m $GOOSE_MODEL"
fi

$LOCAL_BIN/$OUT_FILE configure $CONFIG_ARGS

echo ""
echo "You can now run Goose using: $OUT_FILE session"
