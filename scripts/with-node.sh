#!/usr/bin/env bash
set -euo pipefail

REQUIRED_MAJOR="${1:?usage: with-node.sh <major> command...}"
shift

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
if [[ -s "$NVM_DIR/nvm.sh" ]]; then
	# shellcheck disable=SC1090
	. "$NVM_DIR/nvm.sh"
	if [[ "$REQUIRED_MAJOR" == "22" ]]; then
		nvm use 22.18.0 --silent 2>/dev/null || nvm use 22 --silent
	elif [[ -f .nvmrc ]]; then
		nvm use --silent
	else
		nvm use "${REQUIRED_MAJOR}" --silent 2>/dev/null || true
	fi
fi

ACTUAL_MAJOR="$(node -p "process.versions.node.split('.')[0]")"
if [[ "$REQUIRED_MAJOR" == "22" ]]; then
	if [[ "$ACTUAL_MAJOR" != "22" ]]; then
		echo "Node 22.x required for this target (found $(node -v)). Run: nvm install 22.18.0 && nvm use 22.18.0" >&2
		exit 1
	fi
elif (( ACTUAL_MAJOR < REQUIRED_MAJOR )); then
	echo "Node ${REQUIRED_MAJOR}+ required for this target (found $(node -v)). Run: nvm install && nvm use" >&2
	exit 1
fi

# nvm use updates node but pnpm/npm from another Node version can stay earlier on PATH.
# Pin PATH to the selected Node so child tools (pnpm, electron-forge) use the same runtime.
NODE_BIN_DIR="$(cd "$(dirname "$(command -v node)")" && pwd)"
export PATH="${NODE_BIN_DIR}:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

if ! command -v pnpm >/dev/null 2>&1; then
	corepack enable >/dev/null 2>&1 || true
	corepack prepare pnpm@10.33.0 --activate >/dev/null 2>&1 || true
fi

if ! command -v pnpm >/dev/null 2>&1; then
	echo "pnpm not found for $(node -v). Run: corepack enable && corepack prepare pnpm@10.33.0 --activate" >&2
	exit 1
fi

exec "$@"
