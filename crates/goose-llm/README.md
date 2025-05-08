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

Create Kotlin bindings:
```
cargo build -p goose-llm

cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_llm.dylib --language kotlin --out-dir bindings/kotlin
```


Run from project root directory:
```
# Download JNA once (if you haven’t already)
curl -L -o jna.jar \
  https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar

# Compile both the generated binding and your example in a single jar:
kotlinc \
  bindings/kotlin/example/Usage.kt \
  bindings/kotlin/uniffi/goose_llm/goose_llm.kt \
  -classpath jna.jar \
  -include-runtime \
  -d example.jar

# Run it, pointing JNA at your Rust library:
java \
  -Djna.library.path=$HOME/Development/goose/target/debug \
  -cp example.jar:jna.jar \
  UsageKt
```