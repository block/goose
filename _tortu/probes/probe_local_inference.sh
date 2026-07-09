#!/usr/bin/env bash
# _tortu/probes/probe_local_inference.sh
#
# Diagnostic probe for local LLM inference performance on this machine.
# Read-only / non-destructive -- gathers facts, does not change anything or
# offer opinions. Run it, then hand the report to Cowork/Claude Code for
# interpretation and specific recommendations.
#
# Usage:
#   bash _tortu/probes/probe_local_inference.sh [--benchmark]
#
#   --benchmark   Also send one short live completion request to every
#                 OpenAI-compatible endpoint configured under
#                 _tortu/config/custom_providers/*.json, and time it.
#                 Off by default since it touches whatever server is running.
#
# Output: a timestamped report under _tortu/probes/reports/, plus a copy at
# _tortu/probes/reports/latest.txt for convenience. Also prints to stdout.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORT_DIR="$SCRIPT_DIR/reports"
TIMESTAMP="$(date +"%Y%m%d_%H%M%S")"
REPORT_FILE="$REPORT_DIR/probe_${TIMESTAMP}.txt"
FORK_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

mkdir -p "$REPORT_DIR"
: > "$REPORT_FILE"

RUN_BENCHMARK=false
for arg in "$@"; do
  case "$arg" in
    --benchmark) RUN_BENCHMARK=true ;;
  esac
done

log() {
  printf '%s\n' "$1" | tee -a "$REPORT_FILE"
}

section() {
  log ""
  log "=== $1 ==="
}

log "Local inference probe -- $(date)"
log "Host: $(hostname 2>/dev/null || echo unknown)"
log "Report file: $REPORT_FILE"

section "1. Hardware"
CHIP="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)"
log "Chip: $CHIP"

RAM_BYTES="$(sysctl -n hw.memsize 2>/dev/null || echo 0)"
RAM_GB=$(( RAM_BYTES / 1024 / 1024 / 1024 ))
log "Total RAM: ${RAM_GB} GB"

PCORES="$(sysctl -n hw.perflevel0.physicalcpu 2>/dev/null || echo '?')"
ECORES="$(sysctl -n hw.perflevel1.physicalcpu 2>/dev/null || echo '?')"
log "CPU cores: performance=${PCORES} efficiency=${ECORES}"

if command -v system_profiler >/dev/null 2>&1; then
  GPU_INFO="$(system_profiler SPDisplaysDataType 2>/dev/null | grep -E 'Chipset Model|Total Number of Cores' | head -6)"
  if [ -n "$GPU_INFO" ]; then
    log "GPU:"
    log "$GPU_INFO"
  fi
fi

MACOS_VER="$(sw_vers -productVersion 2>/dev/null || echo '?')"
log "macOS version: $MACOS_VER"

log ""
log "Note: memory bandwidth is not queryable via sysctl on Apple Silicon."
log "Cross-reference Total RAM above against known ceilings to infer the chip variant:"
log "  base M4:    max 32GB RAM,  ~120 GB/s bandwidth"
log "  M4 Pro:     max 64GB RAM,  ~273 GB/s bandwidth"
log "  M4 Max:     max 128GB RAM, ~410-546 GB/s bandwidth (Studio only, not Mini)"

section "2. Memory pressure and swap"
if command -v vm_stat >/dev/null 2>&1; then
  log "$(vm_stat)"
fi
SWAP="$(sysctl vm.swapusage 2>/dev/null || echo '?')"
log "Swap: $SWAP"
if command -v memory_pressure >/dev/null 2>&1; then
  log "Memory pressure snapshot:"
  log "$(memory_pressure 2>&1 | head -6)"
fi

section "3. Inference-related processes"
PROC_MATCH="$(ps aux | grep -iE 'ollama|llama[-_.]server|llama\.cpp|mlx_lm|mlx\.server|lmstudio|vllm|text-generation' | grep -v grep || true)"
if [ -n "$PROC_MATCH" ]; then
  log "$PROC_MATCH"
else
  log "No known inference server processes found running."
fi

section "4. Listening ports commonly used by local inference servers"
for port in 8000 8080 11434 1234; do
  if command -v lsof >/dev/null 2>&1; then
    LISTEN="$(lsof -nP -iTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)"
    if [ -n "$LISTEN" ]; then
      log "Port $port:"
      log "$LISTEN"
    fi
  fi
done

section "5. Shell and environment"
log "\$SHELL (default login shell): ${SHELL:-unset}"
if command -v dscl >/dev/null 2>&1; then
  RECORDED_SHELL="$(dscl . -read "/Users/$(whoami)" UserShell 2>/dev/null | awk '{print $2}')"
  log "macOS account record (dscl UserShell): ${RECORDED_SHELL:-?}"
fi
PARENT_SHELL="$(ps -p "${PPID:-0}" -o comm= 2>/dev/null | xargs -I{} basename {} 2>/dev/null || echo '?')"
log "Shell that launched this script: $PARENT_SHELL"
log "(This script itself always runs under bash regardless of your login shell -- it's invoked"
log " as 'bash probe_local_inference.sh', so zsh vs bash doesn't affect the probe running. It DOES"
log " matter for whether exports like GOOSE_RECIPE_PATH actually reach goose day to day -- zsh"
log " reads ~/.zshrc / ~/.zprofile / ~/.zshenv, NOT ~/.bashrc or ~/.bash_profile. An export added"
log " to the wrong file is invisible to a zsh session even though the file exists and is correct.)"

log ""
log "Shell profile files (existence, size, last modified):"
for profile in "$HOME/.zshrc" "$HOME/.zprofile" "$HOME/.zshenv" "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.profile"; do
  if [ -f "$profile" ]; then
    SIZE="$(wc -c < "$profile" 2>/dev/null | tr -d ' ')"
    MTIME="$(stat -f "%Sm" "$profile" 2>/dev/null || echo '?')"
    log "  $profile  (${SIZE} bytes, modified: ${MTIME})"
  fi
done

log ""
log "Searching profile files for GOOSE_RECIPE_PATH / goose PATH entries:"
FOUND_ANY=false
for profile in "$HOME/.zshrc" "$HOME/.zprofile" "$HOME/.zshenv" "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.profile"; do
  if [ -f "$profile" ]; then
    MATCH="$(grep -nE 'GOOSE_RECIPE_PATH|\.local/bin' "$profile" 2>/dev/null || true)"
    if [ -n "$MATCH" ]; then
      FOUND_ANY=true
      log "  $profile:"
      log "$MATCH"
    fi
  fi
done
if [ "$FOUND_ANY" = false ]; then
  log "  Not found in any checked profile file. If GOOSE_RECIPE_PATH is supposed to be set,"
  log "  it hasn't been added anywhere this probe checks -- see bootstrap.sh's printed suggestion."
fi

log ""
log "Current environment as seen by THIS run (reflects whatever shell invoked bash):"
log "  GOOSE_RECIPE_PATH=${GOOSE_RECIPE_PATH:-<not set>}"
PATH_HAS_LOCAL_BIN="no"
case ":$PATH:" in
  *":$HOME/.local/bin:"*) PATH_HAS_LOCAL_BIN="yes" ;;
esac
log "  PATH includes ~/.local/bin: $PATH_HAS_LOCAL_BIN"

section "6. Goose config (live vs. this fork's copy)"
LIVE_CFG="$HOME/.config/goose/config.yaml"
FORK_CFG="$FORK_ROOT/config/config.yaml"

if [ -f "$LIVE_CFG" ]; then
  log "Live config: $LIVE_CFG"
  log "active_provider: $(grep '^active_provider' "$LIVE_CFG" 2>/dev/null || echo '?')"
  log ""
  log "Provider block (raw excerpt, model/enabled lines):"
  log "$(grep -E '^  [a-z_]+:|^\s+(enabled|model):' "$LIVE_CFG" 2>/dev/null || echo '(providers: section not found in expected shape)')"
else
  log "No live config found at $LIVE_CFG"
fi

if [ -f "$LIVE_CFG" ] && [ -f "$FORK_CFG" ]; then
  if diff -q "$LIVE_CFG" "$FORK_CFG" >/dev/null 2>&1; then
    log ""
    log "Live config matches _tortu/config/config.yaml -- no drift since last bootstrap."
  else
    log ""
    log "NOTE: live config differs from _tortu/config/config.yaml -- possible drift since last bootstrap.sh run."
  fi
fi

section "7. Model files on disk (largest first, likely quantization in filename)"
for dir in "$HOME/.ollama/models" "$HOME/.cache/huggingface/hub" "$HOME/.cache/lm-studio/models" "$HOME/.cache/mlx_lm"; do
  if [ -d "$dir" ]; then
    log "$dir:"
    log "$(du -sh "$dir"/* 2>/dev/null | sort -rh | head -15)"
    log ""
  fi
done
log "Reading tip: bf16/fp16 in a filename means unquantized/full precision (slow, bandwidth-heavy)."
log "Q4/Q5/Q8 or 4bit/8bit in a filename means quantized (faster, usually fine quality at these sizes)."

section "8. Background load (top memory consumers over 1%)"
log "$(ps aux | awk 'NR==1 || $4+0 > 1.0' | sort -rk4 | head -15)"

log ""
log "Sync/indexing agents that compete for the same unified memory pool:"
SYNC_MATCH="$(ps aux | grep -iE 'bird|onedrive|dropbox|CloudDocs|mdworker' | grep -v grep || true)"
if [ -n "$SYNC_MATCH" ]; then
  log "$SYNC_MATCH"
else
  log "None found running."
fi

section "9. Power state"
if command -v pmset >/dev/null 2>&1; then
  log "$(pmset -g batt 2>/dev/null || echo '?')"
  THERM="$(pmset -g therm 2>/dev/null || true)"
  if [ -n "$THERM" ]; then
    log "$THERM"
  else
    log "Thermal state not exposed via pmset on this system."
  fi
fi

if [ "$RUN_BENCHMARK" = true ]; then
  section "10. Live benchmark (configured OpenAI-compatible endpoints)"
  if ! command -v python3 >/dev/null 2>&1; then
    log "python3 not found, skipping benchmark."
  else
    PROVIDERS_DIR="$FORK_ROOT/config/custom_providers"
    if [ -d "$PROVIDERS_DIR" ] && [ -n "$(ls -A "$PROVIDERS_DIR"/*.json 2>/dev/null)" ]; then
      for provider_json in "$PROVIDERS_DIR"/*.json; do
        log "Testing provider defined in: $(basename "$provider_json")"
        python3 - "$provider_json" >> "$REPORT_FILE" 2>&1 <<'PYEOF'
import json, sys, time, urllib.request, urllib.error

path = sys.argv[1]
with open(path) as f:
    cfg = json.load(f)

name = cfg.get("name", "?")
base_url = (cfg.get("base_url") or "").rstrip("/")
models = cfg.get("models") or []
model = models[0].get("name", "?") if models else "?"

if not base_url:
    print(f"  {name}: no base_url configured, skipping")
    sys.exit(0)

url = f"{base_url}/chat/completions"
payload = json.dumps({
    "model": model,
    "messages": [{"role": "user", "content": "Reply with exactly the word: ready"}],
    "max_tokens": 16,
    "stream": False,
}).encode()

req = urllib.request.Request(
    url, data=payload, headers={"Content-Type": "application/json"}, method="POST"
)

print(f"  provider={name} model={model} url={url}")
try:
    start = time.time()
    with urllib.request.urlopen(req, timeout=60) as resp:
        body = json.loads(resp.read())
    elapsed = time.time() - start
    usage = body.get("usage", {})
    completion_tokens = usage.get("completion_tokens")
    print(f"  response time: {elapsed:.2f}s")
    print(f"  completion tokens: {completion_tokens}")
    if isinstance(completion_tokens, int) and completion_tokens > 0 and elapsed > 0:
        print(f"  approx tokens/sec: {completion_tokens / elapsed:.2f}")
except urllib.error.URLError as e:
    print(f"  could not reach {url}: {e}")
except Exception as e:
    print(f"  benchmark failed: {e}")
PYEOF
      done
    else
      log "No custom provider configs found under $PROVIDERS_DIR, skipping."
    fi
  fi
else
  section "10. Live benchmark"
  log "Skipped (run with --benchmark to include a live timed completion test)."
fi

section "Done"
log "Full report: $REPORT_FILE"
cp "$REPORT_FILE" "$REPORT_DIR/latest.txt"
log "Copied to: $REPORT_DIR/latest.txt"
