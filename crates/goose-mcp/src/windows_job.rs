// Stub Windows Job Object helpers for MCP path (no-ops on Windows).
// This file provides the minimal surface required by crates/goose-mcp/src/subprocess.rs
// when built on Windows. The real Windows Job Object integration lives in the goose crate.

#[cfg(windows)]
pub type HANDLE = *mut std::ffi::c_void;

#[cfg(windows)]
pub fn ensure_job_object() -> Option<HANDLE> {
    None
}

#[cfg(windows)]
pub fn attach_pid_to_job(_pid: u32) {}

#[cfg(windows)]
pub fn init_windows_cleanup() {}

#[cfg(windows)]
pub fn windows_cleanup_enabled() -> bool {
    false
}
