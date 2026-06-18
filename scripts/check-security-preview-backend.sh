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

read_config_value() {
  local key="$1"
  curl -sk \
    -H "Content-Type: application/json" \
    -H "X-Secret-Key: $GOOSE_SERVER__SECRET_KEY" \
    -d "{\"key\":\"${key}\",\"is_secret\":false}" \
    "https://127.0.0.1:$GOOSE_PORT/config/read" | \
    node -e "const fs=require('node:fs');const raw=fs.readFileSync(0,'utf8');const parsed=JSON.parse(raw);if(parsed == null){process.stdout.write('');}else if(typeof parsed === 'object' && Object.prototype.hasOwnProperty.call(parsed,'masked_value')){process.stdout.write(String(parsed.masked_value));}else{process.stdout.write(String(parsed));}"
}

BACKEND_PROVIDER="$(read_config_value GOOSE_PROVIDER)"
BACKEND_MODEL="$(read_config_value GOOSE_MODEL)"
BACKEND_HOST="$(read_config_value OPENAI_BASE_URL)"
BACKEND_TELEMETRY_ENABLED="$(read_config_value GOOSE_TELEMETRY_ENABLED)"
BACKEND_POSTHOG_HOST="$(read_config_value GOOSE_POSTHOG_API_HOST)"
BACKEND_POSTHOG_PROJECT_KEY="$(read_config_value GOOSE_POSTHOG_PROJECT_API_KEY)"

if [[ "$BACKEND_PROVIDER" != "openai" ]]; then
  echo "Expected GOOSE_PROVIDER=openai, got '$BACKEND_PROVIDER'" >&2
  exit 1
fi

if [[ "$BACKEND_HOST" != "https://tokenhub.tencentmaas.com/plan/v3" ]]; then
  echo "Expected OPENAI_BASE_URL=https://tokenhub.tencentmaas.com/plan/v3, got '$BACKEND_HOST'" >&2
  exit 1
fi

if [[ "$BACKEND_TELEMETRY_ENABLED" != "true" ]]; then
  echo "Expected GOOSE_TELEMETRY_ENABLED=true, got '$BACKEND_TELEMETRY_ENABLED'" >&2
  exit 1
fi

if [[ "$BACKEND_POSTHOG_HOST" != "https://us.i.posthog.com" ]]; then
  echo "Expected GOOSE_POSTHOG_API_HOST=https://us.i.posthog.com, got '$BACKEND_POSTHOG_HOST'" >&2
  exit 1
fi

if [[ -z "$BACKEND_POSTHOG_PROJECT_KEY" ]]; then
  echo "Expected GOOSE_POSTHOG_PROJECT_API_KEY to be set" >&2
  exit 1
fi

echo "backend=ok"
echo "binary=$SECURITY_PREVIEW_BACKEND_BINARY"
echo "port=$GOOSE_PORT"
echo "provider=$BACKEND_PROVIDER"
echo "model=$BACKEND_MODEL"
echo "host=$BACKEND_HOST"
echo "telemetry_enabled=$BACKEND_TELEMETRY_ENABLED"
echo "posthog_host=$BACKEND_POSTHOG_HOST"
echo "posthog_project_key_set=yes"
echo "stdout_log=$SECURITY_PREVIEW_BACKEND_STDOUT_LOG"
echo "stderr_log=$SECURITY_PREVIEW_BACKEND_STDERR_LOG"
