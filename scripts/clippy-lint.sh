#!/bin/bash

# Combined lint script
# Runs standard clippy (strict) + baseline clippy rules

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the baseline functions
source "$SCRIPT_DIR/clippy-baseline.sh"

echo "🔍 Running all clippy checks..."

FIX_MODE=0
[[ "$1" == "--fix" ]] && FIX_MODE=1

run_clippy() {
  if [[ "$FIX_MODE" -eq 1 ]]; then
    cargo fmt
    cargo clippy --all-targets --jobs 2 \
      --fix --allow-dirty --allow-staged \
      -- -D warnings
  else
    cargo clippy --all-targets --jobs 2 -- -D warnings
  fi
}

if [[ "$FIX_MODE" -eq 1 ]]; then
  echo "🛠  Applying fixes..."
else
  echo "🔍 Running clippy..."
fi

run_clippy
echo ""
check_all_baseline_rules
echo ""
echo "🔒 Checking for banned TLS crates..."
"$SCRIPT_DIR/check-no-native-tls.sh"
echo ""
echo "✅ Done"
