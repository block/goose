#!/usr/bin/env bash
#
# Scans goose catalog extensions with the Cisco AI Defense MCP Scanner.
# Shared by the submission-time PR gate and the periodic catalog audit.
#
# Usage:
#   scan-extensions.sh <servers.json> <out-dir> [ids-file]
#
#   <servers.json>  Path to the catalog file.
#   <out-dir>       Directory for raw per-entry results + summary.
#   [ids-file]      Optional newline-separated list of ids to scan. When
#                   omitted, every non-builtin entry is scanned.
#
# Environment:
#   ANALYZERS        Comma-separated mcp-scanner analyzers (default: yara).
#                    YARA needs no API keys. Only enable LLM/API analyzers in a
#                    context where no untrusted code runs with the API key in
#                    scope, since stdio servers execute on this machine.
#   SCAN_TIMEOUT     Per-entry timeout in seconds (default: 180).
#   STDIO_TIMEOUT    mcp-scanner stdio startup timeout (default: 120).
#
# Exit code is always 0; the overall verdict is written to <out-dir>/summary.json
# so callers decide whether to block.

set -uo pipefail

CATALOG="${1:?path to servers.json required}"
OUT_DIR="${2:?output directory required}"
IDS_FILE="${3:-}"

ANALYZERS="${ANALYZERS:-yara}"
SCAN_TIMEOUT="${SCAN_TIMEOUT:-180}"
STDIO_TIMEOUT="${STDIO_TIMEOUT:-120}"

mkdir -p "$OUT_DIR/raw"

for required_tool in jq mcp-scanner python3 timeout; do
  if ! command -v "$required_tool" >/dev/null 2>&1; then
    echo "$required_tool not found on PATH" >&2
    exit 1
  fi
done

SELECT_IDS_JSON="null"
if [ -n "$IDS_FILE" ] && [ -f "$IDS_FILE" ]; then
  SELECT_IDS_JSON=$(jq -R . "$IDS_FILE" | jq -s .)
fi

jq -c --argjson ids "$SELECT_IDS_JSON" '
  map(select(.is_builtin != true))
  | map(select($ids == null or (.id as $i | $ids | index($i))))
  | .[]
' "$CATALOG" > "$OUT_DIR/entries.jsonl"

ENTRY_COUNT=$(wc -l < "$OUT_DIR/entries.jsonl" | tr -d ' ')
echo "Scanning $ENTRY_COUNT extension(s) with analyzers: $ANALYZERS"

while IFS= read -r entry; do
  [ -z "$entry" ] && continue
  id=$(echo "$entry" | jq -r '.id')
  command=$(echo "$entry" | jq -r '.command // ""')
  url=$(echo "$entry" | jq -r '.url // ""')
  raw="$OUT_DIR/raw/$id.json"

  echo "::group::Scanning $id"

  if [ -n "$url" ]; then
    echo "-> remote: $url"
    timeout "$SCAN_TIMEOUT" mcp-scanner \
      --log-level error --analyzers "$ANALYZERS" --format raw \
      remote --server-url "$url" > "$raw" 2>"$OUT_DIR/raw/$id.err"
    rc=$?
  elif [ -n "$command" ]; then
    echo "-> stdio: $command"
    # Split the command string into launcher + args for mcp-scanner.
    launcher=$(echo "$command" | awk '{print $1}')
    args=()
    for a in $(echo "$command" | cut -s -d' ' -f2-); do
      args+=(--stdio-arg="$a")
    done
    timeout "$SCAN_TIMEOUT" mcp-scanner \
      --log-level error --analyzers "$ANALYZERS" --format raw \
      --stdio-timeout "$STDIO_TIMEOUT" \
      stdio --stdio-command "$launcher" "${args[@]}" > "$raw" 2>"$OUT_DIR/raw/$id.err"
    rc=$?
  else
    echo "-> no command or url; skipping"
    echo '{"_scan_status":"skipped","_reason":"no command or url"}' > "$raw"
    rc=0
  fi

  if [ "$rc" -eq 124 ]; then
    echo '{"_scan_status":"timeout"}' > "$raw"
    echo "TIMEOUT:  timed out after ${SCAN_TIMEOUT}s"
  elif [ "$rc" -ne 0 ] && [ ! -s "$raw" ]; then
    echo "{\"_scan_status\":\"error\",\"_exit_code\":$rc}" > "$raw"
    echo "WARNING:  scanner exited $rc"
  fi
  echo "::endgroup::"
done < "$OUT_DIR/entries.jsonl"

python3 "$(dirname "$0")/summarize.py" "$OUT_DIR"
