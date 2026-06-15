#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit

cleanup_preview_backend() {
  if [[ -n "${SECURITY_PREVIEW_BACKEND_PID:-}" ]]; then
    kill "$SECURITY_PREVIEW_BACKEND_PID" >/dev/null 2>&1 || true
    wait "$SECURITY_PREVIEW_BACKEND_PID" >/dev/null 2>&1 || true
  fi
}

if [[ -f "$ROOT_DIR/distro/security-cn/config/desktop-env.example" ]]; then
  set -a
  source "$ROOT_DIR/distro/security-cn/config/desktop-env.example"
  set +a
fi

node scripts/sync-security-runtime-assets.mjs >/dev/null

export SECURITY_PREVIEW_STATE_DIR="${SECURITY_PREVIEW_STATE_DIR:-$ROOT_DIR/.preview/backend-check}"

while IFS='=' read -r key value; do
  export "$key=$value"
done < <("$ROOT_DIR/scripts/launch-security-preview-backend.sh")

trap cleanup_preview_backend EXIT

curl -sk \
  -H "X-Secret-Key: $GOOSE_SERVER__SECRET_KEY" \
  "https://127.0.0.1:$GOOSE_PORT/status" >/dev/null

echo "backend=ok"
echo "binary=$SECURITY_PREVIEW_BACKEND_BINARY"
echo "port=$GOOSE_PORT"
echo "stdout_log=$SECURITY_PREVIEW_BACKEND_STDOUT_LOG"
echo "stderr_log=$SECURITY_PREVIEW_BACKEND_STDERR_LOG"
