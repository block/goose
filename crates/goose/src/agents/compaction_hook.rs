use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::agents::hooks::{AgentHook, ErrorRecovery, LoopContext, LoopEvent};
use crate::config::Config;
use crate::context_mgmt::{
    check_if_compaction_needed, compact_messages, DEFAULT_COMPACTION_THRESHOLD,
};
use crate::conversation::message::{Message, SystemNotificationType};
use crate::providers::errors::ProviderError;

const COMPACTION_THINKING_TEXT: &str = "goose is compacting the conversation...";
const MAX_RECOVERY_COMPACTION_ATTEMPTS: u32 = 2;

/// Hook that manages conversation compaction (context window management).
///
/// Handles two scenarios:
/// 1. **Pre-inference auto-compaction**: Before each LLM call, checks if the token
///    usage exceeds the configured threshold and compacts proactively.
/// 2. **Recovery compaction**: When the provider returns `ContextLengthExceeded`,
///    compacts the conversation and retries.
pub struct CompactionHook {
    /// Number of recovery compaction attempts in the current reply.
    /// Reset when a successful LLM response is received.
    recovery_attempts: AtomicU32,
}

impl CompactionHook {
    pub fn new() -> Self {
        Self {
            recovery_attempts: AtomicU32::new(0),
        }
    }

    /// Emit compaction progress messages to the UI.
    fn emit_compaction_start(&self, ctx: &LoopContext, reason: &str) {
        let _ = ctx.event_tx.send(LoopEvent::Message(
            Message::assistant().with_system_notification(
                SystemNotificationType::InlineMessage,
                reason,
            ),
        ));
        let _ = ctx.event_tx.send(LoopEvent::Message(
            Message::assistant().with_system_notification(
                SystemNotificationType::ThinkingMessage,
                COMPACTION_THINKING_TEXT,
            ),
        ));
    }

    /// Perform compaction: summarize the conversation and update session state.
    async fn do_compaction(&self, ctx: &mut LoopContext) -> Result<()> {
        let (compacted, usage) = compact_messages(
            ctx.provider.as_ref(),
            &ctx.session_id,
            &ctx.conversation,
            false,
        )
        .await?;

        // Persist the compacted conversation
        ctx.session_manager
            .replace_conversation(&ctx.session_id, &compacted)
            .await?;

        // Update token metrics
        // Note: We call update_session_metrics through the session manager directly.
        // The usage tracking for compaction (is_compaction_usage=true) is handled by
        // the caller via the session manager.
        crate::agents::reply_parts::update_session_metrics_standalone(
            &ctx.session_manager,
            &ctx.session_id,
            ctx.schedule_id.clone(),
            &usage,
            true,
        )
        .await?;

        // Notify the UI
        let _ = ctx
            .event_tx
            .send(LoopEvent::HistoryReplaced(compacted.clone()));
        let _ = ctx.event_tx.send(LoopEvent::Message(
            Message::assistant().with_system_notification(
                SystemNotificationType::InlineMessage,
                "Compaction complete",
            ),
        ));

        ctx.conversation = compacted;
        Ok(())
    }
}

#[async_trait]
impl AgentHook for CompactionHook {
    async fn post_inference(
        &self,
        _response: &Message,
        _ctx: &mut LoopContext,
    ) -> Result<()> {
        // A successful LLM response means context is fine — reset the recovery counter.
        self.reset_recovery_attempts();
        Ok(())
    }

    async fn pre_inference(&self, ctx: &mut LoopContext) -> Result<()> {
        // Check if we've crossed the auto-compaction threshold
        let session = ctx
            .session_manager
            .get_session(&ctx.session_id, false)
            .await?;

        let needs_compact = check_if_compaction_needed(
            ctx.provider.as_ref(),
            &ctx.conversation,
            None,
            &session,
        )
        .await?;

        if !needs_compact {
            return Ok(());
        }

        let config = Config::global();
        let threshold = config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
        let threshold_percentage = (threshold * 100.0) as u32;

        self.emit_compaction_start(
            ctx,
            &format!(
                "Exceeded auto-compact threshold of {}%. Performing auto-compaction...",
                threshold_percentage
            ),
        );

        if let Err(e) = self.do_compaction(ctx).await {
            let _ = ctx.event_tx.send(LoopEvent::Message(
                Message::assistant().with_text(format!(
                    "Ran into this error trying to compact: {e}.\n\nPlease try again or create a new session"
                )),
            ));
            return Err(e);
        }

        Ok(())
    }

    async fn on_error(
        &self,
        error: &ProviderError,
        ctx: &mut LoopContext,
    ) -> Result<ErrorRecovery> {
        let ProviderError::ContextLengthExceeded(_) = error else {
            return Ok(ErrorRecovery::Propagate);
        };

        let attempts = self.recovery_attempts.fetch_add(1, Ordering::Relaxed) + 1;

        if attempts >= MAX_RECOVERY_COMPACTION_ATTEMPTS {
            tracing::error!("Context limit exceeded after compaction - prompt too large");
            let _ = ctx.event_tx.send(LoopEvent::Message(
                Message::assistant().with_system_notification(
                    SystemNotificationType::InlineMessage,
                    "Unable to continue: Context limit still exceeded after compaction. Try using a shorter message, a model with a larger context window, or start a new session."
                ),
            ));
            return Ok(ErrorRecovery::Propagate);
        }

        self.emit_compaction_start(
            ctx,
            "Context limit reached. Compacting to continue conversation...",
        );

        match self.do_compaction(ctx).await {
            Ok(()) => Ok(ErrorRecovery::Retry),
            Err(e) => {
                #[cfg(feature = "telemetry")]
                crate::posthog::emit_error("compaction_failed", &e.to_string());
                tracing::error!("Compaction failed: {}", e);
                let _ = ctx.event_tx.send(LoopEvent::Message(
                    Message::assistant().with_text(format!(
                        "Ran into this error trying to compact: {e}.\n\nPlease try again or create a new session"
                    )),
                ));
                Ok(ErrorRecovery::Propagate)
            }
        }
    }
}

/// Reset the recovery attempt counter. Call this when a successful LLM response is received.
impl CompactionHook {
    pub fn reset_recovery_attempts(&self) {
        self.recovery_attempts.store(0, Ordering::Relaxed);
    }
}
