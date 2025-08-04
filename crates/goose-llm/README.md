## goose-llm 

This crate is meant to be used for foreign function interface (FFI). It's meant to be 
stateless and contain logic related to providers and prompts:
- chat completion with model providers
- detecting read-only tools for smart approval
- methods for summarization / truncation


Run:
```
cargo run -p goose-llm --example simple
```


## Kotlin bindings

Structure:
```
.
└── crates
    └── goose-llm/...
└── target
    └── debug/libgoose_llm.dylib
├── bindings
│   └── kotlin
│       ├── example
│       │   └── Usage.kt              ← your demo app
│       └── uniffi
│           └── goose_llm
│               └── goose_llm.kt   ← auto-generated bindings
```


#### Kotlin -> Rust: run example

The following `just` command creates kotlin bindings, then compiles and runs an example.

```bash
just kotlin-example
```

You will have to download jars in `bindings/kotlin/libs` directory (only the first time):
```bash
pushd bindings/kotlin/libs/
curl -O https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-stdlib/1.9.0/kotlin-stdlib-1.9.0.jar
curl -O https://repo1.maven.org/maven2/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm/1.7.3/kotlinx-coroutines-core-jvm-1.7.3.jar
curl -O https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar
popd
```

To just create the Kotlin bindings (for MacOS):

```bash
# run from project root directory
cargo build -p goose-llm 

cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_llm.dylib --language kotlin --out-dir bindings/kotlin
```

Creating `libgoose_llm.so` for Linux distribution:

Use `cross` to build for the specific target and then create the bindings:
```
# x86-64 GNU/Linux (kGoose uses this)
rustup target add x86_64-unknown-linux-gnu
cross build --release --target x86_64-unknown-linux-gnu -p goose-llm

# The goose_llm.kt bindings produced should be the same whether we use 'libgoose_llm.dylib' or 'libgoose_llm.so'
cross run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/x86_64-unknown-linux-gnu/release/libgoose_llm.so --language kotlin --out-dir bindings/kotlin
```


#### Python -> Rust: generate bindings, run example

```bash
cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_llm.dylib --language python --out-dir bindings/python

DYLD_LIBRARY_PATH=./target/debug python bindings/python/usage.py
```
