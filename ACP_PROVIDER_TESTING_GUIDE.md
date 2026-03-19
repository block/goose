# Testing ACP Providers in Goose

Guide for testing the three ACP providers: **Claude ACP**, **Codex ACP**, and **Gemini ACP**.

All three use the same generic `AcpProvider` infrastructure over stdio.

---

## Prerequisites

### Build Goose

```bash
cd ~/Documents/code/goose
source bin/activate-hermit
cargo build -p goose-cli
```

### Install the ACP agent binaries

Use the system npm with the public registry (hermit's npm config points at Block's internal Artifactory):

```bash
# Claude ACP shim (wraps Claude Code)
/opt/homebrew/bin/npm install -g @zed-industries/claude-agent-acp --registry https://registry.npmjs.org/

# Codex ACP shim (wraps Codex CLI)
/opt/homebrew/bin/npm install -g @zed-industries/codex-acp --registry https://registry.npmjs.org/

# Gemini CLI (speaks ACP natively via --acp flag)
/opt/homebrew/bin/npm install -g @google/gemini-cli --registry https://registry.npmjs.org/
```

Verify:

```bash
which claude-agent-acp
which codex-acp
which gemini
```

### Auth

| Provider | Auth |
|---|---|
| **claude-acp** | Run `claude` once to authenticate, or set `ANTHROPIC_API_KEY` |
| **codex-acp** | Set `OPENAI_API_KEY` env var |
| **gemini-acp** | Run `gemini` once to authenticate via browser |

---

## Quick Test

```bash
# Claude ACP
GOOSE_PROVIDER=claude-acp GOOSE_MODEL=default \
  ./target/debug/goose run -t "say hello and nothing else" --no-profile

# Codex ACP
GOOSE_PROVIDER=codex-acp GOOSE_MODEL=gpt-5.2-codex \
  ./target/debug/goose run -t "say hello and nothing else" --no-profile

# Gemini ACP
GOOSE_PROVIDER=gemini-acp GOOSE_MODEL=default \
  ./target/debug/goose run -t "say hello and nothing else" --no-profile
```

## Interactive Session

```bash
GOOSE_PROVIDER=claude-acp GOOSE_MODEL=default ./target/debug/goose session --no-profile
GOOSE_PROVIDER=codex-acp GOOSE_MODEL=gpt-5.2-codex ./target/debug/goose session --no-profile
GOOSE_PROVIDER=gemini-acp GOOSE_MODEL=default ./target/debug/goose session --no-profile
```

---

## Architecture

All three providers share the same `AcpProvider` code path:

```
goose → AcpProvider (stdio) → ACP agent binary → underlying model
```

| Provider | Binary | How it speaks ACP |
|---|---|---|
| `claude-acp` | `claude-agent-acp` | Shim from @zed-industries wrapping Claude Code |
| `codex-acp` | `codex-acp` | Shim from @zed-industries wrapping Codex CLI |
| `gemini-acp` | `gemini --acp` | Native ACP support in Gemini CLI |

### Source files

- `crates/goose/src/acp/provider.rs` — Generic AcpProvider (shared)
- `crates/goose/src/providers/claude_acp.rs` — Claude ACP config
- `crates/goose/src/providers/codex_acp.rs` — Codex ACP config
- `crates/goose/src/providers/gemini_acp.rs` — Gemini ACP config

### Mode mapping

| Goose Mode | Claude ACP | Codex ACP | Gemini ACP |
|---|---|---|---|
| `auto` | `bypassPermissions` | `never` / `danger-full-access` | `yolo` |
| `approve` | `default` | `on-request` / `read-only` | `default` |
| `smart-approve` | `acceptEdits` | `on-request` / `workspace-write` | `auto_edit` |
| `chat` | `plan` | `never` / `read-only` | `plan` |

---

## Troubleshooting

### Binary not found

The ACP providers resolve binaries via `SearchPaths::builder().with_npm()` which includes the npm global bin directory.

### Wrong npm registry

```bash
# Use public registry explicitly
/opt/homebrew/bin/npm install -g <package> --registry https://registry.npmjs.org/
```

### Debug logging

```bash
RUST_LOG=debug GOOSE_PROVIDER=gemini-acp GOOSE_MODEL=default \
  ./target/debug/goose run -t "hello" --no-profile
```
