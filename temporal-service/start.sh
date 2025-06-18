#!/bin/bash

# Startup script for Temporal service with integrated Temporal server
set -e

echo "Starting Temporal development environment..."

# Check if temporal CLI is available
if ! command -v temporal &> /dev/null; then
    echo "Error: Temporal CLI not found!"
    echo "Please install it first:"
    echo "  brew install temporal"
    echo "  # or download from https://github.com/temporalio/cli/releases"
    exit 1
fi

# Check if temporal-service binary exists
if [ ! -f "./temporal-service" ]; then
    echo "Error: temporal-service binary not found!"
    echo "Please build it first: ./build.sh"
    exit 1
fi

# Set data directory
DATA_DIR="${GOOSE_DATA_DIR:-./data}"
mkdir -p "$DATA_DIR"

echo "Data directory: $DATA_DIR"
echo "Starting Temporal server..."

# Start Temporal server in background
temporal server start-dev \
    --db-filename "$DATA_DIR/temporal.db" \
    --port 7233 \
    --ui-port 8233 \
    --log-level warn &

TEMPORAL_PID=$!
echo "Temporal server started with PID: $TEMPORAL_PID"

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    if [ ! -z "$SERVICE_PID" ]; then
        echo "Stopping temporal-service (PID: $SERVICE_PID)..."
        kill $SERVICE_PID 2>/dev/null || true
    fi
    echo "Stopping Temporal server (PID: $TEMPORAL_PID)..."
    kill $TEMPORAL_PID 2>/dev/null || true
    wait $TEMPORAL_PID 2>/dev/null || true
    echo "Shutdown complete"
}

# Set trap for cleanup
trap cleanup EXIT INT TERM

# Wait for Temporal server to be ready
echo "Waiting for Temporal server to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:7233/api/v1/namespaces > /dev/null 2>&1; then
        echo "Temporal server is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "Error: Temporal server failed to start within 30 seconds"
        exit 1
    fi
    sleep 1
done

# Start the temporal service
echo "Starting temporal-service..."
PORT="${PORT:-8080}" ./temporal-service &
SERVICE_PID=$!

echo ""
echo "🎉 Temporal development environment is running!"
echo ""
echo "Services:"
echo "  - Temporal Server: http://localhost:7233 (gRPC)"
echo "  - Temporal Web UI: http://localhost:8233"
echo "  - Goose Scheduler API: http://localhost:${PORT:-8080}"
echo ""
echo "API Endpoints:"
echo "  - Health: http://localhost:${PORT:-8080}/health"
echo "  - Jobs: http://localhost:${PORT:-8080}/jobs"
echo ""
echo "Press Ctrl+C to stop all services"

# Wait for the service to exit
wait $SERVICE_PID