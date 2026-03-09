# White-Label System

The white-label system lets you declaratively configure the entire Goose desktop app from a single `whitelabel.yaml` file. When built, the config is baked into the binary — no runtime config files needed.

## Quick Start

1. Edit `ui/desktop/whitelabel.yaml` (or create a custom one)
2. Build: `npm run make`
3. The built app uses your config

To use a custom config file:
```bash
WHITELABEL_CONFIG=/path/to/myconfig.yaml npm run make
```

## What You Can Configure

### Branding
- **App name** — shown in title bar, menus, about dialog
- **Tagline** — welcome/loading screens
- **Logo** — custom SVG/PNG for the app icon
- **Greetings** — random messages on the home screen
- **Starter prompts** — quick-action cards on home screen

### Features (toggle on/off)
- Updates, cost tracking, announcements, configuration UI, telemetry UI
- **Navigation items** — choose which nav items to show and their order
- **Settings tabs** — choose which settings tabs to show
- **Hidden sections** — hide specific sections within settings tabs
- **Allowed providers** — restrict to specific LLM providers

### Defaults (pre-seed on first launch)
- Default provider and model
- Pre-installed extensions with config
- Working directory
- System prompt override
- Goosehints content

### Processes (sidecar management)
- Launch external processes alongside the app
- Auto-restart on crash
- Wait for port readiness before proceeding

### Window
- Default dimensions and min width
- Always-on-top, resizable

## Architecture

```
whitelabel.yaml          ← Your config file
       │
       ▼
vite-plugin.ts           ← Reads YAML at build time
       │
       ▼
__WHITELABEL_CONFIG__    ← Baked into JS as a global constant
       │
       ├──▶ WhiteLabelContext.tsx  ← React context for renderer
       ├──▶ main.ts               ← Electron main process
       └──▶ updates.ts            ← Feature flags
```

### Files
- `whitelabel.yaml` — the config file
- `types.ts` — TypeScript type definitions
- `defaults.ts` — default values when no config is provided
- `loader.ts` — Node.js YAML loader (build-time)
- `vite-plugin.ts` — Vite plugin that injects config at build time
- `WhiteLabelContext.tsx` — React context + `useWhiteLabel()` hook
- `processManager.ts` — sidecar process lifecycle management
- `global.d.ts` — TypeScript declaration for `__WHITELABEL_CONFIG__`

## Usage in Components

```tsx
import { useWhiteLabel } from '../whitelabel/WhiteLabelContext';

function MyComponent() {
  const { branding, isNavItemEnabled, isSettingsTabEnabled } = useWhiteLabel();

  return <h1>{branding.appName}</h1>;
}
```

## Example

See `whitelabel.example.yaml` for a full example of a white-labeled "Acme Assistant" build.
