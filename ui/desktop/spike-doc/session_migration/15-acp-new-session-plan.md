# 15 ACP New Session Plan

## Context

ACP load and normal ACP reply are now implemented for desktop chat. Plain
non-recipe new-chat creation has also been migrated to ACP `session/new` behind
a guarded path.

Dependency: this plan assumes the ACP load/reply/mode/permission work from
`14-acp-reply-spike-plan.md` is already in place.

## Goal

Migrate plain non-recipe new chats to ACP `session/new` without changing recipe
or desktop extension override semantics.

## Current Status

Implemented:

- Plain non-recipe `createSession(...)` uses ACP `session/new`.
- `acpNewSession(cwd)` calls:

  ```ts
  client.newSession({
    cwd,
    mcpServers: [],
    _meta: { client: 'goose' },
  });
  ```

- `acpNewSessionToSession(...)` converts the ACP response into the desktop
  `Session` snapshot shape.
- Recipe, recipe deeplink, explicit extension config, and desktop extension
  override sessions still use REST `startAgent`.
- Extension override state is only consumed/cleared by the REST path that still
  understands those overrides.
- New ACP-created plain sessions still route through the existing chat view and
  immediately use ACP `session/load` for state setup.
- Focused tests cover ACP-vs-REST routing behavior for session creation.

Validated:

- Plain new chat can be created through ACP.
- First prompt after ACP new-session creation uses ACP reply.
- Manual mode and inline tool approval work after ACP new-session creation.
- ACP load replay duplicate message IDs for orphan completed tool updates were
  fixed in `sessionNotificationAdapter`.
- Adapter tests and desktop typecheck pass.

## Current New-Chat Flow

Plain new chat now starts from `ui/desktop/src/sessions.ts`:

```text
startNewSession
  -> createSession
      -> ACP session/new for guarded plain sessions
      -> REST startAgent for recipe/override sessions
  -> SESSION_CREATED event
  -> ADD_ACTIVE_SESSION event
  -> navigate pair?resumeSessionId=...
  -> BaseChat/useChatStream loads session
  -> first prompt uses ACP session/prompt
```

## ACP Server Behavior

Goose ACP already implements `session/new`.

Important server behavior:

- request includes `cwd`, `mcpServers`, optional `_meta`
- `_meta.client = "goose"` makes the server create a normal
  `SessionType::User`; without it, the session defaults to `SessionType::Acp`
- response includes `sessionId`, modes, models, and config options
- agent setup continues in the background
- enabled/default extensions are loaded during ACP agent setup
- extra MCP servers from the request are added after default extension loading

Important REST behavior that ACP does not yet mirror directly:

- REST `startAgent` accepts recipe fields:
  - `recipe`
  - `recipe_id`
- REST `startAgent` accepts desktop extension overrides:
  - `extension_overrides`
- REST persists the initial enabled-extension state into session
  `extension_data` using `resolve_extensions_for_new_session(...)`

## Guarded Migration Scope

Use ACP `session/new` only for plain sessions.

ACP path is allowed when all are true:

- no `recipeId`
- no `recipeDeeplink`
- no explicit `extensionConfigs`
- no extension override state to consume

Keep REST `startAgent` when any are true:

- recipe session
- recipe deeplink session
- Hub or caller supplied explicit extension configs
- extension override state exists and needs to be consumed/cleared

This avoids changing recipe semantics or desktop extension override behavior in
the same patch as the lifecycle migration.

## Implemented Shape

1. ACP helper in `ui/desktop/src/acp/sessions.ts`:

   ```ts
   acpNewSession(cwd: string): Promise<NewSessionResponse>
   ```

   The helper calls:

   ```ts
   client.newSession({
     cwd,
     mcpServers: [],
     _meta: { client: 'goose' },
   });
   ```

2. Response-to-desktop-session helper:

   ```ts
   acpNewSessionToSession(response, cwd): Session
   ```

   The desktop snapshot includes:

   - `id = response.sessionId`
   - `name = DEFAULT_CHAT_TITLE`
   - `working_dir = cwd`
   - empty `conversation`
   - `message_count = 0`
   - current timestamps for `created_at` and `updated_at`
   - no recipe fields

3. `createSession(...)` in `ui/desktop/src/sessions.ts` uses ACP only for the
   guarded plain-session case. The existing REST `startAgent` body and behavior
   are preserved for all other cases.

4. Existing desktop events and navigation are preserved.

   Do not change:

   - `SESSION_CREATED`
   - `ADD_ACTIVE_SESSION`
   - `setView('pair', { resumeSessionId })`
   - initial message handoff behavior

5. The immediate `session/load` remains.

   For the first patch it is acceptable for a new session to do:

   ```text
   session/new -> route to chat -> useChatStream session/load
   ```

   This is redundant but keeps all loaded-session state setup in one place.
   A later cleanup can seed the cache from `NewSessionResponse`.

6. Focused tests cover routing behavior:

   - plain `createSession` calls ACP `session/new`
   - recipe `createSession` still calls REST `startAgent`
   - explicit extension config `createSession` still calls REST `startAgent`
   - extension override state still falls back to REST and clears overrides only
     after the REST path consumes them

## Manual Acceptance

- Start New Chat from sidebar.
- Confirm plain creation uses ACP `session/new`.
- Send `hello`.
- Switch session mode to Manual.
- Ask for a filesystem/shell tool action.
- Confirm inline approval buttons appear.
- Allow once and confirm the tool runs.
- Reload the new session and confirm history loads correctly.
- Confirm no `Provider not set` error.
- Confirm no duplicate React key warning appears during ACP load replay.

## Follow-Ups

- Recipe-session load/new-session behavior.
- Keep ACP elicitation responses on `_goose/elicitation/respond`; REST
  `sessionReply` remains only as the non-ACP fallback until REST sessions are
  retired.
- Edit/fork message migration should be treated as a session-history mutation
  migration, not a reply migration. The actual assistant turn after edit/fork
  already uses ACP `session/prompt`; the dead REST `overrideConversation`
  reply branch has been removed from desktop.
- Do not add a desktop-specific ACP method that directly mirrors the old REST
  fork request shape (`copy` + `truncate` + `timestamp`) unless there is no
  cleaner protocol shape available.
- Prefer reusable ACP primitives for external clients:
  - use ACP `unstable_forkSession` for session copy
  - add a Goose ACP custom history method, likely `_goose/session/truncate`
  - prefer truncating by `messageId` if the server can support it cleanly; keep
    timestamp support only as a compatibility bridge if needed
- Desktop fork-edit flow should become:
  - ACP `unstable_forkSession`
  - ACP `_goose/session/truncate` on the forked session
  - navigate to the forked session
  - existing ACP `session/prompt` submits the edited message
- Desktop edit-in-place flow should become:
  - ACP `_goose/session/truncate` on the current session
  - ACP `session/load` to rebuild UI and ACP-side session state from truncated
    DB history
  - existing ACP `session/prompt` submits the edited message
- Decide whether ACP `session/new` should support recipe session creation
  directly, or whether recipe creation should remain REST until recipes have an
  ACP-specific design.
- Decide how desktop extension overrides should map to ACP:
  - convert override `ExtensionConfig[]` into ACP `mcpServers`, where possible
  - or add a Goose-specific ACP `_meta` field for extension override semantics
  - or keep override sessions on REST
- Decide whether ACP new-session response should seed the desktop results cache
  to avoid an immediate `session/load`.
- Verify default enabled extensions in ACP-created sessions match REST-created
  plain sessions, including platform/developer extension behavior.
- Add broader integration coverage for the sidebar Start New Chat path if the
  desktop test harness can exercise the full navigation flow.
