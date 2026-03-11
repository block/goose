#!/usr/bin/env bash
# Verify package-lock.json has cross-platform optional dependency entries.
#
# npm has a bug where running `npm install` with an existing node_modules/
# prunes platform-specific entries from the lockfile, breaking CI on other platforms.
# See: https://github.com/npm/cli/issues/4828
#

set -euo pipefail

LOCKFILE="${1:-package-lock.json}"

fail=0
grep -q '"node_modules/@esbuild/win32-x64"' "$LOCKFILE" || { echo "MISSING: @esbuild/win32-x64"; fail=1; }
grep -q '"node_modules/@esbuild/linux-x64"' "$LOCKFILE" || { echo "MISSING: @esbuild/linux-x64"; fail=1; }

if [ "$fail" -eq 1 ]; then
  echo ""
  echo "ERROR: package-lock.json is missing cross-platform optional dependencies."
  echo "This happens when 'npm install' is run with an existing node_modules/ directory."
  echo ""
  echo "To fix, run from ui/desktop/:"
  echo "  rm -rf node_modules package-lock.json"
  echo "  npm install"
  echo ""
  echo "Then commit the regenerated package-lock.json."
  exit 1
fi

echo "OK: package-lock.json has cross-platform entries"
