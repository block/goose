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

# Determine binary name based on target OS
BINARY_NAME="temporal-service"
if [ "${GOOS:-}" = "windows" ]; then
    BINARY_NAME="temporal-service.exe"
fi

# Build the service with cross-compilation support
echo "Compiling Go binary..."
if [ -n "${GOOS:-}" ] && [ -n "${GOARCH:-}" ]; then
    echo "Cross-compiling for ${GOOS}/${GOARCH}..."
    GOOS="${GOOS}" GOARCH="${GOARCH}" go build -buildvcs=false -o "${BINARY_NAME}" .
else
    echo "Building for current platform..."
    go build -buildvcs=false -o "${BINARY_NAME}" .
fi

# Make it executable (skip on Windows as it's not needed)
if [ "${GOOS:-}" != "windows" ]; then
    chmod +x "${BINARY_NAME}"
fi

echo "Build completed successfully!"
echo "Binary location: $(pwd)/${BINARY_NAME}"

# Only show usage info if not cross-compiling
if [ -z "${GOOS:-}" ] || [ "${GOOS}" = "$(go env GOOS)" ]; then
    echo ""
    echo "Prerequisites:"
    echo "  1. Install Temporal CLI: brew install temporal"
    echo "  2. Start Temporal server: temporal server start-dev"
    echo ""
    echo "To run the service:"
    echo "  ./${BINARY_NAME}"
    echo ""
    echo "Environment variables:"
    echo "  PORT - HTTP port (default: 8080)"
fi