#!/bin/bash
# Force-update the vibes-cli plugin in Claude Code
#
# Usage: ./scripts/dev-update.sh
#
# This script works around Claude Code's marketplace caching issues by:
# 1. Pulling latest commits to the marketplace clone
# 2. Clearing the plugin cache
# 3. Updating the installed version tracking
#
# After running, restart Claude Code for changes to take effect.

set -e

MARKETPLACE_DIR="$HOME/.claude/plugins/marketplaces/vibes-cli"
CACHE_DIR="$HOME/.claude/plugins/cache/vibes-cli"
INSTALLED_PLUGINS="$HOME/.claude/plugins/installed_plugins.json"

echo "=== Vibes CLI Plugin Force Update ==="
echo

# 1. Update marketplace clone
if [ -d "$MARKETPLACE_DIR" ]; then
  echo "Updating marketplace clone..."
  cd "$MARKETPLACE_DIR"
  git fetch origin
  git reset --hard origin/main
  NEW_VERSION=$(grep '"version"' .claude-plugin/plugin.json | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')
  NEW_COMMIT=$(git rev-parse HEAD)
  echo "  → Version: $NEW_VERSION"
  echo "  → Commit: $NEW_COMMIT"
else
  echo "ERROR: Marketplace not found at $MARKETPLACE_DIR"
  echo "Run: /plugin marketplace add popmechanic/vibes-cli"
  exit 1
fi

# 2. Clear plugin cache
if [ -d "$CACHE_DIR" ]; then
  echo
  echo "Clearing plugin cache..."
  rm -rf "$CACHE_DIR"
  echo "  → Cache cleared"
else
  echo
  echo "No cache to clear"
fi

# 3. Update installed_plugins.json if it exists
if [ -f "$INSTALLED_PLUGINS" ]; then
  echo
  echo "Updating installed_plugins.json..."
  # Use node to safely update JSON
  node -e "
    const fs = require('fs');
    const data = JSON.parse(fs.readFileSync('$INSTALLED_PLUGINS', 'utf8'));
    if (data['vibes@vibes-cli']) {
      data['vibes@vibes-cli'].version = '$NEW_VERSION';
      data['vibes@vibes-cli'].gitCommitSha = '$NEW_COMMIT';
      fs.writeFileSync('$INSTALLED_PLUGINS', JSON.stringify(data, null, 2));
      console.log('  → Updated vibes@vibes-cli to $NEW_VERSION');
    } else {
      console.log('  → Plugin not in installed_plugins.json (will be added on install)');
    }
  "
fi

echo
echo "=== Done ==="
echo
echo "Next steps:"
echo "  1. Restart Claude Code"
echo "  2. Run: /plugin install vibes@vibes-cli"
echo
