# Proposal: TypeScript-First Theme Tokens for MCP Apps

**Date**: 2026-02-13
**Branch**: `mcp-apps-styling`
**Status**: Ready to implement

## Problem

The current PR introduces a Rust-based CSS parser (`theme_css.rs`) that:

1. Embeds `main.css` at compile time via `include_str!`
2. Regex-parses `:root {}` and `.dark {}` blocks to extract CSS variables
3. Resolves `var()` references recursively (up to 10 depth)
4. Merges with an optional user `theme.css` file from disk
5. Serves the result as `light-dark()` strings via a `/theme/variables` API route
6. The frontend fetches those strings, parses `light-dark()` back into light/dark values, and injects a `<style>` tag

This round-trip (**CSS → Rust regex → JSON API → JS parser → CSS**) is fragile and provides zero type safety. Renaming a variable breaks things silently at runtime.

## Solution

**A single TypeScript file (`theme-tokens.ts`) as the source of truth** for all MCP spec theme tokens, integrated with Tailwind v4 at runtime.

## Architecture

```
theme-tokens.ts (SOURCE OF TRUTH — typed, compiler-enforced)
    │
    ├──▶ main.css @theme inline {}
    │      Placeholder values (initial) so Tailwind generates
    │      utility classes: bg-background-primary, text-text-secondary, etc.
    │      Tailwind does NOT emit these in CSS output (that's what "inline" means).
    │
    ├──▶ ThemeContext (runtime injection)
    │      On mount + theme change: sets CSS vars on document.documentElement
    │      via root.style.setProperty() from lightTokens/darkTokens
    │
    ├──▶ McpAppRenderer → hostContext.styles.variables
    │      Encodes as light-dark() format per MCP spec
    │      Sent to MCP apps via AppRenderer SDK on ui/initialize
    │      Re-sent on host-context-changed when theme toggles
    │
    └──▶ main.css :root {} / .dark {}
           App-specific aliases (sidebar, highlight, etc.) stay in CSS
           as var() references to the semantic tokens. They resolve
           correctly because the TS-injected values are on the same :root.
```

## Token File Shape

```typescript
// src/theme/theme-tokens.ts

import type { McpUiStyleVariableKey } from '@modelcontextprotocol/ext-apps/app-bridge';

type ThemeTokens = Record<McpUiStyleVariableKey, string>;

export const lightTokens: ThemeTokens = {
  // Colors — backgrounds
  '--color-background-primary': '#ffffff',
  '--color-background-secondary': '#f4f6f7',
  '--color-background-tertiary': '#e3e6ea',
  '--color-background-inverse': '#000000',
  '--color-background-danger': '#fde8e8',
  '--color-background-info': '#e8f0fe',
  '--color-background-ghost': 'transparent',
  '--color-background-success': '#e6f4ea',
  '--color-background-warning': '#fef7e0',
  '--color-background-disabled': '#f4f6f7',

  // Colors — text
  '--color-text-primary': '#171717',
  '--color-text-secondary': '#6b7280',
  '--color-text-tertiary': '#9ca3af',
  '--color-text-inverse': '#ffffff',
  '--color-text-danger': '#dc2626',
  '--color-text-success': '#16a34a',
  '--color-text-warning': '#ca8a04',
  '--color-text-info': '#2563eb',
  '--color-text-disabled': '#d1d5db',
  '--color-text-ghost': '#6b7280',

  // Colors — borders
  '--color-border-primary': '#e5e7eb',
  '--color-border-secondary': '#f3f4f6',
  '--color-border-tertiary': '#d1d5db',
  '--color-border-inverse': '#000000',
  '--color-border-danger': '#dc2626',
  '--color-border-info': '#2563eb',
  '--color-border-ghost': 'transparent',
  '--color-border-success': '#16a34a',
  '--color-border-warning': '#ca8a04',
  '--color-border-disabled': '#e5e7eb',

  // Colors — rings
  '--color-ring-primary': '#2563eb',
  '--color-ring-secondary': '#e5e7eb',
  '--color-ring-inverse': '#ffffff',
  '--color-ring-info': '#2563eb',
  '--color-ring-danger': '#dc2626',
  '--color-ring-success': '#16a34a',
  '--color-ring-warning': '#ca8a04',

  // Typography
  '--font-sans': "'Inter', system-ui, sans-serif",
  '--font-mono': "'JetBrains Mono', monospace",
  '--font-weight-regular': '400',
  '--font-weight-medium': '500',
  '--font-weight-semibold': '600',
  '--font-weight-bold': '700',
  '--font-text-xs-size': '0.75rem',
  '--font-text-sm-size': '0.875rem',
  '--font-text-md-size': '1rem',
  '--font-text-lg-size': '1.125rem',
  '--font-text-xl-size': '1.25rem',
  '--font-text-xs-line-height': '1rem',
  '--font-text-sm-line-height': '1.25rem',
  '--font-text-md-line-height': '1.5rem',
  '--font-text-lg-line-height': '1.75rem',
  '--font-text-xl-line-height': '1.75rem',
  '--font-heading-sm-size': '1.25rem',
  '--font-heading-md-size': '1.5rem',
  '--font-heading-lg-size': '2rem',
  '--font-heading-sm-line-height': '1.75rem',
  '--font-heading-md-line-height': '2rem',
  '--font-heading-lg-line-height': '2.5rem',

  // Layout
  '--border-radius-sm': '4px',
  '--border-radius-md': '8px',
  '--border-radius-lg': '12px',
  '--border-radius-full': '9999px',
  '--border-width-regular': '1px',
  '--shadow-hairline': '0 0 0 1px rgba(0,0,0,0.05)',
  '--shadow-sm': '0 1px 2px rgba(0,0,0,0.05)',
  '--shadow-md': '0 4px 6px -1px rgba(0,0,0,0.1)',
  '--shadow-lg': '0 10px 15px -3px rgba(0,0,0,0.1)',
};

export const darkTokens: ThemeTokens = {
  // Every key must be present — compiler enforces it.
  '--color-background-primary': '#22252a',
  '--color-background-secondary': '#2a2d33',
  // ... all 73 keys required
};
```

**Key property**: If a key is added to `McpUiStyleVariableKey` in the SDK, TypeScript immediately errors on both `lightTokens` and `darkTokens` until values are provided. This is the type safety we're after.

## Tailwind Integration

### `@theme inline {}` in main.css

```css
@theme inline {
  /* Registered so Tailwind generates utility classes.
     "inline" means Tailwind won't emit these in CSS output —
     actual values are injected at runtime from theme-tokens.ts */
  --color-background-primary: initial;
  --color-background-secondary: initial;
  --color-text-primary: initial;
  /* ... all 73 McpUiStyleVariableKey entries ... */
}
```

**How `@theme inline` works in Tailwind v4**:
- `@theme` = register variable + emit it in `:root` in CSS output
- `@theme inline` = register variable (utilities generated) but **don't emit** in CSS output
- The variable is expected to exist in the DOM at runtime (which our ThemeContext provides)
- Placeholder value (`initial`) doesn't matter since it's never emitted

This gives us utility classes like `bg-background-primary`, `text-text-secondary`, `border-border-danger`, `rounded-radius-md`, `shadow-sm`, etc. — all resolved at runtime from the TS token values.

We use `initial` (not the self-referential `var()` trick from the original PR) because it's explicit and avoids any risk of circular resolution in browsers that might try to evaluate it.

### Primitive palette stays in `@theme {}`

The existing primitive tokens (`--color-neutral-50` through `--color-neutral-950`, brand colors, etc.) stay in the regular `@theme {}` block. These are Tailwind build-time primitives, not MCP spec tokens.

### App-specific aliases stay in CSS

Goose-internal aliases that aren't part of the MCP spec stay in `main.css`:

```css
:root {
  --sidebar: var(--color-background-secondary);
  --sidebar-foreground: var(--color-text-primary);
  --placeholder: var(--color-text-secondary);
  --highlight-color: rgba(255, 213, 0, 0.5);
  --shadow-default: 0px 12px 32px ...;
}

.dark {
  --highlight-color: rgba(255, 213, 0, 0.3);
  /* Most aliases don't need dark overrides since they
     reference semantic tokens that already switch values */
}
```

These resolve correctly because the TS-injected semantic tokens are set on the same `:root` element via `style.setProperty()`.

## Runtime Injection

### ThemeContext — Goose Desktop

Applies **resolved per-theme values** to the document:

```typescript
// In ThemeContext or a useThemeTokens hook
import { lightTokens, darkTokens } from '../theme/theme-tokens';

function applyTokensToDocument(theme: 'light' | 'dark') {
  const tokens = theme === 'dark' ? darkTokens : lightTokens;
  const root = document.documentElement;
  for (const [key, value] of Object.entries(tokens)) {
    root.style.setProperty(key, value);
  }
}
```

Called on mount and on theme change. Replaces the current `loadThemeVariables()` → API fetch → `parseLightDark()` → `injectThemeCSS()` chain.

### McpAppRenderer — MCP Apps

`mcpHostStyles` is built **once at module level** in `theme-tokens.ts` — since `light-dark()` values
encode both modes, they're theme-independent and never need recomputation:

```typescript
// theme-tokens.ts (module level, built once on import)
export function buildMcpHostStyles(): McpUiHostStyles {
  const variables = {} as McpUiStyles;
  for (const key of Object.keys(lightTokens) as McpUiStyleVariableKey[]) {
    variables[key] = `light-dark(${lightTokens[key]}, ${darkTokens[key]})`;
  }
  return { variables };
}
```

`ThemeContext` calls `buildMcpHostStyles()` once at module level and exposes it via context.
`McpAppRenderer` consumes it from `useTheme()` and passes it directly to `hostContext.styles`:

```typescript
// ThemeContext.tsx (module level)
const mcpHostStyles = buildMcpHostStyles();

// McpAppRenderer.tsx
const { resolvedTheme, mcpHostStyles } = useTheme();

const hostContext: McpUiHostContext = {
  theme: resolvedTheme,
  styles: mcpHostStyles,
  // ...
};
```

This also fixes the current bug where `host-context-changed` on theme toggle doesn't re-send style variables — since the `light-dark()` format encodes both modes, the variables don't need to change when the theme toggles. The MCP app just needs the updated `theme` field to flip `color-scheme`.

## What Gets Deleted

### Rust (server-side)
- `crates/goose-server/src/theme_css.rs` — entire file (~200 lines)
- `GET /theme/variables` route in `config_management.rs`
- `POST /theme/save` route in `config_management.rs`
- `ThemeVariablesResponse` and `SaveThemeRequest` structs
- `mod theme_css` and `generate_mcp_theme_variables` re-export in `lib.rs` / `main.rs`
- Related OpenAPI schema entries

### Frontend
- `loadThemeVariables()` in ThemeContext (API fetch)
- `parseLightDark()` in ThemeContext
- `injectThemeCSS()` in ThemeContext
- `getThemeVariables` / `saveTheme` SDK calls

### Generated
- OpenAPI spec entries for `/theme/variables` and `/theme/save`
- Generated SDK types/functions for those routes

## MCP Spec Coverage

The `McpUiStyleVariableKey` type from `@modelcontextprotocol/ext-apps` defines ~73 tokens across:

| Category | Count | Examples |
|----------|-------|---------|
| Background colors | 10 | `--color-background-primary`, `--color-background-ghost` |
| Text colors | 10 | `--color-text-primary`, `--color-text-disabled` |
| Border colors | 10 | `--color-border-primary`, `--color-border-ghost` |
| Ring colors | 7 | `--color-ring-primary`, `--color-ring-warning` |
| Typography | 22 | `--font-sans`, `--font-weight-bold`, `--font-text-md-size` |
| Border radius | 4 | `--border-radius-sm`, `--border-radius-full` |
| Border width | 1 | `--border-width-regular` |
| Shadows | 4 | `--shadow-hairline`, `--shadow-lg` |
| Font families | 2 | `--font-sans`, `--font-mono` |

The current PR defines ~25 of these. The TS approach provides **compiler-enforced 100% coverage** — if the SDK adds a new key, the build breaks until both `lightTokens` and `darkTokens` are updated.

## Implementation Steps

1. **Create `src/theme/theme-tokens.ts`** — Define `lightTokens` and `darkTokens` with full `McpUiStyleVariableKey` coverage, importing the type from `@modelcontextprotocol/ext-apps/app-bridge`
2. **Update `main.css`** — Replace the `@theme inline {}` semantic tokens with `initial` placeholders for all 73 keys. Keep primitive palette in `@theme {}`. Keep app aliases in `:root {}` / `.dark {}`
3. **Update `ThemeContext`** — Replace `loadThemeVariables` / `parseLightDark` / `injectThemeCSS` with `applyTokensToDocument()` that reads from the TS object
4. **Update `McpAppRenderer`** — Populate `hostContext.styles.variables` with `light-dark()` encoded values from the TS objects
5. **Delete Rust code** — Remove `theme_css.rs`, API routes, structs, module declarations, OpenAPI entries
6. **Regenerate OpenAPI** — `just generate-openapi`
7. **Verify** — Build, lint, test that Tailwind utilities resolve correctly, MCP apps receive style variables

## User Theme Customization (Future)

The current PR includes plumbing for a `ThemeColorEditor` modal (referenced in `AppSettingsSection.tsx`)
and a server-side `theme.css` file, but neither is functional yet. Our approach replaces that plumbing
with a simpler, type-safe pattern.

### Approach: `Partial<ThemeTokens>` in localStorage

User overrides are stored as a JSON blob in `localStorage` — no server, no CSS file, no API routes.

```typescript
// Shape of stored overrides
interface ThemeOverrides {
  light?: Partial<ThemeTokens>;
  dark?: Partial<ThemeTokens>;
}

// localStorage key
const THEME_OVERRIDES_KEY = 'theme-overrides';
```

### How it works on app launch

```typescript
// In ThemeContext, synchronous on mount (no flash of default theme):
const stored = localStorage.getItem('theme-overrides');
const overrides: ThemeOverrides = stored ? JSON.parse(stored) : {};

const effectiveLight = { ...lightTokens, ...overrides.light };
const effectiveDark = { ...darkTokens, ...overrides.dark };

applyTokensToDocument(resolvedTheme, effectiveLight, effectiveDark);
```

Because `localStorage` is **synchronous**, overrides are applied before first paint.
No API call, no async fetch, no flash of un-customized theme.

### How the ThemeColorEditor would work

```
Settings > Appearance > "Customize Colors"
    └── ThemeColorEditor modal
            │
            ├── Renders color pickers for each token category
            ├── Type-safe: picks are Partial<ThemeTokens>
            ├── Live preview: calls applyTokensToDocument() on change
            │
            ├── "Save" → localStorage.setItem('theme-overrides', JSON.stringify({...}))
            ├── "Reset" → localStorage.removeItem('theme-overrides')
            └── "Cancel" → re-apply previous tokens
```

### Why localStorage over a CSS file

| | localStorage (our approach) | CSS file (current PR approach) |
|---|---|---|
| **Type safety** | ✅ `Partial<ThemeTokens>` — compiler checks keys | ❌ Raw CSS, no validation |
| **Load timing** | ✅ Synchronous, no flash | ❌ Async API fetch on mount |
| **Server involvement** | ✅ None | ❌ Rust parser + 2 API routes |
| **Specificity** | ✅ `style.setProperty()` = inline, wins naturally | ❌ Needs CSS layers or careful ordering |
| **Per-device** | ⚠️ Yes, doesn't roam | ⚠️ Same (file is on local disk) |

### Why not CSS layers / user CSS file

A power-user escape hatch (load raw CSS into a `@layer user {}`) could be added later alongside
this approach. But it's not needed for the color editor use case, and it reintroduces file I/O
and loses type safety on override values.

### Scope

A basic `ThemeColorEditor` playground was built (`src/components/settings/app/ThemeColorEditor.tsx`)
to enable live experimentation with all 76 MCP UI variables. It uses the localStorage override
pattern described above, with live preview and save/reset/cancel support. It's wired into
Settings > Appearance via the "Customize Colors" button in `AppSettingsSection.tsx`.

## Cross-Window Token Sync

Electron runs multiple renderer windows (chat windows, settings, etc.), each with its own DOM.
When the `ThemeColorEditor` saves overrides in one window, all other windows must update.

### Problem

`localStorage` is shared across windows, but there's no automatic DOM update when another window
writes to it. The `storage` event only fires in *other* windows, not the one that wrote.

### Solution: Electron IPC broadcast via `refreshTokens()`

```
ThemeColorEditor saves → refreshTokens()
    │
    ├── 1. Increments tokenVersion state (triggers useEffect in current window)
    │
    └── 2. Broadcasts via Electron IPC:
           window.electron.broadcastThemeChange({
             mode, useSystemTheme, theme, tokensUpdated: true
           })
           │
           └── All other windows receive 'theme-changed' event
               → See tokensUpdated flag
               → Increment their own tokenVersion
               → useEffect fires, re-reads localStorage, re-applies tokens
```

### Implementation details

- **`tokenVersion`**: A counter in ThemeContext state. Incrementing it triggers the `useEffect`
  that reads localStorage and applies tokens. This avoids needing to compare token values.
- **`refreshTokens()`**: Exposed via ThemeContext. Increments `tokenVersion` + broadcasts IPC.
  Called by `ThemeColorEditor` on save and reset.
- **`tokensUpdated` flag**: Added to the `broadcastThemeChange` IPC payload (optional boolean).
  Receiving windows check this flag to know they need to re-apply tokens, not just theme preference.
- **`preload.ts`**: Updated `broadcastThemeChange` type signature to include `tokensUpdated?: boolean`.

### Why not just use the `storage` event?

The `storage` event would work for cross-window sync without IPC, but:
- It doesn't fire in the window that wrote the value (need separate handling)
- It's less reliable in Electron than native IPC
- We already have the `broadcastThemeChange` IPC pattern for theme preference changes
- Piggybacking on it keeps the sync mechanism consistent

## main.css Cleanup

The original `main.css` mixes five concerns with no clear boundaries. The restructured file
uses explicit section headers so developers know exactly what each block is for and where
new variables should go.

### Section Layout

```
┌─────────────────────────────────────────────────────────────┐
│  1. TAILWIND IMPORTS & CONFIG                               │
│     @import, @source, @plugin, @custom-variant              │
├─────────────────────────────────────────────────────────────┤
│  2. TAILWIND PRIMITIVES — @theme {}                         │
│     Palette colors (neutral-50..950, red, blue, etc.)       │
│     Breakpoints, shadow reset                               │
│     These are build-time values Tailwind uses directly.     │
│     NOT part of the MCP spec.                               │
├─────────────────────────────────────────────────────────────┤
│  3. MCP SPEC TOKEN REGISTRATION — @theme inline {}          │
│     All 76 McpUiStyleVariableKey tokens registered here     │
│     so Tailwind generates utility classes.                  │
│     Color tokens: self-referential var() for Tailwind       │
│     Non-color tokens: initial                               │
│     NO actual values — just registration.                   │
├─────────────────────────────────────────────────────────────┤
│  4. GOOSE APP ALIASES — @theme inline {}                    │
│     App-specific variables NOT in the MCP spec:             │
│     --color-background-accent, --font-serif, --ease-g2,    │
│     sidebar aliases, --shadow-default, etc.                 │
├─────────────────────────────────────────────────────────────┤
│  5. CSS BASELINE — :root {}                                 │
│     MCP token values for light theme (pre-React fallback)   │
│     + goose app aliases (sidebar, highlight, legacy --text) │
│     Source of truth is theme-tokens.ts — keep in sync.      │
├─────────────────────────────────────────────────────────────┤
│  6. CSS BASELINE — .dark {}                                 │
│     Same as above but for dark theme.                       │
├─────────────────────────────────────────────────────────────┤
│  7. BASE LAYER + FONTS + ANIMATIONS + COMPONENTS            │
│     @layer base, @font-face, @keyframes, scrollbars,        │
│     toasts, search, KaTeX, etc. — unchanged.                │
└─────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **MCP spec tokens are clearly separated from goose-internal aliases.** A developer adding
   a new MCP token knows to update `theme-tokens.ts` (source of truth), the `@theme inline`
   registration (section 3), and the `:root`/`.dark` baselines (sections 5–6).

2. **`:root` and `.dark` blocks each appear exactly once.** The original file had MCP tokens,
   legacy aliases, sidebar aliases, highlights, and shadows all jumbled in one `:root` block.
   Now each `:root`/`.dark` block has labeled sub-sections.

3. **Legacy aliases (`--text-default`, `--text-muted`, etc.) are marked for deprecation.**
   They duplicate MCP semantic tokens and should be migrated to `--color-text-*` equivalents.

4. **Comments explain the "why", not the "what".** Each section header explains its role in
   the architecture (e.g., "registered here so Tailwind generates utility classes" rather than
   just "theme variables").

### What changes vs the original

| Aspect | Before | After |
|--------|--------|-------|
| Section headers | None | Clear `/* ═══ SECTION ═══ */` dividers |
| `:root` block | One giant block mixing all concerns | Sub-sections: MCP colors, MCP non-color, legacy, app aliases |
| `.dark` block | Same jumble | Same sub-sections |
| `@theme inline` | MCP + goose aliases mixed together | Two separate blocks with different headers |
| Duplicate scrollbar rules | Yes (lines 640-700) | Deduplicated |
| Legacy aliases | Unmarked | Marked `/* LEGACY — migrate to --color-text-* */` |

## Decisions Made

- **Goose desktop**: Per-theme resolved values (`lightTokens` or `darkTokens` based on `resolvedTheme`)
- **MCP apps**: `light-dark()` format via `hostContext.styles.variables`
- **App aliases**: Stay in CSS as `var()` references — no type safety needed for internal wiring
- **User theme customization**: localStorage-based `Partial<ThemeTokens>` overrides, merged on mount (synchronous, no flash). Basic `ThemeColorEditor` playground included.
- **`@theme inline`**: Color tokens use self-referential `var()` (e.g., `--color-border-primary: var(--color-border-primary)`) so Tailwind v4 recognizes them as colors and generates correct utility classes. Non-color tokens (fonts, radii, shadows) use `initial` since they don't need Tailwind utility generation.
- **`mcpHostStyles`**: Built once at module level since `light-dark()` values are theme-independent — no recomputation on theme toggle
