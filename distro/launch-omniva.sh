#!/bin/bash
# Omniva AI Builder — Launcher script
# Ensures Ollama is running, sets config, and starts Goose.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ── Colors ──
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}🔧 Omniva AI Builder — Starting up...${NC}"

# ── 1. Check Ollama is installed ──
if ! command -v ollama &> /dev/null; then
    echo -e "${RED}Ollama is not installed. Install it first:${NC}"
    echo "  brew install ollama"
    exit 1
fi

# ── 2. Start Ollama if not running ──
if ! curl -s http://localhost:11434/api/tags &> /dev/null; then
    echo -e "${YELLOW}Starting Ollama...${NC}"
    ollama serve &> /dev/null &
    sleep 2
fi

# ── 3. Check models are pulled ──
REQUIRED_MODELS=("qwen2.5:14b" "deepseek-r1:14b")
for model in "${REQUIRED_MODELS[@]}"; do
    if ! ollama list | grep -q "$model"; then
        echo -e "${YELLOW}Pulling $model (first time only, this takes a few minutes)...${NC}"
        ollama pull "$model"
    fi
done

echo -e "${GREEN}Models ready.${NC}"

# ── 4. Set Goose environment ──
export GOOSE_PROVIDER=ollama
export GOOSE_MODEL=qwen2.5:14b
export OLLAMA_HOST=http://localhost:11434
export GOOSE_DISABLE_TELEMETRY=1

# Increase context window (Ollama defaults to 2048 which is too short)
export OLLAMA_NUM_CTX=8192

# ── 5. Copy system prompt if not already in place ──
GOOSE_CONFIG_DIR="${HOME}/.config/goose"
mkdir -p "$GOOSE_CONFIG_DIR"

if [ ! -f "$GOOSE_CONFIG_DIR/config.yaml" ]; then
    cp "$SCRIPT_DIR/config.yaml" "$GOOSE_CONFIG_DIR/config.yaml"
    echo -e "${GREEN}Copied Omniva config to $GOOSE_CONFIG_DIR${NC}"
fi

# ── 6. Launch Goose ──
echo -e "${GREEN}Launching Omniva AI Builder...${NC}"
echo -e "  Provider: ollama"
echo -e "  Model:    qwen2.5:14b"
echo -e "  Context:  8192 tokens"
echo ""

# If Goose desktop is installed, open it. Otherwise fall back to CLI.
if command -v goose &> /dev/null; then
    goose session --recipe "$SCRIPT_DIR/recipes/quick-start.yaml"
elif [ -d "/Applications/Goose.app" ]; then
    open /Applications/Goose.app
else
    echo -e "${RED}Goose is not installed.${NC}"
    echo "Install the desktop app from: https://block.github.io/goose"
    echo "Or install the CLI: brew install goose"
    exit 1
fi
