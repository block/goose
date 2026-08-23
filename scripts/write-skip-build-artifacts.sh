#!/usr/bin/env bash
# Write placeholder desktop artifacts at the contracted versioned names.
# Used by skip_build / validate-only CI so workflows can be tested without
# compiling Rust or Electron. These files must never be published as a real
# download (parent workflows skip the GitHub Release job when skip_build).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLATFORM="${1:?platform is required (mac-arm64|mac-x64|win-x64|goose-mac|goose-win)}"
VERSION="${2:-${RELEASE_VERSION:-}}"

ASSETS_JS="$ROOT_DIR/ui/desktop/scripts/release-assets.js"

write_marker() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  printf 'AVCD_SKIP_BUILD placeholder for %s version=%s\n' "$PLATFORM" "$VERSION" >"$path"
}

case "$PLATFORM" in
  goose-mac)
    TARGET="${3:?rust target is required}"
    write_marker "$ROOT_DIR/artifacts/internal-goose-${TARGET}"
    chmod +x "$ROOT_DIR/artifacts/internal-goose-${TARGET}"
    echo "Wrote $ROOT_DIR/artifacts/internal-goose-${TARGET}"
    ;;
  goose-win)
    write_marker "$ROOT_DIR/target/x86_64-pc-windows-msvc/release/goose.exe"
    echo "Wrote $ROOT_DIR/target/x86_64-pc-windows-msvc/release/goose.exe"
    ;;
  mac-arm64|mac-x64)
    if [ "$PLATFORM" = mac-arm64 ]; then FIELD_PREFIX=macArm64; else FIELD_PREFIX=macX64; fi
    APP_DIR=$(node "$ASSETS_JS" "$VERSION" --field "${FIELD_PREFIX}.appDir")
    UPDATE_ZIP=$(node "$ASSETS_JS" "$VERSION" --field "${FIELD_PREFIX}.update")
    WEBSITE_DMG=$(node "$ASSETS_JS" "$VERSION" --field "${FIELD_PREFIX}.website")
    write_marker "$ROOT_DIR/ui/desktop/out/${APP_DIR}/${UPDATE_ZIP}"
    write_marker "$ROOT_DIR/ui/desktop/out/make/${WEBSITE_DMG}"
    echo "Wrote ${APP_DIR}/${UPDATE_ZIP}"
    echo "Wrote out/make/${WEBSITE_DMG}"
    ;;
  win-x64)
    SETUP_EXE=$(node "$ASSETS_JS" "$VERSION" --field winX64.website)
    PORTABLE_ZIP=$(node "$ASSETS_JS" "$VERSION" --field winX64.portableZip)
    write_marker "$ROOT_DIR/${SETUP_EXE}"
    write_marker "$ROOT_DIR/${PORTABLE_ZIP}"
    echo "Wrote ${SETUP_EXE}"
    echo "Wrote ${PORTABLE_ZIP}"
    ;;
  *)
    echo "Unknown platform: $PLATFORM" >&2
    exit 2
    ;;
esac
