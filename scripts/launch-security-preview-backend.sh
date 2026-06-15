#!/usr/bin/env zsh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit >/dev/null

node ui/desktop/scripts/ensure-goosed-dev.js >/dev/null

if [[ -x "$ROOT_DIR/target/release/goosed" ]]; then
  GOOSED_BINARY="$ROOT_DIR/target/release/goosed"
elif [[ -x "$ROOT_DIR/target/debug/goosed" ]]; then
  GOOSED_BINARY="$ROOT_DIR/target/debug/goosed"
else
  GOOSED_BINARY="$ROOT_DIR/ui/desktop/src/bin/goosed"
fi
PREVIEW_DIR="${SECURITY_PREVIEW_STATE_DIR:-$ROOT_DIR/.preview}"
STDOUT_LOG="${SECURITY_PREVIEW_BACKEND_STDOUT_LOG:-$PREVIEW_DIR/goosed-preview.stdout.log}"
STDERR_LOG="${SECURITY_PREVIEW_BACKEND_STDERR_LOG:-$PREVIEW_DIR/goosed-preview.stderr.log}"

mkdir -p "$PREVIEW_DIR"

PREVIEW_PORT="$(
  node -e "const net=require('node:net');const server=net.createServer();server.listen(0,'127.0.0.1',()=>{const {port}=server.address();console.log(port);server.close();});"
)"
PREVIEW_SECRET="$(node -e "console.log(require('node:crypto').randomBytes(32).toString('hex'))")"

LOG_FORMAT=json \
GOOSE_PORT="$PREVIEW_PORT" \
GOOSE_SERVER__SECRET_KEY="$PREVIEW_SECRET" \
HOME="${HOME}" \
PATH="$ROOT_DIR/ui/desktop/src/bin:${PATH}" \
nohup "$GOOSED_BINARY" agent >"$STDOUT_LOG" 2>"$STDERR_LOG" </dev/null &
PREVIEW_BACKEND_PID=$!

cleanup_backend() {
  kill "$PREVIEW_BACKEND_PID" >/dev/null 2>&1 || true
  wait "$PREVIEW_BACKEND_PID" >/dev/null 2>&1 || true
}

for _ in $(seq 1 100); do
  if curl -sk -H "X-Secret-Key: $PREVIEW_SECRET" "https://127.0.0.1:$PREVIEW_PORT/status" >/dev/null; then
    cat <<EOF
GOOSE_EXTERNAL_BACKEND=1
GOOSE_PORT=$PREVIEW_PORT
GOOSE_SERVER__SECRET_KEY=$PREVIEW_SECRET
SECURITY_PREVIEW_BACKEND_PID=$PREVIEW_BACKEND_PID
SECURITY_PREVIEW_BACKEND_BINARY=$GOOSED_BINARY
SECURITY_PREVIEW_BACKEND_STDOUT_LOG=$STDOUT_LOG
SECURITY_PREVIEW_BACKEND_STDERR_LOG=$STDERR_LOG
EOF
    exit 0
  fi
  sleep 0.2
done

echo "Failed to start repo preview backend on port $PREVIEW_PORT" >&2
echo "stderr log: $STDERR_LOG" >&2
echo "stdout log: $STDOUT_LOG" >&2
cleanup_backend
exit 1
