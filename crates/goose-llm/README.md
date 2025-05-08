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
cd bindings/kotlin/

# Download jars in libs/ directory
pushd libs/
curl -O https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-stdlib/1.9.0/kotlin-stdlib-1.9.0.jar
curl -O https://repo1.maven.org/maven2/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm/1.7.3/kotlinx-coroutines-core-jvm-1.7.3.jar
mv kotlinx-coroutines-core-jvm-1.7.3.jar kotlinx-coroutines-core-1.7.3.jar
curl -O https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar
popd

# Compile both the generated binding and your example in a single jar:
kotlinc \
  example/Usage.kt \
  uniffi/goose_llm/goose_llm.kt \
  -classpath "libs/kotlin-stdlib-1.9.0.jar:libs/kotlinx-coroutines-core-1.7.3.jar:libs/jna-5.13.0.jar" \
  -include-runtime \
  -d example.jar

# Run it, pointing JNA at your Rust library:
java \
  -Djna.library.path=$HOME/Development/goose/target/debug \
  -classpath "example.jar:libs/kotlin-stdlib-1.9.0.jar:libs/kotlinx-coroutines-core-1.7.3.jar:libs/jna-5.13.0.jar" \
  UsageKt
```



Run with Gradle:
```
cd bindings/kotlin
gradle run
```