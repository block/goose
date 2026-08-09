---
name: mcp-oauth-connections
description: How Avocado Work connects OAuth-protected MCP servers — the extension toggle drives sign-in and sign-out, authorization URLs are opened by the desktop client, and credentials live in the goose credential store. Use when adding a hosted OAuth MCP connector, changing extension auth behaviour, or debugging a connector that will not sign in.
---

# MCP OAuth Connections

> **Related**: [avcd-agent-custom-distro](../avcd-agent-custom-distro/SKILL.md) | `mcp-oauth-authentication` user skill (the MCP *server* side: Auth0, Traefik, deployment) | [avcd-local-oidc-dev](../../../../.cursor/skills/security/avcd-local-oidc-dev/SKILL.md) (application login, not MCP connectors)

## Overview

Google Workspace was the first hosted OAuth MCP connector in Avocado Work, and its
flow is the template for every other one. Nothing in the implementation is
Google-specific: the behaviour keys off the `streamable_http` extension type, so
any MCP server that speaks OAuth 2.1 with dynamic client registration gets it for
free.

The governing idea is that **the toggle is the connection**. A user should not
have to reason about "enabled" and "signed in" as separate states:

| User action | What happens |
| --- | --- |
| Toggle on | Authorize (reuse or refresh stored tokens, else browser flow), then enable for new chats |
| Toggle off | Clear stored credentials, then disable |
| Sign-in fails | Extension stays off and the switch reverts |

## The contract an MCP server must satisfy

1. OAuth 2.1 authorization server metadata at `/.well-known/oauth-authorization-server`.
2. Dynamic client registration (RFC 7591) at the advertised `registration_endpoint`.
3. PKCE (`code_challenge_methods_supported` including `S256`).
4. A loopback redirect URI accepted at registration time.

Loopback ports do **not** need to be pre-allowlisted on the server. Registration
is dynamic, so whatever port goose binds is registered with the request. The
fixed port in dev exists for Docker port publishing, not for the server.

## The three flows

### Sign in (toggle on)

`on_set_config_extension_enabled` authorizes *before* it flips the enabled flag,
so a failed sign-in cannot leave a connection that is on but unusable.

```130:148:crates/goose/src/acp/server/extensions.rs
        if req.enabled {
            if let Some(entry) = crate::config::extensions::get_extension_entry(&req.config_key) {
                authenticate_config_extension_if_streamable_http(&entry.config, false)
                    .await
                    .internal_err()?;
            }
        } else if let Some(entry) = crate::config::extensions::get_extension_entry(&req.config_key)
        {
            // ... clear stored credentials, best effort ...
        }

        let updated =
            crate::config::extensions::set_extension_enabled(&req.config_key, req.enabled);
```

1. `authenticate_streamable_http_extension(uri, name, force = false)`.
2. With `force = false`, existing credentials are refreshed and reused — no browser.
3. If refresh fails, the bad credentials are cleared and the full flow runs.
4. The full flow registers a client, builds the authorization URL, hands it to the
   client to open, and blocks on a loopback callback (default 300s, override with
   `GOOSE_OAUTH_CALLBACK_TIMEOUT_SECONDS`).
5. Only on success does `set_extension_enabled(key, true)` run.

### Sign out (toggle off)

`deauthenticate_streamable_http_extension` clears the entry from
`GooseCredentialStore`, then the extension is disabled. Clearing is best-effort:
a failure is logged but does not block the user from turning the connection off.

This is a **local** sign-out. It forgets the tokens on this machine; it does not
revoke them. The Google Workspace MCP advertises no `revocation_endpoint`, so
RFC 7009 revocation is not available. Do not label this as "revoke access" in UI
copy.

### Session start

Extensions load with `OAuthInteractivity::StoredCredentialsOnly`, which never
opens a browser. This is deliberate — an interactive flow here would stall
session creation for the whole callback timeout. A dead token surfaces as
"Sign in required (Connections)" instead of a hard failure.

## Who opens the browser

The agent must not open the browser itself. It routinely runs inside a container
(`make dev`) or over a remote transport, where `webbrowser::open` targets the
wrong machine and the user sees nothing while the toggle spins for five minutes.

Instead the agent registers a handler that forwards the URL to whoever owns the
user's browser:

```80:95:crates/goose/src/oauth/mod.rs
fn open_authorization_url(name: &str, authorization_url: &str) {
    if let Some(handler) = authorization_url_handler() {
        handler(AuthorizationPrompt {
            extension_name: name.to_string(),
            authorization_url: authorization_url.to_string(),
        });
        return;
    }

    if let Err(e) = webbrowser::open(authorization_url) {
        warn!(
            "[OAuth:{}] Failed to open browser automatically: {}",
            name, e
        );
    }
}
```

- The ACP server registers one during `initialize`, gated on the client
  advertising `customNotifications`.
- The handler emits `_goose/unstable/extensions/authorization-required` with the
  extension name and authorization URL.
- The desktop client opens it with `window.electron.openExternal` and shows a
  "Finish signing in in your browser" toast until the flow resolves.
- With no handler registered, `open_authorization_url` falls back to
  `webbrowser::open`, which is correct for the CLI.

A process-global handler is used rather than threading a callback through
`add_extension` → `add_extension_with_oauth` → `OAuthInteractivity`. One
registration then covers every entry point: the config toggle, the explicit
authenticate endpoint, and adding an extension to a live session.

## Adding a new OAuth MCP connector

1. Add the entry to `ui/desktop/src/components/settings/extensions/bundled-extensions.json`
   with `"type": "streamable_http"` and `"enabled": false`. Ship it off; the user
   turning it on *is* the consent step.

```json
{
  "id": "google-workspace",
  "name": "google-workspace",
  "display_name": "Google Workspace",
  "description": "Gmail, Drive, Calendar, and Docs via Avocado hosted Google Workspace MCP (OAuth).",
  "enabled": false,
  "type": "streamable_http",
  "uri": "https://dev.avocado.tech/google-workspace/mcp",
  "timeout": 300,
  "bundled": true
}
```

2. If the connector should be hidden in some distributions, gate it in
   `syncBundledExtensions` the way `GOOGLE_WORKSPACE_ENABLED` does in
   `ui/desktop/src/updates.ts`.
3. Nothing else. Do not add per-connector auth code, buttons, or endpoints — the
   `streamable_http` branch already handles authorize, refresh, sign-out, and the
   authenticated badge.

## Dev environment requirements

The callback server binds inside whatever process runs goose.

| Setting | Docker dev (`make dev`) | Packaged app |
| --- | --- | --- |
| `GOOSE_OAUTH_CALLBACK_PORT` | `18787` (fixed, published) | unset — ephemeral port is fine |
| `GOOSE_OAUTH_CALLBACK_BIND` | `0.0.0.0` | unset (loopback) |

In `docker-compose.yml` both are set and the port is published, so the host
browser can reach the redirect. Without them the browser redirect lands nowhere
and the flow times out.

## Verifying a connector by hand

```bash
# 1. Discovery
curl -s https://<host>/<connector>/.well-known/oauth-authorization-server | python3 -m json.tool

# 2. Dynamic registration with a loopback redirect
curl -sS -X POST https://<host>/<connector>/register \
  -H 'Content-Type: application/json' \
  -d '{"client_name":"goose","redirect_uris":["http://127.0.0.1:18787/oauth_callback"],
       "grant_types":["authorization_code","refresh_token"],
       "response_types":["code"],"token_endpoint_auth_method":"none"}'
```

A registration that echoes back the redirect URI and a scope list means the
server satisfies the contract.

## Known limits

| Limit | Impact | Notes |
| --- | --- | --- |
| The enable call blocks until the callback lands | Switch shows "Signing in…" for up to 300s | Row status and toast explain the wait; there is no cancel |
| Sign-out is local only | Tokens are forgotten, not revoked | Needs a `revocation_endpoint` on the MCP server |
| Toggling off destroys credentials | Turning a connection off briefly forces a full re-auth | Deliberate: the toggle means "connected" |
| The consent screen is shared | It is the MCP server's page, serving every registered client | Seeing another product's name means that client started the flow |

## File map

| Concern | Location |
| --- | --- |
| Authorize / refresh / sign out | `crates/goose/src/oauth/mod.rs` |
| Credential storage | `crates/goose/src/oauth/persist.rs` (`GooseCredentialStore`) |
| Toggle, authenticate, deauthenticate handlers | `crates/goose/src/acp/server/extensions.rs` |
| Notification type | `crates/goose-sdk-types/src/custom_notifications.rs` |
| Opening the URL on the host | `ui/desktop/src/acp/extensionAuthorization.ts` |
| Toggle UX and toasts | `ui/desktop/src/components/settings/extensions/` |

After changing any ACP request or notification, run `just generate-acp-types`
and `cd ui/sdk && pnpm run build:ts` — the desktop compiles against the built
SDK, not the generated sources.
