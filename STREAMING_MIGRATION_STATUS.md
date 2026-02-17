# Streaming-Only Architecture Migration Status

## ✅ Completed

### Core Architecture Changes
1. **Provider trait updated** - `stream()` is now the primary method
   - `complete()` and `complete_fast()` call `stream()` and collect results
   - Added `collect_stream()` helper to gather chunks into single message
   - `stream_with_model()` added for fast model routing
   - ✅ Removed `complete_with_model()` from trait definition

2. **All non-streaming providers now have `stream()`**
   - bedrock, claude_code, codex, cursor_agent, gemini_cli
   - litellm, sagemaker_tgi, snowflake, venice, testprovider
   - Currently these call `complete_with_model()` internally (needs fixing)

3. **Removed `supports_streaming()` everywhere**
   - ✅ Deleted from Provider trait
   - ✅ Removed all provider implementations
   - ✅ Removed conditional in `reply_parts.rs`
   - ✅ Removed `supports_streaming` fields from provider structs

4. **Updated all external callers**
   - ✅ `crates/goose-cli/src/commands/configure.rs`
   - ✅ `crates/goose/src/agents/mcp_client.rs`
   - ✅ `crates/goose/examples/databricks_oauth.rs`
   - ✅ `crates/goose/examples/image_tool.rs`

5. **Updated LeadWorkerProvider**
   - ✅ Renamed `complete_with_model()` to `stream_with_model()`
   - ✅ Added `stream()` method
   - ✅ Now calls `stream_with_model()` on child providers

6. **Started removing `complete_with_model()` from providers**
   - ✅ anthropic - deleted
   - ✅ bedrock - inlined into stream()
   - ✅ ollama - deleted
   - ✅ openai - deleted

## 🚧 Remaining Work

### Remove `complete_with_model()` from Streaming Providers (8 providers)

These providers have native streaming and don't use `complete_with_model()` in their `stream()` method.
**Action: Delete the entire `complete_with_model()` method**

1. **chatgpt_codex.rs** (line 889)
2. **databricks.rs** (line 280)
3. **gcpvertexai.rs** (line 617)
4. **githubcopilot.rs** (line 421)
5. **google.rs** (line 161)
6. **openai_compatible.rs** (line 81)
7. **openrouter.rs** (line 275)
8. **tetrate.rs** (line 173)

**Pattern:**
```rust
// BEFORE: Delete this entire method
async fn complete_with_model(...) -> Result<(Message, ProviderUsage), ProviderError> {
    // ... implementation ...
}

// No replacement needed - streaming already works via stream()
```

### Update Non-Streaming Providers (9 providers)

These providers currently have `stream()` methods that call `complete_with_model()`.
**Action: Move the logic from `complete_with_model()` into `stream()`, wrapping result with `stream_from_single_message()`**

1. **claude_code.rs**
2. **codex.rs**
3. **cursor_agent.rs**
4. **gemini_cli.rs**
5. **litellm.rs**
6. **sagemaker_tgi.rs**
7. **snowflake.rs**
8. **venice.rs**
9. **testprovider.rs** (has 2 implementations: TestProvider and MockProvider)

**Pattern (see bedrock.rs as reference):**
```rust
// BEFORE:
async fn complete_with_model(..., session_id: Option<&str>, model_config: &ModelConfig, ...)
    -> Result<(Message, ProviderUsage), ProviderError>
{
    // ... API call logic ...
    let (message, usage) = /* result */;
    Ok((message, ProviderUsage::new(model_name, usage)))
}

async fn stream(...) -> Result<MessageStream, ProviderError> {
    let model_config = self.get_model_config();
    let (message, usage) = self.complete_with_model(Some(session_id), &model_config, system, messages, tools).await?;
    Ok(super::base::stream_from_single_message(message, usage))
}

// AFTER:
async fn stream(..., session_id: &str, ...) -> Result<MessageStream, ProviderError> {
    let model_name = self.model.model_name.clone();
    let session_id_opt = if session_id.is_empty() { None } else { Some(session_id) };

    // ... same API call logic ...
    let (message, usage) = /* result */;

    let provider_usage = ProviderUsage::new(model_name, usage);
    Ok(super::base::stream_from_single_message(message, provider_usage))
}
// Delete complete_with_model() entirely
```

### Update Test Mock Providers

**Files to update:**
- `crates/goose/tests/compaction.rs` - MockProvider
- `crates/goose/tests/agent.rs` - MockProvider
- `crates/goose/tests/mcp_integration_test.rs` - MockProvider
- `crates/goose/src/agents/reply_parts.rs` - test MockProvider (if exists)
- `crates/goose/src/providers/lead_worker.rs` - test MockProviders (2 instances)

**Pattern:**
```rust
// Change MockProvider from:
async fn complete_with_model(...) -> Result<(Message, ProviderUsage), ProviderError> {
    Ok((mock_message, mock_usage))
}

// To:
async fn stream(...) -> Result<MessageStream, ProviderError> {
    Ok(stream_from_single_message(mock_message, mock_usage))
}
```

### Run Tests

```bash
# Test providers
cargo test --package goose --lib providers

# Test agents
cargo test --package goose --lib agents

# Test all
cargo test --workspace
```

## Key Files

- **Provider trait:** `crates/goose/src/providers/base.rs`
- **Main consumer:** `crates/goose/src/agents/reply_parts.rs`
- **Helper functions:** `stream_from_single_message()`, `collect_stream()` in base.rs

## Success Criteria

- ✅ All providers implement only `stream()` method
- ✅ No `complete_with_model()` in any provider file
- ✅ No `supports_streaming()` anywhere
- All tests pass
- Streaming and non-streaming providers both work correctly

## Quick Commands

```bash
# Find remaining complete_with_model implementations
grep -r "async fn complete_with_model" crates/goose/src/providers/*.rs

# Find remaining supports_streaming references
grep -r "supports_streaming" crates/goose/src/providers/*.rs

# Compile check
cargo check --package goose --lib

# Run provider tests
cargo test --package goose --lib providers

# Run all tests
cargo test --workspace
```
