---
name: update
description: Update an existing Vibes app to the latest plugin version without regenerating code
argument-hint: "<file.html> [--apply] [--backup]"
---

# vibes update

Deterministically update a Vibes app's infrastructure (import maps, library versions, components) while preserving your custom code.

## When to Use

Use this command when you have:
- An existing Vibes app that was generated with an older plugin version
- Import map versions that need updating
- Pattern issues like missing `?external=` parameters
- Components (VibesSwitch, HiddenMenuWrapper) that need updating

## Usage

```bash
# Analyze an app (dry-run, no changes made)
node "${CLAUDE_PLUGIN_ROOT}/scripts/update.js" path/to/app.html

# Apply all recommended updates
node "${CLAUDE_PLUGIN_ROOT}/scripts/update.js" path/to/app.html --apply

# Apply specific updates by number
node "${CLAUDE_PLUGIN_ROOT}/scripts/update.js" path/to/app.html --apply=1,2

# Batch analyze a directory
node "${CLAUDE_PLUGIN_ROOT}/scripts/update.js" ./apps/

# Restore from backup
node "${CLAUDE_PLUGIN_ROOT}/scripts/update.js" --rollback path/to/app.html

# Show detailed diffs
node "${CLAUDE_PLUGIN_ROOT}/scripts/update.js" path/to/app.html --verbose
```

## What Gets Updated

### Import Maps
- Library version upgrades (use-vibes, use-fireproof)
- React singleton fixes (`?deps=` → `?external=`)
- Missing `?external=` parameters

### Components (vibes-basic only)
- VibesSwitch updates (improved animations)
- HiddenMenuWrapper updates (better slide animation)

### Config (sell only)
- Missing CONFIG fields added with placeholders

## How It Works

1. **Analyze**: Parses your app to detect template type, versions, and patterns
2. **Compare**: Diffs against current plugin state
3. **Plan**: Shows available updates with priorities
4. **Execute**: Applies deterministic transforms (regex replacements)

## Safety Features

- **Dry-run by default**: No changes without `--apply`
- **Backups**: Creates `.bak.html` before any modification
- **Rollback**: Use `--rollback` to restore from backup
- **Conservative matching**: Fails if patterns don't match exactly
- **Preserves custom code**: Never touches content between app code markers

## Example Output

```
vibes update analysis for: my-app.html

Current state:
  Template: vibes-basic
  use-vibes: 0.20.0
  Era: 0.20.x

Available updates:
  [1] Update import map
      Update library versions to latest
      • useVibes: 0.20.0 → 0.24.3-dev
      [Recommended]

  [2] Update VibesSwitch component
      Update to latest VibesSwitch with improved animations
      [Optional]

Run with --apply to execute all updates
Run with --apply=1,2 to execute specific updates
```

## Notes

- Run `vibes sync` first to ensure your plugin cache is up to date
- Sell templates have a different update path than vibes-basic
- Components are only updated if they haven't been modified
- Use `--verbose` to see exactly what will change before applying
