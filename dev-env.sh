#!/bin/bash
# Goose development environment setup
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export PROTOC_INCLUDE="/tmp/include"

echo "🪿 Goose development environment loaded!"
echo "✅ Rust: $(rustc --version 2>/dev/null || echo 'Not found')"
echo "✅ Cargo: $(cargo --version 2>/dev/null || echo 'Not found')"
echo "✅ Protoc: $(protoc --version 2>/dev/null || echo 'Not found')"
echo "✅ Just: $(just --version 2>/dev/null || echo 'Not found')"
echo ""
echo "Quick commands:"
echo "  cargo check    - Quick compilation check"
echo "  cargo test     - Run tests"
echo "  cargo fmt      - Format code"
echo "  ./scripts/clippy-lint.sh - Run linter"
echo "  just --list    - See all available tasks"
# Test modification
# Test
