#!/bin/bash

# Combined lint script
# Runs standard clippy (strict) + baseline clippy rules

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the baseline functions
source "$SCRIPT_DIR/clippy-baseline.sh"

echo "🔍 Running all clippy checks..."

# Run standard clippy with strict warnings
echo "  → Standard clippy rules (strict)"
cargo clippy --jobs 2 -- -D warnings

# Run baseline rules check
echo ""
check_all_baseline_rules

echo ""
echo "✅ All lint checks passed!"