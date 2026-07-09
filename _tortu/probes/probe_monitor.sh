#!/usr/bin/env bash
# _tortu/probes/probe_monitor.sh
#
# Lightweight, fast, append-only monitor -- the "variable elements" subset of
# probe_local_inference.sh, meant to be run frequently (e.g. every 15 min via
# launchd) to build a time-series picture of memory/swap pressure instead of
# a single snapshot. Read-only / non-destructive. Safe to run often: no du,
# no lsof, no heavy scans -- just fast sysctl/vm_stat/ps reads.
#
# Usage:
#   bash _tortu/probes/probe_monitor.sh
#
# Output: appends one CSV row per run to _tortu/probes/reports/monitor_log.csv
# (header written once if the file doesn't exist yet). Intended to be driven
# by the accompanying launchd plist (com.tortu.goose.probemonitor.plist), not
# run manually on a loop -- see _tortu/probes/INSTALL_MONITOR.md for setup.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORT_DIR="$SCRIPT_DIR/reports"
LOG_FILE="$REPORT_DIR/monitor_log.csv"

mkdir -p "$REPORT_DIR"

if [ ! -f "$LOG_FILE" ]; then
  echo "timestamp,free_ram_gb,swap_used_gb,swap_total_gb,swap_pct,mem_free_pct,load_avg_1m,ollama_running,omlx_running,chrome_running,claude_desktop_running,top_mem_proc" > "$LOG_FILE"
fi

TIMESTAMP="$(date +"%Y-%m-%dT%H:%M:%S%z")"

# Free RAM (GB), from vm_stat page counts
PAGE_SIZE=16384
FREE_PAGES="$(vm_stat 2>/dev/null | awk '/Pages free/ {gsub("\\.",""); print $3}')"
if [ -n "${FREE_PAGES:-}" ]; then
  FREE_RAM_GB="$(awk -v p="$FREE_PAGES" -v s="$PAGE_SIZE" 'BEGIN{printf "%.2f", (p*s)/1024/1024/1024}')"
else
  FREE_RAM_GB=""
fi

# Swap usage (GB + percent), from sysctl vm.swapusage
SWAP_LINE="$(sysctl vm.swapusage 2>/dev/null || true)"
SWAP_USED_MB="$(printf '%s' "$SWAP_LINE" | sed -nE 's/.*used = ([0-9.]+)M.*/\1/p')"
SWAP_TOTAL_MB="$(printf '%s' "$SWAP_LINE" | sed -nE 's/.*total = ([0-9.]+)M.*/\1/p')"
if [ -n "${SWAP_USED_MB:-}" ] && [ -n "${SWAP_TOTAL_MB:-}" ]; then
  SWAP_USED_GB="$(awk -v m="$SWAP_USED_MB" 'BEGIN{printf "%.2f", m/1024}')"
  SWAP_TOTAL_GB="$(awk -v m="$SWAP_TOTAL_MB" 'BEGIN{printf "%.2f", m/1024}')"
  SWAP_PCT="$(awk -v u="$SWAP_USED_MB" -v t="$SWAP_TOTAL_MB" 'BEGIN{ if (t>0) printf "%.1f", (u/t)*100; else print "0.0" }')"
else
  SWAP_USED_GB=""
  SWAP_TOTAL_GB=""
  SWAP_PCT=""
fi

# System-wide free memory percentage, from memory_pressure (best-effort parse)
MEM_FREE_PCT="$(memory_pressure 2>/dev/null | sed -nE 's/.*free percentage: ([0-9]+)%.*/\1/p' | head -1)"

# 1-minute load average
LOAD_AVG_1M="$(sysctl -n vm.loadavg 2>/dev/null | awk '{print $2}')"

# Cheap process presence checks (avoid heavy ps parsing, just yes/no)
ollama_running="no"
pgrep -fq "ollama serve|Ollama.app" 2>/dev/null && ollama_running="yes"

omlx_running="no"
pgrep -fq "mlx_lm.server|mlx\\.server|custom_omlx" 2>/dev/null && omlx_running="yes"
# Fallback: check if anything is actually listening on the omlx port
if [ "$omlx_running" = "no" ] && command -v lsof >/dev/null 2>&1; then
  lsof -nP -iTCP:8000 -sTCP:LISTEN >/dev/null 2>&1 && omlx_running="yes"
fi

chrome_running="no"
pgrep -fq "Google Chrome" 2>/dev/null && chrome_running="yes"

claude_running="no"
pgrep -fq "Claude.app" 2>/dev/null && claude_running="yes"

# Single top memory consumer (name only, no huge command-line dump)
TOP_MEM_PROC="$(ps aux 2>/dev/null | awk 'NR>1' | sort -rk4 | head -1 | awk '{print $11}' | xargs -I{} basename {} 2>/dev/null)"
# CSV-safe: strip commas from the process name if any slipped through
TOP_MEM_PROC="${TOP_MEM_PROC//,/}"

echo "${TIMESTAMP},${FREE_RAM_GB},${SWAP_USED_GB},${SWAP_TOTAL_GB},${SWAP_PCT},${MEM_FREE_PCT},${LOAD_AVG_1M},${ollama_running},${omlx_running},${chrome_running},${claude_running},${TOP_MEM_PROC}" >> "$LOG_FILE"
