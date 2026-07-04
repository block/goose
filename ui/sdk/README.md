# @repo-makeover/gosling-sdk

TypeScript client library for the Gosling Agent Client Protocol (ACP).

This package provides:
- TypeScript types and Zod validators for Gosling ACP extension methods
- A client for communicating with the Gosling ACP server

## Installation

```bash
npm install @repo-makeover/gosling-sdk
```

The native `gosling` binaries are distributed as optional dependencies
and will be automatically installed for your platform.

## Development

### Prerequisites

- Node.js 18+
- Rust toolchain
- (Optional) Cross-compilation toolchains for building all platforms

### Building

```bash
# Build everything (schema + TypeScript)
npm run build

# Build just the schema (requires Rust)
npm run build:schema

# Build just the TypeScript
npm run build:ts

# Build native binary for current platform
npm run build:native

# Build native binaries for all platforms
npm run build:native:all
```

### Local Development with npm link

To use this package locally in another project (e.g., `@repo-makeover/gosling`):

```bash
# In ui/sdk
npm run build
npm link

# In ui/text (or another project)
npm link @repo-makeover/gosling-sdk
```

### Schema Generation

The TypeScript types are generated from Rust schemas defined in `crates/gosling`.
The build process:

1. Builds the `generate-acp-schema` Rust binary
2. Runs it to generate `acp-schema.json` and `acp-meta.json`
3. Uses `@hey-api/openapi-ts` to generate TypeScript types and Zod validators
4. Generates a typed client in `src/generated/client.gen.ts`

To regenerate schemas after changing Rust types:

```bash
npm run build:schema
```

## Native Binary Packages

Platform-specific npm packages for the `gosling` binary are located in
`ui/gosling-binary/`:

| Package | Platform |
|---------|----------|
| `@repo-makeover/gosling-binary-darwin-arm64` | macOS Apple Silicon |
| `@repo-makeover/gosling-binary-darwin-x64` | macOS Intel |
| `@repo-makeover/gosling-binary-linux-arm64` | Linux ARM64 |
| `@repo-makeover/gosling-binary-linux-x64` | Linux x64 |
| `@repo-makeover/gosling-binary-win32-x64` | Windows x64 |

These are published separately from `@repo-makeover/gosling-sdk`.

### Building Native Binaries

```bash
# Build for current platform
npm run build:native

# Build for all platforms (requires cross-compilation toolchains)
npm run build:native:all

# Build for specific platform(s)
npx tsx scripts/build-native.ts darwin-arm64 linux-x64
```

## Publishing

Publishing is handled by GitHub Actions. See `.github/workflows/publish-npm.yml`.

For manual publishing:

```bash
# From repository root
./ui/scripts/publish.sh --real
```

This will:
1. Build and publish `@repo-makeover/gosling-sdk`
2. Publish all native binary packages
3. Publish `@repo-makeover/gosling` (which depends on the above)

## Usage

```typescript
import { GoslingClient } from "@repo-makeover/gosling-sdk";

const client = new GoslingClient({
  // ... configuration
});

// Use the client
const result = await client.someMethod({ ... });
```

See the [main documentation](../../README.md) for more details.
