//! Hook for handling retry logic when the model stops calling tools.
//!
//! If a retry configuration is present, runs success checks and retries
//! the conversation from its initial state when checks fail.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::info;

use super::final_output_tool::FinalOutputTool;
use super::hooks::{AgentHook, ExitAction, ExitReason, LoopContext, LoopEvent};
use super::retry::{RetryManager, RetryResult};
use super::types::SessionConfig;
use crate::conversation::message::Message;

/// Hook that manages retry logic in the agent loop.
///
/// When the model responds without calling tools, this hook:
/// 1. Runs configured success checks (shell commands)
/// 2. If checks fail and retries remain, resets the conversation to its initial state
/// 3. Increments the retry counter
///
/// The hook stores its own `RetryManager` instance, which is created fresh
/// per reply (starting with 0 attempts).
pub struct RetryHook {
    retry_manager: RetryManager,
    session_config: SessionConfig,
    initial_messages: Vec<Message>,
    final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>,
}

impl RetryHook {
    pub fn new(
        session_config: SessionConfig,
        initial_messages: Vec<Message>,
        final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>,
    ) -> Self {
        Self {
            retry_manager: RetryManager::new(),
            session_config,
            initial_messages,
            final_output_tool,
        }
    }
}

#[async_trait]
impl AgentHook for RetryHook {
    async fn on_loop_exit(
        &self,
        reason: &ExitReason,
        ctx: &mut LoopContext,
    ) -> Result<ExitAction> {
        let ExitReason::NoToolCalls = reason else {
            return Ok(ExitAction::Defer);
        };

        let result = self
            .retry_manager
            .handle_retry_logic(
                &mut ctx.conversation,
                &self.session_config,
                &self.initial_messages,
                &self.final_output_tool,
            )
            .await?;

        match result {
            RetryResult::Retried => {
                info!("Retry logic triggered, restarting agent loop");
                // Persist the reset conversation
                ctx.session_manager
                    .replace_conversation(&ctx.session_id, &ctx.conversation)
                    .await?;
                let _ = ctx
                    .event_tx
                    .send(LoopEvent::HistoryReplaced(ctx.conversation.clone()));
                Ok(ExitAction::Continue)
            }
            RetryResult::Skipped | RetryResult::SuccessChecksPassed => {
                // No retry config or checks passed — exit normally.
                Ok(ExitAction::Exit)
            }
            RetryResult::MaxAttemptsReached => {
                // Max retries exceeded — RetryManager already added error message to conversation.
                // Emit it for the UI.
                if let Some(last) = ctx.conversation.messages().last() {
                    let _ = ctx.event_tx.send(LoopEvent::Message(last.clone()));
                }
                Ok(ExitAction::Exit)
            }
        }
    }
}
