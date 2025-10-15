#!/bin/bash
set -e

# Tailscale Tunnel Setup Script
# Supports: macOS (via Homebrew bottles) and Linux (via pkgs.tailscale.com)
# This script starts goosed and creates a Tailscale tunnel for remote access

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
    echo "Tailscale not found, installing..."
    
    # Determine architecture and OS
    OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m)"
    
    # Set up installation directory
    INSTALL_DIR="$HOME/.local/share/goose/bin"
    mkdir -p "$INSTALL_DIR"
    
    TEMP_DIR=$(mktemp -d)
    
    if [ "$OS" = "darwin" ]; then
        # macOS - fetch from Homebrew bottles
        echo "Detected macOS, fetching from Homebrew bottles..."
        
        # Detect macOS version and architecture for bottle selection
        MACOS_VERSION=$(sw_vers -productVersion | cut -d. -f1)
        
        # Determine bottle key based on arch and OS version
        if [ "$ARCH" = "arm64" ]; then
            if [ "$MACOS_VERSION" -ge 15 ]; then
                BOTTLE_KEY="arm64_sequoia"
            elif [ "$MACOS_VERSION" -ge 14 ]; then
                BOTTLE_KEY="arm64_sonoma"
            else
                BOTTLE_KEY="arm64_monterey"
            fi
        else
            # Intel Mac
            if [ "$MACOS_VERSION" -ge 14 ]; then
                BOTTLE_KEY="sonoma"
            else
                BOTTLE_KEY="monterey"
            fi
        fi
        
        echo "Fetching Tailscale bottle info for ${BOTTLE_KEY}..."
        
        # Get bottle URL from Homebrew API
        BOTTLE_JSON=$(curl -sL "https://formulae.brew.sh/api/formula/tailscale.json")
        BOTTLE_URL=$(echo "$BOTTLE_JSON" | jq -r ".bottle.stable.files.${BOTTLE_KEY}.url // .bottle.stable.files.arm64_sonoma.url // .bottle.stable.files.sonoma.url")
        
        if [ -z "$BOTTLE_URL" ] || [ "$BOTTLE_URL" = "null" ]; then
            echo "Error: Failed to get bottle URL for ${BOTTLE_KEY}"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        echo "Downloading Tailscale from ${BOTTLE_URL}..."
        
        # Get token for ghcr.io
        TOKEN=$(curl -s "https://ghcr.io/token?scope=repository:homebrew/core/tailscale:pull" | jq -r .token)
        
        if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
            echo "Error: Failed to get authentication token from ghcr.io"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        # Download bottle with token
        if ! curl -fsSL -H "Authorization: Bearer $TOKEN" "$BOTTLE_URL" -o "$TEMP_DIR/tailscale.tar.gz"; then
            echo "Error: Failed to download Tailscale bottle"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        # Extract binaries from bottle
        echo "Extracting Tailscale binaries from bottle..."
        if ! tar -xzf "$TEMP_DIR/tailscale.tar.gz" -C "$TEMP_DIR"; then
            echo "Error: Failed to extract Tailscale bottle"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        # Find binaries in bottle structure (tailscale/VERSION/bin/)
        BIN_DIR=$(find "$TEMP_DIR" -type d -path "*/tailscale/*/bin" | head -1)
        if [ -z "$BIN_DIR" ]; then
            echo "Error: Could not find bin directory in bottle"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        cp "$BIN_DIR/tailscale" "$INSTALL_DIR/"
        cp "$BIN_DIR/tailscaled" "$INSTALL_DIR/"
        
    else
        # Linux - fetch from Tailscale's package repository
        echo "Detected Linux, fetching from pkgs.tailscale.com..."
        
        # Map architecture names
        case "$ARCH" in
            x86_64)
                TS_ARCH="amd64"
                ;;
            aarch64)
                TS_ARCH="arm64"
                ;;
            armv7l|armv6l)
                TS_ARCH="arm"
                ;;
            i386|i686)
                TS_ARCH="386"
                ;;
            *)
                echo "Error: Unsupported architecture: $ARCH"
                rm -rf "$TEMP_DIR"
                exit 1
                ;;
        esac
        
        # Get latest version
        echo "Fetching latest Tailscale version..."
        LATEST_VERSION=$(curl -sL https://pkgs.tailscale.com/stable/ | grep -oE "tailscale_[0-9]+\.[0-9]+\.[0-9]+_${TS_ARCH}\.tgz" | head -1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+")
        
        if [ -z "$LATEST_VERSION" ]; then
            echo "Error: Failed to fetch latest Tailscale version"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        echo "Downloading Tailscale ${LATEST_VERSION} for ${TS_ARCH}..."
        DOWNLOAD_URL="https://pkgs.tailscale.com/stable/tailscale_${LATEST_VERSION}_${TS_ARCH}.tgz"
        
        if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/tailscale.tgz"; then
            echo "Error: Failed to download Tailscale"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        # Extract binaries
        echo "Extracting Tailscale binaries..."
        if ! tar -xzf "$TEMP_DIR/tailscale.tgz" -C "$TEMP_DIR"; then
            echo "Error: Failed to extract Tailscale"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        # Find and copy binaries
        TS_DIR=$(find "$TEMP_DIR" -type d -name "tailscale_*" | head -1)
        if [ -z "$TS_DIR" ]; then
            echo "Error: Could not find extracted Tailscale directory"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
        
        cp "$TS_DIR/tailscale" "$INSTALL_DIR/"
        cp "$TS_DIR/tailscaled" "$INSTALL_DIR/"
    fi
    
    # Make binaries executable
    chmod +x "$INSTALL_DIR/tailscale" "$INSTALL_DIR/tailscaled"
    
    # Clean up
    rm -rf "$TEMP_DIR"
    
    # Add to PATH for this session
    export PATH="$INSTALL_DIR:$PATH"
    
    # Verify installation
    if ! command -v tailscale &> /dev/null; then
        echo "Error: Tailscale installation failed"
        exit 1
    fi
    
    echo "✓ Tailscale installed successfully to $INSTALL_DIR"
    echo "Note: Add $INSTALL_DIR to your PATH to use Tailscale in other sessions"
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
            # Platform-aware browser opening
            if [ "$OS" = "darwin" ]; then
                open "$URL" 2>/dev/null || echo "Please open this URL manually: $URL"
            elif command -v xdg-open &> /dev/null; then
                xdg-open "$URL" 2>/dev/null || echo "Please open this URL manually: $URL"
            else
                echo "Please open this URL manually: $URL"
            fi
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
