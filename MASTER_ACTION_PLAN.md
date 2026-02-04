# Goose Enterprise Platform - Master Action Plan
## Safe Merge Strategy & Implementation Roadmap

**Date:** 2026-02-03
**Status:** Pre-Merge Audit Phase
**Risk Level:** Medium (Git divergence requires careful resolution)

---

## Executive Summary

This document outlines the comprehensive action plan to safely merge 18 commits (6 upstream, 10+ local enterprise features) while completing Phase 6 agentic capabilities and ensuring zero breakage. The project has substantial work-in-progress including memory systems, swarm coordination, and provider routing that need careful integration.

### Current State Analysis

**Git Status:**
- Branch: `main` (diverged from `origin/main`)
- Local commits ahead: 10 (including Phase 1-7 enterprise features)
- Remote commits behind: 6 (including pctx code mode, provider improvements)
- Modified files: 14 (memory, providers, tests)
- Untracked new features: Memory system (4 files), Swarm (2 files), Provider routing (directory)

**Upstream Changes (6 commits to integrate):**
1. `98ba4a9` - docs: ovhcloud provider
2. `e0a1381` - Allow building with CUDA as the candle backend
3. `adc7732` - docs: gooseignore negation patterns
4. `1e64502` - feat: display subagent tool calls in CLI and UI
5. `7ee634a` - chore(release): release version 1.23.0 (minor)
6. `782ef02` - Document keyboard shortcut menu

**Local Enterprise Work (Phase 1-7):**
- Phase 1-5: Fully tested, 1,012+ tests passing
- Phase 6: Memory system (70% complete - 4 new files, 3 modified)
- Phase 7: Documented but not implemented
- Swarm coordination: New capability (basic structure)
- Provider routing: New capability (in progress)

---

## Risk Assessment

### High Priority Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Merge conflicts in modified files | High | High | Three-way merge with manual review |
| Test failures after merge | Medium | High | Run full test suite before/after |
| Memory system incomplete | High | Medium | Complete implementation before merge |
| Line ending conflicts (LF/CRLF) | High | Low | Normalize before merge |
| Build breaks | Low | High | Test build at each stage |

### Modified Files Requiring Careful Review

1. **Memory System (Critical - In Progress)**
   - `crates/goose/src/memory/mod.rs` (modified)
   - `crates/goose/src/memory/errors.rs` (modified)
   - `crates/goose/src/memory/working_memory.rs` (modified)
   - `crates/goose/src/memory/consolidation.rs` (new)
   - `crates/goose/src/memory/episodic_memory.rs` (new)
   - `crates/goose/src/memory/retrieval.rs` (new)
   - `crates/goose/src/memory/semantic_store.rs` (new)

2. **Provider System**
   - `crates/goose/src/providers/mod.rs` (modified)
   - `crates/goose/src/providers/routing/` (new directory)

3. **Swarm Coordination**
   - `crates/goose/src/swarm/mod.rs` (new)
   - `crates/goose/src/swarm/errors.rs` (new)

4. **Core Integration**
   - `crates/goose/src/lib.rs` (modified)
   - `crates/goose/src/agents/extension_manager.rs` (modified)
   - `crates/goose/Cargo.toml` (modified)
   - `Cargo.lock` (modified)

5. **E2E Tests**
   - 6 gate test files modified (need validation)

---

## Phase 1: Pre-Merge Preparation (Est. 2-3 hours)

### 1.1 Backup Current State ✓
```bash
# Already done: backup-pre-merge-20260203 tag exists
git tag -l backup-pre-merge-*
```

### 1.2 Complete Memory System Implementation

**Priority: CRITICAL**

The memory system is partially implemented and needs completion before merge:

#### Working Memory (70% complete)
- ✅ Basic structure exists
- ⚠️ Missing: Integration with agents
- ⚠️ Missing: Persistence layer connection
- ⚠️ Missing: Memory consolidation triggers

#### Episodic Memory (50% complete)
- ✅ Data structures defined
- ⚠️ Missing: Event recording logic
- ⚠️ Missing: Temporal indexing
- ⚠️ Missing: Retrieval methods

#### Semantic Store (40% complete)
- ✅ Storage interface sketched
- ⚠️ Missing: Vector embedding integration
- ⚠️ Missing: Similarity search
- ⚠️ Missing: Knowledge graph connections

#### Consolidation Engine (30% complete)
- ⚠️ Missing: Short→long term transfer logic
- ⚠️ Missing: Forgetting/pruning strategies
- ⚠️ Missing: Compression algorithms

#### Retrieval System (60% complete)
- ✅ Query interface defined
- ⚠️ Missing: Context-aware search
- ⚠️ Missing: Relevance ranking
- ⚠️ Missing: Multi-source aggregation

**Action Items:**
```rust
// Task 1: Complete working_memory.rs integration
- Implement agent lifecycle hooks
- Add memory save/load on checkpoints
- Wire up to extension_manager

// Task 2: Finish episodic_memory.rs
- Implement EventRecorder trait
- Add temporal indexing with timestamps
- Create retrieval_by_timerange methods

// Task 3: Complete semantic_store.rs
- Integrate embedding provider (OpenAI/local)
- Implement vector similarity search
- Add knowledge graph storage

// Task 4: Build consolidation.rs logic
- Implement periodic consolidation task
- Add importance scoring algorithm
- Create memory pruning strategies

// Task 5: Finish retrieval.rs
- Implement hybrid search (vector + BM25)
- Add context-aware ranking
- Create multi-memory-source queries
```

### 1.3 Complete Swarm Coordination

**Priority: HIGH**

Basic structure exists but needs agent coordination logic:

```rust
// Task 1: Implement swarm/mod.rs core
- Add SwarmCoordinator struct
- Implement task distribution
- Create agent communication protocol

// Task 2: Add coordination strategies
- Implement voting/consensus mechanisms
- Add leader election for complex tasks
- Create conflict resolution

// Task 3: Integration with team module
- Wire swarm to existing team/coordinator.rs
- Add swarm mode to orchestrator.rs
- Update CLI for swarm commands
```

### 1.4 Complete Provider Routing

**Priority: HIGH**

New routing directory needs implementation:

```rust
// Task 1: Create routing/mod.rs
- Implement ProviderRouter trait
- Add load balancing strategies
- Create fallback mechanisms

// Task 2: Build routing/strategies.rs
- Round-robin routing
- Cost-based routing
- Latency-based routing
- Model capability routing

// Task 3: Integrate with providers/mod.rs
- Update ProviderRegistry
- Add routing configuration
- Wire to orchestrator
```

### 1.5 Audit and Fix Test Files

**Priority: CRITICAL**

6 E2E tests modified - need to understand changes:

```bash
# Review each test for:
- What was changed and why
- Are changes compatible with upstream?
- Do tests still validate correct behavior?
- Any hardcoded assumptions broken by new features?

Files to review:
- crates/goose/tests/e2e/gate1_workflow_diffs.rs
- crates/goose/tests/e2e/gate2_patch_artifacts.rs
- crates/goose/tests/e2e/gate3_tool_execution.rs
- crates/goose/tests/e2e/gate4_checkpoint_resume.rs
- crates/goose/tests/e2e/gate5_safety_blocks.rs
- crates/goose/tests/e2e/gate6_mcp_roundtrip.rs
```

### 1.6 Normalize Line Endings

**Priority: MEDIUM**

Git warnings indicate CRLF/LF inconsistencies:

```bash
# Fix line endings before merge
git config core.autocrlf true
git add --renormalize .
git status
```

---

## Phase 2: Safe Merge Execution (Est. 1-2 hours)

### 2.1 Fetch Latest Upstream

```bash
git fetch origin main
git log origin/main --oneline -10
```

### 2.2 Create Merge Branch

```bash
git checkout -b merge/integrate-upstream-20260203
```

### 2.3 Merge with Three-Way Strategy

```bash
# Use recursive strategy with patience diff
git merge origin/main -X patience --no-ff \
  -m "Merge upstream: CUDA support, subagent UI, ovhcloud docs, v1.23.0"
```

### 2.4 Resolve Conflicts (Expected Areas)

**Likely Conflict Files:**
1. `Cargo.lock` - Dependency resolution
2. `crates/goose/Cargo.toml` - Version/dependency changes
3. `crates/goose/src/providers/mod.rs` - Provider additions
4. `crates/goose/src/agents/extension_manager.rs` - Agent system changes

**Conflict Resolution Strategy:**
- **Cargo.lock**: Accept upstream, then `cargo build` to regenerate
- **Cargo.toml**: Merge dependencies manually, keep higher versions
- **Code conflicts**:
  - Preserve local enterprise features
  - Integrate upstream improvements
  - Ensure both features work together

### 2.5 Post-Merge Validation

```bash
# Step 1: Verify merge succeeded
git status

# Step 2: Run clippy (must be zero warnings)
cargo clippy --all-targets --all-features 2>&1 | tee merge-clippy.log

# Step 3: Run full test suite
cargo test --all-features 2>&1 | tee merge-tests.log

# Step 4: Build release
cargo build --release 2>&1 | tee merge-build.log

# Step 5: Run E2E tests specifically
cargo test --test gate1_workflow_diffs
cargo test --test gate2_patch_artifacts
cargo test --test gate3_tool_execution
cargo test --test gate4_checkpoint_resume
cargo test --test gate5_safety_blocks
cargo test --test gate6_mcp_roundtrip
```

---

## Phase 3: Feature Completion (Est. 4-6 hours)

### 3.1 Complete Phase 6 Memory System

Implement remaining components from Phase 1.2 tasks.

**Validation Criteria:**
- [ ] All memory modules compile without warnings
- [ ] Unit tests for each memory component (target: 40+ tests)
- [ ] Integration test: Agent uses working memory across turns
- [ ] Integration test: Episodic memory records and retrieves events
- [ ] Integration test: Semantic store finds relevant knowledge
- [ ] Memory consolidation runs without errors
- [ ] Documentation for memory system API

### 3.2 Complete Swarm Coordination

Implement remaining components from Phase 1.3 tasks.

**Validation Criteria:**
- [ ] Swarm coordinator compiles and runs
- [ ] Multiple agents can join a swarm
- [ ] Task distribution works correctly
- [ ] Unit tests for coordination logic (target: 20+ tests)
- [ ] CLI command: `goose swarm create|join|status`
- [ ] Documentation for swarm usage

### 3.3 Complete Provider Routing

Implement remaining components from Phase 1.4 tasks.

**Validation Criteria:**
- [ ] Provider routing compiles
- [ ] Routing strategies implemented and tested
- [ ] Load balancing works across providers
- [ ] Fallback mechanisms trigger correctly
- [ ] Unit tests for routing (target: 25+ tests)
- [ ] Configuration examples added to docs

### 3.4 Add Missing Tests

Based on windsurf-chat.md Phase 6 goals:

```rust
// Memory System Tests
- test_working_memory_capacity_limits()
- test_episodic_memory_temporal_queries()
- test_semantic_store_vector_search()
- test_memory_consolidation_importance_scoring()
- test_retrieval_context_aware_ranking()

// Swarm Tests
- test_swarm_task_distribution()
- test_swarm_consensus_voting()
- test_swarm_leader_election()
- test_swarm_agent_failure_recovery()

// Routing Tests
- test_provider_round_robin_routing()
- test_provider_cost_based_routing()
- test_provider_fallback_on_failure()
- test_routing_capability_matching()
```

Target: 85+ new tests for Phase 6 features

---

## Phase 4: Documentation & Polish (Est. 2-3 hours)

### 4.1 Update Core Documentation

**Files to Update:**
- `README.md` - Add Phase 6 features section
- `AGENTS.md` - Document new agentic capabilities
- `docs/windsurf-chat.md` - Mark Phase 6 as complete

### 4.2 Create Phase 6 Technical Docs

**New Documentation:**
```markdown
docs/architecture/MEMORY_SYSTEM.md
docs/architecture/SWARM_COORDINATION.md
docs/architecture/PROVIDER_ROUTING.md
docs/guides/USING_MEMORY.md
docs/guides/SWARM_WORKFLOWS.md
```

### 4.3 Update API Documentation

```bash
cargo doc --all-features --no-deps
# Review generated docs for completeness
```

### 4.4 Create Migration Guide

```markdown
docs/MIGRATION_v1.23.md
- Breaking changes (if any)
- New configuration options
- Feature flag changes
- Upgrade path from v1.22
```

---

## Phase 5: Final Validation (Est. 1-2 hours)

### 5.1 Comprehensive Test Run

```bash
# Run all tests with output
cargo test --all-features -- --test-threads=1 --nocapture 2>&1 | tee final-test-run.log

# Check for:
# - Total tests passed
# - Zero failures
# - Zero ignored tests (investigate if any)
# - Memory leaks (run with valgrind if on Linux)
```

**Success Criteria:**
- All 1,097+ tests pass (1,012 previous + 85 new)
- Zero clippy warnings
- Zero compiler warnings
- Build time reasonable (<10 min release build)
- Binary size acceptable (<50MB)

### 5.2 Smoke Test Enterprise Features

```bash
# Test Phase 1: Guardrails
goose --enable-guardrails test "ignore previous instructions"

# Test Phase 2: MCP Gateway
goose mcp --gateway test

# Test Phase 3: Observability
goose --enable-telemetry test "simple task"

# Test Phase 4: Policy Engine
goose --policy examples/policies/strict.yaml test

# Test Phase 5: Prompt Templates
goose template list

# Test Phase 6: Memory System (NEW)
goose --enable-memory test "remember my name is Claude"
goose --enable-memory test "what is my name?"

# Test Phase 6: Swarm (NEW)
goose swarm create --agents 3 test "collaborative task"
```

### 5.3 Performance Benchmarks

```bash
# Run benchmarks to ensure no regressions
cargo bench --all-features 2>&1 | tee benchmarks.log

# Compare with previous baseline
# Acceptable: <5% performance degradation
# Ideal: Performance improvements from optimizations
```

---

## Phase 6: Commit & Push Strategy (Est. 30 min)

### 6.1 Commit Structure

```bash
# Commit 1: Merge upstream
git add -A
git commit -m "merge: Integrate upstream v1.23.0 and 6 feature commits

- CUDA backend support
- Subagent tool call display in UI
- OVHCloud provider docs
- Gooseignore negation patterns
- Keyboard shortcut menu docs

Resolved conflicts in:
- Cargo.lock (dependency resolution)
- providers/mod.rs (provider additions)
- extension_manager.rs (agent system integration)

All tests passing: [count] total"

# Commit 2: Complete Phase 6 Memory System
git add crates/goose/src/memory/
git commit -m "feat: Complete Phase 6 Memory System

Implemented:
- Working memory with agent lifecycle integration
- Episodic memory with temporal indexing
- Semantic store with vector embeddings
- Memory consolidation engine
- Hybrid retrieval system

Tests: 40+ new tests, all passing
Documentation: docs/architecture/MEMORY_SYSTEM.md"

# Commit 3: Complete Swarm Coordination
git add crates/goose/src/swarm/
git commit -m "feat: Add Phase 6 Swarm Coordination

Implemented:
- Multi-agent swarm coordinator
- Task distribution strategies
- Consensus voting and leader election
- Failure recovery mechanisms

Tests: 20+ new tests, all passing
CLI: New 'goose swarm' commands
Documentation: docs/architecture/SWARM_COORDINATION.md"

# Commit 4: Complete Provider Routing
git add crates/goose/src/providers/routing/
git commit -m "feat: Add Provider Routing System

Implemented:
- Provider router with multiple strategies
- Load balancing (round-robin, cost-based, latency-based)
- Automatic fallback mechanisms
- Capability-based routing

Tests: 25+ new tests, all passing
Documentation: docs/architecture/PROVIDER_ROUTING.md"

# Commit 5: Documentation and polish
git add docs/ README.md AGENTS.md
git commit -m "docs: Complete Phase 6 documentation

Updated:
- README with Phase 6 features
- AGENTS.md with new agentic capabilities
- Architecture documentation
- Migration guide for v1.23+

Phase 6 Status: ✅ COMPLETE"
```

### 6.2 Pre-Push Checklist

- [ ] All commits have clear, descriptive messages
- [ ] No debug code or commented-out blocks
- [ ] No TODO/FIXME without GitHub issues
- [ ] All new files have proper module documentation
- [ ] License headers present where required
- [ ] No accidental large files (check .gitignore)
- [ ] All tests pass one final time
- [ ] Build succeeds in release mode
- [ ] Clippy shows zero warnings

### 6.3 Push to Remote

```bash
# Push to fork first
git push fork main

# Create PR to upstream (if contributing back)
gh pr create --title "feat: Phase 6 Enterprise Enhancements + v1.23.0 Merge" \
  --body "See MASTER_ACTION_PLAN.md for details" \
  --base main \
  --head fork:main
```

---

## Phase 7: Post-Merge Monitoring (Est. 1 hour)

### 7.1 CI/CD Verification

Monitor GitHub Actions for:
- [ ] Linux build passes
- [ ] Windows build passes
- [ ] macOS build passes
- [ ] All platform tests pass
- [ ] Release artifacts generated

### 7.2 Create Release

If all CI passes:

```bash
git tag -a v1.24.0 -m "Release v1.24.0 - Phase 6 Complete

New Features:
- Advanced memory system (working, episodic, semantic)
- Multi-agent swarm coordination
- Intelligent provider routing
- Merged upstream v1.23.0 improvements

Statistics:
- 1,097+ tests passing
- 85+ new tests added
- Zero clippy warnings
- Full documentation coverage

Breaking Changes: None
Migration Guide: docs/MIGRATION_v1.23.md"

git push fork v1.24.0
```

### 7.3 Update Documentation Site

If project has docs site:
```bash
cd documentation
npm run build
npm run deploy
```

---

## Contingency Plans

### If Merge Fails

**Scenario 1: Too Many Conflicts**
```bash
git merge --abort
# Strategy: Rebase instead
git rebase origin/main
# Resolve conflicts one commit at a time
```

**Scenario 2: Tests Fail After Merge**
```bash
git merge --abort
# Strategy: Cherry-pick upstream commits one by one
git cherry-pick 782ef02e6  # keyboard shortcuts
git cherry-pick 7ee634eaa  # v1.23.0 release
# ... test after each pick
```

**Scenario 3: Build Breaks**
```bash
# Rollback to backup
git reset --hard backup-pre-merge-20260203
# Analyze upstream changes in isolation
git checkout -b analyze-upstream origin/main
cargo build
```

### If Feature Implementation Takes Too Long

**Option A: Feature Flags**
```rust
// Ship incomplete features behind flags
#[cfg(feature = "experimental-memory")]
pub mod memory;

// In Cargo.toml
[features]
experimental-memory = []
experimental-swarm = []
```

**Option B: Staged Rollout**
```markdown
Phase 6.1: Memory system basics (ship now)
Phase 6.2: Advanced memory (next sprint)
Phase 6.3: Swarm + routing (future)
```

---

## Success Metrics

### Quantitative
- [ ] 100% tests passing (1,097+ total)
- [ ] 0 clippy warnings
- [ ] 0 compiler warnings
- [ ] <5% performance regression (ideally 0%)
- [ ] 85+ new tests added
- [ ] Build time <10 minutes (release)
- [ ] Binary size <50MB

### Qualitative
- [ ] All Phase 6 features functional and documented
- [ ] Upstream changes integrated without breaking local work
- [ ] Code review ready (clean history, good commit messages)
- [ ] Architecture decisions documented
- [ ] Migration path clear for users
- [ ] No regression in existing features

---

## Timeline Estimates

| Phase | Tasks | Time | Cumulative |
|-------|-------|------|------------|
| 1 | Pre-Merge Prep | 2-3h | 2-3h |
| 2 | Safe Merge | 1-2h | 3-5h |
| 3 | Feature Completion | 4-6h | 7-11h |
| 4 | Documentation | 2-3h | 9-14h |
| 5 | Final Validation | 1-2h | 10-16h |
| 6 | Commit & Push | 0.5h | 10.5-16.5h |
| 7 | Post-Merge Monitoring | 1h | 11.5-17.5h |

**Total Estimated Time:** 12-18 hours of focused work

**Recommended Schedule:**
- Day 1 (6h): Phases 1-2
- Day 2 (6h): Phase 3 (Memory + Swarm)
- Day 3 (4h): Phase 3 (Routing) + Phase 4 (Docs)
- Day 4 (2h): Phases 5-7 (Validation & Release)

---

## Next Actions (Immediate)

1. **Review this plan with team/stakeholders** - Ensure alignment on approach
2. **Set up environment** - Ensure all tools available (cargo, git, gh CLI)
3. **Create tracking issues** - Break down tasks into GitHub issues
4. **Begin Phase 1.2** - Start completing memory system (highest priority)
5. **Daily standup** - Progress check at end of each day

---

## Appendix A: File Modification Matrix

| File | Status | Complexity | Merge Risk | Priority |
|------|--------|------------|------------|----------|
| Cargo.lock | Modified | High | High | Critical |
| Cargo.toml | Modified | Medium | Medium | High |
| memory/mod.rs | Modified | High | Low | Critical |
| memory/working_memory.rs | Modified | High | Low | Critical |
| memory/errors.rs | Modified | Low | Low | Medium |
| providers/mod.rs | Modified | Medium | Medium | High |
| lib.rs | Modified | Medium | Medium | High |
| extension_manager.rs | Modified | Medium | Low | Medium |
| 6x gate tests | Modified | Medium | Low | High |

## Appendix B: Dependencies Added

Check `Cargo.toml` diff for new dependencies:
- Likely: Vector DB integration (e.g., `qdrant-client`, `faiss`, `tantivy`)
- Likely: Memory serialization (`bincode`, `postcard`)
- Likely: Async coordination (`tokio` features, `async-trait` extensions)

## Appendix C: Reference Links

- Windsurf Chat: `docs/windsurf-chat.md`
- Phase 6 Original Plan: (section in windsurf-chat.md)
- Upstream Repo: `origin/main`
- Fork Repo: `fork/main`
- CI Workflows: `.github/workflows/`

---

**Plan Status:** Draft - Ready for review and execution
**Last Updated:** 2026-02-03
**Owner:** Development Team
**Reviewers:** [TBD]

