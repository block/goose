### OpenAI Completions → Responses Migration Pack

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Shell](https://img.shields.io/badge/language-bash-121011.svg)

Developer toolkit to migrate applications from the legacy OpenAI Completions/Chat Completions APIs to the unified Responses API, guided by the Codex CLI (local coding agent).

### What this does
- Detects legacy Completions usage across your repo
- Proposes and applies edits to adopt the Responses API
- Updates imports/initialization and request/response shapes
- Runs tests/lints (when present) and summarizes results
- Creates a clean git branch and optional PR

### Why migrate now
- Responses is the consolidated path forward for text, tools, and streaming. Completions/Chat Completions are legacy. See the official guide: [Migrate to Responses](https://platform.openai.com/docs/guides/migrate-to-responses#page-top).
- GPT‑5 works best on Responses. Migrating unlocks better reasoning and modern features (tool calling, steerability, metaprompting) with lower latency and cost. GPT‑5 orchestrates tools as part of its reasoning; legacy endpoints don’t preserve this structure and can cause repeated tool calls and degraded behavior.

Key Responses benefits vs Chat Completions:
- Stateful conversations: optionally use `store: true` and `previous_response_id` to maintain context without resending the full history.
- Encrypted reasoning: keep apps stateless while benefiting from reasoning items.
- Built‑in tools: add `web_search_preview`, `file_search`, or custom functions directly.
- Flexible inputs: send a single string `input` or an items array; use `instructions` for system‑level guidance.
- Better reasoning and lower costs: improved reasoning quality and cache utilization when compared to Chat Completions.
- Event‑driven model: Responses emits semantic events (e.g., specific text deltas) instead of mutating a single `content` string—simplifying multi‑step logic and improving type‑safety.

### Requirements
- macOS or Linux (Windows via WSL2)
- OpenAI API access (or `codex login` interactive flow)

### Demo Video
https://github.com/user-attachments/assets/6ca30b20-4be5-4ef0-9de6-0907eab2569a



### Quick start
GitHub‑only one‑liner (main):

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/openai/completions-responses-migration-pack/main/scripts/completions-to-responses-upgrade.sh)"
```

Interactive run (prompts for repo and model policy):

```bash
bash scripts/completions-to-responses-upgrade.sh
```

Non‑interactive (write mode by default):

```bash
bash scripts/completions-to-responses-upgrade.sh --repo /path/to/repo --no-interactive
```

Dry‑run (no edits, prints a unified diff plan):

```bash
bash scripts/completions-to-responses-upgrade.sh --repo /path/to/repo --no-interactive --dry-run
```

Full‑auto (no approvals, write‑enabled sandbox):

```bash
bash scripts/completions-to-responses-upgrade.sh --repo /path/to/repo --no-interactive --full-auto
```

Approval policy examples:

```bash
# Ask for approval heuristically
bash scripts/completions-to-responses-upgrade.sh --repo /path/to/repo -a on-request

# Headless CI style (ask only on failure)
bash scripts/completions-to-responses-upgrade.sh --repo /path/to/repo --no-interactive -a on-failure
```

Model selection (prompted interactively; default is GPT‑5):

- If you choose GPT‑5, the tool enforces the runtime rule that `temperature` is omitted or set to `1` to avoid errors.

State & retention policy:

- The migration always sets `store: false` on Responses requests, avoids using previous message IDs or server‑stored context, and keeps conversation state local with minimal logged metadata.

### Flags
- `-r, --repo <path>`: target git repo
- `-m, --model <name>`: model override (default: gpt‑5)
- `--full-auto`: automation shortcut (workspace write + on‑failure approvals)
- `-n, --dry-run`: plan only; do not write
- `-a, --approval <policy>`: `untrusted | on-failure | on-request | never`
- `--write`: alias to disable dry‑run
- `--no-interactive`: no prompts (requires `--repo`)
- `--open-pr`: open PR via GitHub CLI if available
- `--dangerous`: bypass approvals and sandbox (not recommended)

### What gets changed
- Endpoint `/v1/completions` → `/v1/responses`
- `prompt` → `input`; `max_tokens` → `max_output_tokens`
- Node/Python: switch to `OpenAI` client + `client.responses.create(...)`
- Streaming preserved only where originally used
- Tools/function‑calling migrated to `tools` with JSON Schema and `tool_choice`
- Multi‑turn: callers updated to pass prior turns explicitly as `input` items

### How it works
1) Ensures Codex CLI is installed and authenticated
2) Creates branch `migrate/openai-responses-<timestamp>`
3) Drops a small docs pack and sets `AGENTS.md` guidance
4) Runs Codex in automation mode with a structured prompt
5) Shows a final summary; optionally opens a PR

### Troubleshooting
- “unexpected argument” errors: ensure the Codex CLI is up to date
- Sandbox write issues: use `--full-auto` or `-a on-request` interactively
- Diffs but no writes: apply with `codex apply <TASK_ID>` (printed at run end)
- Test runner failures from other ecosystems (Go/Maven/Ruby) are ignored unless they belong to your app; the agent focuses on your repo

### Security & data handling
- No API keys written to disk; honors environment configuration
- Unified policy: `store:false` by default, no previous message IDs, client‑managed state only; always include encrypted reasoning tokens.


### License & version
- MIT. See `LICENSE`
- Current version: see `assets/VERSION`

### Contributing
- See `CONTRIBUTING.md` for guidelines on issues and pull requests

Version‑pinned (recommended for CI):

```bash
TAG=v0.1.0
curl -fsSL -o upgrade.sh \
  https://raw.githubusercontent.com/openai/completions-responses-migration-pack/${TAG}/scripts/completions-to-responses-upgrade.sh
# Verify integrity (replace HASH with the expected value from assets/SHA256SUMS for the same tag)
shasum -a 256 upgrade.sh | grep HASH
bash upgrade.sh
```

Alternate version‑pinned using a specific commit:

```bash
COMMIT=<commit_sha>
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/openai/completions-responses-migration-pack/${COMMIT}/scripts/completions-to-responses-upgrade.sh)"
```

Integrity data:

```bash
# Fetch checksums for a tag
curl -fsSL -o SHA256SUMS \
  https://raw.githubusercontent.com/openai/completions-responses-migration-pack/${TAG}/assets/SHA256SUMS
shasum -a 256 upgrade.sh | grep "$(awk '{print $1}' SHA256SUMS)"
```

Optional GitHub Release asset approach:

```bash
TAG=v0.1.0
curl -fsSL -o upgrade.sh \
  https://github.com/openai/completions-responses-migration-pack/releases/download/${TAG}/completions-to-responses-upgrade.sh
curl -fsSL -o SHA256SUMS \
  https://github.com/openai/completions-responses-migration-pack/releases/download/${TAG}/SHA256SUMS
shasum -a 256 upgrade.sh | grep "$(awk '{print $1}' SHA256SUMS)"
bash upgrade.sh
```

Release steps (maintainers):

```bash
# Update version
sed -i.bak 's/^v.*/v0.1.0/' assets/VERSION && rm -f assets/VERSION.bak

# Commit & tag
git add -A && git commit -m "chore: release v0.1.0" && git tag v0.1.0 && git push && git push --tags

# (Optional) Create GitHub Release with assets
gh release create v0.1.0 \
  scripts/completions-to-responses-upgrade.sh \
  assets/SHA256SUMS \
  --title "v0.1.0" --notes "Initial release"
```


