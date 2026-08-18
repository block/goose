# AVCD OpenRouter provider preset

Goal: ship the same multi-model OpenRouter experience as **avcd-ai** (LibreChat), with a stable default and a 13-model catalog.

## Canonical files

| File | Role |
|------|------|
| `config/avcd-openrouter-models.json` | Provider, `defaultModel`, ordered `models[]` (13 entries) |
| `.env.local.example` | Template: `GOOSE_PROVIDER`, `GOOSE_MODEL`, `OPENROUTER_HOST`, `OPENROUTER_API_KEY` |
| `.env.local` | Local secrets (gitignored) — **required for chat** |
| `docker-compose.yml` | `env_file: .env.local`; injects provider env on `server` / `cli` |
| `scripts/prepare-dev-ui-env.sh` | Writes `ui/desktop/.env` with defaults + `GOOSE_PREDEFINED_MODELS` |
| `scripts/validate-openrouter-preset.sh` | Offline always; online when key present |
| Sibling `../avcd-ai/config/avcd-librechat.yaml` | Model ID parity check |

## Defaults

| Key | Value |
|-----|-------|
| Provider | `openrouter` |
| Default model | `deepseek/deepseek-v4-flash` (must be first catalog entry) |
| Host | `https://openrouter.ai/api/v1` (via `OPENROUTER_HOST`) |
| Key env (agent) | `OPENROUTER_API_KEY` |
| Key env (avcd-ai) | `OPENROUTER_KEY` — **map when copying** |

## Catalog shape

```json
{
  "provider": "openrouter",
  "defaultModel": "deepseek/deepseek-v4-flash",
  "models": [
    {
      "name": "deepseek/deepseek-v4-flash",
      "alias": "DeepSeek V4 Flash",
      "subtext": "Open-weight agentic coding"
    }
  ]
}
```

Rules enforced by `validate-openrouter-preset.sh`:

- Exactly **13** models
- Unique `name` values
- Default model is index `0`
- Desktop `GOOSE_PREDEFINED_MODELS` entries all use `provider: openrouter`
- Prefer model IDs matching avcd-ai deploy config (warn if they diverge)

## Docker / backend env

Compose should expose (names may use `${VAR:-default}`):

- `GOOSE_PROVIDER=openrouter`
- `GOOSE_MODEL=deepseek/deepseek-v4-flash`
- `OPENROUTER_HOST=…`
- `OPENROUTER_API_KEY` from `.env.local`
- `GOOSE_DISABLE_KEYRING=true` (containers have no user keyring)

After editing `.env.local`, **recreate** the server (`make dev`) so `env_file` is re-read.

Verify inside the container:

```bash
docker compose exec -T server sh -c 'echo KEY=${OPENROUTER_API_KEY:+set}; echo $GOOSE_PROVIDER $GOOSE_MODEL'
docker compose exec -T server goose info -v   # must mention openrouter + default model
docker compose exec -T server goose run -t 'Reply with exactly: OK'
```

Note: plain `goose info` (no `-v`) may omit provider/model — validation uses `-v`.

## Desktop env (generated)

`make dev-ui` → `scripts/prepare-dev-ui-env.sh` writes `ui/desktop/.env`:

```dotenv
GOOSE_EXTERNAL_BACKEND=true
GOOSE_EXTERNAL_BACKEND_URL=http://127.0.0.1:3000
GOOSE_SERVER__SECRET_KEY='…'
GOOSE_DEFAULT_PROVIDER=openrouter
GOOSE_DEFAULT_MODEL=deepseek/deepseek-v4-flash
GOOSE_PREDEFINED_MODELS='[...13 models...]'
```

Do not hand-edit `ui/desktop/.env` for catalog changes — edit `config/avcd-openrouter-models.json` and re-run `make dev-ui`.

## Validation modes

```bash
./scripts/validate-openrouter-preset.sh catalog
./scripts/validate-openrouter-preset.sh offline
./scripts/validate-openrouter-preset.sh online
./scripts/validate-openrouter-preset.sh all   # make validate-openrouter
```

Online step:

- Reads `OPENROUTER_API_KEY` from env or `.env.local`
- If empty → **SKIP** (not fail)
- If set → `docker compose --profile cli run --rm … cli info -v` and assert provider + model strings

## Copying the key from avcd-ai

avcd-ai uses `OPENROUTER_KEY`; agent uses `OPENROUTER_API_KEY`. Copy value only into `.env.local` (never commit). Typical sources: `../avcd-ai/.env` or `../avcd-ai/.env.dev`.

## Symptom → fix

| Symptom | Fix |
|---------|-----|
| UI opens, chat errors / no model | Key empty → set key → `make dev` |
| validate online SKIP | Expected without key |
| Catalog FAIL count ≠ 13 | Edit JSON; keep default first |
| Desktop missing predefined models | Regenerate via `make dev-ui` |
| goose info no provider | Use `goose info -v` |

## Changing the catalog

1. Update `config/avcd-openrouter-models.json`
2. Align avcd-ai LibreChat model list if product wants parity
3. `make validate-openrouter`
4. Recreate backend + desktop env (`make dev` / `make dev-ui`)
5. Update this reference if default model changes

## Provider lockdown (CUSTOM_DISTROS)

Avocado Work locks provider add/remove per [CUSTOM_DISTROS.md](../../../../CUSTOM_DISTROS.md) §A/§B:

| Knob | Value | Effect |
|------|-------|--------|
| `GOOSE_PROVIDER` / `GOOSE_MODEL` | `openrouter` / catalog default | Backend forced provider |
| `GOOSE_PREDEFINED_MODELS` | 13 OpenRouter models (`prepare-dev-ui-env.sh`) | Desktop curated model list; hides Configure providers when set |
| `PROVIDER_MANAGEMENT_ENABLED` | `false` in `ui/desktop/src/updates.ts` | No add/remove/configure/reset provider UI; `/configure-providers` redirects |
| `CONFIGURATION_ENABLED` | `false` in `updates.ts` | Hides raw config editor escape hatch |

Users can still **Switch models** among the predefined OpenRouter catalog.

To restore full provider management: set `PROVIDER_MANAGEMENT_ENABLED = true` (and optionally `CONFIGURATION_ENABLED = true`).

## Consumer extensions lockdown

Regular (non-developer) defaults for small-business admins:

| Knob | Value | Effect |
|------|-------|--------|
| `EXTENSIONS_UI_ENABLED` | `true` in `updates.ts` | Shows Connections (Extensions) page so users can toggle/auth curated MCPs |
| `EXTENSIONS_INSTALL_ENABLED` | `false` in `updates.ts` | Hides Add custom / browse marketplace / deeplink install |
| `GOOGLE_WORKSPACE_ENABLED` | `true` in `updates.ts` | Syncs hosted Google Workspace connector from `bundled-extensions.json` |
| `GIT_WORKTREES_UI_ENABLED` | `false` in `updates.ts` | Hides Git worktrees section in the directory switcher |
| `APPS_UI_ENABLED` | `false` in `updates.ts` | Hides Apps gallery / nav / standalone windows only — **not** inline Autovisualiser charts in chat |
| Bundled defaults | `developer` + `memory` on; Google Workspace streamable_http on | Fresh installs via `syncBundledExtensions` |
| Platform defaults | `todo` + `chatrecall` on; `apps`/`analyze`/`extensionmanager`/`summon`/`tom`/`skills` off+hidden | `crates/goose/src/agents/platform_extensions/mod.rs` |
| `GOOSE_MODE` | `smart_approve` (Rust `GooseMode` default + desktop env) | Ask before sensitive tool calls |
| `GOOSE_OAUTH_CALLBACK_PORT` | `18787` (`prepare-dev-ui-env.sh`) | Stable callback for hosted Workspace OAuth allowlist |

Google connector URI: `https://dev.avocado.tech/google-workspace/mcp`  
Allowlist Goose callback on Workspace MCP: `http://127.0.0.1:18787/oauth_callback` (`google-workspace-mcp` deploy / `.env.example`).

### Google Workspace MCP OAuth (investigation notes)

**Recommended pattern (not legacy `@modelcontextprotocol/server-gdrive`):**

- Extension type: `streamable_http` → hosted MCP at `https://dev.avocado.tech/google-workspace/mcp`
- MCP OAuth discovery: `WWW-Authenticate` → `https://dev.avocado.tech/.well-known/oauth-protected-resource/google-workspace/mcp`
- Auth server: `https://dev.avocado.tech/google-workspace` (authorization + token endpoints)
- Goose stores tokens as `oauth_creds_google-workspace` in config secrets

**Why extensions “failed to load” before fix:**

1. Google Workspace was **enabled by default** in `bundled-extensions.json`
2. New sessions bulk-load extensions with **`StoredCredentialsOnly`** — no browser OAuth on session start
3. Connections toggle only updated config; it did **not** run OAuth until user added the extension in an active chat

**Fix in avcd-agent:**

- Toggling a streamable HTTP extension **on** in Connections runs browser OAuth (`config/extensions/set-enabled`)
- **Sign in** button on Connections cards re-runs OAuth (`config/extensions/authenticate`)
- Google Workspace defaults to **disabled** until the user enables it
- Session load shows a friendly “sign in from Connections” message instead of a hard error when creds are missing

**Local dev checklist:**

- Docker publishes OAuth callback: `18787:18787` + `GOOSE_OAUTH_CALLBACK_PORT=18787`
- `make dev-ui` writes the same port to `ui/desktop/.env`
- Complete sign-in from **Connections** before expecting Drive/Gmail tools in new chats

**Authentication deeplink (shareable):**

Opens Avocado Work and runs the MCP OAuth browser flow for a configured extension. This is **not** a direct Google URL — goose must host the OAuth callback on loopback.

```
avocado-work://extension-authenticate?configKey=google-workspace
```

Alternate form (legacy `goose://` scheme, dev builds):

```
goose://extension?action=authenticate&configKey=google-workspace
```

Force re-authorization: append `&force=true`. Users can copy the link from the Google Workspace card in **Connections** (“Copy sign-in link”).

To restore open Extensions UI: set `EXTENSIONS_UI_ENABLED = true`. To skip Google sync: `GOOGLE_WORKSPACE_ENABLED = false`.
