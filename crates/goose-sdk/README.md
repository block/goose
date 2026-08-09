# goose-sdk

The bindings layer for Goose. It houses the shared types used for both ACP and
SDK access, and exposes a cross-language version of the Goose API.

With `--features uniffi` the crate compiles to native bindings for Python and
Kotlin (namespace `goose` / `io.github.aaif_goose`). The UniFFI surface lets
callers construct providers, stream provider completions, perform non-streaming
completion, and pass rich message/tool content across the FFI boundary.

```bash
just python   # build bindings + run examples/uniffi/provider.py
just kotlin   # build the Maven artifact + run examples/uniffi/kotlin
```

## Python package

The PyPI package is published as `goose-sdk` and imports as `goose`.
Build a local wheel from the repository root with:

```bash
just --justfile crates/goose-sdk/justfile python-wheel
```

This regenerates the UniFFI Python bindings, copies the release native library
into the package, and writes the wheel to `crates/goose-sdk/python/dist/`.

## Maven package

The Maven Central artifact is published as `io.github.aaif-goose:gdk` and uses
the Rust crate version from `crates/goose-sdk/Cargo.toml`.

```bash
just --justfile crates/goose-sdk/justfile maven-package
```

This regenerates the UniFFI Kotlin bindings and packages them with the native
library in a JVM jar. CI builds the native libraries for supported platforms and
can optionally publish the combined artifact to Maven Central.
