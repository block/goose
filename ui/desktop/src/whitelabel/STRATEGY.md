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
| **System prompt** | `sessions.ts` builds a whitelabel `Recipe` injected into every new session |
| **Skills** | `sessions.ts` appends skill instructions to the recipe system prompt |
| **Tools** | `sessions.ts` appends tool instructions to the recipe system prompt |
| **Process management** | `processManager.ts` full lifecycle — spawn, stdout/stderr logging, restart-on-crash, port wait, `envFromUrl` for credential injection |

That's a substantial amount of working infrastructure. Provider setup, extension management, system prompt injection, process management, feature flags — the backend plumbing is solid.

### Defined but not wired

| Gap | What exists | What's missing |
|---|---|---|
| **Logo** | `branding.logo`, `logoSmall`, `trayIcon` fields in config | Every logo renders hardcoded `<Goose />` SVG (~8 component sites). No asset loading. `main.ts` icon is `path.join(__dirname, '../images/icon.icns')`. |
| **Tagline** | `branding.tagline` field | Only used as fallback in `sessions.ts`. Not displayed in any UI. |
| **Starter prompts** | `branding.starterPrompts` type + field | Zero consumers. `PopularChatTopics.tsx` has its own hardcoded `POPULAR_TOPICS` array. |
| **General provider restriction** | `features.allowedProviders` + `isProviderAllowed()` in context | Zero consumers outside context. The provider picker doesn't filter. (Dictation one *does* work.) |
| **Extension envVars** | `envVars` field on `WhiteLabelExtensionDefault` | `initProvider.ts` doesn't use it when building extension configs. |
| **Default goosehints** | `defaults.goosehints` field | Nothing reads it or writes a `.goosehints` file. |
| **Default workingDir** | `defaults.workingDir` field | `goosed.ts` defaults to `os.homedir()`. Config field is ignored. |
| **Window alwaysOnTop** | `window.alwaysOnTop` field | `main.ts` doesn't pass it to `BrowserWindow`. |

These are all small — each is a single-site wiring fix.

### Completely absent

| Capability | Description |
|---|---|
| **Home screen customization** | Hub is hardcoded: logo → greeting → stats → recent chats. No way to show cards for recipes, apps, quick actions. |
| **Theme/color control** | All design tokens hardcoded in `theme-tokens.ts`. No config surface. |
| **Asset bundling** | No build pipeline to copy brand images into Electron resources or resolve them at runtime. |
| **Config sealing** | Can set defaults but users can change provider, model, extensions freely. |
| **Process visibility** | Sidecar processes run silently. No status in UI, no restart controls. |

---

## The Work

### 1. Connect the disconnected wires

Each of these is a few lines in one file.

- **Starter prompts**: `PopularChatTopics.tsx` — read `branding.starterPrompts` from `useWhiteLabel()`, fall back to current hardcoded list.
- **Provider filtering**: find the provider picker component, add `isProviderAllowed()` filter.
- **Extension envVars**: `initProvider.ts` — when building SSE/stdio configs, include `envVars` from the whitelabel default.
- **goosehints**: `initProvider.ts` or session creation — if `defaults.goosehints` is set and no `.goosehints` file exists in working dir, write it.
- **workingDir**: `goosed.ts` — if `defaults.workingDir` is set, use it instead of `os.homedir()`.
- **alwaysOnTop**: `main.ts` — pass `whiteLabelConfig.window.alwaysOnTop` to `BrowserWindow`.
- **Tagline**: decide where it should show (home screen? loading screen?) and put it there.

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

## Open Questions

1. **Runtime switching** — one build per brand is sufficient, or same binary needs to serve multiple configs?
2. **Extension install lockdown** — should sealed builds block installing *any* extension not in the approved list?
3. **Update channel** — white-labeled apps need their own update server. Configurable URL in YAML?
4. **Telemetry endpoint** — route to custom endpoint for whitelabel builds?
