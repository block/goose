//! Experimental state-machine-based agent loop.
//!
//! Alternative implementation of `Agent::reply` that breaks the monolithic
//! streaming loop into a re-entrant sequence of `Operation`s that observe
//! the `Session` and return declarative outcomes. Gated behind the
//! `GOOSE_STATE_MACHINE` environment variable.
//!
//! See `WIP.md` for design notes and the operation backlog.

mod machine;
mod operation;
mod ops_compaction;
mod ops_exit_on_error;
mod ops_llm;
mod ops_maxturns;
mod ops_slash_command;
mod ops_stop_hook;
mod ops_tool_approval;
mod ops_toolcalling;

pub mod test_helpers;

#[cfg(test)]
mod tests;

pub use machine::reply;

/// Returns true when the experimental state-machine loop should be used.
pub fn enabled() -> bool {
    std::env::var("GOOSE_STATE_MACHINE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false)
}
