#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit >/dev/null

ARCH="arm64"
EXPECT_MODE=""
REQUIRE_NOTARIZED="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch)
      ARCH="$2"
      shift 2
      ;;
    --expect)
      EXPECT_MODE="$2"
      shift 2
      ;;
    --require-notarized)
      REQUIRE_NOTARIZED="1"
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

BUNDLE_NAME="${GOOSE_BUNDLE_NAME:-$(node -p "require('./distro/security-cn/branding/product-metadata.json').productName")}"
BUNDLE_ID="$(node -p "require('./distro/security-cn/branding/product-metadata.json').bundleId")"
APP_DIR="$ROOT_DIR/ui/desktop/out/${BUNDLE_NAME}-darwin-${ARCH}"
APP_BUNDLE="$APP_DIR/${BUNDLE_NAME}.app"
PLIST_PATH="$APP_BUNDLE/Contents/Info.plist"
GOOSED_PATH="$APP_BUNDLE/Contents/Resources/bin/goosed"
DISTRO_PATH="$APP_BUNDLE/Contents/Resources/security-cn"
ICON_PATH="$APP_BUNDLE/Contents/Resources/electron.icns"

if [[ "$ARCH" == "x64" ]]; then
  ZIP_PATH="$APP_DIR/${BUNDLE_NAME}_intel_mac.zip"
else
  ZIP_PATH="$APP_DIR/${BUNDLE_NAME}.zip"
fi

[[ -d "$APP_BUNDLE" ]] || { echo "Missing app bundle: $APP_BUNDLE" >&2; exit 1; }
[[ -f "$PLIST_PATH" ]] || { echo "Missing Info.plist: $PLIST_PATH" >&2; exit 1; }
[[ -x "$GOOSED_PATH" ]] || { echo "Missing packaged goosed: $GOOSED_PATH" >&2; exit 1; }
[[ -d "$DISTRO_PATH" ]] || { echo "Missing packaged security-cn directory: $DISTRO_PATH" >&2; exit 1; }
[[ -f "$ICON_PATH" ]] || { echo "Missing packaged icon: $ICON_PATH" >&2; exit 1; }
[[ -f "$ZIP_PATH" ]] || { echo "Missing packaged zip: $ZIP_PATH" >&2; exit 1; }

PLIST_JSON="$(plutil -convert json -o - "$PLIST_PATH")"
PLIST_JSON="$PLIST_JSON" node - "$BUNDLE_NAME" "$BUNDLE_ID" "$EXPECT_MODE" <<'NODE'
const data = JSON.parse(process.env.PLIST_JSON);
const bundleName = process.argv[2];
const bundleId = process.argv[3];
const expectMode = process.argv[4];

function fail(message) {
  console.error(message);
  process.exit(1);
}

if (data.CFBundleName !== bundleName) {
  fail(`Unexpected CFBundleName: ${data.CFBundleName}`);
}
if (data.CFBundleDisplayName !== bundleName) {
  fail(`Unexpected CFBundleDisplayName: ${data.CFBundleDisplayName}`);
}
if (data.CFBundleIdentifier !== bundleId) {
  fail(`Unexpected CFBundleIdentifier: ${data.CFBundleIdentifier}`);
}

const actualMode = data.SecurityGooseSigningMode;
if (expectMode && actualMode !== expectMode) {
  fail(`Unexpected SecurityGooseSigningMode: ${actualMode}`);
}

const keyringDisabled = Boolean(data.SecurityGooseDisableKeyringByDefault);
const cookieEncryptionEnabled = Boolean(data.SecurityGooseEnableCookieEncryption);
const lsEnv = data.LSEnvironment || {};

if (actualMode === 'local-preview') {
  if (!keyringDisabled) {
    fail('Expected local-preview bundle to disable keyring by default');
  }
  if (cookieEncryptionEnabled) {
    fail('Expected local-preview bundle to disable cookie encryption by default');
  }
  if (lsEnv.GOOSE_DISABLE_KEYRING !== '1') {
    fail('Expected local-preview bundle LSEnvironment.GOOSE_DISABLE_KEYRING=1');
  }
}

if (actualMode === 'signed') {
  if (keyringDisabled) {
    fail('Expected signed bundle to keep keyring enabled by default');
  }
  if (!cookieEncryptionEnabled) {
    fail('Expected signed bundle to keep cookie encryption enabled by default');
  }
}
NODE

/usr/bin/codesign --verify --deep --strict --verbose=4 "$APP_BUNDLE"

CODESIGN_OUTPUT="$(
  /usr/bin/codesign -dv --verbose=4 "$APP_BUNDLE" 2>&1 || true
)"

if [[ "$EXPECT_MODE" == "local-preview" ]] && ! grep -q 'TeamIdentifier=not set' <<<"$CODESIGN_OUTPUT"; then
  echo "Expected local-preview bundle to be ad-hoc signed" >&2
  exit 1
fi

if [[ "$EXPECT_MODE" == "signed" ]] && grep -q 'TeamIdentifier=not set' <<<"$CODESIGN_OUTPUT"; then
  echo "Expected signed bundle to have a TeamIdentifier" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/security-goose-bundle-check.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

ditto -x -k "$ZIP_PATH" "$TMP_DIR"
[[ -d "$TMP_DIR/${BUNDLE_NAME}.app" ]] || {
  echo "Unzipped app bundle missing from $ZIP_PATH" >&2
  exit 1
}

SPCTL_OUTPUT="$(
  spctl -a -vv "$APP_BUNDLE" 2>&1 || true
)"
STAPLER_OUTPUT="not_checked"

if [[ "$EXPECT_MODE" == "signed" || "$REQUIRE_NOTARIZED" == "1" ]]; then
  STAPLER_OUTPUT="$(
    xcrun stapler validate "$APP_BUNDLE" 2>&1 || true
  )"
fi

if [[ "$REQUIRE_NOTARIZED" == "1" ]]; then
  if ! grep -qi 'accepted' <<<"$SPCTL_OUTPUT"; then
    echo "Expected notarized bundle to be accepted by spctl" >&2
    exit 1
  fi

  if ! grep -qi 'The validate action worked' <<<"$STAPLER_OUTPUT"; then
    echo "Expected notarized bundle to pass stapler validate" >&2
    exit 1
  fi
fi

echo "bundle_check=ok"
echo "arch=$ARCH"
echo "bundle=$APP_BUNDLE"
echo "zip=$ZIP_PATH"
echo "expect_mode=${EXPECT_MODE:-unspecified}"
echo "codesign_team=$(grep 'TeamIdentifier=' <<<"$CODESIGN_OUTPUT" | tail -n 1 | cut -d= -f2-)"
echo "spctl=$(tr '\n' ' ' <<<"$SPCTL_OUTPUT" | sed 's/[[:space:]]\\+/ /g')"
echo "stapler=$(tr '\n' ' ' <<<"$STAPLER_OUTPUT" | sed 's/[[:space:]]\\+/ /g')"
