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

for runtime_path in \
  "$ROOT_DIR/.agents/skills/vuln-triage/SKILL.md" \
  "$ROOT_DIR/.agents/skills/alert-triage/SKILL.md" \
  "$ROOT_DIR/.agents/skills/ioc-analysis/SKILL.md" \
  "$ROOT_DIR/.agents/skills/asset-risk-summary/SKILL.md" \
  "$ROOT_DIR/.agents/skills/report-writing/SKILL.md" \
  "$ROOT_DIR/.agents/skills/wooyun-legacy/SKILL.md" \
  "$ROOT_DIR/.goose/recipes/security-vuln-triage.yaml" \
  "$ROOT_DIR/.goose/recipes/alert-investigation.yaml" \
  "$ROOT_DIR/.goose/recipes/web-investigation.yaml"
do
  [[ -f "$runtime_path" ]] || {
    echo "Missing preview runtime asset: $runtime_path" >&2
    exit 1
  }
done

export SECURITY_PREVIEW_STATE_DIR="${SECURITY_PREVIEW_STATE_DIR:-$ROOT_DIR/.preview/backend-chat-check}"
rm -rf "$SECURITY_PREVIEW_STATE_DIR"

while IFS='=' read -r key value; do
  export "$key=$value"
done < <("$ROOT_DIR/scripts/launch-security-preview-backend.sh")

trap cleanup_preview_backend EXIT

curl -sk \
  -H "X-Secret-Key: $GOOSE_SERVER__SECRET_KEY" \
  "https://127.0.0.1:$GOOSE_PORT/status" >/dev/null

CHAT_OUTPUT="$(
  NODE_TLS_REJECT_UNAUTHORIZED=0 \
  SECURITY_CHAT_BASE_URL="https://127.0.0.1:$GOOSE_PORT" \
  SECURITY_CHAT_SECRET="$GOOSE_SERVER__SECRET_KEY" \
  SECURITY_CHAT_WORKDIR="$ROOT_DIR" \
    node scripts/check-security-chat-request.mjs
)"

printf '%s\n' "$CHAT_OUTPUT"

SESSION_ID="$(printf '%s\n' "$CHAT_OUTPUT" | awk -F= '/^session_id=/{print $2; exit}')"
if [[ -z "$SESSION_ID" ]]; then
  echo "Failed to determine session_id from preview chat output" >&2
  exit 1
fi

NODE_TLS_REJECT_UNAUTHORIZED=0 \
SECURITY_CHAT_BASE_URL="https://127.0.0.1:$GOOSE_PORT" \
SECURITY_CHAT_SECRET="$GOOSE_SERVER__SECRET_KEY" \
SECURITY_APPS_SESSION_ID="$SESSION_ID" \
  node scripts/check-security-apps-request.mjs

node scripts/check-security-apps-runtime.mjs "$GOOSE_PATH_ROOT"
