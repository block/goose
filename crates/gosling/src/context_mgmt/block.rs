use crate::conversation::message::Message;
use serde::Serialize;

/// Classification of a piece of context assembled into a [`crate::context_mgmt::packet::ContextPacket`].
///
/// This mirrors the slots a future memory system would need to slot into
/// (`RetrievedMemory` is a placeholder today, always empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSlot {
    System,
    ProjectInstructions,
    RecentConversation,
    OlderConversationSummary,
    RecentToolResults,
    SummarizedToolResults,
    RetrievedMemory,
}

impl ContextSlot {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextSlot::System => "system",
            ContextSlot::ProjectInstructions => "project_instructions",
            ContextSlot::RecentConversation => "recent_conversation",
            ContextSlot::OlderConversationSummary => "older_conversation_summary",
            ContextSlot::RecentToolResults => "recent_tool_results",
            ContextSlot::SummarizedToolResults => "summarized_tool_results",
            ContextSlot::RetrievedMemory => "retrieved_memory",
        }
    }
}

/// How important a block of context is to preserve when the packet is over budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPriority {
    /// Never dropped or summarized: current user message, most recent assistant
    /// response, unresolved tool state, system prompt, project instructions.
    Required,
    /// Kept unless the budget cannot be met even after summarizing/dropping
    /// everything else.
    High,
    /// Summarized first when over budget.
    Medium,
    /// Dropped first when over budget (duplicates, stale/failed attempts).
    Low,
}

/// A unit of conversation classified into a slot with a priority, carrying
/// enough of the source messages to summarize or render it later.
#[derive(Debug, Clone)]
pub struct ContextBlock {
    pub slot: ContextSlot,
    pub priority: ContextPriority,
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
    pub label: String,
    /// Why this block got its priority, e.g. "duplicate_tool_output",
    /// "older_conversation", "long_tool_output". None for required/high blocks
    /// that need no justification.
    pub reason: Option<String>,
}
