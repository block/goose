#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.27.0"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

goose run --recipe "$SCRIPT_DIR/recipe.yaml" --params "version=$1"
