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
PREVIEW_ROOT="${SECURITY_PACKAGED_PREVIEW_ROOT:-$ROOT_DIR/.preview/packaged-preview}"
USER_DATA_DIR="${GOOSE_USER_DATA_DIR:-$PREVIEW_ROOT/user-data}"
GOOSE_PATH_ROOT="${GOOSE_PATH_ROOT:-$PREVIEW_ROOT/goose-path-root}"
WORKDIR="${SECURITY_PACKAGED_WORKDIR:-$PREVIEW_ROOT/workdir}"
PACKAGED_SECRET="${GOOSE_SERVER__SECRET_KEY:-security-goose-packaged-preview-secret}"
STARTUP_LOG_DIR="$USER_DATA_DIR/logs/startup"
STARTUP_LOG_PATH=""

ensure_bundle() {
  if [[ "${SECURITY_PACKAGED_SKIP_REBUILD:-0}" == "1" && -x "$APP_BIN" ]]; then
    return
  fi

  pnpm --dir ui/desktop run bundle:default
}

wait_for_startup_log() {
  for _ in $(seq 1 60); do
    STARTUP_LOG_PATH="$(find "$STARTUP_LOG_DIR" -maxdepth 1 -name 'goosed-startup-*.json' -type f 2>/dev/null | sort | tail -n 1 || true)"
    if [[ -n "$STARTUP_LOG_PATH" ]]; then
      if node -e '
        const fs = require("node:fs");
        const diagnostics = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
        const expectedWorkdir = process.argv[2];
        if (!diagnostics.healthCheckSucceeded) process.exit(1);
        if (diagnostics.workingDir !== expectedWorkdir) process.exit(2);
      ' "$STARTUP_LOG_PATH" "$WORKDIR"; then
        return 0
      fi
    fi
    sleep 1
  done

  echo "Timed out waiting for packaged preview startup diagnostics in $STARTUP_LOG_DIR" >&2
  return 1
}

mkdir -p "$USER_DATA_DIR" "$GOOSE_PATH_ROOT" "$WORKDIR"

if [[ -f "$ROOT_DIR/init-config.yaml" ]]; then
  cp "$ROOT_DIR/init-config.yaml" "$WORKDIR/init-config.yaml"
fi

ensure_bundle
./scripts/check-security-macos-bundle.sh --arch arm64 --expect local-preview >/dev/null
./scripts/stop-security-packaged-app.sh
xattr -cr "$APP_BUNDLE" >/dev/null 2>&1 || true

GOOSE_USER_DATA_DIR="$USER_DATA_DIR" \
GOOSE_PATH_ROOT="$GOOSE_PATH_ROOT" \
GOOSE_SERVER__SECRET_KEY="$PACKAGED_SECRET" \
GOOSE_LOCAL_PREVIEW_BUNDLE=1 \
GOOSE_DISABLE_KEYRING="${GOOSE_DISABLE_KEYRING:-1}" \
  "$APP_BIN" --dir "$WORKDIR" >/dev/null 2>&1 &

wait_for_startup_log

echo "packaged_preview=started"
echo "bundle=$APP_BUNDLE"
echo "app_bin=$APP_BIN"
echo "workdir=$WORKDIR"
echo "user_data_dir=$USER_DATA_DIR"
echo "goose_path_root=$GOOSE_PATH_ROOT"
echo "startup_log=$STARTUP_LOG_PATH"
