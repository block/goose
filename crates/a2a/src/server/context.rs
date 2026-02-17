//! Server request context.

use crate::types::core::{Message, Task};

/// Context for a single A2A request, built from an incoming message.
pub struct RequestContext {
    pub user_message: Message,
    pub task_id: String,
    pub context_id: String,
    pub task: Option<Task>,
    pub reference_tasks: Vec<Task>,
    pub requested_extensions: Vec<String>,
}
