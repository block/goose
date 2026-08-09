---
name: avcd-agent-custom-distro
description: >-
  Use when rebranding Avocado Work (goose fork), maintaining the goose custom distribution /
  white-label fork, aligning desktop packaging and updater names, configuring
  the OpenRouter provider preset, or bringing local Docker + Electron online.
  Covers official CUSTOM_DISTROS branding steps and AVCD-specific file map,
  Makefile targets, and smoke checks.
---

# Avocado Work — Custom Distribution & Rebrand

> **Related**: Upstream [CUSTOM_DISTROS.md](../../../../CUSTOM_DISTROS.md) (Appendix D) | Published docs [Custom Distributions](https://block-goose.mintlify.app/advanced/custom-distributions) | [mcp-oauth-connections](../mcp-oauth-connections/SKILL.md) | [branding-checklist](references/branding-checklist.md) | [openrouter-preset](references/openrouter-preset.md) | [file-map](references/file-map.md)

**Status**: Living skill — update when branding, packaging, or local-dev wiring changes.

---

## Overview

`avcd-agent` is an **Apache-2.0 custom distribution** (white label) of [goose](https://github.com/block/goose) / [aaif-goose](https://github.com/aaif-goose/goose). Upstream documents this under **Custom Distributions**; the branding appendix is **D. Custom Branding and UI**.

Use this skill when:

- Extending or auditing the AVCD rebrand
- Shipping desktop packages / updater assets
- Wiring the OpenRouter multi-model preset (parity with `avcd-ai`)
- Debugging “app starts but chat fails” or external-backend desktop mode
- Onboarding someone to local `make dev` + `make dev-ui`

Do **not** invent a parallel rebrand path. Prefer the official touchpoints below, then apply AVCD conventions.

---

## Canonical product identity (AVCD)

| Concern | AVCD value |
|---------|------------|
| Display / product name | `Avocado Work` |
| Executable / bin name | `avocado-work` |
| npm package name (desktop) | `avcd-agent-app` |
| Protocol scheme | `avocado-work` |
| Protocol display name | `AvocadoWorkProtocol` |
| GitHub publisher defaults | `Avocado-Technology` / `avcd-agent` |
| Bundle / updater asset name | `Avocado Work` (`GOOSE_BUNDLE_NAME`) |
| Electron userData dir | `~/Library/Application Support/Avocado Work` (macOS) |
| System persona | “Avocado Work, created by Avocado Technology” (based on goose / AAIF) |
| Default LLM | OpenRouter · `deepseek/deepseek-v4-flash` (13-model catalog) |
| Local ACP secret (dev) | `avcd-agent-local-development-key` |
| Local ACP URL | `http://127.0.0.1:3000` |

---

## Official rebrand steps (CUSTOM_DISTROS §D)

Upstream order of operations — **always apply all six**, then run AVCD smoke checks.

### 1. Visual assets

Replace icons under `ui/desktop/src/images/`:

- `icon.png`, `icon.ico`, `icon.icns` (and `@2x` / template / light variants as needed)
- Splash / loading assets under `loading-goose/` if still goose-branded

### 2. Electron Forge metadata

Edit `ui/desktop/forge.config.ts`:

- `packagerConfig.name` → `Avocado Work`
- `packagerConfig.executableName` → `avocado-work`
- `packagerConfig.icon` → `src/images/icon`
- `protocols[].name` / `schemes` → AVCD protocol
- macOS usage strings (`NSMicrophoneUsageDescription`, etc.)
- Deb/RPM maker `name` / `bin` → `avcd-agent`
- GitHub publisher defaults → `Avocado-Technology` / `avcd-agent`

### 3. System prompt persona

Edit `crates/goose/src/prompts/system.md` so the agent introduces itself as **Avocado Work** (keep Apache attribution to upstream goose).

### 4. UI copy and chrome

- `ui/desktop/index.html` `<title>`
- Menu / dialog strings in `ui/desktop/src/main.ts`
- i18n English source: `ui/desktop/src/i18n/messages/en.json` (then `make sync-i18n` if that is the workflow)
- Colors / Tailwind only if product design requires it

### 5. Packaging + updater alignment (easy to get wrong)

Static metadata:

- `ui/desktop/package.json` → `productName`, `description`, bundle scripts using `GOOSE_BUNDLE_NAME`
- `ui/desktop/forge.deb.desktop` / `forge.rpm.desktop` → `Name=Avocado Work`
- `ui/desktop/src/app-update.yml` → must **not** target `aaif-goose`

Build-time env (must be consistent everywhere):

```bash
export GITHUB_OWNER="Avocado-Technology"
export GITHUB_REPO="avcd-agent"
export GOOSE_BUNDLE_NAME="Avocado Work"
```

These are baked into the main process via `ui/desktop/vite.main.config.mts` (`define`). Defaults in that file **must match** Forge publisher defaults — mismatch breaks auto-update asset lookup after rebrand.

### 6. Release consistency checklist

Before any release build:

- [ ] Same display name in `forge.config.ts`, `package.json`, `index.html`
- [ ] Release zip / app bundle name matches `GOOSE_BUNDLE_NAME`
- [ ] Updater (`githubUpdater` + `app-update.yml` + Vite defines) looks at **this fork**, not upstream
- [ ] Linux `.desktop` templates use the packaged executable name
- [ ] `make test-smoke` passes (includes rebrand assertions)

Full checklist: [references/branding-checklist.md](references/branding-checklist.md).

---

## Local online stack (what “ready” means)

### Prerequisites

- Docker Desktop running
- Node **24+** on PATH for `make dev-ui` (repo ships `scripts/with-node.sh`)
- OpenRouter API key in `.env.local` as `OPENROUTER_API_KEY` (same value as avcd-ai `OPENROUTER_KEY`)

### Bring backend + chat online

```bash
cd avcd-agent

# 1. Ensure .env.local exists (copied from .env.local.example if missing)
make ensure-local-env   # if target exists; else make dev does this

# 2. Put a real key in .env.local (never commit it)
#    OPENROUTER_API_KEY=sk-or-...

# 3. Start / recreate ACP backend
make dev

# 4. Validate preset (offline + online when key set)
make validate-openrouter

# 5. Desktop against Docker ACP (launch from macOS Terminal — Cursor agent shells often kill Electron)
make dev-ui
```

Expected healthy signals:

| Check | Expect |
|-------|--------|
| `curl -H 'X-Secret-Key: …' http://127.0.0.1:3000/status` | HTTP 200 |
| `make validate-openrouter` | all PASS (online requires key) |
| `docker compose exec server goose run -t 'Reply with exactly: OK'` | model replies `OK` via openrouter |
| Desktop log | Opens `http://localhost:5173/#/?` **without** “Goose binary not found” |
| Renderer env | `GOOSE_DEFAULT_PROVIDER=openrouter`, 13 predefined models |

### Chat not working — triage order

1. **Empty `OPENROUTER_API_KEY`** in `.env.local` → container has KEY missing → recreate with `make dev`
2. **Desktop without external backend** → looks for local `goose` binary under `ui/desktop/src/bin/` → use `make dev-ui` (writes `ui/desktop/.env`)
3. **Stale Electron process** → quit Avocado Work / Terminal `make dev-ui`, relaunch
4. **Secret mismatch** → desktop `GOOSE_SERVER__SECRET_KEY` must equal compose secret
5. **Provider not in session** → backend env `GOOSE_PROVIDER` / `GOOSE_MODEL`; confirm with `goose info -v` inside the container

OpenRouter details: [references/openrouter-preset.md](references/openrouter-preset.md).

---

## Makefile public API (prefer over raw scripts)

| Target | Purpose |
|--------|---------|
| `make dev` | Build/start Docker ACP on `:3000` |
| `make dev-down` | Stop stack |
| `make dev-logs` | Follow server logs |
| `make cli` | Interactive CLI in Docker |
| `make dev-ui` | Prepare desktop `.env` + Electron against Docker |
| `make validate-openrouter` | Catalog + compose + `goose info -v` |
| `make test-smoke` | Backend, CLI, branding, package, license |
| `make package-ui` | Local desktop package |
| `make check-ui` / `make check-core` | UI / Rust checks |
| `make pull-secrets` / `make upload-secrets` | Infisical (when project exists) |

Agents must call **`make <target>`**, not ad-hoc `docker compose` / `pnpm` for documented workflows.

---

## Config precedence (upstream)

Later wins:

1. Built-in defaults  
2. `init-config.yaml` (first run only)  
3. `~/.config/goose/config.yaml` (in Docker: `/home/goose/.config/goose/config.yaml`)  
4. Environment variables (**highest**)

AVCD Docker Compose sets `GOOSE_PROVIDER`, `GOOSE_MODEL`, `OPENROUTER_*`, `GOOSE_DISABLE_KEYRING=true`, and loads `.env.local` via `env_file`.

---

## Distribution (GitHub Releases + website)

Desktop installers ship from **GitHub Releases** on `Avocado-Technology/avcd-agent`. The avocado.tech `/download` page links to the `stable` tag; it does not host binaries.

### Release asset contract

Single source of truth: [`ui/desktop/scripts/release-assets.js`](../../../../ui/desktop/scripts/release-assets.js).

| Platform | Website installer | In-app update check |
|----------|-------------------|---------------------|
| macOS arm64 | `Avocado Work.dmg` | `Avocado Work.zip` |
| macOS x64 | `Avocado Work_intel_mac.dmg` | `Avocado Work_intel_mac.zip` |
| Windows x64 | `Avocado Work-Setup-x64.exe` | same Setup.exe (assisted download) |

Forge makers: `@electron-forge/maker-dmg` + `@electron-forge/maker-squirrel` (`name: avocado-work`, `setupExe: Avocado Work-Setup-x64.exe`) + `maker-zip`.

### Verifier

```bash
bash scripts/verify-release-assets.sh          # no Goose* release paths
bash scripts/verify-release-assets.sh --guards # npm/Maven upstream-only; Docker uses ghcr.io/${{ github.repository }}
bash scripts/verify-release-assets.sh --local ui/desktop/out  # after local make/bundle
```

### CLI install script

[`download_cli.sh`](../../../../download_cli.sh) installs from `Avocado-Technology/avcd-agent`. On-disk binary name stays `goose` (archive names unchanged).

### Website env (`avcd-landingpage`)

```
NEXT_PUBLIC_DESKTOP_RELEASE_OWNER=Avocado-Technology
NEXT_PUBLIC_DESKTOP_RELEASE_REPO=avcd-agent
NEXT_PUBLIC_DESKTOP_RELEASE_TAG=stable
NEXT_PUBLIC_RELEASE_SIGNED=false
```

### Signing / release paths

See [DESKTOP_RELEASE_RUNBOOK.md](../../../../docs/development/manual-tests/DESKTOP_RELEASE_RUNBOOK.md). v1 update UX is **assisted download** (GitHubUpdater saves the installer/zip); keep `ENABLE_MAC_NATIVE_AUTO_UPDATE=false` until a follow-up enables native electron-updater.

---

## Licensing & telemetry (must keep)

- Keep `LICENSE` aligned with upstream Apache-2.0
- Keep `NOTICE` with attribution
- Do not use “Goose” trademarks in a way that implies official endorsement
- Smoke test asserts upstream PostHog key is absent; prefer `GOOSE_DISABLE_TELEMETRY=1` / `GOOSE_TELEMETRY_OFF` for distros

---

## Staying current with upstream

1. Sync the fork regularly  
2. Keep AVCD-only changes in config, catalog JSON, Makefile, skills, and thin UI branding  
3. Prefer recipes / env presets over deep Rust forks when possible  
4. After merge from upstream, re-run `make validate-openrouter` and `make test-smoke`

---

## Agent workflow for rebrand tasks

1. Read [CUSTOM_DISTROS.md](../../../../CUSTOM_DISTROS.md) §D and this skill  
2. Use [file-map](references/file-map.md) — edit the listed files only  
3. Keep names consistent with the identity table above  
4. Align `vite.main.config.mts` defaults with `forge.config.ts` publisher  
5. Run `make validate-openrouter` and (when a package exists) `make test-smoke`  
6. Update this skill if new branding touchpoints appear  

---

## Common issues

| Issue | Cause | Fix |
|-------|-------|-----|
| Chat fails after UI starts | Missing OpenRouter key in container | Set `OPENROUTER_API_KEY` in `.env.local`, `make dev` |
| “Goose binary not found” | Local serve without external backend | `make dev-ui` / set `GOOSE_EXTERNAL_BACKEND` |
| Updater looks at wrong repo | Vite define defaults still `aaif-goose` | Align `GITHUB_*` / `GOOSE_BUNDLE_NAME` |
| `validate-openrouter` fails on provider | Script used `goose info` without `-v` | Use `goose info -v` (script should already) |
| Electron exits immediately | Launched from Cursor agent shell | Launch via Terminal.app / `make dev-ui` |
| Smoke “bundle missing” | Never packaged | `make package-ui` then `make test-smoke` |
| macOS release fails on updater verify | Script still asserted `aaif-goose` | Use fork `app-update.yml` / `GITHUB_OWNER`+`GITHUB_REPO` |
| DMG make fails locally | `macos-alias` / `fs-xattr` natives not built | `pnpm run premake` (ensure-macos-alias.js) |
| Website `/download` 404 on static serve | Extensionless path | Production nginx `try_files $uri.html`; local Playwright uses `serve` |

---

## References

- [branding-checklist.md](references/branding-checklist.md) — release / PR checklist  
- [openrouter-preset.md](references/openrouter-preset.md) — catalog, env, validation  
- [file-map.md](references/file-map.md) — every branding / distro file path  
- [DESKTOP_RELEASE_RUNBOOK.md](../../../../docs/development/manual-tests/DESKTOP_RELEASE_RUNBOOK.md) — unsigned/signed release paths  
- Upstream [CUSTOM_DISTROS.md](../../../../CUSTOM_DISTROS.md)  
- Docs site: https://block-goose.mintlify.app/advanced/custom-distributions  
