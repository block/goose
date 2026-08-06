#!/usr/bin/env bash
# Reproducible wasm build recipe for the roaming web client.
#
# Proven working 2026-07-31: the full iroh 1.0.2 browser transport stack
# (iroh + noq/noq-proto/noq-udp QUIC + tokio-websockets + web-sys + ring +
# ed25519-dalek + rustls) compiles to wasm32-unknown-unknown.
#
# The three non-obvious requirements this script encodes:
#
#   1. TOOLCHAIN: the wasm32 std must belong to the SAME rustc that cargo
#      shells out to. On this machine PATH puts Homebrew's rustc (no wasm std)
#      ahead of the rustup shim, so we pin an explicit rustup toolchain that has
#      the target installed and prepend its bin dir to PATH.
#
#   2. C COMPILER: `ring` compiles C (curve25519 etc.). Apple clang cannot emit
#      wasm32; LLVM clang can. Point CC/AR for the wasm target at Homebrew LLVM.
#
#   3. RNG BACKEND: getrandom needs the browser backend, selected with
#      `--cfg getrandom_backend="wasm_js"` in RUSTFLAGS (and the `wasm_js`
#      feature on the getrandom dep).
#
# Prereqs (one-time):
#   rustup toolchain install 1.96.1
#   rustup target add wasm32-unknown-unknown --toolchain 1.96.1
#   brew install llvm            # provides a wasm-capable clang
set -euo pipefail

TOOLCHAIN="${ROAM_WASM_TOOLCHAIN:-1.96.1-aarch64-apple-darwin}"
TCBIN="${HOME}/.rustup/toolchains/${TOOLCHAIN}/bin"
LLVM="${ROAM_WASM_LLVM:-/opt/homebrew/opt/llvm/bin}"

if [[ ! -x "${TCBIN}/cargo" ]]; then
  echo "error: toolchain ${TOOLCHAIN} not found at ${TCBIN}" >&2
  echo "install it: rustup toolchain install ${TOOLCHAIN%%-*} && rustup target add wasm32-unknown-unknown --toolchain ${TOOLCHAIN%%-*}" >&2
  exit 1
fi
if [[ ! -x "${LLVM}/clang" ]]; then
  echo "error: wasm-capable clang not found at ${LLVM}/clang (brew install llvm)" >&2
  exit 1
fi
if ! "${TCBIN}/rustc" --print target-list | grep -q '^wasm32-unknown-unknown$'; then
  echo "error: wasm32-unknown-unknown not available in ${TOOLCHAIN}" >&2
  exit 1
fi

PKG="${1:-goose-roaming-web}"
MANIFEST_DIR="${2:-$(cd "$(dirname "$0")" && pwd)/goose-roaming-web}"

echo "building ${PKG} for wasm32-unknown-unknown with ${TOOLCHAIN}..."
cd "${MANIFEST_DIR}"
PATH="${TCBIN}:${PATH}" \
RUSTC="${TCBIN}/rustc" \
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
CC_wasm32_unknown_unknown="${LLVM}/clang" \
AR_wasm32_unknown_unknown="${LLVM}/llvm-ar" \
  "${TCBIN}/cargo" build --release --target wasm32-unknown-unknown

echo "ok: $(find target/wasm32-unknown-unknown/release -maxdepth 1 -name '*.wasm' | head -1)"
