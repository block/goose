#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit

if [[ -f "$ROOT_DIR/distro/security-cn/config/desktop-env.example" ]]; then
  set -a
  source "$ROOT_DIR/distro/security-cn/config/desktop-env.example"
  set +a
fi

BUNDLE_NAME="${GOOSE_BUNDLE_NAME:-$(node -p "require('./distro/security-cn/branding/product-metadata.json').productName")}"
APP_BUNDLE="${SECURITY_GOOSE_APP_BUNDLE:-$ROOT_DIR/ui/desktop/out/${BUNDLE_NAME}-darwin-arm64/${BUNDLE_NAME}.app}"
APP_BIN="$APP_BUNDLE/Contents/MacOS/$BUNDLE_NAME"
BUNDLED_GOOSED="$APP_BUNDLE/Contents/Resources/bin/goosed"
SMOKE_ROOT="${SECURITY_PACKAGED_SMOKE_ROOT:-$ROOT_DIR/.preview/packaged-smoke}"
USER_DATA_DIR="${GOOSE_USER_DATA_DIR:-$SMOKE_ROOT/user-data}"
GOOSE_PATH_ROOT="${GOOSE_PATH_ROOT:-$SMOKE_ROOT/goose-path-root}"
WORKDIR="${SECURITY_PACKAGED_WORKDIR:-$SMOKE_ROOT/workdir}"
STARTUP_LOG_DIR="$USER_DATA_DIR/logs/startup"
STARTUP_LOG_PATH=""
APP_PID=""

cleanup() {
  pkill -f "$APP_BIN" >/dev/null 2>&1 || true
  pkill -f "$BUNDLED_GOOSED" >/dev/null 2>&1 || true

  if [[ -n "$APP_PID" ]]; then
    wait "$APP_PID" >/dev/null 2>&1 || true
  fi
}

ensure_bundle() {
  if [[ -x "$APP_BIN" ]]; then
    return
  fi

  pnpm --dir ui/desktop run bundle:default
}

wait_for_runtime_assets() {
  local recipe_path="$WORKDIR/.goose/recipes/security-vuln-triage.yaml"
  local skill_path="$WORKDIR/.agents/skills/vuln-triage/SKILL.md"

  for _ in $(seq 1 60); do
    if [[ -f "$recipe_path" && -f "$skill_path" ]]; then
      return 0
    fi
    sleep 1
  done

  echo "Timed out waiting for packaged runtime assets in $WORKDIR" >&2
  return 1
}

wait_for_startup_log() {
  for _ in $(seq 1 60); do
    STARTUP_LOG_PATH="$(find "$STARTUP_LOG_DIR" -maxdepth 1 -name 'goosed-startup-*.json' -type f 2>/dev/null | sort | tail -n 1 || true)"
    if [[ -n "$STARTUP_LOG_PATH" ]]; then
      if node -e '
        const fs = require("node:fs");
        const diagnostics = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
        const expectedGoosed = process.argv[2];
        const expectedWorkdir = process.argv[3];
        if (!diagnostics.healthCheckSucceeded) process.exit(1);
        if (diagnostics.goosedPath !== expectedGoosed) process.exit(2);
        if (diagnostics.workingDir !== expectedWorkdir) process.exit(3);
      ' "$STARTUP_LOG_PATH" "$BUNDLED_GOOSED" "$WORKDIR"; then
        return 0
      fi
    fi
    sleep 1
  done

  echo "Timed out waiting for packaged startup diagnostics in $STARTUP_LOG_DIR" >&2
  return 1
}

rm -rf "$SMOKE_ROOT"
mkdir -p "$USER_DATA_DIR" "$GOOSE_PATH_ROOT" "$WORKDIR"

trap cleanup EXIT
ensure_bundle
./scripts/check-security-macos-bundle.sh --arch arm64 --expect local-preview >/dev/null
xattr -cr "$APP_BUNDLE" >/dev/null 2>&1 || true
cleanup

GOOSE_USER_DATA_DIR="$USER_DATA_DIR" \
GOOSE_PATH_ROOT="$GOOSE_PATH_ROOT" \
GOOSE_DISABLE_KEYRING="${GOOSE_DISABLE_KEYRING:-1}" \
  "$APP_BIN" --dir "$WORKDIR" >/dev/null 2>&1 &
APP_PID="$!"

for _ in $(seq 1 10); do
  if pgrep -f "$APP_BIN" >/dev/null; then
    break
  fi
  sleep 1
done

if ! pgrep -f "$APP_BIN" >/dev/null; then
  echo "Packaged app failed to stay running: $APP_BIN" >&2
  exit 1
fi

wait_for_runtime_assets
wait_for_startup_log

echo "packaged_smoke=ok"
echo "bundle=$APP_BUNDLE"
echo "app_bin=$APP_BIN"
echo "goosed=$BUNDLED_GOOSED"
echo "startup_log=$STARTUP_LOG_PATH"
echo "workdir=$WORKDIR"
echo "user_data_dir=$USER_DATA_DIR"
echo "goose_path_root=$GOOSE_PATH_ROOT"
