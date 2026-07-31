#!/usr/bin/env bash
# Serve the roaming web client for a manual try.
#
# Offline-friendly: reuses the repo's ui/node_modules (has vite) instead of
# `pnpm install`, and assumes the wasm bindings are already built into
# src/wasm/ (run ../build-web.sh if they're missing).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
UI_NM="/Users/micn/Development/goose/ui/node_modules"
PORT="${PORT:-5178}"

if [[ ! -d "${HERE}/src/wasm" ]] || [[ ! -f "${HERE}/src/wasm/goose_roaming_web_bg.wasm" ]]; then
  echo "wasm bindings missing — building them (needs the wasm toolchain)…"
  "${HERE}/../build-web.sh"
fi

if [[ ! -e "${HERE}/node_modules" ]]; then
  if [[ -d "${UI_NM}/vite" ]]; then
    ln -sfn "${UI_NM}" "${HERE}/node_modules"
    echo "using repo ui/node_modules (offline)"
  else
    echo "ui/node_modules has no vite; run 'pnpm install' here when the registry is reachable" >&2
    exit 1
  fi
fi

echo "serving on http://localhost:${PORT}"
exec "${HERE}/node_modules/.bin/vite" --port "${PORT}" --strictPort
