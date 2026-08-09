# Desktop Release Runbook — Avocado Work

> **Type**: Manual Test / Release Runbook  
> **Category**: Distribution  
> **Related**: [avcd-agent-custom-distro skill](../../../.cursor/skills/architecture/avcd-agent-custom-distro/SKILL.md) | `scripts/verify-release-assets.sh`

## Overview

How to cut unsigned internal beta and signed public desktop releases for Avocado Work from `Avocado-Technology/avcd-agent`, and how to verify website download links.

## Track B — Signing prerequisites (human)

| Item | Owner | Notes |
|------|-------|-------|
| Apple Developer Program | Human | Feeds `APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_ID_PASSWORD`, `APPLE_TEAM_ID` |
| Azure Trusted Signing (or OV/EV cert) | Human | Feeds `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `AZURE_SIGNING_*` |
| GitHub `signing` environment secrets | Human | Repo Settings → Environments → `signing` |
| `ENABLE_MAC_NATIVE_AUTO_UPDATE` | Human | Keep `false` for v1 (assisted download via GitHubUpdater) |

## Canonical release asset names

| Platform | Website | Update check |
|----------|---------|--------------|
| macOS arm64 | `Avocado Work.dmg` | `Avocado Work.zip` |
| macOS x64 | `Avocado Work_intel_mac.dmg` | `Avocado Work_intel_mac.zip` |
| Windows x64 | `Avocado Work-Setup-x64.exe` | same file |

Source of truth: `ui/desktop/scripts/release-assets.js`.

## Preflight (every release)

```bash
source bin/activate-hermit
bash scripts/verify-release-assets.sh
bash scripts/verify-release-assets.sh --guards
make test-smoke   # requires local Docker ACP + packaged app
```

## Path A — Unsigned internal beta

1. Create `release/X.Y.Z` and run `just prepare-release X.Y.Z` (or the repo's release-branch workflow).
2. Merge the release PR.
3. Tag and push (`just tag-push`) **without** the `signing` environment secrets populated.
4. Confirm artifacts with:
   ```bash
   gh release view stable --repo Avocado-Technology/avcd-agent
   ```
5. Distribute internally. Keep `NEXT_PUBLIC_RELEASE_SIGNED=false` on avocado.tech `/download`.

## Path B — Signed public release

1. Complete Track B secrets in the GitHub `signing` environment.
2. Re-run the release tag workflow with signing enabled (or cut a new patch tag).
3. Verify:
   - macOS: `spctl -a -vvv -t install` on the app; `xcrun stapler validate` on the DMG
   - Windows: `Get-AuthenticodeSignature` Valid on `avocado-work.exe` and `Avocado Work-Setup-x64.exe`
4. Set landing `NEXT_PUBLIC_RELEASE_SIGNED=true` and ship via the landingpage CI/CD deploy workflow.
5. Keep `ENABLE_MAC_NATIVE_AUTO_UPDATE=false` until a follow-up enables native electron-updater.

## E2E-C — Install journey checklist

### macOS (clean VM)

- [ ] Open avocado.tech `/download`
- [ ] Download Apple Silicon (or Intel) DMG
- [ ] Drag Avocado Work to Applications and launch
- [ ] Complete onboarding; chat replies with OpenRouter key
- [ ] Help → Check for updates behaves (assisted download; no silent install claim)

### Windows 11 (clean VM)

- [ ] Download `Avocado Work-Setup-x64.exe` from `/download`
- [ ] Run installer; launch from Start menu
- [ ] Chat replies with OpenRouter key
- [ ] Check for updates downloads the Setup.exe to Downloads

## Rollback

- Point `stable` tag/assets back to the previous good release via a new release workflow run.
- On the website, set `NEXT_PUBLIC_RELEASE_SIGNED=false` if a signed build is retracted.
- Do not hot-patch binaries on GitHub without a new tag.

## Website env

```
NEXT_PUBLIC_DESKTOP_RELEASE_OWNER=Avocado-Technology
NEXT_PUBLIC_DESKTOP_RELEASE_REPO=avcd-agent
NEXT_PUBLIC_DESKTOP_RELEASE_TAG=stable
NEXT_PUBLIC_RELEASE_SIGNED=false
```

---

**Last Updated**: 2026-08-09  
**Status**: Draft  
**Maintainer**: Avocado Technology
