---
title: Chinese Language Support Roadmap
description: Scope, milestones, and acceptance criteria for zh-Hans and zh-Hant support
---

# Chinese Language Support Roadmap

This roadmap tracks phased delivery for Chinese language support across goose surfaces.

## Scope

- Phase 1:
  - Documentation i18n (`en`, `zh-Hans`, `zh-Hant`)
  - Desktop renderer i18n with automatic detection and manual override
  - Core onboarding/provider/settings/extensions coverage
- Not in Phase 1:
  - CLI localization
  - Electron native menu localization
  - System notification localization

## Milestones

1. Documentation i18n infrastructure and locale switcher
2. First translated docs batch (getting started and core guides)
3. Localized key landing pages
4. Desktop runtime i18n and persisted language preference
5. Desktop key UI translation and selector hardening for tests

## Acceptance Criteria

- Locale switcher is visible and routes are accessible for all three locales.
- Desktop language setting persists and takes effect after reload.
- Language resolution priority:
  - user setting > system locale > English fallback
- Existing behavior is unaffected when language remains English.

## Next Phase

Phase 2 will propose CLI localization strategy and implementation details.
