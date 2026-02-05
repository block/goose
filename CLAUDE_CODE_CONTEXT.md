# Context for Claude Code / Claude Desktop

**Date:** 2026-02-05  
**Purpose:** Provide comprehensive context for AI-assisted debugging and development

## 🎯 Current Objective

**Fix all CI workflow failures and complete Phase 7-8 integration**

The CI has been failing for several hours with multiple issues. LM Studio provider has been fixed (commit `fcc126160`), now waiting for CI validation at run #21715742168.

## 🔥 Critical Path

1. **Monitor CI run #21715742168** for success/failure
2. If passes: Regenerate OpenAPI schema with `just generate-openapi`
3. If fails: Check logs and fix remaining issues
4. Complete Computer Use implementation
5. Update all documentation

## 📁 Key Files and Their Status

### Recently Modified (Working)
- ✅ `crates/goose/src/providers/lmstudio.rs` - LM Studio provider (just fixed)
- ✅ `crates/goose/src/providers/mod.rs` - Added lmstudio module
- ✅ `crates/goose/src/providers/init.rs` - Registered LM Studio provider
- ✅ `crates/goose-cli/src/computer_use.rs` - Computer Use CLI (integrated, needs implementation)
- ✅ `crates/goose-cli/src/cli.rs` - Added Computer Use command

### Needs Attention
- ⚠️ `ui/desktop/openapi.json` - Outdated, regenerate after CI passes
- ⚠️ `README.md` - Needs Phase 7-8 updates
- ⚠️ `AGENTS.md` - Needs Computer Use and LM Studio details
- ⚠️ Tests in scenario suite - Stalling/timing out

### Documentation Created Today
- ✅ `ISSUES.md` - Comprehensive issue tracking (just created)
- ✅ `CLAUDE_CODE_CONTEXT.md` - This file

## 🏗️ Architecture Overview

### Provider System
```
crates/goose/src/providers/
├── base.rs              # ProviderDef trait, Provider trait, ConfigKey
├── mod.rs               # Module declarations
├── init.rs              # Provider registration
├── lmstudio.rs          # NEW: LM Studio provider
├── ollama.rs            # Reference implementation
└── openai_compatible.rs # Base for LM Studio
```

**Key Pattern:** All providers must implement `ProviderDef` trait:
```rust
pub trait ProviderDef: Send + Sync {
    type Provider: Provider + 'static;
    fn metadata() -> ProviderMetadata;
    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<Self::Provider>>;
}
```

### Computer Use CLI
```
crates/goose-cli/src/
├── cli.rs           # Main CLI entry point
├── computer_use.rs  # Computer Use implementation
└── commands/        # Other CLI commands
```

**Subcommands:**
- `control` - Direct computer control
- `debug` - Interactive debugging
- `test` - Automated testing
- `remote` - Remote access
- `fix` - Workflow failure analysis

## 🐛 Common Issues and Solutions

### Issue 1: ProviderDef Not Implemented
**Error:** `error[E0277]: the trait bound 'XProvider: ProviderDef' is not satisfied`

**Solution:**
```rust
impl ProviderDef for YourProvider {
    type Provider = OpenAiCompatibleProvider; // or your provider type
    
    fn metadata() -> ProviderMetadata {
        Self::metadata() // delegate to existing method
    }
    
    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
            // Implementation
        })
    }
}
```

### Issue 2: ConfigKey Invalid Fields
**Error:** `error[E0560]: struct 'ConfigKey' has no field named 'description'`

**Solution:**
```rust
// WRONG:
ConfigKey {
    name: "API_KEY".to_string(),
    description: Some("...".to_string()), // NO! Field doesn't exist
    required: true,
    secret: true,
}

// CORRECT:
ConfigKey::new("API_KEY", true, true, None)
// Parameters: (name, required, secret, default)
```

### Issue 3: Type Inference in Tests
**Error:** `cannot infer type of the type parameter 'T' declared on the enum 'Option'`

**Solution:**
```rust
// WRONG:
env_lock::lock_env([("VAR", None)])

// CORRECT:
env_lock::lock_env([("VAR", None::<&str>)])
```

## 🔧 Build and Test Commands

### Local Development
```bash
# Activate Hermit environment
source bin/activate-hermit

# Build (debug)
cargo build

# Build (release)
cargo build --release

# Run all tests
cargo test

# Run specific crate tests
cargo test -p goose
cargo test -p goose-cli

# Run specific test file
cargo test --package goose --test mcp_integration_test

# Format code
cargo fmt

# Lint
cargo clippy --fix
./scripts/clippy-lint.sh
```

### CI-Specific Commands
```bash
# Generate OpenAPI schema (MUST run after provider changes)
just generate-openapi

# Release binary
just release-binary

# Record MCP tests
just record-mcp-tests

# Run UI
just run-ui
```

### Debugging CI Failures
```bash
# Get CI run status
gh run list --limit 5

# View specific run
gh run view <run_id>

# Get logs from specific job
gh api repos/Ghenghis/goose/actions/jobs/<job_id>/logs

# Watch run
gh run watch <run_id>
```

## 📊 CI Workflow Jobs

### Main CI Workflow
1. **changes** - Detect which files changed
2. **Build and Test Rust Project** - Main build + cargo test
3. **Check OpenAPI Schema** - Verify openapi.json is current
4. **Check Rust Code Format** - cargo fmt check
5. **Lint Rust Code** - cargo clippy
6. **Test and Lint Electron Desktop App** - UI tests
7. **Run Scenario Tests** - Integration tests (often slow/stalling)

### Common Failure Patterns
- **Build fails:** Usually trait implementation or type errors
- **OpenAPI fails:** Schema out of date, need to regenerate
- **Lint fails:** Clippy warnings, unused code, wrong patterns
- **Scenario tests stall:** Infinite loops, network timeouts, resource issues

## 🎨 Phase 7-8 Feature Details

### LM Studio Provider
**Purpose:** Local AI model hosting with OpenAI-compatible API

**Supported Models:**
- GLM 4.6, 4.7, 4-9b (Chinese models)
- Qwen2.5 Coder (7B, 14B, 32B)
- Qwen3 Coder
- DeepSeek R1 distill (7B, 32B) for reasoning
- Qwen2 VL (vision)
- Meta Llama 3.1, Mistral 7B

**Configuration:**
```bash
export LMSTUDIO_BASE_URL="http://localhost:1234/v1"  # optional
export LMSTUDIO_API_TOKEN="your-token"               # optional
```

**Features:**
- OpenAI-compatible API (`/v1/*`)
- Native LM Studio API (`/api/v1/*`)
- Anthropic-compatible API (`/v1/messages`)
- Model management (load/unload/download)
- MCP integration for tool calling
- Stateful chats with `previous_response_id`
- Speculative decoding with draft models
- Idle TTL and auto-evict
- Enhanced stats (tokens/second, TTFT)

### Computer Use CLI
**Purpose:** AI-driven computer control and debugging interface

**Commands:**
```bash
# Direct computer control
goose computer-use control --keyboard "Hello World"
goose computer-use control --mouse click --x 100 --y 200

# Interactive debugging
goose computer-use debug --attach <process_id>
goose computer-use debug --breakpoint <file>:<line>

# Automated testing
goose computer-use test --suite integration
goose computer-use test --visual

# Remote access
goose computer-use remote --enable --port 8080

# Workflow failure analysis
goose computer-use fix --analyze-failures
```

**Current Implementation Status:**
- ✅ CLI structure and argument parsing
- ✅ Integration into main goose CLI
- ⚠️ Core logic incomplete (many TODOs)
- ❌ No integration tests
- ❌ No documentation

**Needs Implementation:**
- Session management (create, restore, list)
- Vision processing for UI automation
- Remote support for distributed debugging
- Workflow analyzer for CI failures
- Interactive debugger with breakpoints

## 🚨 Known Pitfalls

### 1. Don't Add Fields to ConfigKey
ConfigKey struct has fixed fields. Use `ConfigKey::new()` constructor only.

### 2. Provider Registration Pattern
When adding a provider:
1. Add module to `providers/mod.rs`
2. Import in `providers/init.rs`
3. Register in `init_registry()` function
4. Implement `ProviderDef` trait

### 3. BoxFuture for Async Traits
```rust
use futures::future::BoxFuture;

fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<Self::Provider>> {
    Box::pin(async move {
        // async code here
    })
}
```

### 4. OpenAPI Schema Sync
After ANY changes to `goose-server`, regenerate schema:
```bash
just generate-openapi
git add ui/desktop/openapi.json
git commit -m "Update OpenAPI schema"
```

## 📈 Success Metrics

### CI Must Pass:
- [ ] Build and Test job completes
- [ ] OpenAPI check passes
- [ ] Lint job passes with no warnings
- [ ] Format check passes
- [ ] Desktop tests pass
- [ ] Scenario tests complete in <10 minutes

### Phase 7-8 Complete When:
- [ ] LM Studio provider fully tested
- [ ] Computer Use CLI functional
- [ ] All documentation updated
- [ ] No regression in existing features
- [ ] CI stable for 3+ consecutive runs

## 🔗 External References

- [LM Studio API Docs](https://lmstudio.ai/docs/developer)
- [LM Studio REST API](https://lmstudio.ai/docs/developer/rest/endpoints)
- [Kilo CLI Docs](https://kilo.ai/docs/cli) - Separate tool, not integrated
- [GitHub Actions Docs](https://docs.github.com/en/actions)

## 💡 Tips for Claude Code

1. **Start with ISSUES.md** - Read current status
2. **Check CI logs first** - Don't guess, verify actual errors
3. **Follow existing patterns** - Look at `ollama.rs` for provider examples
4. **Test locally** - Run `cargo test -p goose` before pushing
5. **Update docs as you go** - Keep ISSUES.md current
6. **Use grep to find examples** - Search for similar implementations
7. **One issue at a time** - Fix, test, commit, then move to next
8. **Regenerate OpenAPI** - After provider changes

## 🎯 Next Actions

1. Wait for CI run #21715742168 results
2. If successful: Regenerate OpenAPI, update README
3. If failed: Analyze logs, fix errors, repeat
4. Once CI stable: Implement Computer Use core logic
5. Add integration tests
6. Update all Phase 7-8 documentation
7. Create architecture diagrams
