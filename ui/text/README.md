# gosling ACP TUI

Early stage and part of gosling's broader move to ACP

https://github.com/repo-makeover/gosling/issues/6642
https://github.com/repo-makeover/gosling/discussions/7309

## Running

The TUI launches the gosling ACP server by spawning `gosling acp`. Which binary it spawns is resolved by `@repo-makeover/gosling-sdk`:

1. the `GOSLING_BINARY` environment variable, if set, otherwise
2. the platform's prebuilt `@repo-makeover/gosling-binary-*` package (an optional dependency of the pinned `@repo-makeover/gosling-sdk`).

```bash
cd ui/text
pnpm install   # pulls the pinned @repo-makeover/gosling-sdk and its matching @repo-makeover/gosling-binary-* package
pnpm start     # tsx src/tui.tsx — runs against the released binary, no Rust build
```

The TUI pins a specific `@repo-makeover/gosling-sdk` version, so `pnpm start` always runs against a gosling binary that matches the SDK.

### Building gosling from local source

To test local Rust changes, run the dev launcher directly. It builds a debug binary (`cargo build -p gosling-cli` → `target/debug/gosling`) from the workspace root and points the TUI at it via `GOSLING_BINARY`:

```bash
node scripts/dev-start.mjs
```

If your changes touch the ACP schema, also point the TUI at the in-repo SDK so the two stay matched: set `@repo-makeover/gosling-sdk` to `workspace:*` in `package.json` and re-run `pnpm install`. Otherwise the locally built binary may not match the pinned published SDK's schema. Revert that change before committing — the TUI is meant to stay frozen on its pinned SDK version.

To run any other prebuilt binary, set `GOSLING_BINARY=/path/to/gosling` and use `pnpm start`.

### Custom server URL

To connect to an already-running server instead of spawning a binary:

```bash
pnpm start -- --server http://localhost:8080
```
