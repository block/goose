# Branding Assets

`product-metadata.json` is the current Goal 3 source of truth for:

- branded app name
- macOS preview bundle id
- default desktop locale

Current conflict with the Goal 3 checklist:

- the repo does not yet contain replacement icon or splash assets

Minimal non-breaking resolution in this branch:

- wire name, locale, bundle metadata first
- keep reusing upstream Goose icon files in `ui/desktop/src/images/`
- add real replacement assets here before switching forge/icon paths

Expected future files:

- `icon.icns`
- `icon.png`
- `icon.svg`
- `tray-template.png`
