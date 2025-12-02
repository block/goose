#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

# Codex Starter Pack: Completions -> Responses Upgrade
# Version: v0.1.0
# Homepage: https://openai.com/codex
#
# Safe, idempotent bootstrapper to migrate a repo from legacy OpenAI Completions
# to the Responses API using Codex CLI (local coding agent). Includes dry-run
# mode, approvals on file writes, branch creation, and summary output.

DEFAULT_MODEL="${DEFAULT_MODEL:-gpt-5}"
MODE_FULL_AUTO="${MODE_FULL_AUTO:-0}"
DRY_RUN="${DRY_RUN:-0}"
REPO_DIR=""
NO_INTERACTIVE="${NO_INTERACTIVE:-0}"
OPEN_PR="${OPEN_PR:-0}"
AUTO_COMMIT="${AUTO_COMMIT:-}"
APPROVAL_POLICY="${APPROVAL_POLICY:-}"
DANGEROUS_MODE="${DANGEROUS_MODE:-0}"
MIGRATE_TO_GPT5="${MIGRATE_TO_GPT5:-1}"
BRANCH_NAME="${BRANCH_NAME:-}"
DANGEROUS_CONFIRM="${DANGEROUS_CONFIRM:-}"

# Resolve the script directory robustly, even when executed via stdin (curl | bash)
# Avoid unbound variable under 'set -u' by using default expansion
_SCRIPT_PATH="${BASH_SOURCE[0]:-}"
if [[ -n "$_SCRIPT_PATH" && -e "$_SCRIPT_PATH" ]]; then
  SCRIPT_DIR="$(cd "$(dirname "$_SCRIPT_PATH")" && pwd)"
else
  # Fallback: use current working directory (no local kit copying when run from stdin)
  SCRIPT_DIR="$(pwd)"
fi

log() { printf "[*] %s\n" "$*"; }
info() { printf "[i] %s\n" "$*"; }
warn() { printf "[!] %s\n" "$*" >&2; }
err() { printf "[-] %s\n" "$*" >&2; }

need() { command -v "$1" >/dev/null 2>&1 || { err "Missing required command: $1"; return 1; }; }

usage() {
  cat <<'USAGE'
Usage: completions-to-responses-upgrade.sh [options]

Options:
  -r, --repo <path>        Path to the target git repository
  -m, --model <name>       Model to use (default: gpt-5)
      --full-auto          Run Codex in full automation (no approvals)
  -n, --dry-run            Plan only; no writes. Emit .diff instead of edits
  -a, --approval <policy>  Approval policy: untrusted|on-failure|on-request|never
      --write              Force write-enabled mode (alias for: --dry-run off)
      --dangerous          Bypass approvals and sandbox (NOT recommended)
      --no-interactive     Do not prompt; require --repo
      --open-pr            Attempt to create a PR via GitHub CLI after edits
      --auto-commit        Automatically stage and commit changes after Codex run
      --no-auto-commit     Do not auto-commit changes (overrides defaults)
      --branch <name>      Branch to create for migration (defaults to migrate/openai-responses-<timestamp>)
  -h, --help               Show this help

Env:
  DEFAULT_MODEL, MODE_FULL_AUTO=1, DRY_RUN=1, NO_INTERACTIVE=1, OPEN_PR=1,
  APPROVAL_POLICY, DANGEROUS_MODE=1, MIGRATE_TO_GPT5=0/1,
  DANGEROUS_CONFIRM=1 (skip interactive confirm when using danger-full-access)
USAGE
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -r|--repo) REPO_DIR="$2"; shift 2 ;;
      -m|--model) DEFAULT_MODEL="$2"; shift 2 ;;
      --full-auto) MODE_FULL_AUTO=1; shift ;;
      -n|--dry-run) DRY_RUN=1; shift ;;
      --write) DRY_RUN=0; shift ;;
      -a|--approval) APPROVAL_POLICY="$2"; shift 2 ;;
      --dangerous) DANGEROUS_MODE=1; shift ;;
      --no-interactive) NO_INTERACTIVE=1; shift ;;
      --open-pr) OPEN_PR=1; shift ;;
      --auto-commit) AUTO_COMMIT=1; shift ;;
      --no-auto-commit) AUTO_COMMIT=0; shift ;;
      --branch) BRANCH_NAME="$2"; shift 2 ;;
      -h|--help) usage; exit 0 ;;
      *) err "Unknown argument: $1"; usage; exit 2 ;;
    esac
  done
}

detect_os() {
  OS="$(uname -s)"; ARCH="$(uname -m)"
  case "$OS" in
    Darwin|Linux) : ;;
    *) err "This script supports macOS/Linux. For Windows, use WSL2."; exit 1 ;;
  esac
  # WSL detection
  if grep -qi microsoft /proc/version 2>/dev/null || [[ -n "${WSL_DISTRO_NAME:-}" ]]; then
    info "Detected WSL environment"
  fi
}

preflight() {
  need curl
  need git
  detect_os
  # Avoid leaking secrets
  set +o xtrace || true
}

ensure_clean_or_confirm() {
  if ! git diff --quiet || ! git diff --cached --quiet; then
    warn "Working tree has uncommitted changes."
    if [[ "$NO_INTERACTIVE" == "1" ]]; then
      err "Refusing to proceed with dirty tree in --no-interactive mode"; exit 1
    fi
    read -r -p "Proceed anyway? [y/N] " ans
    [[ "$ans" =~ ^[Yy]$ ]] || { err "Aborted"; exit 1; }
  fi
}

install_codex() {
  if command -v codex >/dev/null 2>&1; then return; fi
  log "Installing Codex CLI..."
  if command -v brew >/dev/null 2>&1; then
    brew install codex || true
  fi
  if ! command -v codex >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
    npm i -g @openai/codex || true
  fi
  if ! command -v codex >/dev/null 2>&1; then
    # Fallback to GitHub release
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT
    log "Downloading Codex binary (fallback)..."
    # Simplified arch mapping; real release names may differ
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)
    # Normalize common arch names
    case "$arch" in
      x86_64) arch=amd64 ;;
      aarch64|arm64) arch=arm64 ;;
    esac
    url="https://github.com/openai/codex/releases/latest/download/codex-${arch}-${os}.tar.gz"
    curl -fsSL -o "$TMP/codex.tgz" "$url" || { err "Failed to download Codex binary"; exit 1; }
    mkdir -p "$HOME/.local/bin"
    tar -xzf "$TMP/codex.tgz" -C "$HOME/.local/bin" || { err "Failed to extract Codex"; exit 1; }
    chmod +x "$HOME/.local/bin"/codex*
    export PATH="$HOME/.local/bin:$PATH"
  fi
  command -v codex >/dev/null 2>&1 || { err "Could not install Codex CLI"; exit 1; }
}

ensure_auth() {
  if [[ -z "${OPENAI_API_KEY:-}" ]]; then
    info "OPENAI_API_KEY not set; attempting 'codex login'..."
    if ! codex login; then
      warn "Codex login failed or cancelled. If you have an API key, export OPENAI_API_KEY and retry."
    fi
  fi
}

choose_repo() {
  if [[ -n "$REPO_DIR" ]]; then
    [[ -d "$REPO_DIR" ]] || { err "Not a directory: $REPO_DIR"; exit 1; }
    cd "$REPO_DIR"
  else
    if [[ "$NO_INTERACTIVE" == "1" ]]; then
      err "--no-interactive set but --repo not provided"; exit 2
    fi
    read -r -p "[?] Path to the repository to migrate: " REPO_DIR
    [[ -d "$REPO_DIR" ]] || { err "Not a directory: $REPO_DIR"; exit 1; }
    cd "$REPO_DIR"
  fi
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || { err "Not a git repo: $PWD"; exit 1; }
}

ask_model_preference() {
  # Ask whether to migrate models to gpt-5. Default yes.
  MIGRATE_TO_GPT5=1
  if [[ "$NO_INTERACTIVE" != "1" ]]; then
    echo -n "[?] Migrate model references to gpt-5? [Y/n]: "
    read -r ans || true
    if [[ "$ans" =~ ^[Nn]$ ]]; then
      MIGRATE_TO_GPT5=0
    fi
  fi

  if [[ "$MIGRATE_TO_GPT5" == "1" ]]; then
    DEFAULT_MODEL="gpt-5"
    info "Will migrate model references to gpt-5 and enforce temperature not set or = 1."
  else
    info "Will keep existing model references; no global model override."
  fi
  export MIGRATE_TO_GPT5
}

# ZDR preference prompt removed; policy is unified:
# - Always set store=false to reduce overhead and avoid server-side retention
# - Do not rely on previous message IDs or server-stored context
# - Always include encrypted reasoning tokens (no plaintext reasoning)

create_branch() {
  local ts default_branch branch_input branch
  ts=$(date +%Y%m%d-%H%M%S)
  default_branch="migrate/openai-responses-${ts}"
  if [[ -n "$BRANCH_NAME" ]]; then
    branch="$BRANCH_NAME"
  elif [[ "$NO_INTERACTIVE" == "1" ]]; then
    branch="$default_branch"
  else
    if read -r -t 60 -p "[?] Branch to create for migration [$default_branch]: " branch_input; then
      branch=${branch_input:-$default_branch}
    else
      echo
      info "No input received; using default: $default_branch"
      branch="$default_branch"
    fi
  fi
  git checkout -b "$branch"
  echo "$branch"
}

# Prefer rich local kit if this script is part of the starter repo
copy_local_kit_if_available() {
  local tgt="$1"
  if [[ -d "$SCRIPT_DIR/../docs" ]]; then
    if command -v rsync >/dev/null 2>&1; then
      rsync -a --exclude ".git" "$SCRIPT_DIR/../docs/" "$tgt/docs/" 2>/dev/null || true
    else
      cp -R "$SCRIPT_DIR/../docs/." "$tgt/docs/" 2>/dev/null || true
    fi
  fi
  if [[ -d "$SCRIPT_DIR/../prompts" ]]; then
    if command -v rsync >/dev/null 2>&1; then
      rsync -a --exclude ".git" "$SCRIPT_DIR/../prompts/" "$tgt/prompts/" 2>/dev/null || true
    else
      cp -R "$SCRIPT_DIR/../prompts/." "$tgt/prompts/" 2>/dev/null || true
    fi
  fi
}

prep_kit() {
  KIT_DIR=$(mktemp -d)
  mkdir -p "$KIT_DIR/docs" "$KIT_DIR/prompts" "$KIT_DIR/docs/language-recipes"

  # Embedded minimal cheat-sheet (fallback if no local kit present)
  cat > "$KIT_DIR/docs/responses-cheatsheet.md" <<'EOF'
# Responses API Cheat‑Sheet (condensed)
- Endpoint: POST /v1/responses
- Basic request: { "model": "gpt-5", "input": "Hello" }
- Streaming: { ..., "stream": true } (SSE)
- Parameter map: prompt→input, max_tokens→max_output_tokens
- Formatting: response_format (chat) → text.format (responses). Use JSON Schema wrapper with name:
  { text: { format: { type: "json_schema", name: "Output", json_schema: { strict: true, schema: { ... } } } } }
- SDK: client.responses.create(...); use response.output_text when available

Content items:
- Replace Chat content[].type: "text" with Responses content[].type: "input_text" for user/system inputs.
- For assistant/tool outputs, prefer content[].type: "output_text".

Multi‑turn conversations:
- Manage state client‑side; pass prior turns explicitly via `input` items.
- Example items: [{ role: "system"|"user"|"assistant"|"tool", content: [ { type: "text", text: "..." } ] }]
- Prefer concise state; avoid leaking secrets. Always set `store: false` to reduce overhead and avoid server retention.

Tool use:
- Define `tools` for function calling; let the model pick with `tool_choice: "auto"` or force a tool.
- When the model requests a tool call, execute it and include its result as a `tool` role item in the next `input`.
- Keep tool JSON schemas minimal and validated.

Data retention & state:
- Set `store: false` on all Responses requests.
- Do not rely on previous message IDs or server‑stored context; keep state in your app.

Reasoning:
- For GPT‑5, set an appropriate `reasoning.effort`.
- Always include encrypted reasoning tokens (no plaintext chain‑of‑thought). Use the SDK/API option for encrypted reasoning traces where available.

GPT‑5 model notes:
- Ensure `temperature` is not specified or equals `1`.
EOF

  # Embedded minimal migration prompt
  cat > "$KIT_DIR/prompts/migrate-to-responses.md" <<'EOF'
You are migrating this repository from legacy OpenAI Completions/Chat Completions to the unified Responses API using gpt-5.

Objectives:
1) Enumerate all call sites using legacy endpoints/SDKs.
2) Propose a per-language migration plan.
3) Apply safe, minimal edits to switch to Responses API.
4) Run tests/lints; fix trivial issues introduced by the migration.
5) Prepare small, reviewable change sets and provide a final summary (do not commit).

Guardrails:
- Modify only files inside the git workspace.
- Keep behavior identical otherwise; do not refactor unrelated logic.
- Preserve streaming semantics if previously used; otherwise keep non-streaming.
 - Do not maintain backward-compat wrapper shapes; update callers to the Responses output schema.
 - Do not leave tombstone/temporary/transition comments, markers, or backup files; submit clean edits suitable for PR review.
 - Do not run git add/commit/push; leave commits to the bootstrap. Make working-tree edits only.

References:
- Read docs/responses-cheatsheet.md and docs/language-recipes/* if present.

Acceptance:
- All legacy Completions/ChatCompletion calls replaced with Responses equivalents.
- Imports/initialization updated. Builds/tests pass or actionable notes provided.
- If model policy is gpt-5, ensure temperature is not set or equals 1; update code accordingly.
- Replace any top-level response_format usage with text.format in Responses (e.g., text: { format: "json" }).
- Always set `store: false`; do not rely on previous message IDs or server-stored context; keep conversation state in-app.
- Always include encrypted reasoning tokens; do not emit plaintext chain-of-thought.
- Summary includes edited files, counts of updated call sites, and next steps.
EOF

  # If this script is inside the starter pack repo, copy richer docs/prompts
  copy_local_kit_if_available "$KIT_DIR"

  # Ensure AGENTS.md exists/augmented so Codex ingests it from repo root
  if [[ -f AGENTS.md ]]; then
    printf "\n\n---\n# Migration Task\nUse docs in %s; migrate OpenAI completions→responses using model %s.\n" "$KIT_DIR" "$DEFAULT_MODEL" >> AGENTS.md
    printf "\n- Do not preserve backward compatibility wrappers; adopt the Responses output shape across the codebase.\n" >> AGENTS.md
    printf "%s\n" "- Do not leave tombstone comments or backup files in the repo." >> AGENTS.md
    printf "%s\n" "- If migrating to gpt-5, ensure 'temperature' is omitted or set to 1 to avoid errors." >> AGENTS.md
    AGENTS_CREATED_BY_SCRIPT=0
  else
    printf "# AGENTS\n\n## Migration Task\nUse docs in %s; migrate OpenAI completions→responses using model %s.\n" "$KIT_DIR" "$DEFAULT_MODEL" > AGENTS.md
    printf "\n- Do not preserve backward compatibility wrappers; adopt the Responses output shape across the codebase.\n" >> AGENTS.md
    printf "%s\n" "- Do not leave tombstone comments or backup files in the repo." >> AGENTS.md
    printf "%s\n" "- If migrating to gpt-5, ensure 'temperature' is omitted or set to 1 to avoid errors." >> AGENTS.md
    AGENTS_CREATED_BY_SCRIPT=1
  fi

  export CODEX_KIT_DIR="$KIT_DIR"
  export AGENTS_CREATED_BY_SCRIPT
}

detect_tests() {
  # Heuristic commands we will suggest Codex to run (and we may run ourselves in the future)
  local cmds=()
  # JS
  if [[ -f package.json ]]; then
    command -v npm >/dev/null 2>&1 && cmds+=("npm test --silent || true")
    command -v pnpm >/dev/null 2>&1 && cmds+=("pnpm test --silent || true")
    command -v yarn >/dev/null 2>&1 && cmds+=("yarn test --silent || true")
  fi
  # Python
  if [[ -f pyproject.toml || -f setup.cfg || -f setup.py || -f requirements.txt || -d tests ]]; then
    command -v pytest >/dev/null 2>&1 && cmds+=("pytest -q || true")
    command -v python >/dev/null 2>&1 && cmds+=("python -m pytest -q || true")
  fi
  # Go
  if [[ -d go.mod || -f go.mod ]]; then
    command -v go >/dev/null 2>&1 && cmds+=("go test ./... || true")
  fi
  # Java (Maven/Gradle)
  if [[ -f pom.xml ]]; then
    command -v mvn >/dev/null 2>&1 && cmds+=("mvn -q -DskipTests=false test || true")
  fi
  if [[ -f build.gradle || -f build.gradle.kts ]]; then
    command -v gradle >/dev/null 2>&1 && cmds+=("gradle test || true")
  fi
  # .NET
  if compgen -G "*.sln" >/dev/null || compgen -G "*.csproj" >/dev/null; then
    command -v dotnet >/dev/null 2>&1 && cmds+=("dotnet test || true")
  fi
  # Ruby
  if [[ -f Gemfile ]]; then
    command -v bundle >/dev/null 2>&1 && cmds+=("bundle exec rspec || true")
  fi
  # Rust
  if [[ -f Cargo.toml ]]; then
    command -v cargo >/dev/null 2>&1 && cmds+=("cargo test || true")
  fi
  printf '%s\n' "${cmds[@]}"
}

detect_build_cmds() {
  # Heuristic build commands for common JS projects
  local cmds=()
  if [[ -f package.json ]]; then
    if grep -q '"build"' package.json 2>/dev/null; then
      command -v npm >/dev/null 2>&1 && cmds+=("npm run build --silent || true")
      command -v pnpm >/dev/null 2>&1 && cmds+=("pnpm build --silent || true")
      command -v yarn >/dev/null 2>&1 && cmds+=("yarn build --silent || true")
    fi
  fi
  # TypeScript direct compile if present
  if command -v tsc >/dev/null 2>&1 && [[ -f tsconfig.json ]]; then
    cmds+=("tsc -p . || true")
  fi
  printf '%s\n' "${cmds[@]}"
}

detect_sdk_upgrade_hints() {
  # Suggest upgrade commands for OpenAI SDK across ecosystems; agent will pick the right one
  local cmds=()
  if [[ -f package.json ]]; then
    if [[ -f pnpm-lock.yaml ]]; then
      cmds+=("pnpm add openai@latest")
    elif [[ -f yarn.lock ]]; then
      cmds+=("yarn add openai@latest")
    elif [[ -f package-lock.json ]]; then
      cmds+=("npm i -S openai@latest")
    else
      cmds+=("npm i -S openai@latest | yarn add openai@latest | pnpm add openai@latest (choose based on lockfile)")
    fi
  fi
  if [[ -f requirements.txt || -f pyproject.toml || -f setup.cfg || -f setup.py ]]; then
    cmds+=("pip install -U openai || poetry add -U openai || pipenv install --selective-upgrade openai")
  fi
  printf '%s\n' "${cmds[@]}"
}

 

run_codex() {
  command -v codex >/dev/null 2>&1 || { err "Codex CLI not available"; exit 1; }

  local tests
  tests="$(detect_tests | paste -sd ';' - || true)"
  local builds
  builds="$(detect_build_cmds | paste -sd ';' - || true)"
  local upgrades
  upgrades="$(detect_sdk_upgrade_hints | paste -sd ' | ' - || true)"
  local prompt
  local model_policy
  if [[ "$MIGRATE_TO_GPT5" == "1" ]]; then
    model_policy="Model policy: migrate to gpt-5; ensure 'temperature' is not set or set to 1 across all calls."
  else
    model_policy="Model policy: keep existing model references (do not force gpt-5)."
  fi
  local compat_policy="Compatibility policy: do NOT keep legacy response shapes; update callers to work with Responses API outputs."
  local comments_policy="Comment policy: do NOT leave tombstone/transition comments or backup files; produce clean edits for PR."
  local unified_policy="State & retention policy: set store=false on all Responses requests; do not rely on previous message IDs or server-stored context; keep conversation state local and minimal. Reasoning policy: always include encrypted reasoning tokens; do not emit plaintext chain-of-thought."

  if [[ "$DRY_RUN" == "1" ]]; then
    prompt="Read $CODEX_KIT_DIR/docs and $CODEX_KIT_DIR/prompts/migrate-to-responses.md; $model_policy $compat_policy $comments_policy $unified_policy Do not run git add/commit/push; leave commits to the bootstrap. Create a migration plan and a DETAILED unified diff of proposed changes WITHOUT writing to disk; include SDK upgrade guidance (e.g., $upgrades); suggest build commands ($builds) and test commands ($tests); summarize call site counts and impacted files."
  else
    prompt="Read $CODEX_KIT_DIR/docs and $CODEX_KIT_DIR/prompts/migrate-to-responses.md; $model_policy $compat_policy $comments_policy $unified_policy Do not run git add/commit/push; leave commits to the bootstrap. Migrate this repo from Completions to Responses; upgrade the OpenAI SDK to a version supporting Responses (e.g., $upgrades) and reinstall deps; run build if present ($builds); run tests if present ($tests); print a final summary."
  fi

  # Build CLI flags: some are GLOBAL (must precede subcommand), others belong to exec
  local global_flags=()
  local exec_flags=(--model "$DEFAULT_MODEL")
  if [[ "$DRY_RUN" == "1" ]]; then
    exec_flags+=(--sandbox read-only)
  else
    # Enable full access within the workspace
    exec_flags+=(--sandbox danger-full-access)
    # Provide an approval policy appropriate to the mode
    if [[ "$MODE_FULL_AUTO" == "1" ]]; then
      global_flags+=(--full-auto)
    else
      if [[ -n "$APPROVAL_POLICY" ]]; then
        global_flags+=(-a "$APPROVAL_POLICY")
      else
        if [[ "$NO_INTERACTIVE" == "1" ]]; then
          global_flags+=(-a on-failure)
        else
          global_flags+=(-a on-request)
        fi
      fi
    fi
  fi

  # Warn if running with full-access sandbox
  if [[ "$DRY_RUN" != "1" ]]; then
    case " ${exec_flags[*]} " in
      *" --sandbox danger-full-access "*)
        if [[ -z "$DANGEROUS_CONFIRM" ]]; then
          if [[ "$NO_INTERACTIVE" == "1" ]]; then
            err "Sandbox mode is danger-full-access and confirmation is required. Set DANGEROUS_CONFIRM=1 to proceed non-interactively, or rerun interactively to confirm."
            exit 1
          fi
          echo "[!] Codex sandbox is set to danger-full-access. This can modify files and run commands without sandboxing."
          echo "[!] Recommended for best results so Codex can run dependency installs, builds, and tests during migration."
          echo "[!] Ensure you have a recent backup or a clean committed working tree before proceeding."
          read -r -p "Proceed with danger-full-access? [y/N]: " ans
          if [[ ! "$ans" =~ ^[Yy]$ ]]; then
            err "Aborted by user."
            exit 1
          fi
        else
          warn "Proceeding with danger-full-access (DANGEROUS_CONFIRM=1 set). This mode is recommended so Codex can run installs, builds, and tests along the way."
          warn "Ensure backups are in place."
        fi
        ;;
    esac
  fi

  # Inherit environment and request higher reasoning effort
  global_flags+=(-c shell_environment_policy.inherit=all)
  global_flags+=(-c model_reasoning_effort=high)

  log "Launching Codex (model=$DEFAULT_MODEL, dry_run=$DRY_RUN)"
  # shellcheck disable=SC2145
  codex ${global_flags[@]} exec ${exec_flags[@]} -- "$prompt"
}

post_run() {
  local COMMIT_MADE=0

  # Always remove AGENTS.md if we created it, regardless of commit mode
  if [[ "${AGENTS_CREATED_BY_SCRIPT:-0}" == "1" ]]; then
    if [[ -f AGENTS.md ]]; then
      info "Removing AGENTS.md (ephemeral migration guidance)"
      rm -f AGENTS.md || true
    fi
  fi

  # Determine auto-commit default if unset: enable for non-interactive, disable otherwise
  if [[ -z "${AUTO_COMMIT:-}" ]]; then
    if [[ "$NO_INTERACTIVE" == "1" && "$DRY_RUN" != "1" ]]; then
      AUTO_COMMIT=1
    else
      AUTO_COMMIT=0
    fi
  fi

  if [[ "$AUTO_COMMIT" == "1" && "$DRY_RUN" != "1" ]]; then
    info "Auto-commit enabled: staging changes..."
    # Clean common backup artifacts
    find . -type f \( -name "*.bak" -o -name "*.orig" -o -name "*.rej" \) -print -delete 2>/dev/null || true
    if ! git diff --quiet || ! git diff --cached --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
      git add -A || true
      # Use a generic migration message
      if git commit -m "migrate: Completions → Responses via Codex migration pack"; then
        COMMIT_MADE=1
      fi
    fi
  fi

  # Final summary (after any auto-commit)
  log "Run summary:"
  git --no-pager status -sb || true
  if [[ "$COMMIT_MADE" == "1" ]]; then
    git --no-pager log -1 --stat || true
  else
    git --no-pager diff --stat || true
  fi

  if [[ "$OPEN_PR" == "1" ]]; then
    if command -v gh >/dev/null 2>&1; then
      info "Attempting to open a PR via GitHub CLI..."
      gh pr create --fill --title "Migrate to OpenAI Responses API" --body "Automated migration using Codex Starter Pack." || warn "gh pr create failed"
    else
      warn "GitHub CLI (gh) not found; skipping PR creation"
    fi
  fi
}

main() {
  parse_args "$@"
  preflight
  install_codex
  ensure_auth
  choose_repo
  ask_model_preference
  
  ensure_clean_or_confirm
  # If non-interactive and not in dry-run, default to full-auto for writing changes unless overridden
  if [[ "$NO_INTERACTIVE" == "1" && "$DRY_RUN" != "1" && -z "$APPROVAL_POLICY" && "$MODE_FULL_AUTO" != "1" && "$DANGEROUS_MODE" != "1" ]]; then
    MODE_FULL_AUTO=1
  fi
  local branch
  branch=$(create_branch)
  info "Created branch: $branch"
  prep_kit
  run_codex
  post_run
  info "Next steps: review changes, push the branch, and open a PR."
}

main "$@"


