# Goose ACP npm implementation progress

Source plan:
`/Users/lifei/Documents/ai-working-docs/goose-acp-client-npm/goose-acp-npm-implementation-plan.md`

Last updated: 2026-09-04

## Current approach

Develop the replacement packages alongside the existing `ui/sdk` package.
Keep Desktop on `@aaif/goose-sdk` until the new client package is ready for a
mechanical migration.

## Milestone 1: Package split

### Task 1: Create the client-only package

- [x] Create `ui/goose-acp-client` without removing `ui/sdk`.
- [x] Set the package name to `@aaif/goose-acp-client`.
- [x] Copy generated Goose types and Zod validators.
- [x] Copy `GooseExtClient` and extension method constants.
- [x] Copy client capability definitions and MCP Apps helpers.
- [x] Copy schema generation and binary compatibility scripts.
- [x] Remove the binary-resolution subpath export.
- [x] Exclude `resolve-binary.ts` and native binary build scripts.
- [x] Exclude native optional dependencies.
- [x] Register the package in the pnpm workspace.
- [x] Add client-only installation and usage documentation.
- [x] Add a package `.gitignore` for generated build and dependency folders.
- [x] Keep generated artifacts synchronized with `ui/sdk` while both exist.
- [ ] Add package-level tests and a `test` script.
- [x] Build the package independently.
- [x] Inspect the packed manifest and tarball contents.
- [x] Verify that installing the tarball does not install a binary package.
- [ ] Run the compatibility check against the matching Goose executable.

### Task 2: Create the binary distribution package

- [ ] Create `ui/goose-acp`.
- [ ] Implement `resolveGooseBinary()` without reading `GOOSE_BINARY`.
- [ ] Add the `goose` command launcher.
- [ ] Add platform selection, missing-package, and unsupported-platform tests.
- [ ] Verify the packed package contains no client code.

### Task 3: Migrate Goose Desktop mechanically

- [ ] Change Desktop imports to `@aaif/goose-acp-client`.
- [ ] Change the Desktop workspace dependency and SDK build script names.
- [ ] Update Vite exclusions, tests, and mocks.
- [ ] Confirm Desktop does not depend on `@aaif/goose-acp`.
- [ ] Verify that Desktop connection and executable lifecycle behavior is
      unchanged.

### Task 4: Establish version alignment

- [ ] Align the client, wrapper, platform packages, Cargo workspace, and Desktop
      versions.
- [ ] Update exact optional dependency versions in `@aaif/goose-acp`.
- [ ] Add a single npm package version-management script.
- [ ] Integrate the script with `just bump-version`.
- [ ] Add read-only version verification locally and in CI.

Current known version drift:

- Goose workspace and Desktop: `1.49.0`
- Existing npm SDK and platform packages: `0.20.2`
- New client scaffold: `0.20.2`

### Task 5: Verify the split locally

- [x] Test client-only tarball installation.
- [ ] Test binary-only tarball installation.
- [ ] Test installing both independent packages.
- [ ] Test a client connected to a separately started `goose acp` process.
- [ ] Test the npm-provided binary over stdio and through `goose serve`.
- [ ] Test unsupported platform behavior.

## Later milestones

- [ ] Milestone 2: Integrate npm packaging and publication with the normal Goose
      release.
- [ ] Milestone 3: Design and add `connectGoose()` and transport adapters.
- [ ] Milestone 4: Add consolidated migration documentation, publish the
      replacements, verify them, and deprecate the old packages.

## Verification log

- 2026-09-04: Confirmed the new package has no binary resolver, native build
  commands, native optional dependencies, or Node-only runtime imports.
- 2026-09-04: Confirmed `ui/sdk` and Desktop remain unchanged.
- 2026-09-04: Confirmed the updated pnpm lockfile with an offline frozen-lockfile
  check. Build and test commands have not been run.
- 2026-09-04: Added package ignore rules and clarified that applications supply
  and own the ACP stream used in the README example.
- 2026-09-04: Updated `just generate-acp-types` and `just check-acp-artifacts` to
  generate and check both client packages during the side-by-side migration.
- 2026-09-04: Built `@aaif/goose-acp-client` independently and inspected its
  packed manifest and tarball. The tarball contains only the README, manifest,
  and compiled client files, with no binary resolver or native build files.
- 2026-09-04: Installed the tarball in a clean temporary project. Importing the
  package returned `GooseExtClient` as a function, and no
  `@aaif/goose-binary-*` package was installed.

## Next step

Create the independent `ui/goose-acp` binary distribution package.
