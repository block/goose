#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RELEASE_ASSETS_JS="$ROOT_DIR/ui/desktop/scripts/release-assets.js"
MODE="scan"
LOCAL_DIR=""

usage() {
  cat <<'EOF'
Usage:
  bash scripts/verify-release-assets.sh              # static scan for Goose desktop artifacts
  bash scripts/verify-release-assets.sh --local DIR  # assert contracted files exist under DIR
  bash scripts/verify-release-assets.sh --guards     # publishing workflow fork guards
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --local)
      MODE="local"
      LOCAL_DIR="${2:-}"
      if [[ -z "$LOCAL_DIR" ]]; then
        usage
        exit 2
      fi
      shift 2
      ;;
    --guards)
      MODE="guards"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

if [[ ! -f "$RELEASE_ASSETS_JS" ]]; then
  echo "FAIL: missing $RELEASE_ASSETS_JS" >&2
  exit 1
fi

BUNDLE_NAME="$(GOOSE_BUNDLE_NAME="${GOOSE_BUNDLE_NAME:-Avocado Work}" node -e "
const { getBundleName } = require('./ui/desktop/scripts/release-assets.js');
process.stdout.write(getBundleName());
")"

ASSET_JSON="$(GOOSE_BUNDLE_NAME="$BUNDLE_NAME" node -e "
const { getReleaseAssets, allReleaseFilenames } = require('./ui/desktop/scripts/release-assets.js');
const assets = getReleaseAssets();
process.stdout.write(JSON.stringify({ assets, all: allReleaseFilenames() }));
")"

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

ok() {
  echo "OK  $1"
}

scan_goose_artifacts() {
  local files=(
    .github/workflows/bundle-macos.yml
    .github/workflows/bundle-windows.yml
    .github/workflows/release.yml
    .github/workflows/canary.yml
    .github/workflows/release-branches.yml
    ui/desktop/scripts/generate-mac-update-manifest.js
    ui/desktop/scripts/verify-mac-update-resources.js
  )
  local offenders=()
  local pattern
  pattern="$(node -e "const { forbiddenGooseArtifactPattern } = require('./ui/desktop/scripts/release-assets.js'); process.stdout.write(forbiddenGooseArtifactPattern().source);")"

  local f
  for f in "${files[@]}"; do
    [[ -f "$f" ]] || continue
    if grep -nE "$pattern" "$f" >/dev/null 2>&1; then
      while IFS= read -r line; do
        offenders+=("$f:$line")
      done < <(grep -nE "$pattern" "$f" || true)
    fi
  done

  # Also catch Goose*.zip globs and Goose.app path fragments used as release artifacts.
  for f in "${files[@]}"; do
    [[ -f "$f" ]] || continue
    if grep -nE 'Goose\*\.zip|Goose\.app|Goose-darwin-|Goose-win32-|name: Goose-|out/Goose-' "$f" >/dev/null 2>&1; then
      while IFS= read -r line; do
        offenders+=("$f:$line")
      done < <(grep -nE 'Goose\*\.zip|Goose\.app|Goose-darwin-|Goose-win32-|name: Goose-|out/Goose-' "$f" || true)
    fi
  done

  # Deduplicate
  if [[ ${#offenders[@]} -gt 0 ]]; then
    printf '%s\n' "${offenders[@]}" | awk '!seen[$0]++' | while IFS= read -r off; do
      echo "OFFENDER  $off" >&2
    done
    fail "found Goose-named desktop release artifact references (bundle=$BUNDLE_NAME). Fix workflows/scripts to use release-assets.js names."
  fi

  ok "no Goose-named desktop release artifacts in release paths (bundle=$BUNDLE_NAME)"
}

check_local() {
  local dir="$1"
  [[ -d "$dir" ]] || fail "local directory does not exist: $dir"

  node -e "
const fs = require('node:fs');
const path = require('node:path');
const { getReleaseAssets } = require('./ui/desktop/scripts/release-assets.js');
const assets = getReleaseAssets();
const root = path.resolve(process.argv[1]);
const platform = process.platform;
const checks = [];
if (platform === 'darwin') {
  checks.push(
    path.join(root, assets.macArm64.appDir, assets.macArm64.update),
    path.join(root, 'make', assets.macArm64.website),
  );
} else if (platform === 'win32') {
  checks.push(path.join(root, 'make', 'squirrel.windows', 'x64', assets.winX64.website));
} else {
  console.error('FAIL: --local is only supported on darwin/win32 hosts');
  process.exit(1);
}
let failed = false;
for (const file of checks) {
  if (fs.existsSync(file)) {
    console.log('OK  ' + path.relative(root, file));
  } else {
    console.error('MISSING  ' + path.relative(root, file));
    failed = true;
  }
}
if (failed) process.exit(1);
" "$dir"
}

check_guards() {
  local failed=0
  check_guarded() {
    local file="$1"
    local label="$2"
    if [[ ! -f "$file" ]]; then
      echo "MISSING  $file" >&2
      failed=1
      return
    fi
    if grep -q "github.repository == 'aaif-goose/goose'" "$file" \
      || grep -q 'github.repository == "aaif-goose/goose"' "$file"; then
      ok "guarded: $label"
    elif [[ "$file" == *publish-docker.yml ]] && grep -Fq 'ghcr.io/${{ github.repository }}' "$file"; then
      ok "fork-correct images: $label"
    else
      echo "UNGUARDED  $label ($file)" >&2
      failed=1
    fi
  }

  check_guarded .github/workflows/publish-docker.yml publish-docker
  check_guarded .github/workflows/publish-npm.yml publish-npm
  check_guarded .github/workflows/maven-sdk.yml maven-sdk

  if grep -q "github.repository == 'aaif-goose/goose'" .github/workflows/canary.yml \
    || grep -q 'github.repository == "aaif-goose/goose"' .github/workflows/canary.yml; then
    ok "guarded: canary release job / workflow"
  else
    # Accept a job-level guard on the release job
    if grep -A5 'name: Release' .github/workflows/canary.yml | grep -q "aaif-goose/goose"; then
      ok "guarded: canary Release job"
    else
      echo "UNGUARDED  canary.yml release publishing" >&2
      failed=1
    fi
  fi

  if grep -Fq 'ghcr.io/${{ github.repository_owner }}/goose' .github/workflows/publish-docker.yml; then
    echo "BAD_IMAGE  publish-docker still targets .../goose" >&2
    failed=1
  fi

  [[ $failed -eq 0 ]] || fail "publishing workflow guards incomplete"
  ok "publishing workflow guards present"
}

case "$MODE" in
  scan)
    scan_goose_artifacts
    ;;
  local)
    check_local "$LOCAL_DIR"
    ;;
  guards)
    check_guards
    ;;
esac
