# Native Binary Packages

This directory contains the npm package scaffolding for distributing the
`goose-acp-server` Rust binary as platform-specific npm packages.

## Packages

| Package | Platform |
|---------|----------|
| `@goose-ai/acp-server-darwin-arm64` | macOS Apple Silicon |
| `@goose-ai/acp-server-darwin-x64` | macOS Intel |
| `@goose-ai/acp-server-linux-arm64` | Linux ARM64 |
| `@goose-ai/acp-server-linux-x64` | Linux x64 |
| `@goose-ai/acp-server-win32-x64` | Windows x64 |

## Building

From the repository root:

```bash
# Build for all platforms (requires cross-compilation toolchains)
./ui/text/scripts/build-native-packages.sh

# Build for a single platform
./ui/text/scripts/build-native-packages.sh darwin-arm64
```

The built binaries are placed into `npm/goose-acp-server-{platform}/bin/`.
These directories are git-ignored.

## Publishing

```bash
./ui/text/scripts/publish.sh
```
