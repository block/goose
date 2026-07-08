# UniFFI examples

These examples exercise the in-process Goose SDK UniFFI bindings from Python and Kotlin.

## Prerequisites

```bash
source bin/activate-hermit
export DEEPSEEK_API_KEY=...
```

## Generate bindings

Regenerate the Python and Kotlin bindings before running the examples:

```bash
just --justfile crates/goose-sdk/justfile _generate python
just --justfile crates/goose-sdk/justfile _generate kotlin
```

This writes generated bindings and the debug native library under `crates/goose-sdk/generated/`.

## Python provider example

```bash
DYLD_LIBRARY_PATH=target/debug LD_LIBRARY_PATH=target/debug \
  uv run --script crates/goose-sdk/examples/uniffi/provider.py
```

## Kotlin provider example

Download JNA if it is not already present:

```bash
curl -sSL -o crates/goose-sdk/examples/uniffi/jna.jar \
  https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.14.0/jna-5.14.0.jar
```

Compile and run:

```bash
kotlinc -cp crates/goose-sdk/examples/uniffi/jna.jar -nowarn \
  crates/goose-sdk/generated/io/aaif/goose/goose.kt \
  crates/goose-sdk/examples/uniffi/Provider.kt \
  -include-runtime -d crates/goose-sdk/examples/uniffi/provider.jar

java -Djna.library.path=target/debug \
  --enable-native-access=ALL-UNNAMED \
  -cp crates/goose-sdk/examples/uniffi/provider.jar:crates/goose-sdk/examples/uniffi/jna.jar \
  aaif.example.ProviderKt
```

On Linux, use the same command; `LD_LIBRARY_PATH=target/debug` can also be set if needed. On macOS, `-Djna.library.path=target/debug` is usually enough, but `DYLD_LIBRARY_PATH=target/debug` can also be set if JNA cannot find `libgoose_sdk.dylib`.
