#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit

if [[ -f "$ROOT_DIR/distro/security-cn/config/desktop-env.example" ]]; then
  set -a
  source "$ROOT_DIR/distro/security-cn/config/desktop-env.example"
  set +a
fi

BUNDLE_NAME="${GOOSE_BUNDLE_NAME:-$(node -p "require('./distro/security-cn/branding/product-metadata.json').productName")}"
APP_BUNDLE="${SECURITY_GOOSE_APP_BUNDLE:-$ROOT_DIR/ui/desktop/out/${BUNDLE_NAME}-darwin-arm64/${BUNDLE_NAME}.app}"
APP_BIN="$APP_BUNDLE/Contents/MacOS/$BUNDLE_NAME"
BUNDLED_GOOSED="$APP_BUNDLE/Contents/Resources/bin/goosed"
SMOKE_ROOT="${SECURITY_PACKAGED_SMOKE_ROOT:-$ROOT_DIR/.preview/packaged-smoke}"
USER_DATA_DIR="${GOOSE_USER_DATA_DIR:-$SMOKE_ROOT/user-data}"
GOOSE_PATH_ROOT="${GOOSE_PATH_ROOT:-$SMOKE_ROOT/goose-path-root}"
WORKDIR="${SECURITY_PACKAGED_WORKDIR:-$SMOKE_ROOT/workdir}"
PACKAGED_SECRET="${GOOSE_SERVER__SECRET_KEY:-security-goose-packaged-smoke-secret}"
STARTUP_LOG_DIR="$USER_DATA_DIR/logs/startup"
STARTUP_LOG_PATH=""
MAIN_LOG_PATH="$USER_DATA_DIR/logs/main.log"
APP_PID=""

cleanup() {
  pkill -f "$APP_BIN" >/dev/null 2>&1 || true
  pkill -f "$BUNDLED_GOOSED" >/dev/null 2>&1 || true

  if [[ -n "$APP_PID" ]]; then
    wait "$APP_PID" >/dev/null 2>&1 || true
  fi
}

copy_init_config() {
  if [[ -f "$ROOT_DIR/init-config.yaml" ]]; then
    cp "$ROOT_DIR/init-config.yaml" "$WORKDIR/init-config.yaml"
    return
  fi

  if [[ -f "$ROOT_DIR/distro/security-cn/config/init-config.yaml.example" ]]; then
    cp "$ROOT_DIR/distro/security-cn/config/init-config.yaml.example" "$WORKDIR/init-config.yaml"
  fi
}

search_in_file() {
  local pattern="$1"
  local file_path="$2"

  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$file_path"
    return
  fi

  grep -En "$pattern" "$file_path"
}

ensure_bundle() {
  if [[ "${SECURITY_PACKAGED_SKIP_REBUILD:-0}" == "1" && -x "$APP_BIN" ]]; then
    return
  fi

  pnpm --dir ui/desktop run bundle:default
}

wait_for_runtime_assets() {
  local required_recipes=(
    "$WORKDIR/.goose/recipes/security-vuln-triage.yaml"
    "$WORKDIR/.goose/recipes/alert-investigation.yaml"
    "$WORKDIR/.goose/recipes/web-investigation.yaml"
  )
  local required_skills=(
    "$WORKDIR/.agents/skills/vuln-triage/SKILL.md"
    "$WORKDIR/.agents/skills/alert-triage/SKILL.md"
    "$WORKDIR/.agents/skills/ioc-analysis/SKILL.md"
    "$WORKDIR/.agents/skills/asset-risk-summary/SKILL.md"
    "$WORKDIR/.agents/skills/report-writing/SKILL.md"
    "$WORKDIR/.agents/skills/wooyun-legacy/SKILL.md"
  )

  for _ in $(seq 1 60); do
    local missing=0

    for recipe_path in "${required_recipes[@]}"; do
      if [[ ! -f "$recipe_path" ]]; then
        missing=1
        break
      fi
    done

    if [[ "$missing" == "0" ]]; then
      for skill_path in "${required_skills[@]}"; do
        if [[ ! -f "$skill_path" ]]; then
          missing=1
          break
        fi
      done
    fi

    if [[ "$missing" == "0" ]]; then
      return 0
    fi
    sleep 1
  done

  echo "Timed out waiting for packaged runtime assets in $WORKDIR" >&2
  return 1
}

wait_for_startup_log() {
  for _ in $(seq 1 60); do
    STARTUP_LOG_PATH="$(find "$STARTUP_LOG_DIR" -maxdepth 1 -name 'goosed-startup-*.json' -type f 2>/dev/null | sort | tail -n 1 || true)"
    if [[ -n "$STARTUP_LOG_PATH" ]]; then
      if node -e '
        const fs = require("node:fs");
        const diagnostics = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
        const expectedGoosed = process.argv[2];
        const expectedWorkdir = process.argv[3];
        if (!diagnostics.healthCheckSucceeded) process.exit(1);
        if (diagnostics.goosedPath !== expectedGoosed) process.exit(2);
        if (diagnostics.workingDir !== expectedWorkdir) process.exit(3);
      ' "$STARTUP_LOG_PATH" "$BUNDLED_GOOSED" "$WORKDIR"; then
        return 0
      fi
    fi
    sleep 1
  done

  echo "Timed out waiting for packaged startup diagnostics in $STARTUP_LOG_DIR" >&2
  return 1
}

assert_no_preview_update_noise() {
  local pattern='Setting up auto-updater|STARTUP UPDATE CHECK|GitHubUpdater:|latest-mac\.yml|Falling back to GitHub API|Using GitHub API fallback'

  for _ in $(seq 1 10); do
    if [[ -f "$MAIN_LOG_PATH" ]] && search_in_file "$pattern" "$MAIN_LOG_PATH" >/dev/null; then
      echo "Unexpected release updater activity detected in local-preview main log: $MAIN_LOG_PATH" >&2
      search_in_file "$pattern" "$MAIN_LOG_PATH" >&2 || true
      return 1
    fi
    sleep 1
  done
}

assert_packaged_backend_defaults() {
  NODE_TLS_REJECT_UNAUTHORIZED=0 \
  SECURITY_BACKEND_BASE_URL="$BASE_URL" \
  SECURITY_BACKEND_SECRET="$PACKAGED_SECRET" \
    node <<'NODE'
const baseUrl = process.env.SECURITY_BACKEND_BASE_URL;
const secret = process.env.SECURITY_BACKEND_SECRET;

const headers = {
  "Content-Type": "application/json",
  "X-Secret-Key": secret,
};

async function readConfigValue(key) {
  const response = await fetch(`${baseUrl}/config/read`, {
    method: "POST",
    headers,
    body: JSON.stringify({ key, is_secret: false }),
  });

  if (!response.ok) {
    throw new Error(`Failed to read config ${key}: ${response.status}`);
  }

  const parsed = await response.json();
  if (parsed == null) {
    return "";
  }
  if (typeof parsed === "object" && Object.prototype.hasOwnProperty.call(parsed, "masked_value")) {
    return String(parsed.masked_value ?? "");
  }
  return String(parsed);
}

const provider = await readConfigValue("GOOSE_PROVIDER");
const model = await readConfigValue("GOOSE_MODEL");
const openaiBaseUrl = await readConfigValue("OPENAI_BASE_URL");

if (provider !== "openai") {
  throw new Error(`Expected GOOSE_PROVIDER=openai, got ${provider || "[empty]"}`);
}

if (model !== "deepseek-v4-flash") {
  throw new Error(`Expected GOOSE_MODEL=deepseek-v4-flash, got ${model || "[empty]"}`);
}

if (openaiBaseUrl !== "https://tokenhub.tencentmaas.com/plan/v3") {
  throw new Error(
    `Expected OPENAI_BASE_URL=https://tokenhub.tencentmaas.com/plan/v3, got ${openaiBaseUrl || "[empty]"}`,
  );
}

console.log(`configured_provider=${provider}`);
console.log(`configured_model=${model}`);
console.log(`configured_base_url=${openaiBaseUrl}`);
NODE
}

has_packaged_chat_credentials() {
  if [[ -n "${TP_ENT_API_KEY:-}" || -n "${OPENAI_API_KEY:-}" ]]; then
    return 0
  fi

  if [[ ! -f "$WORKDIR/init-config.yaml" ]]; then
    return 1
  fi

  local configured_key
  configured_key="$(
    sed -n 's/^OPENAI_API_KEY:[[:space:]]*//p' "$WORKDIR/init-config.yaml" | head -n 1 | tr -d '"' | tr -d "'"
  )"

  [[ -n "$configured_key" && "$configured_key" != *'${'* ]]
}

rm -rf "$SMOKE_ROOT"
mkdir -p "$USER_DATA_DIR" "$GOOSE_PATH_ROOT" "$WORKDIR"
copy_init_config

trap cleanup EXIT
ensure_bundle
./scripts/check-security-macos-bundle.sh --arch arm64 --expect local-preview >/dev/null
xattr -cr "$APP_BUNDLE" >/dev/null 2>&1 || true
cleanup

GOOSE_USER_DATA_DIR="$USER_DATA_DIR" \
GOOSE_PATH_ROOT="$GOOSE_PATH_ROOT" \
GOOSE_SERVER__SECRET_KEY="$PACKAGED_SECRET" \
GOOSE_LOCAL_PREVIEW_BUNDLE=1 \
GOOSE_DISABLE_KEYRING="${GOOSE_DISABLE_KEYRING:-1}" \
  "$APP_BIN" --dir "$WORKDIR" >/dev/null 2>&1 &
APP_PID="$!"

for _ in $(seq 1 10); do
  if pgrep -f "$APP_BIN" >/dev/null; then
    break
  fi
  sleep 1
done

if ! pgrep -f "$APP_BIN" >/dev/null; then
  echo "Packaged app failed to stay running: $APP_BIN" >&2
  exit 1
fi

wait_for_runtime_assets
wait_for_startup_log
assert_no_preview_update_noise

BASE_URL="$(node -e 'const fs = require("node:fs"); const diagnostics = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); if (!diagnostics.baseUrl) process.exit(1); process.stdout.write(String(diagnostics.baseUrl));' "$STARTUP_LOG_PATH")"

curl -sk \
  -H "X-Secret-Key: $PACKAGED_SECRET" \
  "$BASE_URL/status" >/dev/null

assert_packaged_backend_defaults

if has_packaged_chat_credentials; then
  PACKAGED_CHAT_OUTPUT="$(
    NODE_TLS_REJECT_UNAUTHORIZED=0 \
    SECURITY_CHAT_BASE_URL="$BASE_URL" \
    SECURITY_CHAT_SECRET="$PACKAGED_SECRET" \
    SECURITY_CHAT_WORKDIR="$WORKDIR" \
      node scripts/check-security-chat-request.mjs
  )"

  printf '%s\n' "$PACKAGED_CHAT_OUTPUT"
  PACKAGED_SESSION_ID="$(printf '%s\n' "$PACKAGED_CHAT_OUTPUT" | awk -F= '/^session_id=/{print $2; exit}')"
  if [[ -z "$PACKAGED_SESSION_ID" ]]; then
    echo "Failed to determine session_id from packaged chat output" >&2
    exit 1
  fi

  NODE_TLS_REJECT_UNAUTHORIZED=0 \
  SECURITY_CHAT_BASE_URL="$BASE_URL" \
  SECURITY_CHAT_SECRET="$PACKAGED_SECRET" \
  SECURITY_APPS_SESSION_ID="$PACKAGED_SESSION_ID" \
    node scripts/check-security-apps-request.mjs

  echo "packaged_chat=ok"
  echo "packaged_apps_request=ok"
else
  echo "packaged_chat=skipped_no_api_key"
  echo "packaged_apps_request=skipped_no_api_key"
fi

node scripts/check-security-apps-runtime.mjs "$GOOSE_PATH_ROOT"
echo "packaged_update_noise=clean"

echo "packaged_smoke=ok"
echo "bundle=$APP_BUNDLE"
echo "app_bin=$APP_BIN"
echo "goosed=$BUNDLED_GOOSED"
echo "startup_log=$STARTUP_LOG_PATH"
echo "main_log=$MAIN_LOG_PATH"
echo "base_url=$BASE_URL"
echo "workdir=$WORKDIR"
echo "user_data_dir=$USER_DATA_DIR"
echo "goose_path_root=$GOOSE_PATH_ROOT"
