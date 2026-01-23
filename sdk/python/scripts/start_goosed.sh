#!/usr/bin/env bash
# Script to start goosed server for testing Python SDK
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}"/../../.." && pwd)"

GOOSE_HOST="${GOOSE_HOST:-127.0.0.1}"
GOOSE_PORT="${GOOSE_PORT:-3002}"
GOOSE_SERVER__SECRET_KEY="${GOOSE_SERVER__SECRET_KEY:-test-secret}"

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
)
