# Avocado Work branding checklist

Use before merging rebrand changes or cutting a desktop release.

## Identity

- [ ] Display name is **Avocado Work** everywhere user-visible
- [ ] Executable / Linux package name is **avcd-agent**
- [ ] Protocol scheme is **avcd-agent** (not `goose`)
- [ ] System prompt persona says Avocado Work + Avocado Technology, with goose/AAIF attribution
- [ ] No accidental “Goose.app” / `executableName: goose` left in packaging scripts

## Desktop metadata

- [ ] `ui/desktop/forge.config.ts` — `name`, `executableName`, icons, protocols, makers, publisher
- [ ] `ui/desktop/package.json` — `productName`, `description`, bundle scripts use `GOOSE_BUNDLE_NAME`
- [ ] `ui/desktop/index.html` — `<title>Avocado Work</title>`
- [ ] `ui/desktop/forge.deb.desktop` / `forge.rpm.desktop` — `Name=Avocado Work`
- [ ] `ui/desktop/src/app-update.yml` — not pointing at `aaif-goose`
- [ ] `ui/desktop/vite.main.config.mts` defaults:
  - `GITHUB_OWNER` → `Avocado-Technology`
  - `GITHUB_REPO` → `avcd-agent`
  - `GOOSE_BUNDLE_NAME` → `Avocado Work`

## Visuals

- [ ] `ui/desktop/src/images/icon.{png,ico,icns}` and related templates updated
- [ ] Dock / tray template icons not still stock goose if replaced elsewhere
- [ ] Loading / splash assets reviewed under `ui/desktop/src/images/loading-goose/`

## Copy / i18n

- [ ] `ui/desktop/src/main.ts` menus (“About Avocado Work”, hide/focus, error boxes)
- [ ] `ui/desktop/src/i18n/messages/en.json` strings that name the product
- [ ] Ran `make sync-i18n` if English messages changed and that is required for other locales

## Core / CLI

- [ ] `crates/goose/src/prompts/system.md` rebranded
- [ ] Telemetry: upstream PostHog project key absent (`scripts/smoke-test.sh` check)
- [ ] `LICENSE` unchanged vs upstream; `NOTICE` present with Apache 2.0 attribution

## Packaging & updater consistency

Set the same values in CI and local release shells:

```bash
export GITHUB_OWNER="Avocado-Technology"
export GITHUB_REPO="avcd-agent"
export GOOSE_BUNDLE_NAME="Avocado Work"
```

- [ ] Forge publisher repository == Vite-injected updater repository
- [ ] Zip / `.app` basename == `GOOSE_BUNDLE_NAME`
- [ ] GitHub Release asset names match what `githubUpdater.ts` expects
- [ ] Linux desktop files point at `avcd-agent` binary

## Verification commands

```bash
make validate-openrouter   # provider preset (unrelated to icons, but part of distro readiness)
make package-ui            # when you need a local .app for smoke
make test-smoke            # 9 checks including branding + updater fork target
make check-ui              # optional before PR
```

Smoke branding assertions (from `scripts/smoke-test.sh`):

1. Bundle `Avocado Work.app` exists under `ui/desktop/out`
2. `CFBundleName` == `Avocado Work` and executable name does not contain `Goose`
3. Updater / publisher not targeting `aaif-goose`
4. `index.html` title + `system.md` persona rebranded
5. LICENSE/NOTICE attribution OK

## PR review questions

1. Did any upstream merge reintroduce “Goose” product strings?
2. Are Vite define defaults still aligned with Forge?
3. Would a fresh clone with only `.env.local.example` document how to get a key?
4. Does `CUSTOM_DISTROS.md` §D still describe steps this fork follows?
