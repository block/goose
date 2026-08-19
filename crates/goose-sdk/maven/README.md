# Goose SDK Maven package

This project packages the UniFFI-generated Kotlin/JVM bindings for `goose-sdk`
as the Maven artifact `io.github.aaif-goose:gdk`.

The artifact version is read from `crates/goose-sdk/Cargo.toml`, so it stays in
lockstep with the Rust crate version. The jar includes the generated Kotlin API
and native libraries under JNA platform resource directories. Packaging supports
`darwin-aarch64`, `darwin-x86-64`, `linux-x86-64`, `linux-aarch64`, and
`win32-x86-64` resource prefixes; CI is responsible for assembling every native
library into the final published jar.

Build locally from the repository root:

```bash
just --justfile crates/goose-sdk/justfile maven-package
```

Publish to Maven Central from the repository root:

```bash
just --justfile crates/goose-sdk/justfile maven-publish
```

## Kotlin conveniences

The artifact includes hand-authored Kotlin extensions over the generated
UniFFI API:

- `Provider.streamFlow(...)` overloads for plain and structured system content.
- `Provider.complete(...)` overloads with default empty tool lists.
- `ephemeralCacheControl()`, `cachedText(...)`, and `cachedSystemText(...)`.
- `document(...)` for typed binary document content.

Use structured system content when cache breakpoint placement matters:

```kotlin
provider.streamFlow(
    model = model,
    system = listOf(cachedSystemText("You are a helpful assistant.")),
    messages = messages,
)
```

The generated types also expose thinking and redacted-thinking blocks, cached
token accounting, capability reporting, and indexed streaming tool chunks.

Publishing requires the standard Gradle properties used by
`com.vanniktech.maven.publish` for Maven Central credentials and in-memory PGP
signing, for example via environment variables:

- `ORG_GRADLE_PROJECT_mavenCentralUsername`
- `ORG_GRADLE_PROJECT_mavenCentralPassword`
- `ORG_GRADLE_PROJECT_signingInMemoryKey`
- `ORG_GRADLE_PROJECT_signingInMemoryKeyPassword`
