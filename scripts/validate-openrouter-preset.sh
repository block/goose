#!/usr/bin/env bash
# Validate server-owned model catalog + desktop env defaults (no baked model list).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Prefer FIW sibling, then primary checkout sibling.
if [[ -f "${ROOT}/../avcd-llm/config/models-catalog.json" ]]; then
  CATALOG="${ROOT}/../avcd-llm/config/models-catalog.json"
  LLM_ROOT="${ROOT}/../avcd-llm"
elif [[ -f "${ROOT}/../../avcd/avcd-llm/config/models-catalog.json" ]]; then
  CATALOG="${ROOT}/../../avcd/avcd-llm/config/models-catalog.json"
  LLM_ROOT="${ROOT}/../../avcd/avcd-llm"
else
  CATALOG=""
  LLM_ROOT=""
fi
MODE="${1:-offline}"
EXPECTED_PROVIDER="avocado"
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
  if [[ -z "${CATALOG}" || ! -f "${CATALOG}" ]]; then
    fail "avcd-llm config/models-catalog.json not found next to avcd-agent"
  fi

  local model_count
  model_count="$(
    python3 - "${CATALOG}" "${EXPECTED_PROVIDER}" "${EXPECTED_MODEL}" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
expected_provider = sys.argv[2]
expected_model = sys.argv[3]
catalog = json.loads(path.read_text())
models = catalog.get("models")
if catalog.get("provider") != expected_provider:
    raise SystemExit(f"provider must be {expected_provider!r}")
if catalog.get("defaultModel") != expected_model:
    raise SystemExit(f"defaultModel must be {expected_model!r}")
if not isinstance(models, list) or len(models) < 1:
    raise SystemExit("models must be a non-empty list")
names = []
for index, model in enumerate(models):
    for key in ("name", "alias", "subtext"):
        if not isinstance(model.get(key), str) or not model[key].strip():
            raise SystemExit(f"models[{index}].{key} must be a non-empty string")
    names.append(model["name"])
if len(set(names)) != len(names):
    raise SystemExit("model names must be unique")
if names[0] != expected_model:
    raise SystemExit("the default model must be the first catalog entry")
print(len(models))
PY
  )"
  pass "server catalog has ${model_count} unique models with the AVCD default first"

  if [[ -n "${LLM_ROOT}" && -x "${LLM_ROOT}/scripts/validate-catalog.sh" ]]; then
    (cd "${LLM_ROOT}" && ./scripts/validate-catalog.sh) \
      || fail "avcd-llm catalog/litellm sync validation failed"
    pass "avcd-llm catalog ⊆ litellm.yaml"
  elif [[ -n "${LLM_ROOT}" && -f "${LLM_ROOT}/scripts/validate-catalog.sh" ]]; then
    chmod +x "${LLM_ROOT}/scripts/validate-catalog.sh"
    (cd "${LLM_ROOT}" && ./scripts/validate-catalog.sh) \
      || fail "avcd-llm catalog/litellm sync validation failed"
    pass "avcd-llm catalog ⊆ litellm.yaml"
  else
    warn "avcd-llm validate-catalog.sh not found; skipped litellm sync check"
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
import sys
from pathlib import Path

expected_provider = sys.argv[2]
expected_model = sys.argv[3]
values = {}
for raw_line in Path(sys.argv[1]).read_text().splitlines():
    line = raw_line.strip()
    if not line or line.startswith("#") or "=" not in line:
        continue
    key, value = line.split("=", 1)
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        value = value[1:-1]
    values[key] = value

if values.get("GOOSE_DEFAULT_PROVIDER") != expected_provider:
    raise SystemExit("desktop default provider is missing or incorrect")
if values.get("GOOSE_DEFAULT_MODEL") != expected_model:
    raise SystemExit("desktop default model is missing or incorrect")
if "GOOSE_PREDEFINED_MODELS" in values:
    raise SystemExit("GOOSE_PREDEFINED_MODELS must not be generated (server catalog owns the list)")
PY
  then
    rm -f "${temp_env}"
    fail "generated desktop environment does not match server-driven catalog expectations"
  fi

  rm -f "${temp_env}"
  pass "desktop environment sets defaults without baking GOOSE_PREDEFINED_MODELS"
}

validate_compose() {
  (
    cd "${ROOT}"
    docker compose --profile cli -f docker-compose.yml config >/dev/null
  )
  pass "Docker Compose provider configuration is valid"
}

validate_online() {
  printf 'SKIP: online provider check (model catalog is served by avcd-llm)\n'
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
