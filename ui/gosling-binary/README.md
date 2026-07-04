# Native Binary Packages for gosling

This directory contains the npm package scaffolding for distributing the
`gosling` Rust binary as platform-specific npm packages.

## Packages

| Package | Platform |
|---------|----------|
| `@repo-makeover/gosling-binary-darwin-arm64` | macOS Apple Silicon |
| `@repo-makeover/gosling-binary-darwin-x64` | macOS Intel |
| `@repo-makeover/gosling-binary-linux-arm64` | Linux ARM64 |
| `@repo-makeover/gosling-binary-linux-x64` | Linux x64 |
| `@repo-makeover/gosling-binary-win32-x64` | Windows x64 |

## Building

From the repository root:

```bash
# Build for current platform only
cd ui/sdk
npm run build:native

# Build for all platforms (requires cross-compilation toolchains)
npm run build:native:all

# Build for specific platform(s)
npx tsx scripts/build-native.ts darwin-arm64 linux-x64
```

The built binaries are placed into `ui/gosling-binary/gosling-binary-{platform}/bin/`.
These directories are git-ignored.

Linux native binaries are built with local inference Vulkan support. Linux build
hosts need Vulkan headers and `glslc`; Linux runtime hosts need the Vulkan loader
package, such as `libvulkan1` on Debian/Ubuntu or `vulkan-loader` on RPM-based
distributions.

## Publishing

Publishing is handled by GitHub Actions. See `.github/workflows/publish-npm.yml`.

For manual publishing:

```bash
# From repository root
./ui/scripts/publish.sh --real
```

This will publish all native packages along with `@repo-makeover/gosling-sdk` and `@repo-makeover/gosling`.

## Usage

These packages are installed as optional dependencies by `@repo-makeover/gosling` (the TUI).
The appropriate package for the user's platform is automatically selected during
installation.

See `ui/text/scripts/postinstall.mjs` for how the binary path is resolved.
