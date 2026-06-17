# Branding Assets

`product-metadata.json` remains the source of truth for:

- branded app name
- macOS preview bundle id
- default desktop locale

The desktop icon family now has a separate canonical source:

- `ui/desktop/src/images/brand-mark.svg`

That file is pinned to IBM Carbon `AiEnabledEdt`, which is the approved replacement for the old
Goose bird mark in this fork.

Generated desktop assets:

- `ui/desktop/src/images/glyph.svg`
- `ui/desktop/src/images/icon.svg`
- `ui/desktop/src/images/icon.png`
- `ui/desktop/src/images/icon.icns`
- `ui/desktop/src/images/icon.ico`
- `ui/desktop/src/images/iconTemplate*.png`
- `ui/desktop/src/images/iconTemplateUpdate*.png`

Regeneration flow:

- run `ui/desktop/src/images/prepare.sh`
- it first rebuilds `glyph.svg` and `icon.svg` from `brand-mark.svg`
- it then refreshes the PNG / ICNS / ICO / tray assets used by Electron Forge and runtime tray/dock icons

In-app brand icon scope:

- replace the old Goose mascot/logo in sidebar, onboarding, loading, recipe landing, and chat watermark
- keep provider logos, third-party marks, and generic action icons on their existing icon systems
