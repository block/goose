#!/usr/bin/env bash
# gather-catalog.sh (health-check) — items + categories only
#
# Fetches all ITEM and CATEGORY objects via the square CLI, handling pagination,
# and outputs consolidated JSON to stdout.
#
# Usage:
#   gather-catalog.sh [--focus-area AREA]
#
# Focus areas: completeness, duplicates, pricing, categories, full (default)
# When focus is completeness or duplicates, categories are skipped to save time.

set -euo pipefail

FOCUS_AREA="full"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --focus-area) FOCUS_AREA="$2"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

TMPDIR_GATHER=$(mktemp -d)
trap 'rm -rf "$TMPDIR_GATHER"' EXIT

# Fetch all objects of a given type, paginating through all results.
# Writes to a temp file to avoid shell argument length limits.
fetch_all() {
  local obj_type="$1"
  local cursor=""
  local pages_file="$TMPDIR_GATHER/${obj_type}_pages.jsonl"
  > "$pages_file"

  while true; do
    local args=("catalog" "list" "--types" "$obj_type")
    if [[ -n "$cursor" ]]; then
      args+=("--cursor" "$cursor")
    fi

    local resp_file="$TMPDIR_GATHER/${obj_type}_resp.json"
    square "${args[@]}" > "$resp_file"

    jq -c '.objects // []' "$resp_file" >> "$pages_file"

    cursor=$(jq -r '.cursor // empty' "$resp_file")
    if [[ -z "$cursor" ]]; then
      break
    fi
  done

  # Merge all pages into a single array
  jq -s 'add // []' "$pages_file"
}

ITEMS_FILE="$TMPDIR_GATHER/items.json"
CATS_FILE="$TMPDIR_GATHER/categories.json"

# Always fetch items
fetch_all "ITEM" > "$ITEMS_FILE"

# Only fetch categories when needed
if [[ "$FOCUS_AREA" == "categories" || "$FOCUS_AREA" == "pricing" || "$FOCUS_AREA" == "full" ]]; then
  fetch_all "CATEGORY" > "$CATS_FILE"
else
  echo '[]' > "$CATS_FILE"
fi

# Output consolidated JSON using file inputs (avoids argument length limits)
jq -n \
  --slurpfile items "$ITEMS_FILE" \
  --slurpfile categories "$CATS_FILE" \
  --arg focus_area "$FOCUS_AREA" \
  '{focus_area: $focus_area, items: $items[0], categories: $categories[0]}'
