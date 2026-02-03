#!/usr/bin/env bash

# Exit if any command fails
set -e

# Set project root
SCRIPT_DIR=$(dirname "$0")
PROJECT_ROOT="$SCRIPT_DIR/.."

# Application path for FastAPI CLI
APP_PATH="src/main.py"

# Host and port configuration
HOST=${HOST:-"0.0.0.0"}
PORT=${PORT:-8000}

# Worker configuration (recommended: CPU cores + 1)
WORKERS=${WEB_CONCURRENCY:-$(($(nproc) + 1))}

# Start FastAPI production server
echo "Starting FastAPI production server on $HOST:$PORT with $WORKERS workers..."
cd "$PROJECT_ROOT" && exec fastapi run "$APP_PATH" \
    --host "$HOST" \
    --port "$PORT" \
    --workers "$WORKERS"
