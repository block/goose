# goose Desktop App

Native desktop app for goose built with [Electron](https://www.electronjs.org/) and [ReactJS](https://react.dev/). 

# Building and running
goose uses [Hermit](https://github.com/cashapp/hermit) to manage dependencies, so you will need to have it installed and activated.

```
git clone git@github.com:aaif-goose/goose.git
cd goose
source ./bin/activate-hermit
cd ui/desktop
pnpm install
pnpm run start
```

## Platform-specific build requirements

### Linux
For building on Linux distributions, you'll need additional system dependencies:

**Debian/Ubuntu:**
```bash
sudo apt install dpkg fakeroot
```

**Arch/Manjaro:**
```bash
sudo pacman -S dpkg fakeroot
```

**Fedora/RHEL:**
```bash
sudo dnf install dpkg-dev fakeroot
```

# Building notes

This is an electron forge app, using vite and react.js. `goosed` runs as multi process binaries on each window/tab similar to chrome.

## Building for different platforms

### macOS
`pnpm run bundle:default` now produces a local-preview `.app` / `.zip` by default:

- `GOOSE_DESKTOP_SIGN` defaults to `false`
- the bundle is ad-hoc re-signed locally so `codesign --verify` passes
- `spctl` may still reject it because this path is not notarized Apple distribution
- local preview keeps `GOOSE_DISABLE_KEYRING=1` in the app environment to avoid keychain prompts during unsigned install-state testing

For a signed/notarized release rehearsal, use the reusable GitHub workflows or set:

```bash
GOOSE_DESKTOP_SIGN=true
APPLE_CERTIFICATE_BASE64=...
APPLE_CERTIFICATE_PASSWORD=...
APPLE_TEAM_ID=...
APPLE_ID=...
APPLE_ID_PASSWORD=...
```

Then validate preflight with:

```bash
node ../../scripts/check-security-apple-signing-env.mjs --require-signed
```

and validate the built bundle with:

```bash
../../scripts/check-security-macos-bundle.sh --arch arm64 --expect signed --require-notarized
```

If you need a CI rehearsal before pushing a release tag, use the GitHub Actions
workflow `Manual Desktop Bundle` with:

- `signing=true`
- `environment=signing`

That path reuses the existing reusable bundle workflows and now uploads:

- `Security-Goose-macos-release-evidence-arm64`
- `Security-Goose-macos-release-evidence-x64`

Each artifact contains:

- `signing-preflight.txt`
- `bundle-check.txt`
- `summary.json`
- `summary.md`

The same `summary.md` content is also appended to the job summary. For the
Security Goose release-specific operator steps and install acceptance checklist,
see [../../distro/security-cn/docs/signed-release-runbook.md](../../distro/security-cn/docs/signed-release-runbook.md).

### Linux
For Linux builds, first ensure you have the required system dependencies installed (see above), then:

1. Build the Rust backend:
```bash
cd ../..  # Go to project root
cargo build --release -p goose-server
```

2. Copy the server binary to the expected location:
```bash
mkdir -p src/bin
cp ../../target/release/goosed src/bin/
```

3. Build the application:
```bash
# For ZIP distribution (works on all Linux distributions)
pnpm run make --targets=@electron-forge/maker-zip

# For DEB package (Debian/Ubuntu)
pnpm run make --targets=@electron-forge/maker-deb

# For Flatpak (requires flatpak and flatpak-builder)
pnpm run make --targets=@electron-forge/maker-flatpak
```

The built application will be available in:
- ZIP: `out/make/zip/linux/x64/goose-linux-x64-{version}.zip`
- DEB: `out/make/deb/x64/goose_{version}_amd64.deb`
- Flatpak: `out/make/flatpak/x86_64/*.flatpak`
- Executable: `out/goose-linux-x64/goose`

### Windows
Use the existing Windows build process as documented.


# Running with goosed server from source

Set `VITE_START_EMBEDDED_SERVER=yes` to no in `.env`.
Run `cargo run -p goose-server` from parent dir.
`pnpm run start` will then run against this.
You can try server directly with `./test.sh`
