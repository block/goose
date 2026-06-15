#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source bin/activate-hermit

CI=1 pnpm --dir ui install --frozen-lockfile --ignore-scripts
pnpm --dir ui --filter @aaif/goose-sdk run build

node scripts/sync-security-runtime-assets.mjs
node ui/desktop/scripts/ensure-goosed-dev.js
node scripts/smoke-security-extensions.mjs
node scripts/check-security-apple-signing-env.mjs

pnpm --dir ui/desktop exec vitest run \
  src/branding/distro.test.ts \
  src/branding/productText.test.ts \
  src/macosBundleMode.test.ts \
  src/macosSigningReadiness.test.ts \
  src/components/settings/models/predefinedModelsUtils.test.ts \
  src/components/settings/extensions/bundled-extensions.test.ts \
  src/components/settings/extensions/securityBundledExtensions.test.ts \
  src/security/taskCatalog.test.ts \
  src/securityRuntimeBootstrap.test.ts \
  src/components/LauncherView.test.tsx \
  src/components/recipes/RecipesView.test.tsx

pnpm --dir ui/desktop exec tsc --noEmit
pnpm --dir ui/desktop run lint:check
node scripts/validate-security-distro.mjs
git diff --check
