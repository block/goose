#!/usr/bin/env bash
# test.sh — run a managerbot prompt through goose CLI with the whitelabel config
#
# Usage:
#   ./test.sh "how are my sales"
#   ./test.sh -t "increase the price of all my coffee items by \$1"
#   ./test.sh -i          # interactive mode
#   ./test.sh -r          # resume last session
#
# Prerequisites:
#   - managerbot-server running on port 8080
#   - SQUARE_ACCESS_TOKEN set in env (or a running whitelabel app to grab it from)
#   - goose built: target/release/goose

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GOOSE="$REPO_ROOT/target/release/goose"

if [[ ! -x "$GOOSE" ]]; then
  echo "Error: goose not found at $GOOSE — run 'cargo build --release'" >&2
  exit 1
fi

# Add square CLI to PATH
export PATH="$REPO_ROOT/ui/desktop/src/bin:$PATH"

# Try to grab token from running goosed if not already set
if [[ -z "${SQUARE_ACCESS_TOKEN:-}" ]]; then
  GOOSED_PID=$(pgrep -f "goosed agent" 2>/dev/null | head -1 || true)
  if [[ -n "$GOOSED_PID" ]]; then
    TOKEN=$(ps eww "$GOOSED_PID" 2>/dev/null | tr ' ' '\n' | grep SQUARE_ACCESS_TOKEN | cut -d= -f2 || true)
    if [[ -n "$TOKEN" ]]; then
      export SQUARE_ACCESS_TOKEN="$TOKEN"
      echo "Grabbed token from goosed (pid $GOOSED_PID)" >&2
    fi
  fi
fi

if [[ -z "${SQUARE_ACCESS_TOKEN:-}" ]]; then
  echo "Error: SQUARE_ACCESS_TOKEN not set and no running goosed found" >&2
  exit 1
fi

# Build the system prompt (same as buildWhiteLabelSystemPrompt in sessions.ts)
PROMPT_FILE=$(mktemp)
trap 'rm -f "$PROMPT_FILE"' EXIT

python3 -c "
import yaml, os

with open('$SCRIPT_DIR/whitelabel.yaml') as f:
    config = yaml.safe_load(f)

parts = []

sp = config['defaults'].get('systemPrompt', '')
if sp:
    parts.append(sp.strip())

skills = config['defaults'].get('skills', [])
if skills:
    lines = ['# Skills', '']
    for s in skills:
        lines.append(f\"## {s['name']}\")
        lines.append(s['description'])
        resolved = os.path.abspath(os.path.join('$SCRIPT_DIR', s['path']))
        lines.append(f'Skill directory: \`{resolved}\`')
        lines.append('Read the SKILL.md in this directory for detailed instructions before starting this type of work.')
        lines.append('')
    parts.append('\n'.join(lines))

print('\n\n'.join(parts))
" > "$PROMPT_FILE"

export GOOSE_SYSTEM_PROMPT_FILE_PATH="$PROMPT_FILE"

# Parse args
INTERACTIVE=false
RESUME=false
TEXT=""
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -i|--interactive) INTERACTIVE=true; shift ;;
    -r|--resume) RESUME=true; shift ;;
    -t) TEXT="$2"; shift 2 ;;
    -*) EXTRA_ARGS+=("$1"); shift ;;
    *) TEXT="$1"; shift ;;
  esac
done

GOOSE_ARGS=(
  --provider anthropic
  --model claude-opus-4-6
  --no-profile
  --with-builtin developer
  --with-builtin summon
)

if [[ "$RESUME" == true ]]; then
  exec "$GOOSE" session --resume "${GOOSE_ARGS[@]}" "${EXTRA_ARGS[@]}"
elif [[ "$INTERACTIVE" == true ]]; then
  exec "$GOOSE" session "${GOOSE_ARGS[@]}" "${EXTRA_ARGS[@]}"
elif [[ -n "$TEXT" ]]; then
  exec "$GOOSE" run -t "$TEXT" "${GOOSE_ARGS[@]}" "${EXTRA_ARGS[@]}"
else
  echo "Usage: $0 \"prompt\" | -i (interactive) | -r (resume)" >&2
  exit 1
fi
