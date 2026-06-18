#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit

stop_repo_preview_processes() {
  local electron_path="$ROOT_DIR/ui/node_modules/electron/dist/Electron.app/Contents/MacOS/Electron"
  local staged_goosed_path="$ROOT_DIR/ui/desktop/src/bin/goosed"
  local release_goosed_path="$ROOT_DIR/target/release/goosed"
  local debug_goosed_path="$ROOT_DIR/target/debug/goosed"

  pkill -f "$electron_path" 2>/dev/null || true
  pkill -f "$staged_goosed_path" 2>/dev/null || true
  pkill -f "$release_goosed_path" 2>/dev/null || true
  pkill -f "$debug_goosed_path" 2>/dev/null || true
}

schedule_repo_preview_focus() {
  SECURITY_PREVIEW_FOCUS_ATTEMPTS="${SECURITY_PREVIEW_FOCUS_ATTEMPTS:-20}" \
    SECURITY_PREVIEW_FOCUS_DELAY_SECONDS="${SECURITY_PREVIEW_FOCUS_DELAY_SECONDS:-1}" \
    "$ROOT_DIR/scripts/focus-security-preview-window.sh" >/dev/null 2>&1 &
}

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

export GOOSE_USER_DATA_DIR="${GOOSE_USER_DATA_DIR:-$ROOT_DIR/.preview/user-data}"
export GOOSE_PREVIEW_WORKING_DIR="${GOOSE_PREVIEW_WORKING_DIR:-$ROOT_DIR}"

mkdir -p "$GOOSE_USER_DATA_DIR"
stop_repo_preview_processes
export_preview_backend_env
trap cleanup_preview_backend EXIT
schedule_repo_preview_focus

pnpm --dir ui/desktop run start-gui -- --dir "$ROOT_DIR"
