mod client;
mod handle;
pub mod types;
mod utils;

pub use client::GoosedClient;
pub use handle::GoosedHandle;
pub use types::{
    ExtensionLoadResultResponse, GetPromptResultResponse, PlanProposalTask, PromptArgumentResponse,
    PromptResponse, SseEvent, ToolInfoResponse,
};

#[cfg(test)]
mod tests;
