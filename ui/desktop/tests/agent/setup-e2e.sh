#!/usr/bin/env bash
# Setup script for agent-browser e2e tests
# Cleans up and recreates temp test directories, copies fixtures

set -euo pipefail

E2E_ROOT="/tmp/e2e-test"
E2E_WORKSPACE="/tmp/e2e-test-workspace"
FIXTURES_DIR="$(dirname "$0")/fixtures"

echo "Setting up e2e test environment..."

# Clean up and recreate temp directories
rm -rf "$E2E_ROOT"
mkdir -p "$E2E_ROOT"

rm -rf "$E2E_WORKSPACE"
mkdir -p "$E2E_WORKSPACE"

# Copy goosehints to workspace
cp "$FIXTURES_DIR/e2e-goosehints" "$E2E_WORKSPACE/.goosehints"

echo "E2E setup complete:"
echo "  Root:      $E2E_ROOT"
echo "  Workspace: $E2E_WORKSPACE"
echo "  Hints:     $E2E_WORKSPACE/.goosehints"
