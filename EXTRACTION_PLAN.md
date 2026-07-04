# Extraction & Removal Plan: Local Inference, Telegram, and Phone-Home Features

This document is a plan only — no code changes accompany it. It describes, in
implementation-ready detail, how to:

1. **Extract local inference** (llama.cpp / MLX backends) **and the ability to
   download models from Hugging Face** out of this repo.
2. **Remove the Telegram capability** entirely.
3. **Clean up Settings → App**: remove the *Help & Feedback* card and the
   *Privacy* card, and make "do not report data" the effective default — by
   removing the code paths that communicate with reporting servers
   (telemetry, feedback, and updates) rather than merely toggling them off.

Each workstream below lists the current state (with file/line references as of
commit `c5fbbd7`), the exact deletions and edits, shared-code entanglements
that must be untangled, decision points with recommendations, and verification
steps. A suggested PR sequencing is at the end.

---

## Workstream A — Extract local inference + Hugging Face downloads

### A.1 Current state

Local inference is a self-contained module tree gated behind the Cargo feature
`local-inference` (with `mlx` / `cuda` / `vulkan` sub-features). The desktop UI
talks to it exclusively through ACP custom methods, so the UI coupling is thin.

**Rust core — `crates/gosling/src/providers/local_inference/`** (plus the
declaring module `crates/gosling/src/providers/local_inference.rs`):

| File | Role |
|---|---|
| `local_inference.rs` (module root) | `InferenceRuntime` singleton, `LocalInferenceProvider` (provider id `"local"`), backend selection. Constants `PROVIDER_NAME`, `DEFAULT_MODEL` (`bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M`), `LOCAL_LLM_MODEL_CONFIG_KEY` |
| `backend.rs` | `LocalInferenceBackend` / `BackendLoadedModel` traits — the abstraction seam |
| `llamacpp/` (`mod.rs`, `inference_engine.rs`, `inference_native_tools.rs`, `inference_emulated_tools.rs`) | llama.cpp backend via `llama-cpp-2` FFI |
| `mlx.rs` | MLX backend (`#[cfg(feature = "mlx")]`, stub otherwise) |
| `hf_models.rs` (~2,280 lines) | **Hugging Face discovery + download**: `hf-hub` crate plus raw `reqwest` calls to `https://huggingface.co/api/models`; search, GGUF-variant resolution, download progress bridging |
| `management.rs` | Orchestration facade called by the ACP server: `list_models`, `search_huggingface_models`, `download_model`, `cancel_download`, `delete_model`, `get/update_model_settings`, `list_builtin_chat_templates`, `ensure_featured_models_current` |
| `local_model_registry.rs` | Per-model persisted settings + `FEATURED_MODELS` catalog, registry file in the config dir |
| `multimodal.rs`, `native_tool_parsing.rs`, `tool_emulation.rs`, `tool_parsing.rs` | Vision/mmproj and tool-calling helpers |

**Wiring points:**

- Module + provider registration: `crates/gosling/src/providers/mod.rs:53` and
  `crates/gosling/src/providers/init.rs:74-75` (both `#[cfg(feature = "local-inference")]`).
- ACP handlers: `crates/gosling/src/acp/server/local_inference.rs` (8 handlers,
  each cfg-gated with a `local_inference_unavailable()` stub fallback);
  dispatch in `crates/gosling/src/acp/server/custom_dispatch.rs:642+`;
  capability advertisement `"localInference": {}` in
  `crates/gosling/src/acp/server.rs:257`.
- Wire DTOs: `crates/gosling-sdk-types/src/custom_requests.rs` (`LocalInference*`
  types, `ModelSettings` DTO mirror at `:1728+`); schema metadata in
  `crates/gosling/acp-meta.json:395+` and `acp-schema.json`; OpenAPI merge in
  `crates/gosling-server/src/openapi.rs:591` (`LocalInferenceApiDoc`).
- Feature definitions: `crates/gosling/Cargo.toml:23-44`; enabled **by default**
  in `crates/gosling-cli/Cargo.toml:76` and `crates/gosling-server/Cargo.toml:17`,
  with `cuda`/`vulkan`/`mlx` passthrough features in both.
- Optional dependencies pulled in by the feature (`crates/gosling/Cargo.toml`):
  `llama-cpp-2` + `llama-cpp-sys-2` (workspace-pinned `=0.1.146`),
  `hf-hub = "1.0.0-rc.1"`, `mlx-rs`/`mlx-lm`/`mlx-lm-utils` (git), and — used
  only by Whisper dictation, see A.3 — `candle-core`, `candle-nn`,
  `candle-transformers`, `tokenizers`, `symphonia`, `rubato`, `byteorder`.
  macOS-specific metal features at workspace `Cargo.toml:240-244`.
- Config/env keys: `LOCAL_LLM_MODEL` (also read by
  `crates/gosling/src/providers/toolshim.rs:34,58,97,112`),
  `GOSLING_LOCAL_DRAFT_MODEL`, `GOSLING_LOCAL_ENABLE_THINKING`.
- Tests: `crates/gosling/tests/local_inference_integration.rs`,
  `crates/gosling/tests/local_inference_perf.rs`, entries in
  `crates/gosling/tests/providers.rs`.
- Build/CI: justfile `generate-acp-schema` recipe passes
  `--features code-mode,local-inference,...` (line 174) and a
  `linux_vulkan_features` helper (line 343); `.github/workflows/build-cli.yml`
  has `vulkan` and `cuda` matrix variants (lines 65-176, installs
  `libvulkan-dev`).

**Desktop UI (`ui/desktop/`):**

- ACP client wrapper: `src/acp/local-inference.ts`.
- Settings tab: `src/components/settings/localInference/`
  (`LocalInferenceSection.tsx`, `LocalInferenceSettings.tsx`,
  `HuggingFaceModelSearch.tsx`, `ModelSettingsPanel.tsx`), conditionally
  rendered in `src/components/settings/SettingsView.tsx:25,87,234-239` behind
  the `localInference` capability flag
  (`src/acp/capabilities.ts:16,34`, `src/contexts/FeaturesContext.tsx:5`).
- Onboarding: `src/components/onboarding/LocalModelPicker.tsx`, plus
  references in `ProviderSelector.tsx` and `OnboardingSuccess.tsx`.
- HF auth UI: `src/components/settings/auth/HuggingFaceSignInPrompt.tsx`
  (+ `AuthSettingsSection.test.tsx` coverage).
- Generated SDK types: `ui/sdk/src/generated/*` (regenerated, not hand-edited).

**Shared code that is NOT exclusively local-inference:**

- `crates/gosling/src/download_manager.rs` — generic download
  progress/cancellation state machine, top-level module (`lib.rs:17`), used by
  both HF model downloads and Whisper dictation downloads.
- `crates/gosling/src/providers/huggingface_auth.rs` — HF OAuth device flow +
  `HF_TOKEN` secret; consumed by `hf_models.rs`, by the **remote** HF
  inference provider (`providers/huggingface.rs`), and by
  `gosling-server/src/routes/utils.rs:4`.
- Whisper dictation (`crates/gosling/src/dictation/whisper.rs:61-79`) downloads
  Whisper GGUF models **directly from `huggingface.co/.../resolve/...`** via
  `reqwest` (not `hf-hub`), gated behind the same `local-inference` feature.

### A.2 Extraction approach

"Extract" here means: **this repo no longer contains or builds the local
inference capability or any Hugging Face download path.** The code is carved
out in a way that preserves it as a reusable artifact rather than destroying
it.

**Step 1 — Carve out a standalone crate (preservation).**
Create a new repository (working name `gosling-local-inference`) seeded with:

- The entire `crates/gosling/src/providers/local_inference/` tree and its module
  root.
- A copy of `download_manager.rs` (this repo keeps its own copy only if
  dictation local models are retained — see A.3; if dictation downloads are
  removed too, the module moves out entirely).
- The `hf-hub`, `llama-cpp-2`/`llama-cpp-sys-2`, and `mlx-*` dependency
  declarations plus the `local-inference`/`mlx`/`cuda`/`vulkan` feature wiring
  and the macOS metal overrides from the workspace `Cargo.toml`.
- The two integration/perf test files.

The extracted crate's public API is the `management.rs` facade plus the
`Provider` implementation. It will need a small compatibility layer for the
types it currently borrows from `gosling`: `Provider`/`ProviderDescriptor`,
`ModelConfig`, `ProviderError`, `ProviderUsage`, message/tool formatting, and
the config-dir paths used by `local_model_registry.rs`. Two options:

- *(a)* Depend on `gosling` as a library from the new repo — simplest, keeps the
  trait contract intact, but inverts the dependency direction.
- *(b)* Define a minimal `LocalInferenceHost` trait (config paths, secrets
  lookup, provider-trait re-exports) in the new crate and implement it in any
  host. More work, cleaner long-term.

**Recommendation:** *(a)* for the initial carve-out; the new repo can evolve
toward *(b)* independently. This repo's plan does not block on either choice —
the removal below is identical regardless.

**Step 2 — Remove from this repo.** In dependency-safe order:

1. **UI first** (so nothing renders against a missing capability):
   - Delete `ui/desktop/src/components/settings/localInference/` and the
     conditional tab in `SettingsView.tsx` (imports at :25, capability read at
     :87, tab trigger + content at :234-239).
   - Delete `ui/desktop/src/acp/local-inference.ts`.
   - Delete `LocalModelPicker.tsx`; strip local-model branches from
     `ProviderSelector.tsx` and `OnboardingSuccess.tsx`.
   - Delete `HuggingFaceSignInPrompt.tsx` **only if** the remote HF provider
     is also dropped (Decision A-3); otherwise keep it — it serves remote HF
     auth too.
   - Remove `localInference` from `FeaturesContext.tsx` and
     `capabilities.ts`.
2. **ACP/API surface:**
   - Delete `crates/gosling/src/acp/server/local_inference.rs`; remove its
     dispatch arms in `custom_dispatch.rs` and the `"localInference"`
     capability entry in `acp/server.rs:257`.
   - Remove `LocalInference*` DTOs and the `ModelSettings` mirror from
     `crates/gosling-sdk-types/src/custom_requests.rs`.
   - Remove `LocalInferenceApiDoc` from `crates/gosling-server/src/openapi.rs`.
   - Regenerate `acp-meta.json` / `acp-schema.json` (justfile
     `generate-acp-schema`, minus the feature flag) and the UI SDK
     (`ui/sdk/src/generated/*`) and `ui/desktop/openapi.json`.
3. **Rust core:**
   - Delete `crates/gosling/src/providers/local_inference.rs` and the
     `local_inference/` directory.
   - Remove the module decl (`providers/mod.rs:53`) and registration
     (`providers/init.rs:6-7,74-75`).
   - Remove `LOCAL_LLM_MODEL` reads from `providers/toolshim.rs` (it should
     fall back to its non-local default path).
   - Delete the two local-inference test files and the `providers.rs` entries.
4. **Features & dependencies:**
   - Remove `local-inference`, `cuda`, `vulkan`, `mlx` feature definitions
     from `crates/gosling/Cargo.toml:23-44` and their passthroughs in
     `gosling-cli` / `gosling-server` Cargo.tomls (including the
     default-features entries).
   - Remove optional deps `llama-cpp-2`, `llama-cpp-sys-2`, `hf-hub`,
     `mlx-rs`, `mlx-lm`, `mlx-lm-utils`; drop the workspace pins and the
     macOS metal override block (`Cargo.toml:240-244`) for `llama-cpp-2`.
   - Candle/tokenizers/symphonia/rubato/byteorder: see A.3.
   - `hf-hub/rustls-tls` reference in the `rustls-tls` feature
     (`crates/gosling/Cargo.toml:59`) goes with it. Keep `Cargo.lock`
     consistent (`cargo update` will prune).
5. **Build/CI/docs:**
   - justfile: drop `local-inference` from the `generate-acp-schema` recipe
     and delete the `linux_vulkan_features` helper.
   - `.github/workflows/build-cli.yml`: delete the `vulkan` and `cuda` matrix
     variants and the `libvulkan-dev` install step.
   - Remove local-inference / HF model documentation pages under
     `documentation/` and env-var docs for `GOSLING_LOCAL_*` / `LOCAL_LLM_MODEL`.

### A.3 Entanglements & decision points

**Decision A-1 — Whisper dictation (recommendation: remove local Whisper).**
The `local-inference` feature also gates local Whisper dictation, and
`dictation/whisper.rs` downloads Whisper models from `huggingface.co`. Since
the mandate is to remove *the ability to download from Hugging Face*, local
Whisper model download must go too. Recommended: delete the Whisper backend
(`dictation/whisper.rs`, the candle/tokenizers/symphonia/rubato/byteorder
optional deps, `ui/desktop/src/components/settings/dictation/LocalModelManager.tsx`
and the local-model branches of `DictationSettings.tsx`), keeping the
cloud dictation providers (see `DICTATION_ALLOWED_PROVIDERS` in
`ui/desktop/src/updates.ts`). If local dictation must be preserved instead, it
needs its own feature flag (`dictation-local`) and its HF download replaced or
accepted as an exception — not recommended.

**Decision A-2 — `download_manager.rs`.** If A-1 removes Whisper downloads,
nothing else uses it: move it out with the extracted crate and delete the
top-level module (`lib.rs:17`). Otherwise it stays.

**Decision A-3 — Remote Hugging Face inference provider
(recommendation: remove).** `providers/huggingface.rs` is a *remote* inference
client for the HF Inference API — it does not download models, but it does
communicate with `huggingface.co` and is the only remaining consumer of
`huggingface_auth.rs` (OAuth against `huggingface.co/oauth/*`) after
extraction. For a clean "no Hugging Face communication" posture, remove the
provider, `huggingface_auth.rs`, its consumer in
`gosling-server/src/routes/utils.rs`, the `HF_TOKEN` secret key, the
`GOSLING_HUGGINGFACE_OAUTH_CLIENT_ID` compile-time env, and
`HuggingFaceSignInPrompt.tsx`. If only *downloads* are in scope, keep all of
this — flagging for an explicit owner call before implementation.

---

## Workstream B — Remove Telegram capability

### B.1 Current state

Telegram is implemented as the **only** concrete "gateway": a CLI-only,
long-polling bridge (`https://api.telegram.org`) that relays chat (including
voice notes) between a Telegram bot and a gosling session, with a pairing-code
flow. There is no server route, no desktop UI, no MCP server entry, and no
Telegram-specific dependency (plain `reqwest`). The generic gateway framework
(`handler.rs`, `manager.rs`, `pairing.rs`) exists solely to serve it —
`manager.rs`'s `check_auto_start` is already dead code (never called).

### B.2 Removal steps

Because Telegram is the sole implementation, **remove the entire gateway
feature**, not just the Telegram arm:

1. Delete `crates/gosling/src/gateway/` in full: `telegram.rs` (787 lines,
   includes the Bot API client and tests), `telegram_format.rs`
   (markdown→Telegram-HTML), `handler.rs`, `manager.rs`, `pairing.rs`,
   `mod.rs`.
2. Remove `pub mod gateway;` from `crates/gosling/src/lib.rs:20`.
3. Delete `crates/gosling-cli/src/commands/gateway.rs`; remove
   `pub mod gateway;` from `crates/gosling-cli/src/commands/mod.rs:3`.
4. `crates/gosling-cli/src/cli.rs`: remove the `GatewayCommand` enum (533-561),
   the `Gateway` subcommand + `gw` alias (780-788), the command-name mapping
   (1125), `handle_gateway_command` (1641-1656), and the dispatch arm (1971).
5. Remove `SessionType::Gateway` from
   `crates/gosling/src/session/session_manager.rs:51`; remove `'gateway'` from
   `ui/desktop/src/types/session.ts:32` and regenerate
   `ui/desktop/openapi.json` (enum at :4894).
6. Docs: delete
   `documentation/docs/experimental/remote-access/telegram-gateway.md` and
   `documentation/docs/experimental/remote-access/index.md`; remove the
   Remote Access card from `documentation/docs/experimental/index.md:27-31`;
   remove `GOSLING_GATEWAY_MAX_TURNS` from
   `documentation/docs/guides/environment-variables.md:159,200`.
7. Optional hygiene: drop the `tg:` / `telegram:` URL-scheme allowlist entries
   in `ui/desktop/src/utils/urlSecurity.ts:29-30` (generic deep-link handling,
   but nothing in-product needs them once the gateway is gone).

No `Cargo.toml` / `package.json` changes are needed. Runtime-created config
keys (`gateway_configs`, `gateway_platform_config_*`, `gateway_pairings`,
`gateway_pending_codes`) simply become inert; no migration required.

---

## Workstream C — Settings → App cleanup and phone-home removal

The instruction is to remove the two cards *and* extract the underlying
ability to communicate with the servers they represent — including feedback
and updates — with "do not report data" as the default. Inventory of every
phone-home path found, and its disposition:

| Path | Endpoint | Today | Disposition |
|---|---|---|---|
| Backend telemetry (`crates/gosling/src/posthog.rs`) | `https://us.i.posthog.com/capture/` (hardcoded key) | Opt-in (`GOSLING_TELEMETRY_ENABLED`, default off) **but** onboarding-funnel events bypass consent | **Remove entirely** (C.2) |
| Server ingress `POST /telemetry/event` (`gosling-server/src/routes/telemetry.rs`) | forwards to PostHog | feature-gated | **Remove** (C.2) |
| Frontend analytics (`ui/desktop/src/utils/analytics.ts`) | none — intentionally no-op | disabled | **Remove module + call sites** (C.2) |
| Help & Feedback card | links to `github.com/repo-makeover/gosling/issues` | UI only | **Remove card** (C.1) |
| Desktop auto-update (`autoUpdater.ts`, `githubUpdater.ts`) | GitHub Releases + `api.github.com` | ON by default, checks 5s after launch | **Remove** (C.3) |
| CLI self-update (`gosling-cli/src/commands/update.rs`) | `github.com/repo-makeover/gosling/releases` + attestations API | on-demand | **Remove** (C.3) |
| OTLP export (`crates/gosling/src/otel/`) | user-configured endpoint only; **no default** | off unless `OTEL_EXPORTER_OTLP_ENDPOINT` set | **Keep** (Decision C-1) |
| Sentry / crash reporting / feedback POST / remote config / announcements fetch | — | none exist | nothing to do |

### C.1 Remove the Help & Feedback card

The card is inline in
`ui/desktop/src/components/settings/app/AppSettingsSection.tsx:504-537`
(two buttons opening GitHub issue templates). Remove that block and its i18n
messages (`settings.help.title`, `settings.help.description`,
`settings.help.reportBug`, `settings.help.requestFeature`, lines 97-98 and the
button labels) from every locale file.

### C.2 Remove the Privacy card and all telemetry (default: no reporting)

Removing the reporting capability makes "off" the default by construction —
there is no toggle left to default. Note two things the current code gets
wrong that removal fixes: (1) the Privacy toggle *displays* ON when unset even
though the backend treats unset as off (`TelemetrySettings.tsx:54,61` vs
`posthog.rs:50-52`); (2) `posthog.rs::emit_event` lets `onboarding_*` and
`telemetry_preference_set` events **bypass the opt-in check** — data is sent
before consent. Both disappear with the module.

**Rust:**

1. Delete `crates/gosling/src/posthog.rs` and its module decl; delete the
   `telemetry` Cargo feature (`crates/gosling/Cargo.toml:13`) and its entries in
   `gosling-cli` / `gosling-server` default features.
2. Delete `crates/gosling-server/src/routes/telemetry.rs` and its router
   registration.
3. Remove all `emit_*` / `set_session_context` call sites:
   `crates/gosling/src/session/session_manager.rs:1327`,
   `crates/gosling/src/agents/agent.rs` (lines 1040, 2219, 2265, 2278, 2302,
   2316, 2327), `gosling-server/src/routes/session.rs:153,174,193`,
   `gosling-server/src/routes/agent.rs:125,149,245,254,557`,
   `gosling-cli/src/session/builder.rs:474`.
4. Remove the CLI consent dialogs: `configure_telemetry_consent_dialog` /
   `configure_telemetry_dialog` in `gosling-cli/src/commands/configure.rs`
   (lines 54, 1375) and their first-run triggers in `gosling-cli/src/cli.rs`
   (1386-1388, 1566-1568, 1835-1837).
5. Retire the config keys `GOSLING_TELEMETRY_ENABLED` / `GOSLING_TELEMETRY_OFF`
   (leave stale values in user config files untouched; nothing reads them).
   The `telemetry_installation.json` UUID file stops being written; no
   migration needed.

**Desktop UI:**

6. Delete `ui/desktop/src/components/settings/app/TelemetrySettings.tsx` and
   its render at `AppSettingsSection.tsx:502`.
7. Delete `ui/desktop/src/components/TelemetryConsentPrompt.tsx` and
   `ui/desktop/src/components/onboarding/PrivacyInfoModal.tsx`; remove the
   consent gate in `onboarding/OnboardingGuard.tsx:42`.
8. Delete `ui/desktop/src/utils/analytics.ts` and
   `ui/desktop/src/hooks/useAnalytics.ts` (already no-ops) and strip every
   `track*` call site (ErrorBoundary, settings, update flow, etc.) rather
   than keeping dead instrumentation.
9. Remove `TELEMETRY_UI_ENABLED` from `ui/desktop/src/updates.ts` and the
   `telemetrySettings.*` / consent-prompt i18n messages.

**Docs:** remove telemetry/consent sections and the
`GOSLING_TELEMETRY_ENABLED` / `GOSLING_TELEMETRY_OFF` entries from
`documentation/docs/guides/environment-variables.md` and any privacy docs.

### C.3 Remove update capability (desktop + CLI)

**Desktop:**

1. Delete `ui/desktop/src/utils/autoUpdater.ts` (electron-updater against
   GitHub owner `repo-makeover`, startup check 5s after launch,
   `autoInstallOnAppQuit`) and `ui/desktop/src/utils/githubUpdater.ts`
   (REST fallback + asset download to `~/Downloads`).
2. Delete `ui/desktop/src/components/settings/app/UpdateSection.tsx` and its
   render in `AppSettingsSection.tsx:561-573`; remove the `update` deep-link
   section mapping in `SettingsView.tsx:99-112`.
3. Remove the update IPC surface from `preload.ts` and main-process handlers
   (`check-for-updates`, `downloadUpdate`, `installUpdate`, `getUpdateState`,
   `isUsingGitHubFallback`, `getAutoDownloadDisabled`), the tray
   "update available" indicator, and the persisted `disableAutoDownload`
   app setting.
4. Set/remove `UPDATES_ENABLED` in `ui/desktop/src/updates.ts` — with all
   consumers gone the flag itself should be deleted.
5. Drop the `electron-updater` dependency from `ui/desktop/package.json:78`.
   Keep `electron-log` — `src/utils/logger.ts` uses it independently.
6. Retire env knobs `GOSLING_DISABLE_AUTO_DOWNLOAD`, `ENABLE_DEV_UPDATES`,
   `GITHUB_OWNER`, `GITHUB_REPO`, `GOSLING_BUNDLE_NAME` and their docs.
   The Version card (`AppSettingsSection.tsx:540-558`) is local-only and
   stays.

**CLI:**

7. Delete `crates/gosling-cli/src/commands/update.rs` (GitHub release download
   + Sigstore/SLSA attestation verification), the `gosling update` subcommand
   wiring in `cli.rs`, the now-pointless `disable-update` feature
   (`gosling-cli/Cargo.toml:100`), and the `sigstore-verify` dependency (verify
   nothing else uses `tar`/`bzip2`/`zip` before pruning those).
8. Update docs (`gosling update` guide, install/update pages). The standalone
   install scripts (`download_cli.sh` / `download_cli.ps1`) run outside the
   product; out of scope, but note they also point at
   `github.com/repo-makeover/gosling`.

**Decision C-1 — OpenTelemetry export (recommendation: keep).**
`crates/gosling/src/otel/otlp.rs` exports traces/metrics/logs **only** if the
user sets standard `OTEL_*` env vars; there is no default endpoint and nothing
is Block/AAIF-operated. This is user-directed observability, not vendor
reporting, so it does not violate "default to not report data" (the default is
already no export). Keep it. If a stricter posture is wanted later, it is
feature-gated (`otel`) and can be compiled out by removing the feature from
default feature lists.

**Decision C-2 — announcements.** `AnnouncementModal.tsx` content is bundled
locally and `ANNOUNCEMENTS_ENABLED` is already `false`; no network involved.
Leave as-is (or delete opportunistically in C.3's `updates.ts` cleanup).

---

## Sequencing, verification, and acceptance

### Suggested PR sequence

Independent workstreams; ordered to keep every intermediate state green:

1. **PR 1 — Telegram/gateway removal** (Workstream B). Smallest, zero
   entanglements.
2. **PR 2 — Telemetry removal** (C.2 + C.1). Rust + UI + docs in one PR so
   the consent UI never points at a missing backend.
3. **PR 3 — Update removal** (C.3). Desktop and CLI can be split if reviewers
   prefer.
4. **PR 4 — Local inference extraction** (Workstream A), preceded by the
   carve-out into the new repo. Largest; do last so the extracted crate can be
   cut from a stable tree. Sub-order within the PR: UI → ACP/DTOs/schema
   regen → Rust core → features/deps → CI/docs.

### Verification per PR

- `cargo fmt` && `cargo clippy --all-targets -- -D warnings` &&
  `cargo build` && `cargo test -p gosling -p gosling-cli -p gosling-server`
- `cd ui/desktop && pnpm run typecheck && pnpm test`
- Schema/SDK regeneration where ACP/OpenAPI types changed (justfile
  `generate-acp-schema`; regenerate `ui/sdk` and `ui/desktop/openapi.json`)
  with a clean-diff check that no `LocalInference*`/`gateway` types remain.
- Manual smoke: desktop app boots, Settings → App shows Appearance / Theme /
  Language / Version only; onboarding completes without consent prompt or
  local-model picker; CLI `gosling configure` runs without telemetry dialog;
  `gosling gateway` and `gosling update` are unknown commands.

### Acceptance criteria (network egress audit)

After all PRs, a grep/audit of the tree must show **no remaining references**
to: `posthog`, `us.i.posthog.com`, `api.telegram.org`, `hf-hub`,
`huggingface.co` (subject to Decision A-3 for the remote provider),
`electron-updater`, `releases/download`, `attestations/sha256`. Remaining
sanctioned egress: user-configured LLM provider APIs, user-configured MCP
extensions, and user-configured OTLP collectors — all opt-in with no default
endpoint. Default behavior on a fresh install: **no data is reported
anywhere.**

### Risks

- **Schema regeneration drift** (A.2 step 2, B.2 step 5): `openapi.json` and
  `ui/sdk/src/generated/*` must be regenerated, never hand-edited (repo rule);
  stale generated types are the likeliest source of typecheck breakage.
- **Feature-flag fallout**: `local-inference` sits in default feature lists of
  two crates and in CI matrix builds; missing one reference breaks
  `--all-features` builds used by justfile recipes.
- **`toolshim.rs`** reads `LOCAL_LLM_MODEL` outside the feature gate — confirm
  its non-local fallback path before deleting the constant.
- **Shared-dep pruning** (C.3 step 7, A.2 step 4): verify with `cargo tree`
  that `reqwest`/`tar`/`zip`/`sha2` etc. have other consumers before removing.
