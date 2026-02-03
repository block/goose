# Strict Completion Contract - Goose Enterprise Platform

**Version:** 2.0 (Phase 6 Complete)
**Last Verified:** February 2, 2026
**Status:** All Gates Passing

---

## Production Quality Requirements

All code must be **production-complete**. The following are strictly prohibited:

### Prohibited Code Patterns

| Pattern | Detection | Action |
|---------|-----------|--------|
| `TODO` / `FIXME` / `XXX` / `HACK` comments | `rg -n "TODO\|FIXME\|XXX\|HACK"` | Remove or implement |
| `todo!()` macro | `rg -n "todo!\("` | Implement functionality |
| `unimplemented!()` macro | `rg -n "unimplemented!\("` | Implement functionality |
| `panic!("TODO")` or similar | `rg -n 'panic!\("TODO'` | Implement proper logic |
| Placeholder returns | Code review | Replace with real logic |
| Mock/fake data in production | `rg -n "mock\|fake\|dummy"` | Replace with production code |
| Unwired modules | `cargo clippy` dead code warnings | Wire into runtime or remove |

---

## Definition of Done (All Required)

### Build & Quality Gates

| Gate | Command | Required Result |
|------|---------|-----------------|
| Format | `cargo fmt --all -- --check` | Exit code 0 |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Zero warnings |
| Build | `cargo build --workspace --all-features` | Successful compilation |
| Tests | `cargo test --workspace --all-features` | 672+ tests passing |
| Stub Scan | `rg -n -S -f scripts/patterns.stub_todo.txt crates/` | Zero matches in production |

### Integration Requirements

| Requirement | Verification Method |
|-------------|---------------------|
| Module Reachability | All modules wired into runtime/CLI (no dead code) |
| CLI Commands | Every command has working implementation + tests |
| Error Handling | All error paths have proper handling (no panics in production) |
| Security | Approval policies integrated for shell operations |
| Observability | Logging and cost tracking enabled |

---

## Current Compliance Status

### Goose Enterprise Platform (Phase 6)

```
✅ cargo fmt --all -- --check           → Formatted
✅ cargo clippy --workspace ...         → Zero warnings
✅ cargo build --workspace ...          → Successful
✅ cargo test --lib -p goose            → 672 tests passing
✅ Stub/TODO scan                       → Zero matches in production code
```

### Enterprise Components Verified

| Component | File | Lines | Tests | Status |
|-----------|------|-------|-------|--------|
| AgentOrchestrator | `orchestrator.rs` | 1,022 | ✓ | Production |
| WorkflowEngine | `workflow_engine.rs` | 831 | ✓ | Production |
| CodeAgent | `specialists/code_agent.rs` | 568 | ✓ | Production |
| TestAgent | `specialists/test_agent.rs` | 695 | ✓ | Production |
| DeployAgent | `specialists/deploy_agent.rs` | 972 | ✓ | Production |
| SecurityAgent | `specialists/security_agent.rs` | 817 | ✓ | Production |
| DocsAgent | `specialists/docs_agent.rs` | 69 | ✓ | Production |
| Planner System | `planner.rs` | 1,173 | ✓ | Production |
| Critic System | `critic.rs` | 951 | ✓ | Production |
| StateGraph Engine | `state_graph/mod.rs` | 595 | ✓ | Production |
| Checkpointing | `persistence/` | 1,130 | ✓ | Production |
| Reasoning Patterns | `reasoning.rs` | 760 | ✓ | Production |
| Reflexion Agent | `reflexion.rs` | 715 | ✓ | Production |
| Observability | `observability.rs` | 796 | ✓ | Production |
| Approval Policies | `approval/` | 692 | ✓ | Production |
| Done Gate | `done_gate.rs` | 427 | ✓ | Production |

---

## Mandatory Work Method

### Development Workflow

```
Phase A: Audit
├── Run audit scripts (run_audit.ps1 or run_audit.sh)
├── Catalog all findings into checklist
└── Prioritize by blocking vs. non-blocking

Phase B: Implementation
├── Fix items in small batches
├── Keep repo building at all times
└── Commit frequently with clear messages

Phase C: Verification (After Each Batch)
├── cargo fmt --all
├── cargo clippy --workspace --all-targets --all-features -- -D warnings
├── cargo test --workspace --all-features
└── Verify no regressions

Phase D: Final Audit
├── Full stub/TODO scan
├── Complete test suite execution
├── Integration verification
└── Documentation check
```

### Gate Failure Protocol

If any gate fails:
1. **Stop immediately** - Do not proceed to next task
2. **Fix the failure** - Address root cause, not symptoms
3. **Re-run all gates** - Ensure fix didn't break other things
4. **Document the fix** - Update completion report

---

## Completion Report Requirements

Every implementation must include a completion report with:

### 1. Build Evidence

```bash
$ cargo fmt --all -- --check
# [output or "No formatting changes needed"]

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
# [output showing zero warnings]

$ cargo test --lib -p goose
# [output with test counts: "672 passed"]
```

### 2. Files Changed

| File | Change Type | Description |
|------|-------------|-------------|
| `path/to/file.rs` | Modified | Brief description |
| `path/to/new.rs` | Added | Purpose of new file |

### 3. Stub Resolution Mapping

| Previous Stub | Resolution | Location |
|---------------|------------|----------|
| `todo!("implement X")` | Implemented with full logic | `file.rs:123` |
| `// TODO: add validation` | Added validation function | `file.rs:456` |

### 4. Risk Assessment

| Category | Status |
|----------|--------|
| Remaining risks | None |
| Known limitations | [List if any] |
| Future considerations | [Optional improvements] |

---

## Code Quality Standards

### Rust Best Practices

```rust
// DO: Use Result for fallible operations
fn process(input: &str) -> Result<Output, ProcessError> { ... }

// DON'T: Panic in production paths
fn process(input: &str) -> Output {
    panic!("not implemented") // ❌ PROHIBITED
}

// DO: Implement proper error types
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// DO: Document public APIs
/// Processes the input and returns the result.
///
/// # Arguments
/// * `input` - The input string to process
///
/// # Returns
/// * `Ok(Output)` on success
/// * `Err(ProcessError)` on failure
pub fn process(input: &str) -> Result<Output, ProcessError> { ... }
```

### Security Requirements

- Never expose secrets in logs (use `SecretString` or redaction)
- Use approval policies for shell operations (`ShellGuard`)
- Validate all external input before processing
- Implement timeouts for external operations
- Use secure random generation (`rand::thread_rng()`)

---

## Verification Commands

```bash
# Full audit suite
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --lib -p goose && \
echo "All gates passed!"

# Stub/TODO scan
rg -n -S "TODO|FIXME|todo!\(|unimplemented!\(" crates/goose/src/

# Dead code check
cargo clippy --workspace -- -W dead_code
```

---

**Goose Enterprise Platform Completion Contract**
*All gates passing | Zero technical debt | Production ready*
