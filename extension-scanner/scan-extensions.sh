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
#                    Used for remote URL scans.
#   STDIO_ANALYZERS  Analyzers for stdio scans (default: yara). Stdio servers
#                    execute third-party code locally, so their scanner process
#                    runs with scanner API keys removed from the environment.
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
STDIO_ANALYZERS="${STDIO_ANALYZERS:-yara}"
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
echo "Scanning $ENTRY_COUNT extension(s) with remote analyzers: $ANALYZERS; stdio analyzers: $STDIO_ANALYZERS"

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
    argv_file="$OUT_DIR/raw/$id.argv"
    if ! python3 - "$command" > "$argv_file" <<'PY'
import shlex
import sys

try:
    parts = shlex.split(sys.argv[1])
except ValueError as exc:
    print(f"invalid command syntax: {exc}", file=sys.stderr)
    sys.exit(2)

if not parts:
    print("empty command", file=sys.stderr)
    sys.exit(2)

for part in parts:
    sys.stdout.buffer.write(part.encode())
    sys.stdout.buffer.write(b"\0")
PY
    then
      echo '{"_scan_status":"error","_reason":"invalid command syntax"}' > "$raw"
      echo "WARNING:  invalid stdio command syntax"
      rc=0
    else
      command_parts=()
      while IFS= read -r -d '' part; do
        command_parts+=("$part")
      done < "$argv_file"
      launcher="${command_parts[0]}"
      args=()
      for a in "${command_parts[@]:1}"; do
        args+=(--stdio-arg="$a")
      done
      timeout "$SCAN_TIMEOUT" env \
        -u MCP_SCANNER_LLM_API_KEY \
        -u MCP_SCANNER_API_KEY \
        -u OPENAI_API_KEY \
        mcp-scanner \
        --log-level error --analyzers "$STDIO_ANALYZERS" --format raw \
        --stdio-timeout "$STDIO_TIMEOUT" \
        stdio --stdio-command "$launcher" "${args[@]}" > "$raw" 2>"$OUT_DIR/raw/$id.err"
      rc=$?
    fi
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
