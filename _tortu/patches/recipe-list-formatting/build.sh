#!/usr/bin/env bash
# Build, install, and dogfood the recipe-list-formatting patch against a real
# aaif-goose/goose checkout. Draft script — written from source-reading in a
# sandbox with no Rust toolchain, so it has NOT been run. Check every path and
# assumption below before trusting it; treat as a starting point for Claude
# Code on Doug's actual machine, not a verified tool.
#
# Usage: ./build.sh <path-to-goose-checkout>
set -euo pipefail

GOOSE_DIR="${1:?Usage: build.sh <path-to-goose-checkout>}"
PATCH_TARGET="$GOOSE_DIR/crates/goose-cli/src/commands/recipe.rs"

if [ ! -f "$PATCH_TARGET" ]; then
  echo "error: $PATCH_TARGET not found — is \$1 a real aaif-goose/goose checkout?" >&2
  exit 1
fi

echo "==> Patch target found: $PATCH_TARGET"
echo "==> This script does NOT apply the patch automatically — open PATCH.md"
echo "    alongside this checkout and apply the handle_list replacement + new"
echo "    import + two helper functions by hand or via your editor, then re-run"
echo "    this script from step 2 onward (comment out this exit once applied)."
exit 1  # remove this line once the patch has been applied to $PATCH_TARGET

# --- everything below assumes the patch is already applied ---

cd "$GOOSE_DIR"

echo "==> Building goose-cli"
cargo build -p goose-cli

echo "==> Running existing recipe command tests"
cargo test -p goose-cli commands::recipe

CURRENT_GOOSE="$(command -v goose || true)"
if [ -z "$CURRENT_GOOSE" ]; then
  echo "warning: no 'goose' binary found on PATH — skipping install/dogfood steps."
  echo "Built binary is at: $GOOSE_DIR/target/debug/goose"
  exit 0
fi

BACKUP="${CURRENT_GOOSE}.bak"
if [ ! -f "$BACKUP" ]; then
  echo "==> Backing up current binary to $BACKUP"
  cp "$CURRENT_GOOSE" "$BACKUP"
else
  echo "==> Backup already exists at $BACKUP (not overwriting)"
fi

echo "==> Installing patched binary to $CURRENT_GOOSE"
cp "$GOOSE_DIR/target/debug/goose" "$CURRENT_GOOSE"

echo "==> Dogfood: run these yourself and eyeball the output against"
echo "    ../mockups/after-table.txt and ../mockups/after-verbose.txt"
echo
echo "    goose recipe list"
echo "    goose recipe list --verbose"
echo "    goose recipe list --format json   # should be unchanged from before"
echo
echo "==> Rollback if needed: cp \"$BACKUP\" \"$CURRENT_GOOSE\""
