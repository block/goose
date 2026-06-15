#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit

export_preview_backend_env() {
  while IFS='=' read -r key value; do
    export "$key=$value"
  done < <("$ROOT_DIR/scripts/launch-security-preview-backend.sh")
}

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

node scripts/sync-security-runtime-assets.mjs

export GOOSE_E2E_WORKING_DIR="${GOOSE_E2E_WORKING_DIR:-$ROOT_DIR}"
export GOOSE_USER_DATA_DIR="${GOOSE_USER_DATA_DIR:-$ROOT_DIR/.preview/e2e-user-data}"
export SECURITY_PREVIEW_STATE_DIR="${SECURITY_PREVIEW_STATE_DIR:-$ROOT_DIR/.preview/e2e-backend}"

mkdir -p "$GOOSE_USER_DATA_DIR"
export_preview_backend_env
trap cleanup_preview_backend EXIT

pnpm --dir ui/desktop exec playwright test tests/e2e/security-visual-smoke.spec.ts --reporter=list
