# MCP Discovery URI — Implementation Plan

Implements IETF [draft-serra-mcp-discovery-uri](https://datatracker.ietf.org/doc/draft-serra-mcp-discovery-uri/) (client/consumer side) in goose.

## Clarified Problem Statement

**Goal:** Resolve an `mcp://host[:port][/path]` URI to a working MCP server connection across CLI, server API, and the desktop app.

**What the spec defines (resolution order, Section 4):**
1. **DNS TXT (optional, "fast mode")** — query `_mcp.{host}` TXT for `v=mcp1; src={url}` / `registry={url}` / `auth={type}`. Skippable. `endpoint=` is a deprecated alias for `src=`.
2. **`.well-known/mcp-server` manifest (REQUIRED)** — `GET https://{host}/.well-known/mcp-server`, parse JSON manifest.
3. **Direct handshake (fallback)** — attempt MCP at `https://{host}/mcp`.

**Manifest fields:**
- Required: `mcp_version`, `name`, `endpoint`, `transport` ("http" | "sse" | "stdio").
- Recommended: `description`, `auth`, `capabilities`, `trust_class` ("public" | "sandbox" | "enterprise" | "regulated").
- Optional: `categories`, `languages`, `coverage`, `contact`, `docs`, `last_updated`, `crawl`, `expires`, `cache_ttl`, `signature`, `server_card`, `payment_required`, `payment_methods`, `*_preview`.
- `auth` object: `required`, `methods` (["none","bearer","mtls","apikey","oauth2"]), `endpoint`, `metadata_url`, `apikey_header`, `scopes`. Required when `trust_class` is enterprise/regulated.

**Security rules (mandatory):**
- HTTPS-only for manifest + endpoint.
- Endpoint host MUST equal-or-be-subdomain of the manifest host.
- `.well-known` manifest takes precedence over DNS on conflict.
- JWS signature: manifest `signature` = `{ alg, kid, value }` (detached, base64url, over canonical JSON excluding `signature`). Public key from `_mcp-key.{host}` TXT: `v=mcp1jwk; kid=...; jwk={...}`.
- Missing trust declaration defaults to `public` (most restrictive safe default).
- User confirmation required before adding (surface name, trust_class, auth requirements).

**Constraints:**
- Funnel discovery into `ExtensionConfig::StreamableHttp` and the existing `add_extension` path (`crates/goose/src/agents/extension_manager.rs:807`).
- Do not break existing extension config (de)serialization or generated openapi.
- DCO sign-off (`git commit -s`); run `just generate-openapi` after server route changes.

**Non-goals (v1):**
- Server side (publishing goose's own `.well-known/mcp-server`).
- SEP-2127 Server Card fetch, `payment_required` handling, registry crawling/indexing — parse-and-ignore.
- Registry browser UI.

**Success criteria:**
- `goose run --with-mcp-extension mcp://example.com` resolves, verifies, connects.
- `POST /config/extensions/discover {uri}` returns a resolved `ExtensionConfig` or a structured error (bad host-match / sig fail / no manifest).
- Desktop `goose://mcp?uri=<mcp-uri>` link opens a confirm dialog (name/trust_class/auth) then installs.
- Unit tests: host-match rejection, JWS pass/fail, DNS-vs-well-known precedence, `/mcp` fallback, malformed manifest.

## Chosen Approach: A — Eager resolver module + thin surface adapters

New `crates/goose/src/mcp_discovery/` module exposing:

```rust
async fn resolve(uri: &str) -> Result<DiscoveredServer>
```

doing the 3-step resolution + JWS verification + host-match check, returning metadata
(name, trust_class, auth, endpoint) plus `to_extension_config() -> ExtensionConfig::StreamableHttp`.

Each surface calls `resolve()`, shows confirmation, then routes through the existing
`add_extension`/persistence path. Resolution happens once, at add-time (concrete cached endpoint).

**Why A over B/C:** cleanest reuse of the existing transport/persistence stack; resolve→confirm→add
maps exactly onto the spec security model; no `ExtensionConfig` enum churn (Approach B's large blast
radius across serde/openapi/UI types). Endpoint-rotation refresh (`goose mcp refresh`, Approach C) is
a clean additive follow-up — not in v1.

**Affected files:**
- New: `crates/goose/src/mcp_discovery/{mod,manifest,dns,jws}.rs`
- `crates/goose-cli/src/cli.rs` (~161-195): add `--with-mcp-extension` flag
- `crates/goose-cli/src/commands/configure.rs`: accept `mcp://` in dialog
- `crates/goose-server/src/routes/config_management.rs`: new `POST /config/extensions/discover` route + openapi
- `ui/desktop/src/main.ts`: `goose://mcp?uri=` protocol handler + confirm dialog component

## Build sequence

1. **Resolver core** (goose crate): manifest types + parse, host-match validation, HTTPS enforcement, well-known fetch, `/mcp` fallback, `to_extension_config()`. Unit-tested in isolation.
2. **DNS fast-path** (optional step 1): TXT lookup for `_mcp.{host}` (`hickory-resolver` or feature-gated), well-known precedence on conflict.
3. **JWS verification**: fetch `_mcp-key.{host}` JWK, verify detached signature over canonical JSON (reuse existing jose/jwt crate if present, else add).
4. **CLI surface**: `--with-mcp-extension` flag + `mcp://` acceptance in `configure`.
5. **Server surface**: `POST /config/extensions/discover`, regenerate openapi.
6. **Desktop surface**: `goose://mcp?uri=` handler + confirm dialog.

## Open questions (non-blocking)

- Desktop scheme: confirmed lean toward `goose://mcp?uri=<mcp-uri>` (matches existing recipe/session deeplinks in `recipe_deeplink.rs`) over claiming a generic OS-level `mcp://` scheme.
- DNS crate: `hickory-resolver` vs feature-gating the optional DNS step to avoid a heavy dep.
- JWS crate: reuse `jsonwebtoken`/`josekit` if already in tree vs add one.
