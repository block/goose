#!/usr/bin/env bash
set -euo pipefail

# Build the reusable Docker image for Windows cross-compilation
IMAGE_NAME="goose/windows-build:latest"
DOCKERFILE_DIR="docker/windows-build"

echo "Building Docker image ${IMAGE_NAME} from ${DOCKERFILE_DIR}..."

docker build -t "${IMAGE_NAME}" "${DOCKERFILE_DIR}"

echo "Built ${IMAGE_NAME}"
