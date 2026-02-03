# Changelog

All notable changes to the Vibes CLI plugin are documented here.

## [0.1.37] - 2026-01-20

### Changed
- **BREAKING**: Migrated from use-vibes@0.18.9 to use-vibes@0.24.3-dev
- Updated Fireproof documentation with new `toCloud()` cloud sync features
- All templates now include cloud sync by default

### Added
- `toCloud()` function for automatic Fireproof Cloud synchronization
- `attach` object in useFireproof() return for sync status tracking
- Sync states: "initial" | "attaching" | "attached" | "error"
- Token management via `attach.ctx.tokenAndClaims.reset()`

### Updated
- skills/vibes/SKILL.md: All examples now use cloud sync pattern
- cache/fireproof.txt: Complete rewrite with cloud sync documentation
- CLAUDE.md: Updated version policy from 0.18.9 to 0.24.3-dev

## [0.1.18] - 2026-01-03

### Added
- CHANGELOG.md with version history
- CONTRIBUTING.md development guide
- Quick Start section in README
- Client-Side Multi-Tenancy documentation in README
- "How Data Works" section explaining Fireproof

### Fixed
- Document missing cache files (vibes-menu.js, vibes-variables.css) in CLAUDE.md
- Clarify stable version 0.18.9 vs upstream dev versions
- Expand Cache Locations section with complete file inventory

## [0.1.17] - 2026-01-03

### Fixed
- Sync menu components from upstream to templates
- Remove redundant commands and fix documentation gaps

## [0.1.16] - 2025-12-28

### Fixed
- Improve set-public error handling with retry and clear messaging
- Document set-public step in exe.dev deployment skill
- Fix landing page component conflicts with user app code

## [0.1.15] - 2025-12-27

### Fixed
- Fix multi-line import stripping in stripImports function
- Remove Cloudflare implementation, consolidate on exe.dev

## [0.1.14] - 2025-12-26

### Fixed
- Fix hardcoded plugin paths causing installation/update failures

## [0.1.13] - 2025-12-25

### Added
- exe-sell skill for client-side only SaaS on exe.dev

### Fixed
- Fix shell escaping: use backticks instead of $()

## [0.1.12] - 2025-12-24

### Added
- exe.dev deployment integration with SSH automation
- Testing infrastructure with Vitest (unit, integration, e2e)

## [0.1.2] - 2025-12-20

### Fixed
- 27 code quality issues from plugin audit
- Add dev-reinstall.sh for testing plugin updates

## [0.1.1] - 2025-12-19

### Added
- `update` command for deterministic app updates
- Passkey-first authentication flow for sell skill

### Changed
- Updated README with installation troubleshooting and commands section

## [0.1.0] - 2025-12-18

### Changed
- Renamed plugin from vibes-skill to vibes-cli
- Renamed plugin from vibes-diy to vibes

### Added
- Sell skill for multi-tenant SaaS transformation
- Riff skill for parallel app generation
- Conceptual philosophy documentation

## [0.0.x] - Pre-release

Initial development of the Vibes plugin with core vibes skill for generating React apps with Fireproof.

---

## Version Numbering

This plugin uses semantic versioning:
- MAJOR: Breaking changes to skill interfaces or generated app structure
- MINOR: New features, skills, or commands
- PATCH: Bug fixes and documentation updates

## Stable Library Version

This plugin pins to `use-vibes@0.18.9` (stable). Development versions from upstream (0.19.x-dev) are not used due to known bugs.
