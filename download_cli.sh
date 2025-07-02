#!/usr/bin/env bash
set -eu

##############################################################################
# Goose CLI Install Script
#
# This script downloads the latest stable 'goose' CLI binary from GitHub releases
# and installs it to your system.
#
# Supported OS: macOS (darwin), Linux, Windows (MSYS2/Git Bash/WSL)
# Supported Architectures: x86_64, arm64
#
# Usage:
#   curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
#
# Environment variables:
#   GOOSE_BIN_DIR  - Directory to which Goose will be installed (default: $HOME/.local/bin)
#   GOOSE_VERSION  - Optional: specific version to install (e.g., "v1.0.25"). Overrides CANARY. Can be in the format vX.Y.Z, vX.Y.Z-suffix, or X.Y.Z
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

# Check for tar or unzip (depending on OS)
if ! command -v tar >/dev/null 2>&1 && ! command -v unzip >/dev/null 2>&1; then
  echo "Error: Either 'tar' or 'unzip' is required to extract Goose. Please install one and try again."
  exit 1
fi


# --- 2) Variables ---
REPO="block/goose"
OUT_FILE="goose"
GOOSE_BIN_DIR="${GOOSE_BIN_DIR:-"$HOME/.local/bin"}"
RELEASE="${CANARY:-false}"
CONFIGURE="${CONFIGURE:-true}"
if [ -n "${GOOSE_VERSION:-}" ]; then
  # Validate the version format
  if [[ ! "$GOOSE_VERSION" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+(-.*)?$ ]]; then
    echo "[error]: invalid version '$GOOSE_VERSION'."
    echo "  expected: semver format vX.Y.Z, vX.Y.Z-suffix, or X.Y.Z"
    exit 1
  fi
  GOOSE_VERSION=$(echo "$GOOSE_VERSION" | sed 's/^v\{0,1\}/v/') # Ensure the version string is prefixed with 'v' if not already present
  RELEASE_TAG="$GOOSE_VERSION"
else
  # If GOOSE_VERSION is not set, fall back to existing behavior for backwards compatibility
  RELEASE_TAG="$([[ "$RELEASE" == "true" ]] && echo "canary" || echo "stable")"
fi

# --- 3) Detect OS/Architecture ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Handle Windows environments (MSYS2, Git Bash, Cygwin, WSL)
case "$OS" in
  linux|darwin) ;;
  mingw*|msys*|cygwin*)
    OS="windows"
    ;;
  *)
    echo "Error: Unsupported OS '$OS'. Goose currently supports Linux, macOS, and Windows."
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
  EXTRACT_CMD="tar"
elif [ "$OS" = "windows" ]; then
  # Windows only supports x86_64 currently
  if [ "$ARCH" != "x86_64" ]; then
    echo "Error: Windows currently only supports x86_64 architecture."
    exit 1
  fi
  FILE="goose-$ARCH-pc-windows-gnu.zip"
  EXTRACT_CMD="unzip"
  OUT_FILE="goose.exe"
else
  FILE="goose-$ARCH-unknown-linux-gnu.tar.bz2"
  EXTRACT_CMD="tar"
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
set +e  # Disable immediate exit on error

if [ "$EXTRACT_CMD" = "tar" ]; then
  tar -xjf "$FILE" -C "$TMP_DIR" 2> tar_error.log
  extract_exit_code=$?
  
  # Check for tar errors
  if [ $extract_exit_code -ne 0 ]; then
    if grep -iEq "missing.*bzip2|bzip2.*missing|bzip2.*No such file|No such file.*bzip2" tar_error.log; then
      echo "Error: Failed to extract $FILE. 'bzip2' is required but not installed. See details below:"
    else
      echo "Error: Failed to extract $FILE. See details below:"
    fi
    cat tar_error.log
    rm tar_error.log
    exit 1
  fi
  rm tar_error.log
else
  # Use unzip for Windows
  unzip -q "$FILE" -d "$TMP_DIR" 2> unzip_error.log
  extract_exit_code=$?
  
  # Check for unzip errors
  if [ $extract_exit_code -ne 0 ]; then
    echo "Error: Failed to extract $FILE. See details below:"
    cat unzip_error.log
    rm unzip_error.log
    exit 1
  fi
  rm unzip_error.log
fi

set -e  # Re-enable immediate exit on error

rm "$FILE" # clean up the downloaded archive

# Determine the extraction directory (handle subdirectory in Windows packages)
# Windows releases may contain files in a 'goose-package' subdirectory
EXTRACT_DIR="$TMP_DIR"
if [ "$OS" = "windows" ] && [ -d "$TMP_DIR/goose-package" ]; then
  echo "Found goose-package subdirectory, using that as extraction directory"
  EXTRACT_DIR="$TMP_DIR/goose-package"
fi

# Make binary executable
if [ "$OS" = "windows" ]; then
  chmod +x "$EXTRACT_DIR/goose.exe"
else
  chmod +x "$EXTRACT_DIR/goose"
fi

# --- 5) Install to $GOOSE_BIN_DIR ---
if [ ! -d "$GOOSE_BIN_DIR" ]; then
  echo "Creating directory: $GOOSE_BIN_DIR"
  mkdir -p "$GOOSE_BIN_DIR"
fi

echo "Moving goose to $GOOSE_BIN_DIR/$OUT_FILE"
if [ "$OS" = "windows" ]; then
  mv "$EXTRACT_DIR/goose.exe" "$GOOSE_BIN_DIR/$OUT_FILE"
else
  mv "$EXTRACT_DIR/goose" "$GOOSE_BIN_DIR/$OUT_FILE"
fi

# Also move temporal-service and temporal CLI if they exist
if [ "$OS" = "windows" ]; then
  if [ -f "$EXTRACT_DIR/temporal-service.exe" ]; then
    echo "Moving temporal-service to $GOOSE_BIN_DIR/temporal-service.exe"
    mv "$EXTRACT_DIR/temporal-service.exe" "$GOOSE_BIN_DIR/temporal-service.exe"
    chmod +x "$GOOSE_BIN_DIR/temporal-service.exe"
  fi
  
  # Move temporal CLI if it exists
  if [ -f "$EXTRACT_DIR/temporal.exe" ]; then
    echo "Moving temporal CLI to $GOOSE_BIN_DIR/temporal.exe"
    mv "$EXTRACT_DIR/temporal.exe" "$GOOSE_BIN_DIR/temporal.exe"
    chmod +x "$GOOSE_BIN_DIR/temporal.exe"
  fi
  
  # Copy Windows runtime DLLs if they exist
  for dll in "$EXTRACT_DIR"/*.dll; do
    if [ -f "$dll" ]; then
      echo "Moving Windows runtime DLL: $(basename "$dll")"
      mv "$dll" "$GOOSE_BIN_DIR/"
    fi
  done
else
  if [ -f "$EXTRACT_DIR/temporal-service" ]; then
    echo "Moving temporal-service to $GOOSE_BIN_DIR/temporal-service"
    mv "$EXTRACT_DIR/temporal-service" "$GOOSE_BIN_DIR/temporal-service"
    chmod +x "$GOOSE_BIN_DIR/temporal-service"
  fi
  
  # Move temporal CLI if it exists
  if [ -f "$EXTRACT_DIR/temporal" ]; then
    echo "Moving temporal CLI to $GOOSE_BIN_DIR/temporal"
    mv "$EXTRACT_DIR/temporal" "$GOOSE_BIN_DIR/temporal"
    chmod +x "$GOOSE_BIN_DIR/temporal"
  fi
fi

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
