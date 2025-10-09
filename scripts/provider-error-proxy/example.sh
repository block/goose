#!/bin/bash
# Example: Testing the Provider Error Proxy with Goose

# This script demonstrates how to use the provider error proxy to test
# Goose's error handling and retry logic.

set -e

PROXY_PORT=8888
ERROR_INTERVAL=3

echo "=================================================="
echo "Provider Error Proxy - Example Usage"
echo "=================================================="
echo ""
echo "This example will:"
echo "1. Start the error proxy on port $PROXY_PORT"
echo "2. Configure Goose to use the proxy"
echo "3. Run a simple Goose command"
echo "4. The proxy will inject errors every $ERROR_INTERVAL requests"
echo ""
echo "Press Ctrl+C to stop the proxy when done."
echo ""
echo "=================================================="
echo ""

# Check if uv is available
if ! command -v uv &> /dev/null; then
    echo "Error: uv is not installed. Please install it first:"
    echo "  curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

# Start the proxy in the background
echo "Starting proxy on port $PROXY_PORT..."
cd "$(dirname "$0")"
uv run python proxy.py --port $PROXY_PORT --error-interval $ERROR_INTERVAL &
PROXY_PID=$!

# Give the proxy time to start
sleep 2

# Set environment variables for Goose
export OPENAI_HOST=http://localhost:$PROXY_PORT
export ANTHROPIC_HOST=http://localhost:$PROXY_PORT

echo ""
echo "Proxy started (PID: $PROXY_PID)"
echo ""
echo "Environment variables set:"
echo "  OPENAI_HOST=$OPENAI_HOST"
echo "  ANTHROPIC_HOST=$ANTHROPIC_HOST"
echo ""
echo "Now you can run Goose commands and observe error handling."
echo "For example:"
echo "  goose session start --profile openai \"tell me a joke\""
echo ""
echo "Press Ctrl+C to stop the proxy."
echo ""

# Wait for user to stop
trap "kill $PROXY_PID 2>/dev/null; echo 'Proxy stopped.'; exit 0" INT TERM

wait $PROXY_PID
