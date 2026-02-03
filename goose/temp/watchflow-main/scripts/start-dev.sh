#!/usr/bin/env bash

# Exit if any command fails
set -e

# Set project root
SCRIPT_DIR=$(dirname "$0")
PROJECT_ROOT="$SCRIPT_DIR/.."

# Application path for FastAPI CLI
APP_PATH="src/main.py"

# Set host and port
HOST=${HOST:-"0.0.0.0"}
PORT=${PORT:-8000}

# Start FastAPI development server
# Auto-reload is also enabled by default in fastapi dev
echo "Starting FastAPI development server on $HOST:$PORT..."
cd "$PROJECT_ROOT" && exec fastapi dev "$APP_PATH" --host "$HOST" --port "$PORT"
