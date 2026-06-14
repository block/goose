# Live testing ModelConfig default resolution

This checklist is for manually validating that `GOOSE_CONTEXT_LIMIT`, `GOOSE_MAX_TOKENS`, and thinking-effort config are now resolved outside `ModelConfig` and materialized into the active provider `ModelConfig` at provider creation / request boundaries.

## What should be true

Expected precedence for active provider configs:

```text
explicit/session ModelConfig value
> user config / environment default
> predefined/canonical model metadata
> provider metadata fallback
> hardcoded runtime fallback
```

For thinking effort:

```text
request_params["thinking_effort"] / model suffix like gpt-5-high
> GOOSE_THINKING_EFFORT
> legacy CLAUDE_THINKING_TYPE / CLAUDE_THINKING_ENABLED / GEMINI3_THINKING_LEVEL
> none
```

`ModelConfig::new(...)` itself should not read `GOOSE_CONTEXT_LIMIT`, `GOOSE_MAX_TOKENS`, or `GOOSE_THINKING_EFFORT`.

## Setup

Use an isolated goose home so the test does not touch your real config or sessions:

```bash
source bin/activate-hermit
cargo build -p goose-cli

export GOOSE_BIN="$PWD/target/debug/goose"
export GOOSE_TEST_ROOT="$(mktemp -d)"
export GOOSE_PATH_ROOT="$GOOSE_TEST_ROOT"
mkdir -p "$GOOSE_PATH_ROOT/config"
```

If you want the run to actually complete, provide provider credentials. For OpenAI:

```bash
export OPENAI_API_KEY="..."
```

If credentials are missing or the model request fails, the session may still be created after provider initialization. For pure config-validation failures, the command should fail before making a provider request.

Create a minimal config:

```bash
cat > "$GOOSE_PATH_ROOT/config/config.yaml" <<'YAML'
active_provider: openai
providers:
  openai:
    enabled: true
    configured: true
    model: gpt-4o
YAML
```

Useful helper to inspect the latest persisted session model config:

```bash
inspect_latest_model_config() {
  python3 - <<'PY'
import json, os, sqlite3, sys
root = os.environ.get("GOOSE_PATH_ROOT")
if not root:
    sys.exit("GOOSE_PATH_ROOT is not set")
db = os.path.join(root, "data", "sessions", "sessions.db")
if not os.path.exists(db):
    sys.exit(f"No sessions DB found at {db}")
conn = sqlite3.connect(db)
row = conn.execute(
    """
    SELECT id, provider_name, model_config_json
    FROM sessions
    WHERE model_config_json IS NOT NULL
    ORDER BY updated_at DESC
    LIMIT 1
    """
).fetchone()
if not row:
    sys.exit("No session with model_config_json found")
session_id, provider, model_json = row
print(f"session_id: {session_id}")
print(f"provider:   {provider}")
print(json.dumps(json.loads(model_json), indent=2, sort_keys=True))
PY
}
```

Run helper:

```bash
run_probe() {
  "$GOOSE_BIN" run \
    --provider openai \
    --model "${1:-gpt-4o}" \
    --no-profile \
    --text "Reply with exactly OK."
}
```

Do **not** use `--no-session` for these checks, because inspecting the persisted `model_config_json` is the easiest way to verify resolution.

## Test 1: baseline canonical values

No user defaults:

```bash
unset GOOSE_CONTEXT_LIMIT GOOSE_MAX_TOKENS GOOSE_THINKING_EFFORT
unset CLAUDE_THINKING_TYPE CLAUDE_THINKING_ENABLED GEMINI3_THINKING_LEVEL

run_probe gpt-4o || true
inspect_latest_model_config
```

Expected for `openai/gpt-4o`:

```json
{
  "model_name": "gpt-4o",
  "context_limit": 128000,
  "max_tokens": 16384,
  "reasoning": false
}
```

There should be no `request_params.thinking_effort` unless it came from a suffix or other explicit source.

## Test 2: config.yaml defaults override canonical values

Add defaults to config:

```bash
cat > "$GOOSE_PATH_ROOT/config/config.yaml" <<'YAML'
active_provider: openai
providers:
  openai:
    enabled: true
    configured: true
    model: gpt-4o
GOOSE_CONTEXT_LIMIT: 250000
GOOSE_MAX_TOKENS: 1234
GOOSE_THINKING_EFFORT: high
YAML

unset GOOSE_CONTEXT_LIMIT GOOSE_MAX_TOKENS GOOSE_THINKING_EFFORT
unset CLAUDE_THINKING_TYPE CLAUDE_THINKING_ENABLED GEMINI3_THINKING_LEVEL

run_probe gpt-4o || true
inspect_latest_model_config
```

Expected:

```json
{
  "model_name": "gpt-4o",
  "context_limit": 250000,
  "max_tokens": 1234,
  "request_params": {
    "thinking_effort": "high"
  }
}
```

The config values should win over canonical `gpt-4o` values.

## Test 3: environment variables override config.yaml defaults

Keep the config from Test 2, then run:

```bash
export GOOSE_CONTEXT_LIMIT=333333
export GOOSE_MAX_TOKENS=2222
export GOOSE_THINKING_EFFORT=low

run_probe gpt-4o || true
inspect_latest_model_config
```

Expected:

```json
{
  "context_limit": 333333,
  "max_tokens": 2222,
  "request_params": {
    "thinking_effort": "low"
  }
}
```

This verifies that `Config::get_param` env precedence still applies after moving the reads out of `ModelConfig`.

## Test 4: explicit thinking effort beats the default

Model suffixes should materialize an explicit request param and win over `GOOSE_THINKING_EFFORT`.

```bash
export GOOSE_THINKING_EFFORT=low
unset GOOSE_CONTEXT_LIMIT GOOSE_MAX_TOKENS

run_probe gpt-5-high || true
inspect_latest_model_config
```

Expected:

```json
{
  "model_name": "gpt-5",
  "request_params": {
    "thinking_effort": "high"
  }
}
```

The important check is that `thinking_effort` is `high`, not the default `low`.

## Test 5: legacy thinking fallbacks are still defaults

### Claude legacy type

```bash
unset GOOSE_THINKING_EFFORT
export CLAUDE_THINKING_TYPE=enabled
unset CLAUDE_THINKING_ENABLED GEMINI3_THINKING_LEVEL

run_probe gpt-4o || true
inspect_latest_model_config
```

Expected:

```json
{
  "request_params": {
    "thinking_effort": "high"
  }
}
```

Also test `adaptive`; it should map to `high`. `disabled` should map to `off`.

### Claude legacy boolean

```bash
unset GOOSE_THINKING_EFFORT CLAUDE_THINKING_TYPE GEMINI3_THINKING_LEVEL
export CLAUDE_THINKING_ENABLED=false

run_probe gpt-4o || true
inspect_latest_model_config
```

Expected thinking effort: `off`.

### Gemini 3 legacy level

```bash
unset GOOSE_THINKING_EFFORT CLAUDE_THINKING_TYPE CLAUDE_THINKING_ENABLED
export GEMINI3_THINKING_LEVEL=high

run_probe gpt-4o || true
inspect_latest_model_config
```

Expected thinking effort: `high`.

## Test 6: invalid config fails before provider request

These should fail during provider creation / model-config normalization, before any model request is made.

```bash
GOOSE_MAX_TOKENS=0 run_probe gpt-4o
GOOSE_MAX_TOKENS=-100 run_probe gpt-4o
GOOSE_MAX_TOKENS=not_a_number run_probe gpt-4o
GOOSE_CONTEXT_LIMIT=0 run_probe gpt-4o
GOOSE_CONTEXT_LIMIT=not_a_number run_probe gpt-4o
```

Expected errors should mention the invalid key, for example:

```text
GOOSE_MAX_TOKENS must be greater than 0
GOOSE_CONTEXT_LIMIT must be greater than 0
```

For non-numeric values, expect a deserialize/invalid-value style error.

## Test 7: predefined model metadata still works

Unset user defaults so predefined metadata can apply:

```bash
unset GOOSE_CONTEXT_LIMIT GOOSE_MAX_TOKENS GOOSE_THINKING_EFFORT
export GOOSE_PREDEFINED_MODELS='[{"name":"my-test-model","context_limit":77777,"request_params":{"thinking_effort":"medium"}}]'

run_probe my-test-model || true
inspect_latest_model_config
```

Expected:

```json
{
  "model_name": "my-test-model",
  "context_limit": 77777,
  "request_params": {
    "thinking_effort": "medium"
  }
}
```

Now set a default context limit:

```bash
export GOOSE_CONTEXT_LIMIT=88888
run_probe my-test-model || true
inspect_latest_model_config
```

Expected `context_limit` should be `88888`, because user config defaults beat predefined/canonical metadata.

## Test 8: resumed sessions keep materialized values

This is an important semantic check.

1. Start a session with defaults:

```bash
export GOOSE_CONTEXT_LIMIT=111111
export GOOSE_MAX_TOKENS=1111
export GOOSE_THINKING_EFFORT=low
run_probe gpt-4o || true
inspect_latest_model_config
```

2. Change defaults and resume the same/latest session from the CLI:

```bash
export GOOSE_CONTEXT_LIMIT=222222
export GOOSE_MAX_TOKENS=2222
export GOOSE_THINKING_EFFORT=high

"$GOOSE_BIN" run --resume --text "Reply with exactly OK." || true
inspect_latest_model_config
```

Expected: if the resumed session already has `context_limit`, `max_tokens`, or `request_params.thinking_effort` materialized, those existing values should remain. Defaults only fill missing values; they do not override explicit/session values.

If this is not the desired UX, then the design needs another distinction between “materialized default” and “explicit session override.”

## Test 9: fast model caveat

Some providers create `fast_model_config` inside `ProviderDef::from_env`, after provider-registry normalization. Test this if the provider uses a fast model, such as OpenAI direct.

```bash
export GOOSE_CONTEXT_LIMIT=444444
export GOOSE_MAX_TOKENS=4444
export GOOSE_THINKING_EFFORT=medium

run_probe gpt-4o || true
inspect_latest_model_config
```

Look for `fast_model_config` in the persisted JSON.

Expected ideal behavior:

```json
{
  "fast_model_config": {
    "context_limit": 444444,
    "max_tokens": 4444,
    "request_params": {
      "thinking_effort": "medium"
    }
  }
}
```

If `fast_model_config` instead has canonical limits or missing defaults, that means defaults are being applied too early for fast models. A likely fix would be to apply context/max-token defaults after provider construction as well, or make `with_fast` inherit defaults from the parent model config.

## Optional debug output

If persisted session inspection is not enough, add a temporary debug print at the end of `ProviderEntry::normalize_model_config` in `crates/goose/src/providers/provider_registry.rs`:

```rust
if std::env::var_os("GOOSE_DEBUG_MODEL_CONFIG").is_some() {
    eprintln!(
        "resolved model config: provider={} model={} context_limit={:?} max_tokens={:?} thinking_effort={:?} request_params={:?} fast_model_config={:?}",
        self.metadata.name,
        model.model_name,
        model.context_limit,
        model.max_tokens,
        model.thinking_effort(),
        model.request_params,
        model.fast_model_config,
    );
}
```

Then run:

```bash
GOOSE_DEBUG_MODEL_CONFIG=1 run_probe gpt-4o
```

Do not commit this `eprintln!` unless you intentionally turn it into a real debug feature.

If you prefer tracing instead of stderr, use:

```rust
tracing::debug!(
    provider = %self.metadata.name,
    model = %model.model_name,
    context_limit = ?model.context_limit,
    max_tokens = ?model.max_tokens,
    thinking_effort = ?model.thinking_effort(),
    request_params = ?model.request_params,
    fast_model_config = ?model.fast_model_config,
    "resolved model config"
);
```

Then inspect logs under:

```bash
find "$GOOSE_PATH_ROOT/state/logs" -type f -name '*.log' -print
```

and run with a debug filter, for example:

```bash
RUST_LOG='goose::providers::provider_registry=debug,goose=info' run_probe gpt-4o
```

## Cleanup

```bash
rm -rf "$GOOSE_TEST_ROOT"
unset GOOSE_PATH_ROOT GOOSE_TEST_ROOT GOOSE_BIN
unset GOOSE_CONTEXT_LIMIT GOOSE_MAX_TOKENS GOOSE_THINKING_EFFORT
unset CLAUDE_THINKING_TYPE CLAUDE_THINKING_ENABLED GEMINI3_THINKING_LEVEL
unset GOOSE_PREDEFINED_MODELS
```
