#!/usr/bin/env bash
# Start avcd-agent-gateway for local desktop dev when not already listening.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GATEWAY_PORT="${GATEWAY_PORT:-3100}"
GATEWAY_HOST="${GATEWAY_HOST:-127.0.0.1}"
GATEWAY_URL="http://${GATEWAY_HOST}:${GATEWAY_PORT}"
PID_FILE="${ROOT}/.gateway-dev.pid"
LOG_FILE="${ROOT}/.gateway-dev.log"

if [[ -z "${ZITADEL_OAUTH_ENV:-}" ]]; then
  for candidate in \
    "${ROOT}/config/avcd-agent-oauth.env" \
    "${ROOT}/../avcd-zitadel/config/avcd-agent-oauth.env"; do
    if [[ -f "${candidate}" ]]; then
      ZITADEL_OAUTH_ENV="${candidate}"
      break
    fi
  done
fi

if [[ ! -f "${ZITADEL_OAUTH_ENV:-/nonexistent}" ]]; then
  echo "Missing Zitadel oauth env. After terraform apply, run:" >&2
  echo "  cd ../avcd-zitadel && make write-avcd-agent-oauth-env" >&2
  echo "  cp ../avcd-zitadel/config/avcd-agent-oauth.env config/avcd-agent-oauth.env" >&2
  exit 1
fi

GOOSE_BIN="${GOOSE_BIN:-${ROOT}/ui/desktop/src/bin/goose}"
if [[ ! -x "${GOOSE_BIN}" ]]; then
  echo "goose binary not found at ${GOOSE_BIN}. Run: make build-desktop-binary" >&2
  exit 1
fi

if curl -sf "${GATEWAY_URL}/healthz" >/dev/null 2>&1; then
  echo "Gateway already running at ${GATEWAY_URL}"
  exit 0
fi

if [[ -f "${PID_FILE}" ]]; then
  old_pid="$(cat "${PID_FILE}")"
  if kill -0 "${old_pid}" 2>/dev/null; then
    echo "Waiting for gateway (pid ${old_pid})..."
  else
    rm -f "${PID_FILE}"
  fi
fi

AVCD_AGENT_DATA_ROOT="${AVCD_AGENT_DATA_ROOT:-${ROOT}/.local/avcd-agent-data}"
mkdir -p "${AVCD_AGENT_DATA_ROOT}"

set -a
# shellcheck disable=SC1090
source "${ZITADEL_OAUTH_ENV}"
[[ -f "${ROOT}/.env.local" ]] && source "${ROOT}/.env.local"
set +a

export PORT="${GATEWAY_PORT}"
export GATEWAY_HOST="0.0.0.0"
export JWT_REQUIRED="${JWT_REQUIRED:-true}"
export AVCD_AGENT_DATA_ROOT
export GOOSE_BIN
export AVCD_GATEWAY_MAIN=1

cd "${ROOT}/services/avcd-agent-gateway"
if [[ ! -d node_modules ]]; then
  npm ci
fi

nohup npm run dev >"${LOG_FILE}" 2>&1 &
echo $! >"${PID_FILE}"

deadline=$((SECONDS + 45))
until curl -sf "${GATEWAY_URL}/healthz" >/dev/null 2>&1; do
  if ! kill -0 "$(cat "${PID_FILE}")" 2>/dev/null; then
    echo "Gateway exited during startup. Log:" >&2
    tail -40 "${LOG_FILE}" >&2 || true
    exit 1
  fi
  if (( SECONDS > deadline )); then
    echo "Timed out waiting for gateway at ${GATEWAY_URL}. Log:" >&2
    tail -40 "${LOG_FILE}" >&2 || true
    exit 1
  fi
  sleep 0.25
done

echo "Started gateway at ${GATEWAY_URL} (pid $(cat "${PID_FILE}"), log: ${LOG_FILE})"
