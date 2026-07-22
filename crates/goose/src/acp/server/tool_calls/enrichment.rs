use crate::acp::tool_call_notifier::ToolCallNotifier;
use crate::agents::Agent;
use crate::conversation::message::{
    Message, MessageContent, ToolRequest, TOOL_META_CHAIN_SUMMARY_KEY,
};
use crate::model_config::get_fast_model;
use crate::session::SessionManager;
use crate::session_context::with_session_id;
use crate::tool_call_labels::generate_tool_title;
use agent_client_protocol::schema::v1::{
    Meta, SessionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields,
};
use serde_json::{json, Map, Value};
use std::slice::from_ref;
use std::sync::Arc;
use std::time::Duration;
use tokio::{spawn, time::sleep};
use tracing::warn;

pub(crate) fn tool_chain_summary(summary: &str, count: usize) -> (String, Value) {
    (
        "toolChainSummary".to_string(),
        json!({
            "summary": summary,
            "count": count,
        }),
    )
}

fn build_chain_summary_update(tool_call_id: String, summary: &str, count: usize) -> ToolCallUpdate {
    let goose_meta = Map::from_iter([tool_chain_summary(summary, count)]);
    let mut meta = Meta::default();
    meta.insert("goose".to_string(), Value::Object(goose_meta));
    ToolCallUpdate::new(ToolCallId::new(tool_call_id), ToolCallUpdateFields::new()).meta(Some(meta))
}

pub(crate) struct ToolTitleEnrichmentContext {
    agent: Arc<Agent>,
    tool_call_notifier: ToolCallNotifier,
    session_manager: Arc<SessionManager>,
    session_id: String,
    message_id_for_persist: Option<String>,
}

impl ToolTitleEnrichmentContext {
    pub(crate) fn new(
        agent: &Arc<Agent>,
        tool_call_notifier: &ToolCallNotifier,
        session_manager: &Arc<SessionManager>,
        session_id: &str,
        message_id_for_persist: Option<&str>,
    ) -> Self {
        Self {
            agent: agent.clone(),
            tool_call_notifier: tool_call_notifier.clone(),
            session_manager: session_manager.clone(),
            session_id: session_id.to_string(),
            message_id_for_persist: message_id_for_persist.map(str::to_string),
        }
    }

    pub(crate) fn spawn_title_enrichment(self, tool_request: &ToolRequest) {
        let tool_request = tool_request.clone();

        spawn(async move {
            if let Some(title) = generate_tool_title(
                self.agent.as_ref(),
                self.session_manager.as_ref(),
                &self.session_id,
                self.message_id_for_persist.as_deref(),
                &tool_request,
            )
            .await
            {
                let _ = self.tool_call_notifier.send_update(ToolCallUpdate::new(
                    ToolCallId::new(tool_request.id),
                    ToolCallUpdateFields::new().title(title),
                ));
            }
        });
    }
}

pub(crate) struct ChainSummaryEnrichmentContext {
    agent: Arc<Agent>,
    session_id: SessionId,
    tool_call_notifier: ToolCallNotifier,
    session_manager: Arc<SessionManager>,
}

impl ChainSummaryEnrichmentContext {
    pub(crate) fn new(
        agent: &Arc<Agent>,
        session_id: &SessionId,
        tool_call_notifier: &ToolCallNotifier,
        session_manager: &Arc<SessionManager>,
    ) -> Self {
        Self {
            agent: agent.clone(),
            session_id: session_id.clone(),
            tool_call_notifier: tool_call_notifier.clone(),
            session_manager: session_manager.clone(),
        }
    }

    pub(crate) fn spawn_chain_summary(
        self,
        first_tool_call_id: String,
        message_id_for_persist: String,
        steps: Vec<(String, String)>,
        chain_count: usize,
    ) {
        let Self {
            agent,
            session_id,
            tool_call_notifier,
            session_manager,
        } = self;

        ChainSummaryEnrichmentJob {
            agent,
            sid: session_id,
            first_tool_call_id,
            message_id_for_persist,
            steps,
            chain_count,
            tool_call_notifier,
            session_manager,
        }
        .spawn();
    }
}

struct ChainSummaryEnrichmentJob {
    agent: Arc<Agent>,
    sid: SessionId,
    first_tool_call_id: String,
    message_id_for_persist: String,
    steps: Vec<(String, String)>,
    chain_count: usize,
    tool_call_notifier: ToolCallNotifier,
    session_manager: Arc<SessionManager>,
}

impl ChainSummaryEnrichmentJob {
    fn spawn(self) {
        spawn(async move {
            let Self {
                agent,
                sid,
                first_tool_call_id,
                message_id_for_persist,
                steps,
                chain_count,
                tool_call_notifier,
                session_manager,
            } = self;

            let provider = match agent.provider().await {
                Ok(provider) => provider,
                Err(error) => {
                    warn!(
                        "tool chain summary: failed to get provider for chain anchored at {first_tool_call_id}: {error}",
                    );
                    return;
                }
            };
            if provider.manages_own_context() {
                warn!(
                    "tool chain summary: provider manages own context; skipping chain anchored at {first_tool_call_id}",
                );
                return;
            }

            let system = "Summarize this sequence of tool calls in a short lowercase phrase \
                 (3-8 words). No punctuation. No quotes. \
                 Examples: applied dark mode polish, scanned for security issues, \
                 refactored config loading";

            let mut user_text = String::from("Tool call sequence:\n");
            for (index, (name, args)) in steps.iter().enumerate() {
                user_text.push_str(&format!("Step {}: {} {}\n", index + 1, name, args));
            }
            let message = Message::user().with_text(&user_text);
            let model_config = match agent.model_config_for_session(&sid.0).await {
                Ok(config) => config,
                Err(_) => return,
            };
            let fast_model_config = match get_fast_model(provider.get_name(), &model_config).await {
                Ok(config) => config,
                Err(_) => return,
            };

            // Match the per-tool retry policy: one retry on empty/error keeps
            // the chain header reliable when the fast model is rate-limited or
            // momentarily flaky, without escalating to the regular model.
            let mut summary: Option<String> = None;
            for attempt in 0..2 {
                match with_session_id(
                    Some(sid.0.to_string()),
                    provider.complete(&fast_model_config, system, from_ref(&message), &[]),
                )
                .await
                {
                    Ok((response, _)) => {
                        let generated_summary = response
                            .content
                            .iter()
                            .filter_map(|content: &MessageContent| content.as_text())
                            .collect::<String>()
                            .trim()
                            .to_string();
                        if !generated_summary.is_empty() {
                            summary = Some(generated_summary);
                            break;
                        }
                        if attempt == 0 {
                            warn!(
                                "tool chain summary: fast_complete returned empty for chain anchored at {first_tool_call_id} ({} steps), retrying once",
                                steps.len(),
                            );
                            sleep(Duration::from_millis(150)).await;
                        }
                    }
                    Err(error) => {
                        if attempt == 0 {
                            warn!(
                                "tool chain summary: fast_complete errored for chain anchored at {first_tool_call_id}: {error}, retrying once",
                            );
                            sleep(Duration::from_millis(150)).await;
                        } else {
                            warn!(
                                "tool chain summary: fast_complete errored for chain anchored at {first_tool_call_id} after retry: {error}",
                            );
                        }
                    }
                }
            }
            let Some(summary) = summary else {
                warn!(
                    "tool chain summary: no LLM summary produced for chain anchored at {first_tool_call_id} — replay will fall back to the deterministic phrase",
                );
                return;
            };

            let patch = json!({
                (TOOL_META_CHAIN_SUMMARY_KEY): {
                    "summary": &summary,
                    "count": chain_count,
                },
            });
            if let Err(error) = session_manager
                .update_tool_request_meta(
                    &sid.0,
                    &message_id_for_persist,
                    &first_tool_call_id,
                    patch,
                )
                .await
            {
                warn!(
                    "tool chain summary: persist failed for chain anchored at {first_tool_call_id} in {message_id_for_persist}: {error}",
                );
            }

            let _ = tool_call_notifier.send_update(build_chain_summary_update(
                first_tool_call_id,
                &summary,
                chain_count,
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    mod build_chain_summary_update {
        use super::super::build_chain_summary_update;
        use serde_json::json;

        #[test]
        fn contains_only_the_chain_summary_delta() {
            let update = build_chain_summary_update("req_1".to_string(), "applied dark mode", 4);

            assert_eq!(
                serde_json::to_value(update).expect("update should serialize"),
                json!({
                    "toolCallId": "req_1",
                    "_meta": {
                        "goose": {
                            "toolChainSummary": {
                                "summary": "applied dark mode",
                                "count": 4,
                            },
                        },
                    },
                }),
            );
        }
    }
}
