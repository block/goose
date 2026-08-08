#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pass_count=0

pass() {
  pass_count=$((pass_count + 1))
  printf 'PASS %s/9: %s\n' "$pass_count" "$1"
}

fail() {
  printf 'FAIL %s/9: %s\n' "$((pass_count + 1))" "$1" >&2
  exit 1
}

docker compose ps --status running --services | grep -qx server \
  && pass "server container is running" \
  || fail "server container is not running"

http_code="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  "http://localhost:${SERVER_PORT:-3000}/acp" || true)"
case "$http_code" in
  000|502|'') fail "ACP backend did not respond (HTTP ${http_code:-none})" ;;
  *) pass "ACP backend responds (HTTP $http_code)" ;;
esac

version_output="$(docker compose --profile cli run --rm cli --version 2>&1)" \
  || fail "CLI --version exited non-zero"
printf '%s' "$version_output" | grep -Eq '[0-9]+\.[0-9]+\.[0-9]+' \
  && pass "CLI reports a semantic version" \
  || fail "CLI output contains no semantic version"

app_path="$(find ui/desktop/out -type d -name 'AVCD Agent.app' -print -quit 2>/dev/null || true)"
[ -n "$app_path" ] \
  && pass "AVCD Agent desktop bundle exists" \
  || fail "AVCD Agent desktop bundle is missing"

plist="$app_path/Contents/Info.plist"
bundle_name="$(plutil -extract CFBundleName raw "$plist" 2>/dev/null || true)"
executable_name="$(plutil -extract CFBundleExecutable raw "$plist" 2>/dev/null || true)"
if [ "$bundle_name" = "AVCD Agent" ] && [[ "$executable_name" != *Goose* ]]; then
  pass "desktop bundle metadata is rebranded"
else
  fail "desktop bundle metadata still contains Goose or has the wrong name"
fi

if ! grep -q 'aaif-goose' ui/desktop/src/app-update.yml \
  && ! grep -A8 'publisher-github' ui/desktop/forge.config.ts | grep -q "'aaif-goose'"; then
  pass "desktop updater targets the fork"
else
  fail "desktop updater still targets aaif-goose"
fi

if ! grep -Rqs 'phc_RyX5CaY01VtZJCQyhSR5KFh6qimUy81YwxsEpotAftT' crates; then
  pass "upstream PostHog key is absent"
else
  fail "upstream PostHog key is still present"
fi

if grep -q '<title>AVCD Agent</title>' ui/desktop/index.html \
  && grep -q 'called AVCD Agent' crates/goose/src/prompts/system.md; then
  pass "desktop title and system persona are rebranded"
else
  fail "desktop title or system persona is not rebranded"
fi

git diff --quiet upstream/main -- LICENSE \
  || fail "LICENSE differs from upstream"
[ -s NOTICE ] \
  && grep -q 'Apache License 2.0' NOTICE \
  && pass "license and attribution are present" \
  || fail "NOTICE is missing Apache 2.0 attribution"

printf 'All %s smoke checks passed.\n' "$pass_count"
