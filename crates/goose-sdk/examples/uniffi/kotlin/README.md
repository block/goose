# Kotlin/JVM GDK smoke test

This is a small downstream Kotlin/JVM app that consumes the Maven artifact
`io.github.aaif-goose:gdk` from `mavenLocal()`.

From the repository root, first build and publish the Maven artifact locally:

```bash
source bin/activate-hermit
just --justfile crates/goose-sdk/justfile maven-package
```

Then run the smoke test:

```bash
export ANTHROPIC_API_KEY=...
cd crates/goose-sdk/examples/uniffi/kotlin
gradle --no-daemon run
```

The example exercises the hand-authored Kotlin conveniences in addition to the
generated UniFFI API. It uses structured system content with an explicit cache
breakpoint, consumes thinking and redacted-thinking chunks, prints stable tool
indices, and reports cached-token accounting.

Documents can be supplied without a JSON shim:

```kotlin
val pdf = document(
    mimeType = "application/pdf",
    data = java.io.File("report.pdf").readBytes(),
    filename = "report.pdf",
)
```

Assistant thinking blocks returned by a provider can be replayed as
`MessageContent.Thinking(thinking, signature)` or
`MessageContent.RedactedThinking(data)` on the next request.

The important native-loading failure to watch for is `UnsatisfiedLinkError` or
a missing native-library resource. The example sets
`--enable-native-access=ALL-UNNAMED` because JNA loads the bundled Goose native
library. Newer JDKs warn when native access is not enabled explicitly, and
future JDKs may require it.
