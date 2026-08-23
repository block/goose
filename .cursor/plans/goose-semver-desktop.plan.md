# Smooth versioning for the Avocado Work desktop app

`Cargo.toml` `workspace.package.version` is the single source of truth. Resolution order: explicit `RELEASE_VERSION` / workflow input, then a `v*` tag on `GITHUB_REF`, then `Cargo.toml`. `ui/desktop/package.json` must match, enforced by `.github/workflows/version-consistency.yml`.

See the implementation in `scripts/release-version.sh` and the desktop release workflows.
