#!/usr/bin/env bash
#
# Smoke test for the MCP discovery URI feature (draft-serra-mcp-discovery-uri).
#
# It stands up a local HTTPS server that publishes a `/.well-known/mcp-server`
# manifest plus an MCP `/mcp` endpoint, starts `goosed`, and drives the real
# discovery pipeline end-to-end over real TLS via
# `POST /config/extensions/discover`, asserting the server is resolved into a
# streamable_http extension config.
#
# Why goosed (and not `goose run --with-mcp-extension`)? The discover endpoint
# exercises the exact same resolver but needs no LLM provider, so the test is
# hermetic and deterministic.
#
# TLS trust: the resolver requires HTTPS and validates certificates. goose's
# reqwest uses rustls-platform-verifier -> rustls-native-certs, which honours
# $SSL_CERT_FILE on Linux, so we point it at a bundle containing our throwaway
# CA. The PR smoke-test job runs on Linux. On macOS the platform verifier reads
# the Keychain and ignores $SSL_CERT_FILE; run on Linux (or in CI) for a green
# result there.
#
# Usage: bash scripts/test_mcp_discovery.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

for tool in openssl python3 curl jq; do
  command -v "$tool" >/dev/null 2>&1 || { echo "FAIL: required tool '$tool' not found"; exit 1; }
done

WORK="$(mktemp -d)"
GOOSE_HOME_DIR="$(mktemp -d)"
declare -a PIDS=()
KEYCHAIN=""           # macOS: throwaway keychain holding the trusted test CA
KEYCHAIN_ORIG=""      # macOS: original user keychain search list to restore
cleanup() {
  for pid in "${PIDS[@]:-}"; do kill "$pid" 2>/dev/null || true; done
  if [ -n "$KEYCHAIN" ]; then
    # shellcheck disable=SC2086
    [ -n "$KEYCHAIN_ORIG" ] && security list-keychains -d user -s $KEYCHAIN_ORIG 2>/dev/null || true
    security delete-keychain "$KEYCHAIN" 2>/dev/null || true
  fi
  rm -rf "$WORK" "$GOOSE_HOME_DIR"
}
trap cleanup EXIT

free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}
MANIFEST_PORT="$(free_port)"
GOOSED_PORT="$(free_port)"
SECRET="smoke-secret-$$"
EXPECTED_ENDPOINT="https://localhost:${MANIFEST_PORT}/mcp"

echo "==> Generating throwaway CA and localhost certificate"
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$WORK/ca.key" -out "$WORK/ca.pem" \
  -subj "/CN=goose-smoke-ca" -days 2 >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "$WORK/leaf.key" -out "$WORK/leaf.csr" \
  -subj "/CN=localhost" >/dev/null 2>&1
printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\n' > "$WORK/ext.cnf"
openssl x509 -req -in "$WORK/leaf.csr" -CA "$WORK/ca.pem" -CAkey "$WORK/ca.key" \
  -CAcreateserial -out "$WORK/leaf.pem" -days 2 -extfile "$WORK/ext.cnf" >/dev/null 2>&1
cat "$WORK/leaf.pem" "$WORK/leaf.key" > "$WORK/leaf.combined.pem"

# Make goosed's HTTP client trust the throwaway CA so the resolver can validate
# the local HTTPS manifest server. The mechanism is platform-specific because
# rustls-platform-verifier reads a different trust source on each OS.
SSL_CERT_FILE_VAL=""
case "$(uname -s)" in
  Linux)
    # rustls-native-certs honours $SSL_CERT_FILE. Bundle our CA with the system
    # roots so goosed trusts localhost without losing the wider web.
    BUNDLE="$WORK/bundle.pem"
    cat "$WORK/ca.pem" > "$BUNDLE"
    for sys in /etc/ssl/certs/ca-certificates.crt /etc/pki/tls/certs/ca-bundle.crt; do
      if [ -f "$sys" ]; then cat "$sys" >> "$BUNDLE"; break; fi
    done
    SSL_CERT_FILE_VAL="$BUNDLE"
    ;;
  Darwin)
    # The macOS verifier reads the Keychain, not $SSL_CERT_FILE. Add the CA to a
    # throwaway keychain on the user search list (restored/deleted on exit).
    KEYCHAIN="$WORK/smoke.keychain-db"
    security create-keychain -p smoke "$KEYCHAIN" >/dev/null 2>&1
    KEYCHAIN_ORIG="$(security list-keychains -d user | sed 's/[" ]//g' | tr '\n' ' ')"
    # shellcheck disable=SC2086
    security list-keychains -d user -s $KEYCHAIN_ORIG "$KEYCHAIN" >/dev/null 2>&1
    security add-trusted-cert -r trustRoot -k "$KEYCHAIN" "$WORK/ca.pem" >/dev/null 2>&1
    ;;
  *)
    echo "WARN: unsupported OS '$(uname -s)'; relying on \$SSL_CERT_FILE"
    SSL_CERT_FILE_VAL="$WORK/ca.pem"
    ;;
esac

echo "==> Starting HTTPS manifest server on port ${MANIFEST_PORT}"
cat > "$WORK/server.py" <<'PY'
import http.server, ssl, json, sys

port = int(sys.argv[1])
manifest = {
    "mcp_version": "2025-06-18",
    "name": "Goose Smoke MCP",
    "endpoint": f"https://localhost:{port}/mcp",
    "transport": "http",
    "description": "smoke test server",
}

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def _send(self, code, body):
        payload = body.encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        if self.path == "/.well-known/mcp-server":
            self._send(200, json.dumps(manifest))
        else:
            self._send(404, "{}")

    def do_POST(self):
        # Minimal MCP initialize response so the direct-handshake fallback would
        # also succeed if the manifest were absent.
        if self.path == "/mcp":
            self._send(200, json.dumps({
                "jsonrpc": "2.0", "id": 1,
                "result": {"protocolVersion": "2025-06-18", "capabilities": {},
                           "serverInfo": {"name": "smoke", "version": "0"}},
            }))
        else:
            self._send(404, "{}")

httpd = http.server.HTTPServer(("127.0.0.1", port), Handler)
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain(sys.argv[2])
httpd.socket = ctx.wrap_socket(httpd.socket, server_side=True)
httpd.serve_forever()
PY
python3 "$WORK/server.py" "$MANIFEST_PORT" "$WORK/leaf.combined.pem" &
PIDS+=("$!")
disown

for _ in $(seq 1 30); do
  curl -sk "https://localhost:${MANIFEST_PORT}/.well-known/mcp-server" >/dev/null 2>&1 && break || sleep 0.3
done

if [ ! -x target/debug/goosed ]; then
  echo "==> Building goosed (first run only)"
  source bin/activate-hermit 2>/dev/null || true
  cargo build -p goose-server --bin goosed
fi

echo "==> Starting goosed on port ${GOOSED_PORT}"
declare -a GOOSED_ENV=(
  "HOME=$GOOSE_HOME_DIR"
  "GOOSE_PORT=$GOOSED_PORT"
  "GOOSE_SERVER__SECRET_KEY=$SECRET"
  "GOOSE_DISABLE_KEYRING=1"
)
[ -n "$SSL_CERT_FILE_VAL" ] && GOOSED_ENV+=("SSL_CERT_FILE=$SSL_CERT_FILE_VAL")
env "${GOOSED_ENV[@]}" target/debug/goosed agent > "$WORK/goosed.log" 2>&1 &
PIDS+=("$!")
disown

# goosed serves HTTPS with a self-signed cert by default, so the test client
# talks to it over https with -k (this is the local control channel, unrelated
# to the discovery TLS validation under test).
GOOSED_URL="https://127.0.0.1:${GOOSED_PORT}"
for _ in $(seq 1 60); do
  curl -sk "${GOOSED_URL}/status" >/dev/null 2>&1 && break || sleep 0.5
done

echo "==> Resolving mcp://localhost:${MANIFEST_PORT} via /config/extensions/discover"
RESPONSE="$(curl -sk -X POST "${GOOSED_URL}/config/extensions/discover" \
  -H "X-Secret-Key: ${SECRET}" \
  -H "Content-Type: application/json" \
  -d "{\"uri\":\"mcp://localhost:${MANIFEST_PORT}\"}" || true)"
echo "    response: ${RESPONSE}"

ENDPOINT="$(printf '%s' "$RESPONSE" | jq -r '.endpoint // empty')"
SOURCE="$(printf '%s' "$RESPONSE" | jq -r '.source // empty')"
CONFIG_URI="$(printf '%s' "$RESPONSE" | jq -r '.config.uri // empty')"
CONFIG_TYPE="$(printf '%s' "$RESPONSE" | jq -r '.config.type // empty')"

if [ "$ENDPOINT" = "$EXPECTED_ENDPOINT" ] \
  && [ "$SOURCE" = "well_known" ] \
  && [ "$CONFIG_URI" = "$EXPECTED_ENDPOINT" ] \
  && [ "$CONFIG_TYPE" = "streamable_http" ]; then
  echo "PASS: discovered ${ENDPOINT} (source=${SOURCE}, type=${CONFIG_TYPE})"
else
  echo "FAIL: expected endpoint=${EXPECTED_ENDPOINT} source=well_known type=streamable_http"
  echo "---- goosed log (tail) ----"
  tail -n 25 "$WORK/goosed.log" 2>/dev/null || true
  exit 1
fi
