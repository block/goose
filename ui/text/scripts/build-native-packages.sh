#!/usr/bin/env bash
set -euo pipefail

# Builds the goose-acp-server binary for each target platform and places it
# into the corresponding npm package directory under npm/.
#
# Usage:
#   ./ui/text/scripts/build-native-packages.sh              # build all targets
#   ./ui/text/scripts/build-native-packages.sh darwin-arm64  # build one target
#
# Prerequisites:
#   - Rust cross-compilation toolchains installed for each target
#   - Run from the repository root

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
NPM_DIR="${REPO_ROOT}/npm"

declare -A RUST_TARGETS=(
  ["darwin-arm64"]="aarch64-apple-darwin"
  ["darwin-x64"]="x86_64-apple-darwin"
  ["linux-arm64"]="aarch64-unknown-linux-gnu"
  ["linux-x64"]="x86_64-unknown-linux-gnu"
  ["win32-x64"]="x86_64-pc-windows-msvc"
)

build_target() {
  local platform="$1"
  local rust_target="${RUST_TARGETS[$platform]}"
  local pkg_dir="${NPM_DIR}/goose-acp-server-${platform}"
  local bin_dir="${pkg_dir}/bin"

  echo "==> Building goose-acp-server for ${platform} (${rust_target})"

  cargo build --release --target "${rust_target}" --bin goose-acp-server

  mkdir -p "${bin_dir}"

  local ext=""
  if [[ "$platform" == win32-* ]]; then
    ext=".exe"
  fi

  cp "${REPO_ROOT}/target/${rust_target}/release/goose-acp-server${ext}" "${bin_dir}/goose-acp-server${ext}"
  chmod +x "${bin_dir}/goose-acp-server${ext}"

  echo "    Placed binary at ${bin_dir}/goose-acp-server${ext}"
}

if [ $# -gt 0 ]; then
  # Build specific target(s)
  for target in "$@"; do
    if [[ -z "${RUST_TARGETS[$target]+x}" ]]; then
      echo "Unknown target: ${target}"
      echo "Valid targets: ${!RUST_TARGETS[*]}"
      exit 1
    fi
    build_target "$target"
  done
else
  # Build all targets
  for platform in "${!RUST_TARGETS[@]}"; do
    build_target "$platform"
  done
fi

echo "==> Done. Native packages staged in ${NPM_DIR}/"
