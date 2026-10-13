#!/usr/bin/env bash
# Runs the goose -> Buzz tools (MCP-over-ACP) harness against each provider.
#   BUZZ_APP=~/Development/buzz-app scripts/acp-buzz-harness/run.sh
# Needs ANTHROPIC_API_KEY and OPENAI_API_KEY, and a built target/debug/goose.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
buzz_app="${BUZZ_APP:-$repo/../../buzz-app}"
goose="${GOOSE_BIN:-$repo/target/debug/goose}"

(cd "$buzz_app" && node_modules/.bin/rolldown src/buzz-mcp/rpc.ts \
  --format esm --platform node --file "$here/buzz-rpc.mjs" >/dev/null)

status=0
for mode in mcp-servers enabled-extensions; do
  for run in "anthropic claude-sonnet-5-5" "openai gpt-6-luna"; do
    set -- $run
    root="$(mktemp -d)"
    echo "===== $1 / $2 / $mode"
    (cd "$root" && HARNESS_MODE="$mode" GOOSE_PATH_ROOT="$root" GOOSE_DISABLE_KEYRING=1 \
      node "$here/acp-buzz-harness.mjs" "$goose" "$1" "$2") || status=1
  done
done
exit $status
