# @aaif/goose-acp

Install and resolve the Goose executable through npm.

This package distributes the Goose CLI using platform-specific optional npm
dependencies. It does not contain or depend on the Goose ACP client.

## Installation

```bash
npm install @aaif/goose-acp
```

## Usage

```typescript
import { resolveGooseBinary } from "@aaif/goose-acp";

const binaryPath = resolveGooseBinary();
```

`resolveGooseBinary()` selects the package matching `process.platform` and
`process.arch`, verifies that its executable exists, and returns an absolute
path. It resolves only the npm-provided executable and does not read
`GOOSE_BINARY`.

Supported platforms:

| Operating system | Architecture |
| ---------------- | ------------ |
| macOS            | ARM64        |
| macOS            | x64          |
| Linux            | ARM64        |
| Linux            | x64          |
| Windows          | x64          |

Package managers must install optional dependencies. If optional dependencies
are disabled, the resolver reports which platform package is missing.
