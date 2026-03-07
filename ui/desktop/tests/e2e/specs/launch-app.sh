#!/bin/bash
#
# 🦢🔍 Goose Tester — App Launcher
#
# Launches the Goose Electron app with remote debugging enabled for testing.
# Supports both dev server and bundled/packaged apps.
#
# Usage:
#   ./launch-app.sh dev [project_dir] [port]     Launch dev server with Vite hot reload
#   ./launch-app.sh app <path-to-app> [port]     Launch a bundled .app
#   ./launch-app.sh status [port]                Check if app is running on port
#   ./launch-app.sh stop [port]                  Stop the app on port
#
# Examples:
#   ./launch-app.sh dev /Users/zane/Development/goose/ui/desktop 9224
#   ./launch-app.sh app "/Users/zane/Downloads/Goose 43.app" 9223
#   ./launch-app.sh status 9224
#   ./launch-app.sh stop 9224
#

set -e

DEFAULT_PORT=9222
SCREEN_SESSION_PREFIX="goose-tester"

usage() {
    echo "Usage:"
    echo "  $0 dev [project_dir] [port]     Launch dev server"
    echo "  $0 app <path-to-app> [port]     Launch bundled .app"
    echo "  $0 status [port]                Check if running"
    echo "  $0 stop [port]                  Stop the app"
    exit 1
}

wait_for_port() {
    local port=$1
    local timeout=${2:-30}
    local elapsed=0
    echo "Waiting for port $port..."
    while [ $elapsed -lt $timeout ]; do
        if lsof -i ":$port" 2>/dev/null | grep -q LISTEN; then
            echo "✅ Port $port is listening (${elapsed}s)"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    echo "❌ Timed out waiting for port $port after ${timeout}s"
    return 1
}

cmd_dev() {
    local project_dir="${1:-/Users/zane/Development/goose/ui/desktop}"
    local port="${2:-$DEFAULT_PORT}"
    local session_name="${SCREEN_SESSION_PREFIX}-dev-${port}"

    # Check if already running
    if lsof -i ":$port" 2>/dev/null | grep -q LISTEN; then
        echo "⚠️  Port $port is already in use:"
        lsof -i ":$port" | grep LISTEN
        echo ""
        echo "Use '$0 stop $port' to stop it first, or choose a different port."
        exit 1
    fi

    # Check if screen session exists
    if screen -ls 2>/dev/null | grep -q "$session_name"; then
        echo "⚠️  Screen session '$session_name' already exists. Cleaning up..."
        screen -S "$session_name" -X quit 2>/dev/null
        sleep 1
    fi

    # Verify project directory
    if [ ! -f "$project_dir/package.json" ]; then
        echo "❌ No package.json found in $project_dir"
        exit 1
    fi

    echo "🦢 Launching Goose dev server..."
    echo "   Project: $project_dir"
    echo "   Debug port: $port"
    echo "   Screen session: $session_name"
    echo ""

    # Launch in a screen session with ENABLE_PLAYWRIGHT
    screen -dmS "$session_name" bash -c "
        cd '$project_dir'
        export ENABLE_PLAYWRIGHT=1
        export PLAYWRIGHT_DEBUG_PORT=$port
        npm run start-gui 2>&1 | tee /tmp/goose-tester-dev-${port}.log
    "

    # Wait for the debug port to become available
    if wait_for_port "$port" 30; then
        echo ""
        echo "🎉 Dev server is ready!"
        echo "   Connect with: electron_connect port=$port"
        echo "   Logs: /tmp/goose-tester-dev-${port}.log"
        echo "   Stop with: $0 stop $port"
    else
        echo ""
        echo "Check logs: cat /tmp/goose-tester-dev-${port}.log"
        exit 1
    fi
}

cmd_app() {
    local app_path="$1"
    local port="${2:-$DEFAULT_PORT}"

    if [ -z "$app_path" ]; then
        echo "❌ App path is required"
        usage
    fi

    if [ ! -d "$app_path" ]; then
        echo "❌ App not found: $app_path"
        exit 1
    fi

    # Check if already running
    if lsof -i ":$port" 2>/dev/null | grep -q LISTEN; then
        echo "⚠️  Port $port is already in use:"
        lsof -i ":$port" | grep LISTEN
        echo ""
        echo "Use '$0 stop $port' to stop it first, or choose a different port."
        exit 1
    fi

    echo "🦢 Launching bundled Goose app..."
    echo "   App: $app_path"
    echo "   Debug port: $port"
    echo ""

    # Launch bundled app with --args to pass Chromium flags
    open -a "$app_path" --args --remote-debugging-port="$port"

    # Wait for the debug port to become available
    if wait_for_port "$port" 15; then
        echo ""
        echo "🎉 Bundled app is ready!"
        echo "   Connect with: electron_connect port=$port"
        echo "   Stop with: $0 stop $port"
    else
        echo ""
        echo "❌ App may not have started with debug port."
        echo "   Try: open -a '$app_path' --args --remote-debugging-port=$port"
        exit 1
    fi
}

cmd_status() {
    local port="${1:-$DEFAULT_PORT}"

    echo "🔍 Checking port $port..."
    if lsof -i ":$port" 2>/dev/null | grep -q LISTEN; then
        echo "✅ App is running on port $port:"
        lsof -i ":$port" | grep LISTEN
        echo ""
        # Check screen sessions
        if screen -ls 2>/dev/null | grep -q "$SCREEN_SESSION_PREFIX"; then
            echo "Screen sessions:"
            screen -ls 2>/dev/null | grep "$SCREEN_SESSION_PREFIX"
        fi
    else
        echo "❌ Nothing listening on port $port"
    fi
}

cmd_stop() {
    local port="${1:-$DEFAULT_PORT}"

    echo "🛑 Stopping app on port $port..."

    # Find and kill the process on the port
    local pid
    pid=$(lsof -ti ":$port" -sTCP:LISTEN 2>/dev/null)
    if [ -n "$pid" ]; then
        echo "   Killing PID $pid..."
        kill "$pid" 2>/dev/null
        sleep 2
        # Force kill if still running
        if kill -0 "$pid" 2>/dev/null; then
            echo "   Force killing..."
            kill -9 "$pid" 2>/dev/null
        fi
    fi

    # Clean up screen sessions for this port
    local session_name="${SCREEN_SESSION_PREFIX}-dev-${port}"
    if screen -ls 2>/dev/null | grep -q "$session_name"; then
        echo "   Cleaning up screen session '$session_name'..."
        screen -S "$session_name" -X quit 2>/dev/null
    fi

    sleep 1
    if lsof -i ":$port" 2>/dev/null | grep -q LISTEN; then
        echo "⚠️  Port $port is still in use"
    else
        echo "✅ Port $port is free"
    fi
}

# Main
case "${1:-}" in
    dev)    cmd_dev "$2" "$3" ;;
    app)    cmd_app "$2" "$3" ;;
    status) cmd_status "$2" ;;
    stop)   cmd_stop "$2" ;;
    *)      usage ;;
esac
