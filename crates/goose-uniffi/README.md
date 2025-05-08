### goose-uniffi 

This crate is meant to create bindings so we can call goose-llm rust code from kotlin.

Docs:
- https://mozilla.github.io/uniffi-rs/latest/tutorial/foreign_language_bindings.html

Examples:
- https://github.com/mozilla/uniffi-rs/tree/main/examples
- https://github.com/mozilla/uniffi-rs/tree/main/examples/async-api-client



```
.
└── crates
    └── goose-uniffi/...
└── target
    └── debug/libgoose_uniffi.dylib
├── bindings
│   └── kotlin
│       ├── example
│       │   └── Usage.kt              ← your demo app
│       └── uniffi
│           └── goose_uniffi
│               └── goose_uniffi.kt   ← auto-generated bindings
```

Create Kotlin bindings:
```
cargo build -p goose-uniffi

cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_uniffi.dylib --language kotlin --out-dir bindings/kotlin
```

Download jars in `bindings/kotlin/libs` directory (only need to do this once):
```
pushd bindings/kotlin/libs/
curl -O https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-stdlib/1.9.0/kotlin-stdlib-1.9.0.jar
curl -O https://repo1.maven.org/maven2/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm/1.7.3/kotlinx-coroutines-core-jvm-1.7.3.jar
curl -O https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar
popd
```


Compile & Run usage example from Kotlin -> Rust:
```
pushd bindings/kotlin/

kotlinc \
  example/Usage.kt \
  uniffi/goose_uniffi/goose_uniffi.kt \
  -classpath "libs/kotlin-stdlib-1.9.0.jar:libs/kotlinx-coroutines-core-jvm-1.7.3.jar:libs/jna-5.13.0.jar" \
  -include-runtime \
  -d example.jar

java \
  -Djna.library.path=$HOME/Development/goose/target/debug \
  -classpath "example.jar:libs/kotlin-stdlib-1.9.0.jar:libs/kotlinx-coroutines-core-jvm-1.7.3.jar:libs/jna-5.13.0.jar" \
  UsageKt
  
popd
```