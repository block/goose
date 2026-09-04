# Native Binary Packages for goose

This directory contains the npm package scaffolding for distributing the
`goose` Rust binary as platform-specific npm packages.

## Packages

| Package | Platform |
|---------|----------|
| `@aaif/goose-binary-darwin-arm64` | macOS Apple Silicon |
| `@aaif/goose-binary-darwin-x64` | macOS Intel |
| `@aaif/goose-binary-linux-arm64` | Linux ARM64 |
| `@aaif/goose-binary-linux-x64` | Linux x64 |
| `@aaif/goose-binary-win32-x64` | Windows x64 |

## Usage

These are platform-specific implementation dependencies and are not intended
to be installed directly. Install `@aaif/goose-acp` instead. It installs the
appropriate package automatically and provides the `goose` command. Each
binary package contains its native executable but intentionally provides no
command of its own.

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

The built binaries are placed into `ui/goose-binary/goose-binary-{platform}/bin/`.
These directories are git-ignored.

Linux native binaries are built with local inference Vulkan support. Linux build
hosts need Vulkan headers and `glslc`; Linux runtime hosts need the Vulkan loader
package, such as `libvulkan1` on Debian/Ubuntu or `vulkan-loader` on RPM-based
distributions.

## Release preparation

The `.github/workflows/publish-npm.yml` workflow downloads the binaries from an
exact versioned Goose release and prepares the platform package tarballs.
By default it only uploads the verified tarballs as a workflow artifact. Set
the manual `publish` input to publish them through the protected npm production
environment.
