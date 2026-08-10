#!/usr/bin/env bash
# Build the roaming web client end to end:
#   1. compile goose-roaming-web to wasm (via build-wasm.sh)
#   2. run wasm-bindgen --target web into webapp/src/wasm/
#
# wasm-bindgen CLI must match the wasm-bindgen crate version the cdylib was
# built with (0.2.126). If it's missing, this fetches the prebuilt binary.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
WASM_PKG="goose-roaming-web"
WASM_CRATE_DIR="${HERE}/${WASM_PKG}"
OUT_DIR="${HERE}/webapp/src/wasm"
WB_VERSION="0.2.126"

# 1. compile to wasm
"${HERE}/build-wasm.sh" "${WASM_PKG}" "${WASM_CRATE_DIR}"

WASM_FILE="${WASM_CRATE_DIR}/target/wasm32-unknown-unknown/release/goose_roaming_web.wasm"
[[ -f "${WASM_FILE}" ]] || { echo "error: wasm not found at ${WASM_FILE}" >&2; exit 1; }

# 2. locate (or fetch) a version-matched wasm-bindgen CLI
find_wasm_bindgen() {
  if command -v wasm-bindgen >/dev/null 2>&1 &&
     [[ "$(wasm-bindgen --version | awk '{print $2}')" == "${WB_VERSION}" ]]; then
    command -v wasm-bindgen
    return 0
  fi
  local cached="/tmp/wasm-bindgen-${WB_VERSION}-aarch64-apple-darwin/wasm-bindgen"
  if [[ -x "${cached}" ]]; then echo "${cached}"; return 0; fi
  echo "fetching wasm-bindgen ${WB_VERSION}..." >&2
  local url="https://github.com/rustwasm/wasm-bindgen/releases/download/${WB_VERSION}/wasm-bindgen-${WB_VERSION}-aarch64-apple-darwin.tar.gz"
  curl -sSL -m 120 -o "/tmp/wb-${WB_VERSION}.tar.gz" "${url}"
  tar xzf "/tmp/wb-${WB_VERSION}.tar.gz" -C /tmp
  echo "${cached}"
}

WB="$(find_wasm_bindgen)"
echo "using $("${WB}" --version)"

mkdir -p "${OUT_DIR}"
"${WB}" --target web --out-dir "${OUT_DIR}" --out-name goose_roaming_web "${WASM_FILE}"

# 3. make the output a proper npm package so any web project can consume the
# transport (file: dep today, publishable later). Version tracks the crate.
CRATE_VERSION="$(grep -m1 '^version' "${WASM_CRATE_DIR}/Cargo.toml" | sed 's/.*"\(.*\)"/\1/')"
cat > "${OUT_DIR}/package.json" <<PKG
{
  "name": "@aaif/goose-roam-web",
  "version": "${CRATE_VERSION}",
  "description": "goose roam transport for browsers: iroh compiled to wasm (RoamClient/RoamConnection) with a roam handshake and byte duplex",
  "license": "Apache-2.0",
  "type": "module",
  "main": "goose_roaming_web.js",
  "types": "goose_roaming_web.d.ts",
  "sideEffects": ["./goose_roaming_web.js"],
  "files": [
    "goose_roaming_web.js",
    "goose_roaming_web.d.ts",
    "goose_roaming_web_bg.wasm",
    "goose_roaming_web_bg.wasm.d.ts"
  ]
}
PKG

echo "ok: npm package in ${OUT_DIR}"
ls -la "${OUT_DIR}"
