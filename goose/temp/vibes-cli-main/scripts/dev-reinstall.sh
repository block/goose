#!/bin/bash

# dev-reinstall.sh - Clean reinstall of vibes-cli plugin for development testing
#
# Use this when Claude Code isn't picking up your latest changes.
# Run this script, then restart Claude Code.

set -e

echo "Cleaning vibes-cli plugin cache..."

# Remove cached plugin
rm -rf ~/.claude/plugins/cache/vibes-cli
echo "  Removed cache/vibes-cli"

# Remove marketplace cache
rm -rf ~/.claude/plugins/marketplaces/vibes-cli
echo "  Removed marketplaces/vibes-cli"

# Remove from installed_plugins.json
INSTALLED_PLUGINS=~/.claude/plugins/installed_plugins.json
if [ -f "$INSTALLED_PLUGINS" ]; then
  # Use jq if available, otherwise warn user
  if command -v jq &> /dev/null; then
    jq 'del(.plugins["vibes@vibes-cli"])' "$INSTALLED_PLUGINS" > "${INSTALLED_PLUGINS}.tmp" && mv "${INSTALLED_PLUGINS}.tmp" "$INSTALLED_PLUGINS"
    echo "  Removed vibes@vibes-cli from installed_plugins.json"
  else
    echo "  WARNING: jq not installed. Manually remove 'vibes@vibes-cli' from:"
    echo "    $INSTALLED_PLUGINS"
  fi
fi

# Remove from known_marketplaces.json
KNOWN_MARKETPLACES=~/.claude/plugins/known_marketplaces.json
if [ -f "$KNOWN_MARKETPLACES" ]; then
  if command -v jq &> /dev/null; then
    jq 'del(.["vibes-cli"])' "$KNOWN_MARKETPLACES" > "${KNOWN_MARKETPLACES}.tmp" && mv "${KNOWN_MARKETPLACES}.tmp" "$KNOWN_MARKETPLACES"
    echo "  Removed vibes-cli from known_marketplaces.json"
  else
    echo "  WARNING: jq not installed. Manually remove 'vibes-cli' from:"
    echo "    $KNOWN_MARKETPLACES"
  fi
fi

echo ""
echo "Done! Now:"
echo "  1. Restart Claude Code"
echo "  2. Run: /plugin marketplace add popmechanic/vibes-cli"
echo "  3. Run: /plugin install vibes@vibes-cli"
