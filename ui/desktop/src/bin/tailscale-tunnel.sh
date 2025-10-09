#!/bin/bash
set -e

# Usage information
usage() {
    echo "Usage: $0 <path_to_goosed> <port> <secret> <output_json_file>" >&2
    echo "Example: $0 ./goosed 62997 my_secret_key /tmp/tunnel.json" >&2
    exit 1
}

# Check arguments
if [ $# -ne 4 ]; then
    usage
fi

GOOSED_PATH="$1"
PORT="$2"
SECRET="$3"
OUTPUT_FILE="$4"

# Validate goosed path
if [ ! -f "$GOOSED_PATH" ]; then
    echo "Error: goosed not found at: $GOOSED_PATH"
    exit 1
fi

# Validate port is a number
if ! [[ "$PORT" =~ ^[0-9]+$ ]]; then
    echo "Error: Port must be a number"
    exit 1
fi

# Check if port is available
if lsof -i:$PORT >/dev/null 2>&1; then
    echo "Error: Port $PORT is already in use"
    exit 1
fi

# Check if tailscale is available, install if not
if ! command -v tailscale &> /dev/null; then
    echo "Tailscale not found, installing via Homebrew..."
    if command -v brew &> /dev/null; then
        brew install tailscale >/dev/null 2>&1
        if ! command -v tailscale &> /dev/null; then
            echo "Error: Failed to install tailscale"
            exit 1
        fi
        echo "✓ Tailscale installed successfully"
    else
        echo "Error: Homebrew not found and tailscale is not installed. Please install Homebrew first: https://brew.sh"
        exit 1
    fi
fi

# Cleanup function
cleanup() {
    echo ""
    echo "Shutting down..."
    if [ ! -z "$GOOSED_PID" ]; then
        echo "Stopping goosed (PID: $GOOSED_PID)"
        kill $GOOSED_PID 2>/dev/null || true
    fi
    if [ ! -z "$TAILSCALE_SERVE_PID" ]; then
        echo "Stopping Tailscale serve (PID: $TAILSCALE_SERVE_PID)"
        kill $TAILSCALE_SERVE_PID 2>/dev/null || true
    fi
    # Reset tailscale serve
    tailscale serve reset >/dev/null 2>&1 || true
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# Start goosed in the background
echo "Starting goosed on port ${PORT}..."
export GOOSE_PORT=$PORT
export GOOSE_SERVER__SECRET_KEY="$SECRET"
$GOOSED_PATH agent > /dev/null 2>&1 &
GOOSED_PID=$!

# Wait for goosed to be ready
echo "Waiting for goosed to start..."
for i in {1..30}; do
    if curl -s "http://localhost:${PORT}/health" > /dev/null 2>&1; then
        echo "✓ Goosed is running (PID: $GOOSED_PID)"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "Error: goosed failed to start"
        exit 1
    fi
    sleep 0.5
done

# Setup Tailscale
TS_STATE="$HOME/.local/share/tailscale"
TS_SOCK="$HOME/.cache/tailscaled.sock"
LOG_FILE="/tmp/tailscaled.log"

mkdir -p "$TS_STATE" "$(dirname "$TS_SOCK")"

echo "Setting up Tailscale..."

# Start tailscaled if not running
if ! pgrep -f "tailscaled --tun=userspace-networking" >/dev/null; then
    echo "▶️  Starting userspace tailscaled..."
    nohup tailscaled \
        --tun=userspace-networking \
        --statedir "$TS_STATE" \
        --socket "$TS_SOCK" \
        >"$LOG_FILE" 2>&1 &

    # Wait for tailscaled to start
    for i in {1..30}; do
        if pgrep -f "tailscaled --tun=userspace-networking" >/dev/null; then
            echo "✓ tailscaled started"
            break
        fi
        sleep 0.5
    done
else
    echo "✓ tailscaled already running"
fi

# Wait for LocalAPI
for i in {1..50}; do
    if curl -sf --unix-socket "$TS_SOCK" \
        http://local-tailscaled.sock/localapi/v0/status >/dev/null 2>&1; then
        break
    fi
    sleep 0.2
done

# Bring up Tailscale and handle authentication if needed
echo "🔐 Bringing up Tailscale..."
tailscale --socket "$TS_SOCK" up 2>&1 | {
    AUTH_OPENED=false
    while read line; do
        echo "$line"
        if [[ "$AUTH_OPENED" == false ]] && echo "$line" | grep -q "https://login.tailscale.com/"; then
            URL=$(echo "$line" | grep -o "https://login.tailscale.com/[^\s]*")
            echo "🌐 Opening authentication URL in browser..."
            open "$URL"
            AUTH_OPENED=true
        fi
    done
} || true

# Get Tailscale connection info
echo "Getting Tailscale connection info..."
HOST=$(tailscale --socket $TS_SOCK status --json | jq -r '.Self.DNSName' | sed 's/\.$//')
V4=$(tailscale --socket "$TS_SOCK" ip -4 2>/dev/null | head -n1)
V6=$(tailscale --socket "$TS_SOCK" ip -6 2>/dev/null | head -n1)

# Setup Tailscale serve to map port 80 to our local goosed
echo "Setting up Tailscale serve (port 80 → localhost:${PORT})..."
tailscale --socket "$TS_SOCK" serve reset >/dev/null 2>&1 || true
tailscale --socket "$TS_SOCK" serve --tcp=80 127.0.0.1:$PORT >/dev/null &
TAILSCALE_SERVE_PID=$!

# Wait a moment for serve to start
sleep 1

echo "✓ Tailscale serve established (PID: $TAILSCALE_SERVE_PID)"

# Build the JSON output
# Use MagicDNS as primary, fallback to IPv4
TUNNEL_URL=""
if [[ -n "$HOST" && "$HOST" != "null" ]]; then
    TUNNEL_URL="http://$HOST"
elif [[ -n "$V4" ]]; then
    TUNNEL_URL="http://$V4"
else
    echo "Error: No Tailscale IP addresses available"
    exit 1
fi

# Write JSON to output file
jq -n \
    --arg url "$TUNNEL_URL" \
    --arg ipv4 "${V4:-null}" \
    --arg ipv6 "${V6:-null}" \
    --arg hostname "${HOST:-null}" \
    --arg secret "$SECRET" \
    --arg port "$PORT" \
    --arg goosed_pid "$GOOSED_PID" \
    --arg tailscale_pid "$TAILSCALE_SERVE_PID" \
    '{
        url: $url,
        ipv4: $ipv4,
        ipv6: $ipv6,
        hostname: $hostname,
        secret: $secret,
        port: ($port | tonumber),
        pids: {
            goosed: ($goosed_pid | tonumber),
            tailscale_serve: ($tailscale_pid | tonumber)
        }
    }' > "$OUTPUT_FILE"

echo "✓ Tunnel established! Connection info written to: $OUTPUT_FILE"
echo "Press Ctrl+C to stop the tunnel"

# Keep the script running
wait
