# Goose v1.45.0 Codex retry and Windows GGUF patch

This branch is an unofficial derivative of Goose v1.45.0.

## Included changes

### ChatGPT Codex recovery

- Maps request transport failures to `NetworkError`.
- Retries transient ChatGPT Codex errors up to eight times.
- Uses exponential backoff starting at two seconds and capped at 30 seconds.
- Retries network, server, and rate-limit failures only.
- Does not retry deterministic client or authentication failures indefinitely.

This recovery covers request establishment and HTTP status failures. It does not
replay a response after streaming output has begun, because replaying could
duplicate text, tool calls, or file mutations.

### Optional Windows GGUF selection

`tools/windows-gguf` provides a local installer and model picker for existing
GGUF files. It registers the selected file with a local llama.cpp server and a
Goose OpenAI-compatible provider. Model files are never included or uploaded.

## Validation

- `cargo test -p goose-provider-types retry::tests::`: 8 passed.
- `cargo check -p goose`: passed.
- Patched Windows binary completed a real ChatGPT Codex request.
- The upstream full provider-types suite has seven pre-existing Windows path
  failures involving temporary image paths; 440 other tests passed.

## Windows build used for the distributed binary

The provided binary was built with:

```powershell
cargo build --release -p goose-cli --no-default-features --features "portable-default,system-keyring"
```

It intentionally excludes Code Mode and embedded local inference. External
llama.cpp/OpenAI-compatible providers remain supported.

## Do not publish

- `.gguf` model files
- `target/` build output except a deliberately attached release binary
- Goose user configuration, OAuth credentials, sessions, or databases
- Clash/proxy subscriptions or credentials
- machine-specific `local-models.json`
- logs and crash dumps

