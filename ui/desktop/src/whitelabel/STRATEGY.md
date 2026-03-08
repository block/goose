# White-Label Strategy

## What Actually Works Today

The whitelabel system is more complete than a first glance suggests. Here's the honest inventory.

### Fully working, end-to-end

| Capability | How |
|---|---|
| **App name** | `main.ts` about dialog, `sessions.ts` recipe title/description |
| **Greetings** | `Greeting.tsx` → `useWhiteLabel().getRandomGreeting()` |
| **Window dimensions** | `main.ts` reads `window.width/height/minWidth/resizable` |
| **Nav item filtering** | `NavigationPanel.tsx` → `isNavItemEnabled()` |
| **Settings tab filtering** | `SettingsView.tsx` → `isSettingsTabEnabled()` + `isSectionHidden()` |
| **Updates toggle** | `UPDATES_ENABLED` → `main.ts` (updater setup), `AppSettingsSection` (hide update UI) |
| **Cost tracking toggle** | `COST_TRACKING_ENABLED` → `AppSettingsSection`, `ChatInput` |
| **Announcements toggle** | `ANNOUNCEMENTS_ENABLED` → `AnnouncementModal` |
| **Configuration toggle** | `CONFIGURATION_ENABLED` → `SettingsView` (hides ConfigSettings) |
| **Telemetry UI toggle** | `TELEMETRY_UI_ENABLED` → `TelemetrySettings`, `TelemetryOptOutModal` |
| **Dictation provider restriction** | `DICTATION_ALLOWED_PROVIDERS` → `DictationSettings` filters providers |
| **Provider registration** | `initProvider.ts` registers custom `providerDefinition` with goosed on startup |
| **Provider/model defaults** | `initProvider.ts` sets `GOOSE_PROVIDER` / `GOOSE_MODEL` via config API |
| **Extension defaults** | `initProvider.ts` reconciles extensions — adds missing, sets enabled state, disables extras |
| **System prompt** | `sessions.ts` builds a whitelabel `Recipe` injected into every new session — **but this is wrong, see below** |
| **Skills** | `sessions.ts` appends skill instructions to the recipe system prompt |
| **Tools** | `sessions.ts` appends tool instructions to the recipe system prompt |
| **Process management** | `processManager.ts` full lifecycle — spawn, stdout/stderr logging, restart-on-crash, port wait, `envFromUrl` for credential injection |

That's a substantial amount of working infrastructure. Provider setup, extension management, system prompt injection, process management, feature flags — the backend plumbing is solid.

### Defined but not wired

| Gap | What exists | What's missing | Status |
|---|---|---|---|
| **Logo** | `branding.logo`, `logoSmall`, `trayIcon` fields in config | Every logo renders hardcoded `<Goose />` SVG (~8 component sites). No asset loading. `main.ts` icon is hardcoded. | TODO |
| **Tagline** | `branding.tagline` field | — | ✅ Fixed |
| **Starter prompts** | `branding.starterPrompts` type + field | — | ✅ Fixed |
| **General provider restriction** | `features.allowedProviders` + `isProviderAllowed()` | — | ✅ Fixed |
| **Extension envVars** | `envVars` on `WhiteLabelExtensionDefault` | — | ✅ Fixed |
| **Default workingDir** | `defaults.workingDir` field | — | ✅ Fixed |
| **Window alwaysOnTop** | `window.alwaysOnTop` field | — | ✅ Fixed |

### Completely absent

| Capability | Description |
|---|---|
| **Home screen customization** | Hub is hardcoded: logo → greeting → stats → recent chats. No way to show cards for recipes, apps, quick actions. |
| **Theme/color control** | All design tokens hardcoded in `theme-tokens.ts`. No config surface. |
| **Asset bundling** | No build pipeline to copy brand images into Electron resources or resolve them at runtime. |
| **Config sealing** | Can set defaults but users can change provider, model, extensions freely. |
| **Process visibility** | Sidecar processes run silently. No status in UI, no restart controls. |

---

## System Prompt: The Recipe Hack Needs to Go

The whitelabel system prompt is currently funneled through a fake `Recipe` object
(`sessions.ts` → `buildWhiteLabelRecipe()`). This is wrong for several reasons:

1. **It doesn't override identity.** The server applies recipe instructions via
   `agent.extend_system_prompt("recipe", ...)` which *appends* to the base prompt.
   The agent still sees "You are a general-purpose AI agent called goose" from
   `system.md`, then extension instructions like "Use the developer extension to
   build software", and only *then* the whitelabel prompt saying "you are Managerbot."
   The identity fights itself.

2. **Extension instructions can't be reframed.** A whitelabel build might enable
   the developer extension for its tools (shell, edit, write, tree) but not want the
   agent to think it's a developer. Extension instructions are baked into the
   extension's `InitializeResult.with_instructions()` and rendered by `system.md`
   before any recipe extras. There's no way to override them.

3. **It's a recipe when it isn't one.** Recipes are user-created reusable task
   templates. The whitelabel system prompt is the base identity of the app. Conflating
   them means the whitelabel prompt competes with actual user recipes.

### What to do

The agent already supports `override_system_prompt()` which **replaces** `system.md`
entirely. The override is a Tera template with access to the same context — `extensions`,
`current_date_time`, etc. The CLI uses this via `GOOSE_SYSTEM_PROMPT_FILE_PATH`.

**Server change (Rust):**
- Add `system_prompt: Option<String>` to `StartAgentRequest`
- When set, call `agent.override_system_prompt(system_prompt)` instead of (or before)
  recipe handling
- This replaces "You are goose" with the whitelabel identity while still rendering
  extension tool info through the template context

**UI change:**
- `sessions.ts` sends `system_prompt` directly in the `startAgent` request body
  instead of wrapping it in a fake recipe
- Remove `buildWhiteLabelRecipe()` — the system prompt, skills, and tools go as
  `system_prompt` not `recipe.instructions`
- Actual user recipes still work through the existing `recipe` / `recipe_id` fields

**Extension instruction control:**
With a system prompt override template, the whitelabel config controls *how* extension
instructions render. The default `system.md` template does:
```
{% for extension in extensions %}
## {{extension.name}}
{{extension.instructions}}
{% endfor %}
```
A whitelabel override template could:
- Render extension instructions differently (reframe, filter, omit)
- Replace the identity paragraph entirely
- Still include `{% for extension in extensions %}` to get tool usage info without
  the "you are a developer" framing

For example, Managerbot's system prompt template could be:
```
You are Managerbot — an AI COO for Square merchants.

# Available Tools
{% for extension in extensions %}
## {{extension.name}}
{% if extension.instructions %}{{extension.instructions}}{% endif %}
{% endfor %}
```

This gives the extension tools but under Managerbot's identity, not "goose the developer."

---

## The Work

### 0. Fix system prompt delivery (server + UI)

**Server (`crates/goose-server/src/routes/agent.rs`):**
- Add `system_prompt: Option<String>` to `StartAgentRequest`
- In `start_agent`, if `system_prompt` is set, call `agent.override_system_prompt()`
- Generate openapi after

**UI (`ui/desktop/src/sessions.ts`):**
- Send `defaults.systemPrompt` as `system_prompt` in the `startAgent` body
- Skills and tools instructions become part of the system prompt template string
  (same content, just sent as `system_prompt` not `recipe.instructions`)
- Remove `buildWhiteLabelRecipe()`

### 1. Connect the disconnected wires ✅ DONE

All fixed in this branch:

- ✅ **Starter prompts**: `PopularChatTopics.tsx` reads `branding.starterPrompts` from context
- ✅ **Provider filtering**: `ProviderGrid.tsx` filters via `isProviderAllowed()`
- ✅ **Extension envVars**: `initProvider.ts` passes `envVars` as `envs` on stdio configs
- ✅ **SSE extensions**: `initProvider.ts` handles `type: 'sse'` with `uri`
- ✅ **workingDir**: goosed startup uses `defaults.workingDir` as fallback
- ✅ **alwaysOnTop**: `BrowserWindow` reads `window.alwaysOnTop` from config
- ✅ **Tagline**: `Greeting.tsx` renders `branding.tagline` as subtitle

### 2. Home screen composition

The Hub needs to go from hardcoded layout to data-driven.

**Config shape:**
```yaml
branding:
  homeScreen:
    sections:
      - type: greeting
      - type: cards
        cards:
          - title: "Code Review"
            icon: "code"
            action: { type: recipe, recipeId: "code-review" }
          - title: "Internal Docs"
            icon: "book-open"
            action: { type: prompt, text: "Search our docs for..." }
          - title: "Dashboard"
            icon: "layout-dashboard"
            action: { type: app, name: "dashboard" }
      - type: recentChats
        maxItems: 5
      - type: stats
      - type: starterPrompts
```

**What to build:**
- `HomeScreenRenderer` — maps section types to components
- `CardsGrid` — new component. Grid of clickable cards with icon + title + description. Action dispatch: `recipe` → start recipe, `prompt` → inject into chat, `app` → open MCP app, `link` → external URL, `navigate` → internal route
- Default config (no `homeScreen` set) produces the exact current Hub layout
- Wire `starterPrompts` from branding into this same system

Card actions are just a switch statement dispatching to functions that already exist (`startNewSession` with recipe, `createSession` with initial message, navigate to app route, `window.open`).

### 3. Theme tokens

**Config shape:**
```yaml
theme:
  light:
    background-primary: "#ffffff"
    text-primary: "#1a1a2e"
    info: "#0066cc"
  dark:
    background-primary: "#0f0f1a"
  base:
    font-sans: "'Inter', sans-serif"
  fontFaces: |
    @font-face { font-family: 'BrandFont'; src: url('./brand/BrandFont.woff2') format('woff2'); }
```

**What to build:**
- `WhiteLabelTheme` type
- Vite plugin validates keys against `McpUiStyleVariableKey` at build
- `applyThemeTokens()` already iterates all tokens and sets `:root` properties — after it runs, overlay any whitelabel overrides
- `fontFaces` string gets injected as a `<style>` tag
- `buildMcpHostStyles()` picks up overrides automatically

The runtime change is ~10 lines in `renderer.tsx`.

### 4. Logo / asset pipeline

**What to build:**
- `<BrandLogo />` wrapper component: checks `branding.logo` from context, renders `<img>` if set, falls back to `<Goose />`. Replace the ~8 hardcoded `<Goose />` usages.
- Vite plugin: scan config for path-like values, copy referenced files to build output under `resources/brand/`, rewrite paths in baked config
- `main.ts`: register `brand://` custom protocol to serve from resources dir. Or simpler: resolve to `file://` paths relative to `app.getPath('resources')`.
- Tray icon: `main.ts` reads `branding.trayIcon` instead of hardcoded path

### 5. Config sealing

**Config shape:**
```yaml
defaults:
  sealProvider: true
  sealModel: true
  extensions:
    - name: "internal-tools"
      sealed: true
```

**What to build:**
- Add `sealed?: boolean` to `WhiteLabelExtensionDefault`, `sealProvider?: boolean` and `sealModel?: boolean` to `WhiteLabelDefaults`
- Context exposes `isExtensionSealed(name)`, `isProviderSealed`, `isModelSealed`
- Extension UI: check sealed → disable toggle, hide config button, show lock indicator
- Provider/model picker: check sealed → show value but disable switching
- `initProvider.ts`: skip reconciliation for sealed extensions (don't override user changes if sealed — the point is the initial state is locked)

### 6. Process visibility

**Config shape:**
```yaml
processes:
  - name: "auth-proxy"
    showInUI: true
    displayName: "Authentication"
    statusIcon: "shield"
```

**What to build:**
- `processManager.ts` sends IPC events: `{ name, state: 'starting' | 'running' | 'crashed' | 'stopped' }`
- `ProcessStatusPanel` component — small status bar or settings section showing visible processes with colored dots and restart buttons
- Gated by `showInUI` per process (default: hidden, backward-compatible)

---

## Dependency Order

```
1. Connect disconnected wires     ← no dependencies, enables everything else
2. Home screen composition        ← independent
3. Theme tokens                   ← independent
4. Logo / asset pipeline          ← needs asset resolution for home screen card images too
5. Config sealing                 ← independent (but needs provider filtering from #1)
6. Process visibility             ← independent
```

Items 2-6 are independent of each other. Item 1 should go first because it fixes the foundation.

---

## End State

```bash
WHITELABEL_CONFIG=./acme/whitelabel.yaml npm run make
```

Produces a `.dmg`/`.exe`/`.AppImage` where:
- Acme branding everywhere — name, logo, colors, fonts, greeting
- Home screen shows cards for internal recipes, apps, and quick actions
- Provider and model pre-configured and locked
- Internal extensions installed and sealed
- Sidecar processes running with visible status
- Irrelevant nav/settings hidden
- System prompt and skills baked into every session
- One artifact, no post-install configuration

---

## Known Bugs / Weird Behavior

### Agent restart loses all non-persisted prompt state

`restart_agent_internal` (in `crates/goose-server/src/routes/agent.rs`) does this:

1. Deletes the existing agent entirely (`agent_manager.remove_session`)
2. Creates a brand new agent
3. Restores provider, extensions, and recipe from the **Session** object

Anything set via `override_system_prompt()` or `extend_system_prompt()` that
wasn't driven by session-persisted data is silently lost. The recipe prompt
survives because `Session.recipe` is persisted. The system_prompt override
does not — it's only in-memory on the Agent's PromptManager.

This affects:
- **Whitelabel system prompt**: sent on start/resume, lost on restart. The agent
  reverts to default "you are goose" identity after an extension failure restart.
- **Any future `extend_system_prompt` callers** with non-persisted keys.

**Fix**: either persist `system_prompt` on the Session (like recipe is), or make
the restart endpoint accept `system_prompt` so the UI can re-send it. The former
is cleaner — the system prompt is part of the session's identity, not a transient
override. This would mean adding a `system_prompt_override: Option<String>` field
to Session, SessionUpdateBuilder, and the storage layer.

This isn't just a whitelabel problem — it's a general server architecture issue
where the Agent is treated as disposable but carries non-persisted state.

## Open Questions

1. **Runtime switching** — one build per brand is sufficient, or same binary needs to serve multiple configs?
2. **Extension install lockdown** — should sealed builds block installing *any* extension not in the approved list?
3. **Update channel** — white-labeled apps need their own update server. Configurable URL in YAML?
4. **Telemetry endpoint** — route to custom endpoint for whitelabel builds?
