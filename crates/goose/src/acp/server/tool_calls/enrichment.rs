use crate::acp::tool_call_notifier::ToolCallNotifier;
use crate::agents::Agent;
use crate::conversation::message::{
    Message, MessageContent, TOOL_META_CHAIN_SUMMARY_KEY, TOOL_META_TITLE_KEY,
};
use crate::model_config::get_fast_model;
use crate::session::SessionManager;
use crate::session_context::with_session_id;
use crate::utils::safe_truncate;
use agent_client_protocol::schema::v1::{
    Meta, SessionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields,
};
use rmcp::model::CallToolRequestParams;
use serde_json::{json, to_string, Map, Number, Value};
use std::slice::from_ref;
use std::sync::Arc;
use std::time::Duration;
use tokio::{spawn, time::sleep};
use tracing::warn;

/// Add `goose.toolChainSummary = { summary, count }` to a `Meta` blob,
/// preserving any existing `goose.*` keys such as `goose.toolCall`.
pub(crate) fn with_tool_chain_summary_meta(
    base: Option<Meta>,
    summary: &str,
    count: usize,
) -> Option<Meta> {
    let mut meta = base.unwrap_or_default();
    let goose_entry = meta
        .entry("goose".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let goose_obj = match goose_entry {
        Value::Object(obj) => obj,
        other => {
            *other = Value::Object(Map::new());
            match other {
                Value::Object(obj) => obj,
                _ => unreachable!(),
            }
        }
    };
    let mut chain = Map::new();
    chain.insert("summary".to_string(), Value::String(summary.to_string()));
    chain.insert("count".to_string(), Value::Number(Number::from(count)));
    goose_obj.insert("toolChainSummary".to_string(), Value::Object(chain));
    Some(meta)
}

pub(crate) struct ToolTitleEnrichmentContext {
    agent: Arc<Agent>,
    session_id: SessionId,
    tool_call_notifier: ToolCallNotifier,
    session_manager: Arc<SessionManager>,
    session_id_for_persist: String,
    message_id_for_persist: Option<String>,
}

impl ToolTitleEnrichmentContext {
    pub(crate) fn new(
        agent: &Arc<Agent>,
        session_id: &SessionId,
        tool_call_notifier: &ToolCallNotifier,
        session_manager: &Arc<SessionManager>,
        session_id_for_persist: &str,
        message_id_for_persist: Option<&str>,
    ) -> Self {
        Self {
            agent: agent.clone(),
            session_id: session_id.clone(),
            tool_call_notifier: tool_call_notifier.clone(),
            session_manager: session_manager.clone(),
            session_id_for_persist: session_id_for_persist.to_string(),
            message_id_for_persist: message_id_for_persist.map(str::to_string),
        }
    }

    pub(crate) fn spawn_title_enrichment(
        self,
        request_id: String,
        tool_call: &CallToolRequestParams,
        identity_meta: Option<Meta>,
        fallback_title: String,
    ) {
        let args_json = tool_call
            .arguments
            .as_ref()
            .map(|a| {
                let s = to_string(a).unwrap_or_default();
                if s.len() > 300 {
                    format!("{}…", safe_truncate(&s, 300))
                } else {
                    s
                }
            })
            .unwrap_or_default();

        let Self {
            agent,
            session_id,
            tool_call_notifier,
            session_manager,
            session_id_for_persist,
            message_id_for_persist,
        } = self;

        ToolTitleEnrichmentJob {
            agent,
            sid: session_id,
            request_id,
            tool_call_notifier,
            name: tool_call.name.to_string(),
            identity_meta,
            fallback_title,
            session_id_for_persist,
            message_id_for_persist,
            session_manager,
            args_json,
        }
        .spawn();
    }
}

struct ToolTitleEnrichmentJob {
    agent: Arc<Agent>,
    sid: SessionId,
    request_id: String,
    tool_call_notifier: ToolCallNotifier,
    name: String,
    identity_meta: Option<Meta>,
    fallback_title: String,
    session_id_for_persist: String,
    message_id_for_persist: Option<String>,
    session_manager: Arc<SessionManager>,
    args_json: String,
}

impl ToolTitleEnrichmentJob {
    fn spawn(self) {
        spawn(async move {
            let Self {
                agent,
                sid,
                request_id,
                tool_call_notifier,
                name,
                identity_meta,
                fallback_title,
                session_id_for_persist,
                message_id_for_persist,
                session_manager,
                args_json,
            } = self;

            let (title, from_llm) = match agent.provider().await {
                Ok(provider) => {
                    if provider.manages_own_context() {
                        return;
                    }

                    let system =
                        "Summarize this tool call in a short lowercase phrase (3-8 words). \
                         No punctuation. No quotes. Examples: reading project configuration, \
                         checking network connectivity, listing files in src directory";
                    let user_text = format!("Tool: {name}\nArguments: {args_json}");
                    let message = Message::user().with_text(&user_text);
                    let model_config = match agent.model_config_for_session(&sid.0).await {
                        Ok(config) => config,
                        Err(_) => return,
                    };
                    let fast_model_config =
                        match get_fast_model(provider.get_name(), &model_config).await {
                            Ok(config) => config,
                            Err(_) => return,
                        };
                    // The fast model occasionally returns an empty response
                    // under load (rate limiting, transient network). One
                    // retry with a short backoff is enough to recover the
                    // common cases without paying for the regular model.
                    let mut llm_outcome: Option<String> = None;
                    for attempt in 0..2 {
                        match with_session_id(
                            Some(sid.0.to_string()),
                            provider.complete(&fast_model_config, system, from_ref(&message), &[]),
                        )
                        .await
                        {
                            Ok((response, _)) => {
                                let summary: String = response
                                    .content
                                    .iter()
                                    .filter_map(|c: &MessageContent| c.as_text())
                                    .collect::<String>()
                                    .trim()
                                    .to_string();
                                if !summary.is_empty() {
                                    llm_outcome = Some(summary);
                                    break;
                                }
                                if attempt == 0 {
                                    warn!(
                                        "tool call summary: fast_complete returned empty for {request_id} ({name}), retrying once",
                                    );
                                    sleep(Duration::from_millis(150)).await;
                                }
                            }
                            Err(e) => {
                                if attempt == 0 {
                                    warn!(
                                        "tool call summary: fast_complete errored for {request_id} ({name}): {e}, retrying once",
                                    );
                                    sleep(Duration::from_millis(150)).await;
                                } else {
                                    warn!(
                                        "tool call summary: fast_complete errored for {request_id} ({name}) after retry: {e}",
                                    );
                                }
                            }
                        }
                    }
                    match llm_outcome {
                        Some(summary) => (summary, true),
                        None => {
                            warn!(
                                "tool call summary: falling back to deterministic title for {request_id} ({name}) — replay will not show an LLM summary for this call",
                            );
                            (fallback_title.clone(), false)
                        }
                    }
                }
                Err(e) => {
                    warn!("tool call summary: failed to get provider: {e}");
                    (fallback_title.clone(), false)
                }
            };

            let fields = ToolCallUpdateFields::new().title(title.clone());
            let _ = tool_call_notifier.send_update(
                ToolCallUpdate::new(ToolCallId::new(request_id.clone()), fields)
                    .meta(identity_meta),
            );

            // Best-effort persistence: only persist the LLM-generated title
            // (not the deterministic fallback) so reload uses fallback_title
            // for older or failed cases just like today.
            if from_llm {
                if let Some(msg_id) = message_id_for_persist {
                    let patch = json!({
                        (TOOL_META_TITLE_KEY): title,
                    });
                    if let Err(e) = session_manager
                        .update_tool_request_meta(
                            &session_id_for_persist,
                            &msg_id,
                            &request_id,
                            patch,
                        )
                        .await
                    {
                        warn!(
                            "tool call summary: persist failed for {request_id} in {msg_id}: {e}",
                        );
                    }
                } else {
                    warn!(
                        "tool call summary: missing message_id for {request_id} — title will not survive reload",
                    );
                }
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
        identity_meta: Option<Meta>,
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
            identity_meta,
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
    identity_meta: Option<Meta>,
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
                identity_meta,
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

            let meta = with_tool_chain_summary_meta(identity_meta, &summary, chain_count);
            let fields = ToolCallUpdateFields::new();
            let _ = tool_call_notifier.send_update(
                ToolCallUpdate::new(ToolCallId::new(first_tool_call_id), fields).meta(meta),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    mod with_tool_chain_summary_meta {
        use super::super::with_tool_chain_summary_meta;
        use crate::acp::server::tool_calls::conversion::tool_call_identity_meta;
        use crate::conversation::message::ToolRequest;
        use rmcp::model::CallToolRequestParams;
        use serde_json::json;

        #[test]
        fn creates_fresh_when_none() {
            let meta = with_tool_chain_summary_meta(None, "applied dark mode", 4)
                .expect("meta should be created");
            assert_eq!(
                meta.get("goose"),
                Some(&json!({
                    "toolChainSummary": { "summary": "applied dark mode", "count": 4 },
                })),
            );
        }

        #[test]
        fn preserves_existing_tool_call_identity() {
            let existing = tool_call_identity_meta(&ToolRequest {
                id: "req_1".to_string(),
                tool_call: Ok(CallToolRequestParams::new("developer__shell")),
                metadata: None,
                tool_meta: None,
            });
            let meta = with_tool_chain_summary_meta(existing, "ran two commands", 2)
                .expect("meta should be created");
            let goose = meta.get("goose").expect("goose key");
            assert_eq!(
                goose.get("toolCall"),
                Some(&json!({
                    "toolName": "developer__shell",
                    "extensionName": "developer",
                })),
            );
            assert_eq!(
                goose.get("toolChainSummary"),
                Some(&json!({ "summary": "ran two commands", "count": 2 })),
            );
        }
    }
}
