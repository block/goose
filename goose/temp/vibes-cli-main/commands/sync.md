---
name: sync
description: Sync cached style prompts, import maps, and CSS variables from upstream sources
argument-hint: "[--force]"
---

# Sync Vibes DIY

This command syncs cached assets from upstream Vibes DIY sources.

## Instructions

Run the sync script from the plugin root:

```bash
cd "${CLAUDE_PLUGIN_ROOT}/scripts" && [ -d node_modules/esbuild ] || npm install
cd "${CLAUDE_PLUGIN_ROOT}" && node scripts/sync.js --force
```

This updates:
- Style prompts from vibes.diy repository
- Import map versions (React, use-vibes, @fireproof/clerk)
- CSS variables for theming

## Building Components

**Note:** Menu components (VibesSwitch, VibesPanel, etc.) are now built from local
sources in the `components/` directory, not synced from upstream.

To rebuild components and update templates:

```bash
cd "${CLAUDE_PLUGIN_ROOT}" && node scripts/build-components.js && node scripts/merge-templates.js
```

## When to Use

Run `/vibes:sync` when:
- The skill warns that the cache is older than 30 days
- You want the latest style guidance
- There's a security update to dependencies

Run `build-components.js` when:
- You've modified local components in `components/`
- You want to regenerate templates with updated components

## What Gets Synced

- **Style prompt** - UI styling guidance
- **Import map** - Package versions and CDN URLs
- **CSS variables** - Theme colors and animations

## Cache Staleness

The skill will warn you if the cache is older than 30 days.
