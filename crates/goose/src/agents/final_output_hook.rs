//! Hook for handling the final output tool lifecycle.
//!
//! Checks whether the final output tool has been called and manages
//! the nudge/exit logic when the model stops calling tools.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::warn;

use super::final_output_tool::{FinalOutputTool, FINAL_OUTPUT_CONTINUATION_MESSAGE};
use super::hooks::{AgentHook, ExitAction, ExitReason, LoopContext, LoopEvent};
use crate::conversation::message::Message;

/// Hook that manages the final output tool's lifecycle in the agent loop.
///
/// Handles two scenarios:
/// 1. **Pre-inference**: If a previous iteration's tool call produced final output,
///    signals the loop to exit (optionally emitting the output message).
/// 2. **On loop exit (NoToolCalls)**: If the model stopped calling tools but final
///    output was expected, either nudges the model to call it or emits the output.
pub struct FinalOutputHook {
    final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>,
}

impl FinalOutputHook {
    pub fn new(final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>) -> Self {
        Self { final_output_tool }
    }

    /// Check the current state of the final output tool.
    /// Returns Some(None) if tool exists but no output yet, Some(Some(output)) if output ready, None if no tool.
    async fn check(&self) -> Option<Option<String>> {
        let guard = self.final_output_tool.lock().await;
        guard.as_ref().map(|fot| fot.final_output.clone())
    }
}

#[async_trait]
impl AgentHook for FinalOutputHook {
    async fn pre_inference(&self, ctx: &mut LoopContext) -> Result<()> {
        // Check if a previous iteration's tool call produced final output.
        if let Some(ref output) = self.check().await {
            if let Some(text) = output {
                let _ = ctx
                    .event_tx
                    .send(LoopEvent::Message(Message::assistant().with_text(text)));
            }
            ctx.should_exit = true;
        }
        Ok(())
    }

    async fn on_loop_exit(
        &self,
        reason: &ExitReason,
        ctx: &mut LoopContext,
    ) -> Result<ExitAction> {
        let ExitReason::NoToolCalls = reason else {
            return Ok(ExitAction::Defer);
        };

        match self.check().await {
            Some(None) => {
                // Final output tool exists but hasn't been called yet — nudge the model.
                warn!("Final output tool has not been called yet. Continuing agent loop.");
                let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                ctx.conversation.push(message.clone());
                let _ = ctx.event_tx.send(LoopEvent::Message(message));
                Ok(ExitAction::Continue)
            }
            Some(Some(output)) => {
                // Final output produced — emit and exit.
                let message = Message::assistant().with_text(output);
                ctx.conversation.push(message.clone());
                let _ = ctx.event_tx.send(LoopEvent::Message(message));
                Ok(ExitAction::Exit)
            }
            None => {
                // No final output tool configured — defer to next hook.
                Ok(ExitAction::Defer)
            }
        }
    }
}
