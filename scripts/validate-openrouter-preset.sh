#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CATALOG="${ROOT}/config/avcd-openrouter-models.json"
AVCD_AI_CATALOG="${ROOT}/../avcd-ai/config/avcd-librechat.yaml"
MODE="${1:-offline}"
EXPECTED_PROVIDER="openrouter"
EXPECTED_MODEL="deepseek/deepseek-v4-flash"

pass() {
  printf 'PASS: %s\n' "$1"
}

warn() {
  printf 'WARN: %s\n' "$1" >&2
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

validate_catalog() {
  python3 - "${CATALOG}" "${EXPECTED_PROVIDER}" "${EXPECTED_MODEL}" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
expected_provider = sys.argv[2]
expected_model = sys.argv[3]

if not path.is_file():
    raise SystemExit(f"catalog not found: {path}")

catalog = json.loads(path.read_text())
models = catalog.get("models")
if catalog.get("provider") != expected_provider:
    raise SystemExit(f"provider must be {expected_provider!r}")
if catalog.get("defaultModel") != expected_model:
    raise SystemExit(f"defaultModel must be {expected_model!r}")
if not isinstance(models, list) or len(models) != 13:
    raise SystemExit(f"models must contain exactly 13 entries, found {len(models or [])}")

names = []
for index, model in enumerate(models):
    if not isinstance(model, dict):
        raise SystemExit(f"models[{index}] must be an object")
    for key in ("name", "alias", "subtext"):
        if not isinstance(model.get(key), str) or not model[key].strip():
            raise SystemExit(f"models[{index}].{key} must be a non-empty string")
    names.append(model["name"])

if len(set(names)) != len(names):
    raise SystemExit("model names must be unique")
if names[0] != expected_model:
    raise SystemExit("the default model must be the first catalog entry")
PY
  pass "catalog has 13 unique OpenRouter models and the AVCD default"

  if [[ ! -f "${AVCD_AI_CATALOG}" ]]; then
    warn "avcd-ai catalog not found; skipped sibling catalog comparison"
    return
  fi

  if python3 - "${CATALOG}" "${AVCD_AI_CATALOG}" <<'PY'
import json
import re
import sys
from pathlib import Path

catalog = json.loads(Path(sys.argv[1]).read_text())
expected = [model["name"] for model in catalog["models"]]
lines = Path(sys.argv[2]).read_text().splitlines()

in_default_models = False
actual = []
for line in lines:
    if line == "        default:":
        in_default_models = True
        continue
    if in_default_models and line == "        fetch: false":
        break
    if in_default_models:
        match = re.fullmatch(r"\s{10}- ['\"]([^'\"]+)['\"]", line)
        if match:
            actual.append(match.group(1))

if actual != expected:
    raise SystemExit(
        "avcd-ai model IDs differ from Avocado Work catalog\n"
        f"  avcd-ai: {actual}\n"
        f"  agent:   {expected}"
    )
PY
  then
    pass "catalog model IDs match avcd-ai deploy configuration"
  else
    warn "catalog differs from avcd-ai; update both catalogs deliberately"
  fi
}

validate_desktop_env() {
  local temp_env
  temp_env="$(mktemp)"

  PREPARE_DEV_UI_ENV_FILE="${temp_env}" \
    SERVER_PORT=3000 \
    GOOSE_SERVER__SECRET_KEY=validation-secret \
    "${ROOT}/scripts/prepare-dev-ui-env.sh" >/dev/null \
    || {
      rm -f "${temp_env}"
      fail "desktop environment generation failed"
    }

  if ! python3 - "${temp_env}" "${EXPECTED_PROVIDER}" "${EXPECTED_MODEL}" <<'PY'
import json
import sys
from pathlib import Path

values = {}
for raw_line in Path(sys.argv[1]).read_text().splitlines():
    line = raw_line.strip()
    if not line or line.startswith("#") or "=" not in line:
        continue
    key, value = line.split("=", 1)
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        value = value[1:-1]
    values[key] = value

if values.get("GOOSE_DEFAULT_PROVIDER") != sys.argv[2]:
    raise SystemExit("desktop default provider is missing or incorrect")
if values.get("GOOSE_DEFAULT_MODEL") != sys.argv[3]:
    raise SystemExit("desktop default model is missing or incorrect")

models = json.loads(values.get("GOOSE_PREDEFINED_MODELS", "null"))
if not isinstance(models, list) or len(models) != 13:
    raise SystemExit("desktop predefined model list must contain 13 entries")
if any(model.get("provider") != sys.argv[2] for model in models):
    raise SystemExit("every desktop predefined model must use the openrouter provider")
PY
  then
    rm -f "${temp_env}"
    fail "generated desktop environment does not match the OpenRouter preset"
  fi

  rm -f "${temp_env}"
  pass "desktop environment contains 13 predefined OpenRouter models"
}

validate_compose() {
  (
    cd "${ROOT}"
    docker compose --profile cli -f docker-compose.yml config >/dev/null
  )
  pass "Docker Compose provider configuration is valid"
}

read_openrouter_key() {
  if [[ -n "${OPENROUTER_API_KEY:-}" ]]; then
    printf '%s' "${OPENROUTER_API_KEY}"
    return
  fi

  python3 - "${ROOT}/.env.local" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
if not path.is_file():
    raise SystemExit(0)
for raw_line in path.read_text().splitlines():
    line = raw_line.strip()
    if not line or line.startswith("#") or "=" not in line:
        continue
    key, value = line.split("=", 1)
    if key.strip() == "OPENROUTER_API_KEY":
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
            value = value[1:-1]
        print(value, end="")
        break
PY
}

validate_online() {
  local key info
  key="$(read_openrouter_key)"
  if [[ -z "${key}" ]]; then
    printf 'SKIP: online provider check (OPENROUTER_API_KEY is not set)\n'
    return
  fi

  info="$(
    cd "${ROOT}"
    docker compose --profile cli run --rm \
      -e "OPENROUTER_API_KEY=${key}" \
      cli info -v 2>&1
  )" || {
    printf '%s\n' "${info}" >&2
    fail "goose info failed with the configured OpenRouter key"
  }

  [[ "${info}" == *"${EXPECTED_PROVIDER}"* ]] \
    || fail "goose info did not report provider ${EXPECTED_PROVIDER}"
  [[ "${info}" == *"${EXPECTED_MODEL}"* ]] \
    || fail "goose info did not report model ${EXPECTED_MODEL}"
  pass "goose info reports the AVCD OpenRouter provider and default model"
}

case "${MODE}" in
  catalog)
    validate_catalog
    ;;
  offline)
    validate_catalog
    validate_desktop_env
    validate_compose
    ;;
  online)
    validate_online
    ;;
  all)
    validate_catalog
    validate_desktop_env
    validate_compose
    validate_online
    ;;
  *)
    fail "unknown mode ${MODE}; expected catalog, offline, online, or all"
    ;;
esac
