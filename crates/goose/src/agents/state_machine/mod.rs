mod machine;
mod operation;
mod ops_bang_shell;
mod ops_compaction;
mod ops_exit_on_error;
mod ops_llm;
mod ops_maxturns;
mod ops_retry;
mod ops_slash_command;
mod ops_steer;
mod ops_stop_hook;
mod ops_tool_approval;
mod ops_tool_pair_compaction;
mod ops_toolcalling;

pub mod test_helpers;

#[cfg(test)]
mod tests;

pub use machine::reply;

pub fn enabled() -> bool {
    std::env::var("GOOSE_STATE_MACHINE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false)
}
