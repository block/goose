#!/bin/bash
# Verify that the docs map was generated in the build output

BUILD_DIR="${1:-build}"
DOCS_MAP_FILE="goose-docs-map.md"

if [ ! -f "$BUILD_DIR/$DOCS_MAP_FILE" ]; then
  echo "Error: $DOCS_MAP_FILE not found in $BUILD_DIR"
  exit 1
fi

echo "✓ $DOCS_MAP_FILE found in $BUILD_DIR"
