#!/usr/bin/env bash
# _tortu/bootstrap.sh
#
# Stand up tortu-custom goose on a fresh machine from a clean clone of this
# fork. Installs the toolchain, wires in _tortu/config, builds the CLI, and
# installs it onto PATH — no manual reconfiguration required.
#
# Usage: _tortu/bootstrap.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TORTU_DIR="$REPO_ROOT/_tortu"
GOOSE_CONFIG_DIR="$HOME/.config/goose"
INSTALL_DIR="$HOME/.local/bin"

echo "==> Bootstrapping tortu goose from $REPO_ROOT"

# --- 1. Toolchain -----------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
    echo "==> Installing Rust toolchain (rustup via Homebrew)"
    brew install rustup
    rustup-init -y --no-modify-path
fi
if ! command -v cmake >/dev/null 2>&1; then
    echo "==> Installing cmake (required by llama-cpp-sys-2)"
    brew install cmake
fi

# --- 2. Secrets --------------------------------------------------------------
SECRETS_FILE="$TORTU_DIR/config/secrets.env"
SECRETS_TEMPLATE="$TORTU_DIR/config/secrets.env.example"

if [ ! -f "$SECRETS_FILE" ]; then
    echo "==> No $SECRETS_FILE found. Let's create one from the template."
    echo "    (Values are written locally only; this file is gitignored and never committed.)"
    cp "$SECRETS_TEMPLATE" "$SECRETS_FILE"
    # Prompt for each VAR= line in the template and fill in the real file.
    while IFS='=' read -r key _; do
        case "$key" in
            ""|\#*) continue ;;
        esac
        read -r -p "    $key: " value
        # Escape any '|' or '&' the user might paste; sed -e keeps this simple.
        escaped_value=$(printf '%s' "$value" | sed -e 's/[&|]/\\&/g')
        sed -i '' "s|^${key}=.*|${key}=${escaped_value}|" "$SECRETS_FILE"
    done < "$SECRETS_TEMPLATE"
    echo "==> Wrote $SECRETS_FILE"
else
    echo "==> Found existing $SECRETS_FILE, using it as-is"
fi

# shellcheck disable=SC1090
set -a; source "$SECRETS_FILE"; set +a

# --- 3. Goose config ----------------------------------------------------------
echo "==> Linking _tortu/config into $GOOSE_CONFIG_DIR"
mkdir -p "$GOOSE_CONFIG_DIR"
cp "$TORTU_DIR/config/config.yaml" "$GOOSE_CONFIG_DIR/config.yaml"
mkdir -p "$GOOSE_CONFIG_DIR/custom_providers"
cp "$TORTU_DIR/config/custom_providers/"*.json "$GOOSE_CONFIG_DIR/custom_providers/" 2>/dev/null || true
mkdir -p "$GOOSE_CONFIG_DIR/skills"
cp -R "$TORTU_DIR/config/skills/." "$GOOSE_CONFIG_DIR/skills/" 2>/dev/null || true
cp "$TORTU_DIR/config/goosehints" "$GOOSE_CONFIG_DIR/.goosehints"

# --- 4. Build + install --------------------------------------------------------
echo "==> Building goose-cli (release)"
cargo build --release -p goose-cli --manifest-path "$REPO_ROOT/Cargo.toml"

mkdir -p "$INSTALL_DIR"
cp "$REPO_ROOT/target/release/goose" "$INSTALL_DIR/goose"
echo "==> Installed goose to $INSTALL_DIR/goose"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo "==> NOTE: $INSTALL_DIR is not on your PATH. Add this to your shell profile:"
    echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo "==> NOTE: set GOOSE_RECIPE_PATH to $TORTU_DIR/recipes in your shell profile:"
echo "    export GOOSE_RECIPE_PATH=\"$TORTU_DIR/recipes\""

echo "==> Bootstrap complete."
