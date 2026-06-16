# goose ACP TUI

Early stage and part of goose's broader move to ACP

https://github.com/aaif-goose/goose/issues/6642
https://github.com/aaif-goose/goose/discussions/7309

## Running

The TUI launches the goose ACP server by spawning `goose acp`. Which binary it spawns is resolved by `@aaif/goose-sdk`:

1. the `GOOSE_BINARY` environment variable, if set, otherwise
2. the platform's prebuilt `@aaif/goose-binary-*` package (an optional dependency of the pinned `@aaif/goose-sdk`).

```bash
cd ui/text
pnpm install   # pulls the pinned @aaif/goose-sdk and its matching @aaif/goose-binary-* package
pnpm start     # tsx src/tui.tsx — runs against the released binary, no Rust build
```

The TUI pins a specific `@aaif/goose-sdk` version, so `pnpm start` always runs against a goose binary that matches the SDK.

### Building goose from local source

To test local Rust changes, run the dev launcher directly. It builds a debug binary (`cargo build -p goose-cli` → `target/debug/goose`) from the workspace root and points the TUI at it via `GOOSE_BINARY`:

```bash
node scripts/dev-start.mjs
```

If your changes touch the ACP schema, also point the TUI at the in-repo SDK so the two stay matched: set `@aaif/goose-sdk` to `workspace:*` in `package.json` and re-run `pnpm install`. Otherwise the locally built binary may not match the pinned published SDK's schema. Revert that change before committing — the TUI is meant to stay frozen on its pinned SDK version.

To run any other prebuilt binary, set `GOOSE_BINARY=/path/to/goose` and use `pnpm start`.

### Custom server URL

To connect to an already-running server instead of spawning a binary:

```bash
pnpm start -- --server http://localhost:8080
```
