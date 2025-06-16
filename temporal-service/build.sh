#!/bin/bash

# Build script for Temporal service
set -e

echo "Building Temporal service..."

# Change to temporal-service directory
cd "$(dirname "$0")"

# Initialize Go module if not already done
if [ ! -f "go.sum" ]; then
    echo "Initializing Go module..."
    go mod tidy
fi

# Build the service
echo "Compiling Go binary..."
go build -o temporal-service main.go

# Make it executable
chmod +x temporal-service

echo "Build completed successfully!"
echo "Binary location: $(pwd)/temporal-service"
echo ""
echo "Prerequisites:"
echo "  1. Install Temporal CLI: brew install temporal"
echo "  2. Start Temporal server: temporal server start-dev"
echo ""
echo "To run the service:"
echo "  ./temporal-service"
echo ""
echo "Environment variables:"
echo "  PORT - HTTP port (default: 8080)"