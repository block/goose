//! Runs an ordered, re-entrant pipeline over persisted conversation state.
//!
//! Callers persist incoming messages, construct `Step`s from their own operations,
//! and choose whether to call `StateMachine::step`, `StateMachine::apply`, or
//! `StateMachine::run`. Goose's concrete operations remain internal because their
//! configuration is part of `Agent::reply`, not the state-machine protocol.

mod machine;
mod operation;
mod ops_bang_shell;
mod ops_compaction;
mod ops_doctor;
mod ops_exit_on_error;
mod ops_llm;
mod ops_maxturns;
mod ops_project;
mod ops_recipe;
mod ops_retry;
mod ops_skills;
mod ops_slash_command;
mod ops_steer;
mod ops_stop_hook;
mod ops_tool_approval;
mod ops_tool_pair_compaction;
mod ops_toolcalling;
mod ops_unknown_tool;
mod usage;

#[cfg(test)]
mod tests;

pub use machine::{StateMachine, Step};
pub use operation::{
    applied, not_applicable, yielded, yielded_with, Emitter, Inference, InferenceInput, Operation,
    OperationResult, SlashCommand, StateEffect, StepResult,
};

pub(super) use ops_bang_shell::{bang_shell_command, BangShellOperation};
pub(super) use ops_compaction::CompactionOperation;
pub(super) use ops_doctor::DoctorOperation;
pub(super) use ops_exit_on_error::ExitOnErrorOperation;
pub(super) use ops_llm::InferenceRunner;
pub(super) use ops_maxturns::{MaxTurnsOperation, MAX_TURNS_MESSAGE};
pub(super) use ops_project::ProjectOperation;
pub(super) use ops_recipe::RecipeOperation;
pub(super) use ops_retry::RetryOperation;
pub(super) use ops_skills::SkillOperation;
pub(super) use ops_slash_command::SlashCommandOperation;
pub(super) use ops_steer::{SteerOperation, SteerQueue};
pub(super) use ops_stop_hook::StopHookOperation;
pub(super) use ops_tool_approval::ToolApprovalOperation;
pub(super) use ops_tool_pair_compaction::ToolPairCompactionOperation;
pub(super) use ops_toolcalling::ToolExecutionOperation;
pub(super) use ops_unknown_tool::UnknownToolOperation;

pub fn enabled() -> bool {
    std::env::var("GOOSE_STATE_MACHINE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false)
}
