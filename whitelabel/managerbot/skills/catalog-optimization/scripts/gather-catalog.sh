#!/usr/bin/env bash
# gather-catalog.sh (optimization) — items + categories + sales signal
#
# Fetches all ITEM and CATEGORY objects via the square CLI, plus best-effort
# sales reporting data, and outputs consolidated JSON to stdout.
#
# Usage:
#   gather-catalog.sh [--focus-area AREA] [--sales-limit N]
#
# Focus areas: images, pricing, categories, full (default)
# Sales data is fetched for images and full (where revenue weighting matters),
# skipped for pricing and categories focus areas.

set -euo pipefail

FOCUS_AREA="full"
SALES_LIMIT=200

while [[ $# -gt 0 ]]; do
  case "$1" in
    --focus-area) FOCUS_AREA="$2"; shift 2 ;;
    --sales-limit) SALES_LIMIT="$2"; shift 2 ;;
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

  # Merge all pages into a single array, write to output file
  jq -s 'add // []' "$pages_file"
}

# Best-effort sales signal via reporting API.
# Writes JSON to stdout. Gracefully degrades on failure.
fetch_sales_signal() {
  local limit="$1"

  # Try to get reporting metadata
  local meta_file="$TMPDIR_GATHER/meta.json"
  if ! square reporting meta > "$meta_file" 2>/dev/null; then
    echo '{"available": false, "status": "unavailable", "note": "Reporting unavailable", "rows": []}'
    return
  fi

  # Find a compatible cube with revenue + item dimensions
  local query_result
  query_result=$(python3 -c "
import json, sys, re

with open('$meta_file') as f:
    meta = json.load(f)
limit = $limit

for cube in (meta.get('cubes') or []):
    measures = [m['name'] for m in (cube.get('measures') or []) if m.get('name')]
    dimensions = [d['name'] for d in (cube.get('dimensions') or []) if d.get('name')]
    if not measures or not dimensions:
        continue

    def first_match(names, patterns):
        for p in patterns:
            rx = re.compile(p, re.IGNORECASE)
            for n in names:
                if rx.search(n):
                    return n
        return None

    revenue = first_match(measures, [r'\.net_sales\$', r'\.total_revenue\$', r'\.gross_sales\$', r'revenue', r'sales'])
    quantity = first_match(measures, [r'quantity', r'sold', r'count'])
    item_name = first_match(dimensions, [r'item.*name', r'line.*item.*name', r'\.name\$'])
    item_id = first_match(dimensions, [r'item.*id', r'\.id\$'])

    if not revenue or (not item_name and not item_id):
        continue

    query = {
        'measures': [revenue] + ([quantity] if quantity else []),
        'dimensions': [d for d in [item_name, item_id] if d],
        'limit': max(20, min(limit, 500)),
    }
    json.dump({'cube': cube.get('name', 'unknown'), 'query': query, 'revenue_field': revenue, 'quantity_field': quantity or '', 'item_name_field': item_name or '', 'item_id_field': item_id or ''}, sys.stdout)
    sys.exit(0)

json.dump({'cube': None}, sys.stdout)
" 2>/dev/null) || true

  if [[ -z "$query_result" ]] || [[ "$(echo "$query_result" | jq -r '.cube')" == "null" ]]; then
    echo '{"available": false, "status": "unavailable", "note": "No compatible reporting cube found", "rows": []}'
    return
  fi

  local cube_name query_json
  cube_name=$(echo "$query_result" | jq -r '.cube')
  query_json=$(echo "$query_result" | jq -r '.query')

  local data_file="$TMPDIR_GATHER/sales_data.json"
  if ! square reporting query --raw "$query_json" > "$data_file" 2>/dev/null; then
    echo '{"available": false, "status": "unavailable", "note": "Reporting query failed", "rows": []}'
    return
  fi

  local query_info_file="$TMPDIR_GATHER/query_info.json"
  echo "$query_result" > "$query_info_file"

  # Transform raw reporting rows into normalized format
  python3 -c "
import json

with open('$query_info_file') as f:
    query_info = json.load(f)
with open('$data_file') as f:
    data = json.load(f)

rows = []
for row in (data.get('data') or []):
    if not isinstance(row, dict):
        continue
    item_name_field = query_info.get('item_name_field', '')
    item_id_field = query_info.get('item_id_field', '')
    revenue_field = query_info['revenue_field']
    quantity_field = query_info.get('quantity_field', '')

    def parse_num(v):
        if v is None: return 0.0
        if isinstance(v, (int, float)): return float(v)
        if isinstance(v, str):
            try: return float(v.replace(',', '').strip())
            except: return 0.0
        return 0.0

    rows.append({
        'item_name': str(row.get(item_name_field, '')) if item_name_field else '',
        'item_id': str(row.get(item_id_field, '')) if item_id_field else '',
        'net_sales': parse_num(row.get(revenue_field)),
        'quantity_sold': parse_num(row.get(quantity_field)) if quantity_field else 0.0,
    })

rows.sort(key=lambda r: r['net_sales'], reverse=True)
cube_name = query_info.get('cube', 'unknown')
json.dump({
    'available': True,
    'status': 'available',
    'note': f'Reporting available ({cube_name})',
    'cube': cube_name,
    'rows': rows[:$limit],
}, sys.stdout)
" 2>/dev/null || echo '{"available": false, "status": "unavailable", "note": "Failed to parse reporting data", "rows": []}'
}

ITEMS_FILE="$TMPDIR_GATHER/items.json"
CATS_FILE="$TMPDIR_GATHER/categories.json"
SALES_FILE="$TMPDIR_GATHER/sales.json"

# Fetch items and categories
fetch_all "ITEM" > "$ITEMS_FILE"
fetch_all "CATEGORY" > "$CATS_FILE"

# Only fetch sales signal when it adds value
if [[ "$FOCUS_AREA" == "images" || "$FOCUS_AREA" == "full" ]]; then
  fetch_sales_signal "$SALES_LIMIT" > "$SALES_FILE"
else
  echo '{"available": false, "status": "skipped", "note": "Sales data not needed for this focus area", "rows": []}' > "$SALES_FILE"
fi

# Output consolidated JSON using file inputs (avoids argument length limits)
jq -n \
  --slurpfile items "$ITEMS_FILE" \
  --slurpfile categories "$CATS_FILE" \
  --slurpfile sales_signal "$SALES_FILE" \
  --arg focus_area "$FOCUS_AREA" \
  '{focus_area: $focus_area, items: $items[0], categories: $categories[0], sales_signal: $sales_signal[0]}'
