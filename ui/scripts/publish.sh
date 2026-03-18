#!/usr/bin/env bash
set -euo pipefail

# Publishes @goose-ai/acp, @goose-ai/cli, and all native binary packages to npm.
#
# Usage:
#   ./ui/scripts/publish.sh         # publish all (dry-run)
#   ./ui/scripts/publish.sh --real   # publish for real
#
# Prerequisites:
#   - pnpm login to the @block scope
#   - Native binaries built via build-native-packages.sh

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NPM_DIR="${REPO_ROOT}/npm"
ACP_DIR="${REPO_ROOT}/ui/acp"
TEXT_DIR="${REPO_ROOT}/ui/text"

DRY_RUN="--dry-run"
if [[ "${1:-}" == "--real" ]]; then
  DRY_RUN=""
  echo "==> Publishing for real"
else
  echo "==> Dry run (pass --real to publish)"
fi

# Build and publish @goose-ai/acp first (dependency of @goose-ai/cli)
echo "==> Building @goose-ai/acp"
(cd "${ACP_DIR}" && pnpm run build)

echo "==> Publishing @goose-ai/acp"
(cd "${ACP_DIR}" && pnpm publish --access public ${DRY_RUN})

# Build @goose-ai/cli
echo "==> Building @goose-ai/cli"
(cd "${TEXT_DIR}" && pnpm run build)

NATIVE_PACKAGES=(
  "goose-acp-server-darwin-arm64"
  "goose-acp-server-darwin-x64"
  "goose-acp-server-linux-arm64"
  "goose-acp-server-linux-x64"
  "goose-acp-server-win32-x64"
)

# Publish native binary packages
for pkg in "${NATIVE_PACKAGES[@]}"; do
  pkg_dir="${NPM_DIR}/${pkg}"

  if [ ! -f "${pkg_dir}/bin/goose-acp-server" ] && [ ! -f "${pkg_dir}/bin/goose-acp-server.exe" ]; then
    echo "    SKIP ${pkg} (no binary found — run build-native-packages.sh first)"
    continue
  fi

  echo "==> Publishing @goose-ai/${pkg}"
  (cd "${pkg_dir}" && pnpm publish --access public ${DRY_RUN})
done

# Publish the main package
echo "==> Publishing @goose-ai/cli"
(cd "${TEXT_DIR}" && pnpm publish --access public ${DRY_RUN})

echo "==> Done"
