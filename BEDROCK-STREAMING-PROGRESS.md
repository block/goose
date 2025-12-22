# Bedrock True Streaming Implementation - Progress Report

## Status: In Progress (80% Complete)

Branch: `feat/bedrock-true-streaming`

## Completed Work

### 1. ✅ Research & Planning
- Analyzed AWS SDK v1.120.0 ConverseStream API
- Documented event flow: MessageStart → ContentBlockStart → ContentBlockDelta → ContentBlockStop → MessageStop → Metadata
- Created comprehensive implementation plan (BEDROCK-STREAMING-PLAN.md)
- Updated TODO-BEDROCK.md with recent improvements

### 2. ✅ Code Structure Added
- Added imports for ConverseStreamError in bedrock.rs
- Created BedrockStreamAccumulator struct in formats/bedrock.rs
- Added streaming helper methods framework
- Replaced fake streaming with real converse_stream() API call

### 3. 🔧 Partially Complete
- BedrockStreamAccumulator implementation (needs minor fixes)
- converse_stream_internal() method (implemented but needs testing)
- Stream event handling logic (implemented)
- Error mapping for streaming errors (implemented)

## Remaining Issues to Fix

### Compilation Errors (5 errors)

1. **Conflicting trait implementations** (2 errors)
   - Issue: Double #[derive(Debug, Default)] declarations
   - Fix: Remove duplicate derives from accumulator struct definition
   
2. **unwrap_or on &str** (2 errors)  
   - Issue: `tool_use.name()` and `tool_use_id()` return `&str`, not `Option<&str>`
   - Fix: Change from `.unwrap_or("")` to just use the value directly
   
3. **Type mismatch in tool_call**
   - Issue: CallToolRequestParam expects different argument structure
   - Fix: Check exact field structure in rmcp::model::CallToolRequestParam

### Quick Fixes Needed

```rust
// In BedrockStreamAccumulator::handle_content_block_start
bedrock::ContentBlockStart::ToolUse(tool_use) => {
    // FIX: Remove .unwrap_or() calls
    let tool_use_id = tool_use.tool_use_id().to_string();  // Direct access
    let name = tool_use.name().to_string();                // Direct access
    self.tool_blocks.insert(index, (tool_use_id, name, String::new()));
}

// Remove duplicate #[derive(Debug, Default)] from struct definition
// Keep only one set of derives
```

## Files Modified

1. `crates/goose/src/providers/bedrock.rs`
   - Added ConverseStreamError import
   - Added converse_stream_internal() method
   - Replaced fake stream() implementation with real streaming
   - Added map_converse_stream_error() helper

2. `crates/goose/src/providers/formats/bedrock.rs`
   - Added BedrockStreamAccumulator struct
   - Added streaming event handlers
   - Added message building logic

3. `TODO-BEDROCK.md` - Updated with completion status
4. `BEDROCK-STREAMING-PLAN.md` - Created comprehensive plan

## Next Steps (15-30 minutes to complete)

### Step 1: Fix Compilation Errors (10 mins)
```bash
cd crates/goose/src/providers/formats
# Edit bedrock.rs to fix the 5 compilation errors above
```

### Step 2: Test Compilation (2 mins)
```bash
cd crates/goose
cargo build
```

### Step 3: Run Format & Lint (3 mins)
```bash
cargo fmt
./scripts/clippy-lint.sh
```

### Step 4: Test Streaming (10 mins)
```bash
# Add AWS credentials if not already configured
export AWS_PROFILE=your-profile
export AWS_REGION=us-east-1

# Run the streaming test (will need credentials)
cargo test -p goose test_bedrock_supports_streaming -- --ignored
```

### Step 5: Commit Changes
```bash
git add -A
git commit -m "feat: implement true Bedrock streaming with converse_stream API

- Replace fake streaming with real AWS ConverseStream API
- Add BedrockStreamAccumulator for incremental message building
- Support token-by-token streaming for text responses
- Accumulate and parse tool use requests from stream
- Add proper error handling for streaming errors
- Maintain backward compatibility with non-streaming mode

Addresses: TODO-BEDROCK.md Phase 2 Priority 1"
```

## Testing Strategy

### Manual Testing Required
1. Test with simple text prompt - verify token-by-token streaming
2. Test with tool calling - verify tools are properly accumulated
3. Test error scenarios - verify graceful error handling
4. Test usage statistics - verify token counts are reported

### Test Command
```bash
# Simple streaming test
goose session start --provider bedrock --model us.anthropic.claude-sonnet-4-5-20250929-v1:0
> Count to 10 slowly
# Should see numbers appearing one at a time

# Tool calling test  
> What's the current time? (use bash tool)
# Should see tool request built incrementally
```

## Expected Behavior After Fix

- ✅ Messages stream token-by-token in real-time
- ✅ Users see partial responses as they're generated
- ✅ Tool calls are properly assembled from streamed chunks
- ✅ Errors during streaming are handled gracefully
- ✅ Usage statistics (token counts) are reported at end
- ✅ No regression in non-streaming (complete) mode

## Performance Expectations

- **Latency**: First token in ~200-500ms (vs 2-5s for complete response)
- **Throughput**: Similar total time, but progressive display
- **User Experience**: Significantly improved perceived performance

## Documentation Updates Needed

After successful implementation:
1. Update TODO-BEDROCK.md - mark Phase 2 as complete
2. Add streaming example to docs/providers/bedrock.md
3. Update CHANGELOG.md with streaming feature

## Key Insights from Implementation

1. **AWS SDK Structure**: ConverseStreamOutput enum has 6 variants for different events
2. **No Text Variant**: ContentBlockStart has ToolUse/Image/ToolResult, but no Text variant
3. **Message Constructor**: Message::new() takes (role, created, content), not just (role)
4. **Tool Request API**: with_tool_request() takes (id, Result<CallToolRequestParam>)
5. **String References**: AWS SDK returns &str, not Option<&str> for most string fields

## Estimated Remaining Time

- Fix compilation errors: **10 minutes**
- Test and validate: **10 minutes**  
- Format and lint: **3 minutes**
- **Total: ~25 minutes to completion**

## Success Metrics

- ✅ Code compiles without errors
- ✅ All existing tests pass
- ✅ Streaming test shows incremental output
- ✅ Tool calling works in streaming mode
- ✅ Error handling tested
- ✅ cargo fmt passes
- ✅ ./scripts/clippy-lint.sh passes
