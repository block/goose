#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BUNDLE_NAME="${GOOSE_BUNDLE_NAME:-$(node -p "require('./distro/security-cn/branding/product-metadata.json').productName")}"

for arch in arm64 x64; do
  APP_BIN="$ROOT_DIR/ui/desktop/out/${BUNDLE_NAME}-darwin-${arch}/${BUNDLE_NAME}.app/Contents/MacOS/${BUNDLE_NAME}"
  BUNDLED_GOOSED="$ROOT_DIR/ui/desktop/out/${BUNDLE_NAME}-darwin-${arch}/${BUNDLE_NAME}.app/Contents/Resources/bin/goosed"

  pkill -f "$APP_BIN" >/dev/null 2>&1 || true
  pkill -f "$BUNDLED_GOOSED" >/dev/null 2>&1 || true
done
