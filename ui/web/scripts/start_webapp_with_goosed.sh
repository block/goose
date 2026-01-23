#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
WEB_DIR="${ROOT_DIR}/ui/web"

GOOSE_HOST="${GOOSE_HOST:-127.0.0.1}"
GOOSE_PORT="${GOOSE_PORT:-3000}"
GOOSE_SERVER__SECRET_KEY="${GOOSE_SERVER__SECRET_KEY:-test}"

cleanup() {
  if [[ -n "${GOOSED_PID:-}" ]] && kill -0 "${GOOSED_PID}" 2>/dev/null; then
    kill "${GOOSED_PID}"
    wait "${GOOSED_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

echo "Starting goosed at http://${GOOSE_HOST}:${GOOSE_PORT}"
(
  cd "${ROOT_DIR}"
  GOOSE_HOST="${GOOSE_HOST}" \
  GOOSE_PORT="${GOOSE_PORT}" \
  GOOSE_SERVER__SECRET_KEY="${GOOSE_SERVER__SECRET_KEY}" \
  cargo run -p goose-server --bin goosed -- agent
) &
GOOSED_PID=$!

echo "Starting web app (Vite) in ${WEB_DIR}"
cd "${WEB_DIR}"
npm run dev -- --host 127.0.0.1
