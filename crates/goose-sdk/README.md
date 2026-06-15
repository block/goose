# goose-sdk

The bindings layer for Goose. It houses the shared types used for both ACP and
SDK access, and exposes a cross-language version of the Goose API.

With `--features uniffi` the crate compiles to native bindings for Python and
Kotlin (namespace `aaif_goose` / `aaif.goose`). The published surface is
currently a `ping` -> `pong` stub in `src/bindings.rs` — the scaffold for the
real implementation.

```bash
just python   # build bindings + run examples/uniffi/ping.py
just kotlin   # build bindings + run examples/uniffi/Ping.kt
```

Both print `pong: aaif.io`.
