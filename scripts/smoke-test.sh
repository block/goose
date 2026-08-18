#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TOTAL=11
pass_count=0

pass() {
  pass_count=$((pass_count + 1))
  printf 'PASS %s/%s: %s\n' "$pass_count" "$TOTAL" "$1"
}

fail() {
  printf 'FAIL %s/%s: %s\n' "$((pass_count + 1))" "$TOTAL" "$1" >&2
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

app_path="$(find ui/desktop/out -type d -name 'Avocado Work.app' -print -quit 2>/dev/null || true)"
[ -n "$app_path" ] \
  && pass "Avocado Work desktop bundle exists" \
  || fail "Avocado Work desktop bundle is missing"

plist="$app_path/Contents/Info.plist"
bundle_name="$(plutil -extract CFBundleName raw "$plist" 2>/dev/null || true)"
executable_name="$(plutil -extract CFBundleExecutable raw "$plist" 2>/dev/null || true)"
if [ "$bundle_name" = "Avocado Work" ] \
  && [[ "$executable_name" != *Goose* ]] \
  && [[ "$executable_name" != *AVCD* ]] \
  && [[ "$executable_name" != *avcd* ]]; then
  pass "desktop bundle metadata is rebranded to Avocado Work"
else
  fail "desktop bundle metadata still contains Goose/AVCD or has the wrong name (got name=${bundle_name:-none} exe=${executable_name:-none})"
fi

if ! grep -q 'aaif-goose' ui/desktop/src/app-update.yml \
  && ! grep -A8 'publisher-github' ui/desktop/forge.config.ts | grep -q "'aaif-goose'" \
  && grep -q "GOOSE_BUNDLE_NAME.*Avocado Work" ui/desktop/vite.main.config.mts; then
  pass "desktop updater targets the fork and Avocado Work bundle name"
else
  fail "desktop updater still targets aaif-goose or GOOSE_BUNDLE_NAME is not Avocado Work"
fi

if ! grep -Rqs 'phc_RyX5CaY01VtZJCQyhSR5KFh6qimUy81YwxsEpotAftT' crates; then
  pass "upstream PostHog key is absent"
else
  fail "upstream PostHog key is still present"
fi

if grep -q '<title>Avocado Work</title>' ui/desktop/index.html \
  && grep -q 'called Avocado Work' crates/goose/src/prompts/system.md; then
  pass "desktop title and system persona are Avocado Work"
else
  fail "desktop title or system persona is not Avocado Work"
fi

if grep -q '#5c7230' ui/desktop/src/images/icon.svg; then
  pass "app icon uses avocado.tech brand green"
else
  fail "app icon.svg is missing brand fill #5c7230"
fi

git diff --quiet upstream/main -- LICENSE \
  || fail "LICENSE differs from upstream"
[ -s NOTICE ] \
  && grep -q 'Apache License 2.0' NOTICE \
  && pass "license and attribution are present" \
  || fail "NOTICE is missing Apache 2.0 attribution"

if grep -q 'REPO="Avocado-Technology/avcd-agent"' download_cli.sh \
  && ! grep -q 'aaif-goose/goose' download_cli.sh; then
  pass "download_cli.sh installs from Avocado-Technology/avcd-agent"
else
  fail "download_cli.sh still references aaif-goose/goose"
fi

printf 'All %s smoke checks passed.\n' "$pass_count"
