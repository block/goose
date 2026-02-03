//! End-to-End Reality Gate Tests for Goose Enterprise Platform
//!
//! These tests prove the platform does REAL work, not just documentation.
//! Each "Gate" validates a critical capability with concrete evidence.
//!
//! Reality Gates:
//! - Gate 1: Workflow produces real git diffs
//! - Gate 2: Agents emit PatchArtifact objects
//! - Gate 3: Tool execution is real (process tree, logs, exit codes)
//! - Gate 4: Checkpoints survive simulated crash and resume
//! - Gate 5: ShellGuard blocks dangerous commands in reality
//! - Gate 6: MCP/Extensions roundtrip real data

pub mod gate1_workflow_diffs;
pub mod gate2_patch_artifacts;
pub mod gate3_tool_execution;
pub mod gate4_checkpoint_resume;
pub mod gate5_safety_blocks;
pub mod gate6_mcp_roundtrip;
