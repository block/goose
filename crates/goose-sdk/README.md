# goose-sdk

The bindings layer for Goose. It houses the shared types used for both ACP and
SDK access, and exposes a cross-language version of the Goose API.

With `--features uniffi` the crate compiles to native bindings for Python and
Kotlin (namespace `goose` / `io.aaif.goose`). The UniFFI surface currently lets
callers construct declarative providers from JSON and stream provider
completions.

```bash
just python   # build bindings + run examples/uniffi/provider.py
just kotlin   # build bindings + run examples/uniffi/Provider.kt
```

## Python package

The PyPI package is published as `goose-sdk` and imports as `goose`.
Build a local wheel from the repository root with:

```bash
just --justfile crates/goose-sdk/justfile python-wheel
```

This regenerates the UniFFI Python bindings, copies the release native library
into the package, and writes the wheel to `crates/goose-sdk/python/dist/`.
