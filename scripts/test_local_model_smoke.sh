#!/usr/bin/env bash
set -euo pipefail

show_usage() {
  echo "Usage: $0 [options]"
  echo ""
  echo "Options:"
  echo "  -n, --top-n NUM          Number of recommended models to test (default: 3)"
  echo "  -m, --models MODELS      Comma-separated download ids. Skips search."
  echo "  -o, --output-dir DIR     Directory for logs (default: ./local-model-smoke-results)"
  echo "      --ram-gb NUM         Override RAM passed to goose lm search"
  echo "      --instruction TEXT   Prompt to send to each model"
  echo "      --repo-prefix TEXT   Forwarded to goose lm search"
  echo "      --repo-suffix TEXT   Forwarded to goose lm search"
  echo "      --quant TEXT         Forwarded to goose lm search"
  echo "      --keep-downloads     Do not delete models after testing"
  echo "  -h, --help               Show this help message"
  echo ""
  echo "Environment:"
  echo "  GOOSE_BIN                Optional goose binary path"
  echo "  SKIP_BUILD               Skip cargo build when set"
}

TOP_N=3
OUTPUT_DIR="./local-model-smoke-results"
MODEL_LIST=""
RAM_GB=""
INSTRUCTION="Say hello in one short sentence. Do not use tools."
REPO_PREFIX=""
REPO_SUFFIX=""
QUANT=""
KEEP_DOWNLOADS=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--top-n)
      TOP_N="$2"
      shift 2
      ;;
    -o|--output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    -m|--models)
      MODEL_LIST="$2"
      shift 2
      ;;
    --ram-gb)
      RAM_GB="$2"
      shift 2
      ;;
    --instruction)
      INSTRUCTION="$2"
      shift 2
      ;;
    --repo-prefix)
      REPO_PREFIX="$2"
      shift 2
      ;;
    --repo-suffix)
      REPO_SUFFIX="$2"
      shift 2
      ;;
    --quant)
      QUANT="$2"
      shift 2
      ;;
    --keep-downloads)
      KEEP_DOWNLOADS=true
      shift
      ;;
    -h|--help)
      show_usage
      exit 0
      ;;
    *)
      echo "Error: Unknown option: $1"
      show_usage
      exit 1
      ;;
  esac
done

if ! [[ "$TOP_N" =~ ^[0-9]+$ ]] || [[ "$TOP_N" -eq 0 ]]; then
  echo "Error: --top-n must be a positive integer"
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required"
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -z "${SKIP_BUILD:-}" && -z "${GOOSE_BIN:-}" ]]; then
  echo "Building goose..."
  (cd "$REPO_ROOT" && cargo build -p goose-cli --features local-inference --bin goose)
  echo ""
fi

GOOSE_BIN="${GOOSE_BIN:-$REPO_ROOT/target/debug/goose}"
if [[ ! -x "$GOOSE_BIN" ]]; then
  echo "Error: goose binary not found or not executable: $GOOSE_BIN"
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

EXISTING_MODELS_FILE="$OUTPUT_DIR/existing-models.txt"
"$GOOSE_BIN" lm list | awk 'NR > 2 && $4 == "✓" { print $1 }' > "$EXISTING_MODELS_FILE"

MODELS=()
if [[ -n "$MODEL_LIST" ]]; then
  IFS=',' read -ra REQUESTED_MODELS <<< "$MODEL_LIST"
  for model in "${REQUESTED_MODELS[@]}"; do
    repo="${model%%:*}"
    variant="${model#*:}"
    if [[ "$variant" = "$model" ]]; then
      variant="manual"
    fi
    MODELS+=("$repo"$'\t'"$model"$'\t'"$model"$'\t'"$variant"$'\t'"0")
  done
else
  SEARCH_LIMIT=$((TOP_N * 20))
  if [[ "$SEARCH_LIMIT" -lt 50 ]]; then
    SEARCH_LIMIT=50
  fi
  SEARCH_ARGS=(lm search --limit "$SEARCH_LIMIT" --json)
  if [[ -n "$RAM_GB" ]]; then
    SEARCH_ARGS+=(--ram-gb "$RAM_GB")
  fi
  if [[ -n "$REPO_PREFIX" ]]; then
    SEARCH_ARGS+=(--repo-prefix "$REPO_PREFIX")
  fi
  if [[ -n "$REPO_SUFFIX" ]]; then
    SEARCH_ARGS+=(--repo-suffix "$REPO_SUFFIX")
  fi
  if [[ -n "$QUANT" ]]; then
    SEARCH_ARGS+=(--quant "$QUANT")
  fi

  SEARCH_JSON="$OUTPUT_DIR/search.json"
  echo "Finding recommended local models..."
  "$GOOSE_BIN" "${SEARCH_ARGS[@]}" > "$SEARCH_JSON"

  while IFS= read -r model_row; do
    MODELS+=("$model_row")
  done < <(
    jq -r --argjson limit "$TOP_N" '
      [.[] | select(.recommended_variant != null)]
      | .[:$limit][]
      | [
          .repo_id,
          .recommended_variant.model_id,
          .recommended_variant.download_id,
          .recommended_variant.label,
          (.recommended_variant.size_bytes | tostring)
        ]
      | @tsv
    ' "$SEARCH_JSON"
  )
fi

if [[ ${#MODELS[@]} -eq 0 ]]; then
  echo "No recommended models found."
  exit 1
fi

RESULTS=()
OVERALL_SUCCESS=true

echo "Testing ${#MODELS[@]} model(s)"
echo ""

for row in "${MODELS[@]}"; do
  IFS=$'\t' read -r repo_id model_id download_id label size_bytes <<< "$row"
  safe_model=$(echo "$model_id" | tr '/:' '__' | tr -cd '[:alnum:]_.-')
  download_log="$OUTPUT_DIR/$safe_model.download.log"
  run_log="$OUTPUT_DIR/$safe_model.run.log"
  delete_log="$OUTPUT_DIR/$safe_model.delete.log"
  size_gb=$(awk "BEGIN { printf \"%.1f\", $size_bytes / 1024 / 1024 / 1024 }")

  echo "=========================================================="
  echo "Model:    $model_id"
  echo "Repo:     $repo_id"
  echo "Variant:  $label (${size_gb}GB)"
  echo "=========================================================="

  downloaded=false
  if "$GOOSE_BIN" lm download "$download_id" 2>&1 | tee "$download_log"; then
    downloaded=true
  else
    echo "Download failed for $model_id"
    RESULTS+=("FAIL $model_id - download failed")
    OVERALL_SUCCESS=false
  fi

  if [[ "$downloaded" = true ]]; then
    set +e
    GOOSE_PROVIDER=local GOOSE_MODEL="$model_id" "$GOOSE_BIN" run --text "$INSTRUCTION" 2>&1 | tee "$run_log"
    run_status=${PIPESTATUS[0]}
    set -e

    if [[ ! -s "$run_log" ]]; then
      echo "Run produced no output for $model_id"
      RESULTS+=("FAIL $model_id - empty output")
      OVERALL_SUCCESS=false
    elif [[ "$run_status" -eq 0 ]]; then
      echo "Run passed for $model_id"
      RESULTS+=("PASS $model_id")
    else
      echo "Run replied but exited with status $run_status for $model_id"
      RESULTS+=("FAIL $model_id - replied but exited $run_status")
      OVERALL_SUCCESS=false
    fi
  fi

  existed_before=false
  if grep -Fxq "$model_id" "$EXISTING_MODELS_FILE"; then
    existed_before=true
  fi

  if [[ "$KEEP_DOWNLOADS" = false && "$downloaded" = true && "$existed_before" = false ]]; then
    if "$GOOSE_BIN" lm delete "$model_id" 2>&1 | tee "$delete_log"; then
      echo "Deleted $model_id"
    else
      echo "Delete failed for $model_id"
      RESULTS+=("FAIL $model_id - delete failed")
      OVERALL_SUCCESS=false
    fi
  elif [[ "$KEEP_DOWNLOADS" = false && "$downloaded" = true ]]; then
    echo "Keeping $model_id because it existed before this run"
  fi

  echo ""
done

echo "=== Test Summary ==="
for result in "${RESULTS[@]}"; do
  echo "$result"
done

if [[ "$OVERALL_SUCCESS" = false ]]; then
  echo ""
  echo "Some local model smoke tests failed."
  exit 1
fi

echo ""
echo "All local model smoke tests passed."
