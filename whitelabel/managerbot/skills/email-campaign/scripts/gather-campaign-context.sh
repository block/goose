#!/usr/bin/env bash
# gather-campaign-context.sh — Collect catalog, campaign history, themes, and brand style
#
# Usage:
#   gather-campaign-context.sh [--itemKeywords <keyword1,keyword2,...>] [--maxItemPages <n>]
#
# Outputs consolidated JSON with:
#   - catalogItems: matching catalog items (searched by keywords if provided;
#                   otherwise all items up to --maxItemPages pages, default 10)
#   - recentCampaigns: last 20 email campaigns for tone/style reference
#   - themes: available email themes
#   - userStyle: merchant brand colors/style
#   - sites: Square Online sites (for button link URLs)
#
# All failures fall back to empty objects/arrays so downstream always gets valid JSON.

set -euo pipefail

ITEM_KEYWORDS=""
MAX_ITEM_PAGES=10

while [[ $# -gt 0 ]]; do
  case $1 in
    --itemKeywords)
      ITEM_KEYWORDS="$2"
      shift 2
      ;;
    --maxItemPages)
      MAX_ITEM_PAGES="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Paginate catalog search results into a single objects+related_objects array.
# $1: base jq body expression (without cursor)
# $2: max pages
paginate_catalog() {
  local base_body="$1" max_pages="$2"
  local cursor="" all_objects="[]" all_related="[]"
  for _ in $(seq 1 "$max_pages"); do
    local body
    if [[ -n "$cursor" ]]; then
      body=$(echo "$base_body" | jq --arg c "$cursor" '. + {cursor: $c}')
    else
      body="$base_body"
    fi
    local page
    page=$(square catalog search --body "$body" 2>/dev/null || echo '{"objects":[]}')
    local objects related
    objects=$(echo "$page" | jq '.objects // []')
    related=$(echo "$page" | jq '.related_objects // []')
    all_objects=$(printf '%s\n%s' "$all_objects" "$objects" | jq -sc 'add')
    all_related=$(printf '%s\n%s' "$all_related" "$related" | jq -sc 'add')
    cursor=$(echo "$page" | jq -r '.cursor // empty')
    [[ -z "$cursor" ]] && break
  done
  echo "{\"objects\":$all_objects,\"related_objects\":$all_related}"
}

# 1. Catalog items
if [[ -n "$ITEM_KEYWORDS" ]]; then
  # Search each keyword separately and merge, deduplicating by ID
  all_objects="[]"
  all_related="[]"
  while IFS= read -r kw; do
    [[ -z "$kw" ]] && continue
    BASE_BODY=$(jq -n --arg kw "$kw" \
      '{"object_types":["ITEM"],"include_related_objects":true,"query":{"text_query":{"keywords":[$kw]}}}')
    page_result=$(paginate_catalog "$BASE_BODY" "$MAX_ITEM_PAGES" 2>/dev/null \
      || echo '{"objects":[],"related_objects":[]}')
    page_objects=$(echo "$page_result" | jq '.objects // []')
    page_related=$(echo "$page_result" | jq '.related_objects // []')
    all_objects=$(printf '%s\n%s' "$all_objects" "$page_objects" | jq -sc 'add | unique_by(.id)')
    all_related=$(printf '%s\n%s' "$all_related" "$page_related" | jq -sc 'add | unique_by(.id)')
  done < <(echo "$ITEM_KEYWORDS" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  echo "{\"objects\":$all_objects,\"related_objects\":$all_related}" > "$TMPDIR/catalog.json"
else
  # No keywords — paginate all items
  BASE_BODY='{"object_types":["ITEM"],"include_related_objects":true}'
  paginate_catalog "$BASE_BODY" "$MAX_ITEM_PAGES" > "$TMPDIR/catalog.json" \
    || echo '{"objects":[],"related_objects":[]}' > "$TMPDIR/catalog.json"
fi

# 2. Recent campaigns for tone/style reference
square email-campaigns list --per-page 20 \
  > "$TMPDIR/campaigns.json" 2>/dev/null || echo '{"campaigns":[]}' > "$TMPDIR/campaigns.json"

# 3. Available themes
square email-campaigns themes list --channel email --themes classic \
  > "$TMPDIR/themes.json" 2>/dev/null || echo '{"themes":[]}' > "$TMPDIR/themes.json"

# 4. Merchant brand style
square email-campaigns user-style get \
  > "$TMPDIR/user_style.json" 2>/dev/null || echo '{}' > "$TMPDIR/user_style.json"

# 5. Square Online sites (for button link URLs)
square sites list \
  > "$TMPDIR/sites.json" 2>/dev/null || echo '{"sites":[]}' > "$TMPDIR/sites.json"

# Combine into single JSON output.
# Build a lookup of image_id → url from related_objects, then annotate each
# catalog item with its primary image_url for easy access.
jq -n \
  --slurpfile catalogItems "$TMPDIR/catalog.json" \
  --slurpfile recentCampaigns "$TMPDIR/campaigns.json" \
  --slurpfile themes "$TMPDIR/themes.json" \
  --slurpfile userStyle "$TMPDIR/user_style.json" \
  --slurpfile sites "$TMPDIR/sites.json" \
  --arg item "$ITEM_KEYWORDS" \
  '
  ($catalogItems[0].related_objects // []
    | map(select(.type == "IMAGE"))
    | map({(.id): .image_data.url})
    | add // {}) as $imgMap |
  {
    metadata: {
      item_search: (if $item == "" then null else $item end),
      generated_at: (now | todate)
    },
    catalogItems: {
      objects: ($catalogItems[0].objects // [] | map(
        . + {image_url: ((.item_data.image_ids // [])[0] as $iid | if $iid then $imgMap[$iid] else null end)}
      ))
    },
    recentCampaigns: $recentCampaigns[0],
    themes: $themes[0],
    userStyle: $userStyle[0],
    sites: $sites[0]
  }'
