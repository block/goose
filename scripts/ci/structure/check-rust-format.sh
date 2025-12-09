#!/usr/bin/env bash
set -e

# Check Rust code formatting

echo "Checking Rust code formatting..."

if ! cargo fmt --all --check; then
  echo ""
  echo "ERROR: Rust code is not properly formatted"
  echo ""
  echo "To fix this issue:"
  echo "  cargo fmt --all"
  echo ""
  exit 1
fi

echo "Rust code formatting is correct"
