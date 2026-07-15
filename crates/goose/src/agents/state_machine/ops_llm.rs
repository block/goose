use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use rmcp::model::{Role, Tool};

use crate::agents::agent::attach_turn_usage;
use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnOutcome};
use crate::agents::state_machine::ops_maxturns::turns_taken_this_request;
use crate::agents::state_machine::ops_toolcalling::current_request_start;
use crate::agents::{Agent, AgentEvent};
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::providers::base::{Provider, ProviderUsage};
use crate::session::Session;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

/// The system prompt and tools baked at reply start, plus the extension-set
/// version they were built against — a `manage_extensions` call mid-reply
/// bumps the version (and newly discovered subdirectory hints count too), and
/// the next turn rebuilds everything.
struct PromptState {
    system_prompt: String,
    tools: Vec<Tool>,
    toolshim_tools: Vec<Tool>,
    tools_version: u64,
}

/// Calls the LLM when the last message in the conversation is from the user.
pub struct LlmOperation<'a> {
    agent: &'a Agent,
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    prompt_state: tokio::sync::Mutex<PromptState>,
    schedule_id: Option<String>,
    max_turns: u32,
}

impl<'a> LlmOperation<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent: &'a Agent,
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        system_prompt: String,
        tools: Vec<Tool>,
        toolshim_tools: Vec<Tool>,
        schedule_id: Option<String>,
        max_turns: u32,
    ) -> Self {
        let tools_version = agent.extension_manager.tools_version();
        Self {
            agent,
            provider,
            model_config,
            prompt_state: tokio::sync::Mutex::new(PromptState {
                system_prompt,
                tools,
                toolshim_tools,
                tools_version,
            }),
            schedule_id,
            max_turns,
        }
    }

    async fn current_prompt_and_tools(
        &self,
        session: &Session,
    ) -> Result<(String, Vec<Tool>, Vec<Tool>)> {
        let mut state = self.prompt_state.lock().await;
        let current_version = self.agent.extension_manager.tools_version();
        let has_new_hints = self
            .agent
            .prompt_manager
            .lock()
            .await
            .load_subdirectory_hints(&session.working_dir);
        if state.tools_version != current_version || has_new_hints {
            let (tools, toolshim_tools, system_prompt, _model_config) = self
                .agent
                .prepare_tools_and_prompt(&session.id, &session.working_dir)
                .await?;
            state.tools = tools;
            state.toolshim_tools = toolshim_tools;
            state.system_prompt = system_prompt;
            state.tools_version = current_version;
        }
        Ok((
            state.system_prompt.clone(),
            state.tools.clone(),
            state.toolshim_tools.clone(),
        ))
    }

    async fn error_outcome(&self, err: &ProviderError, emit: &Emitter) -> TurnOutcome {
        #[cfg(feature = "telemetry")]
        crate::posthog::emit_error(err.telemetry_type(), &err.to_string());
        tracing::error!("LLM provider error: {err}");
        let message = Message::from_provider_error(err);
        emit.emit(AgentEvent::Message(message.clone())).await;
        vec![message.into()]
    }
}

#[async_trait]
impl Operation for LlmOperation<'_> {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if conversation.last().and_then(|m| m.error_kind()).is_some() {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let answered: std::collections::HashSet<&str> = conversation
            .messages()
            .iter()
            .flat_map(|m| m.get_tool_response_ids())
            .collect();

        // Unanswered tool requests from before the current request are stale
        // leftovers (a crash mid-execution). They stay in the transcript but
        // must not reach the provider, which rejects a tool call without a
        // matching result. Requests from the current request are kept — an
        // approval may still be pending on them.
        let start = current_request_start(conversation.messages());
        let messages_for_provider: Vec<_> = conversation
            .messages()
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_agent_visible())
            .map(|(idx, m)| {
                let mut m = m.agent_visible_content();
                if idx < start {
                    m.content.retain(|c| match c {
                        MessageContent::ToolRequest(request) => {
                            answered.contains(request.id.as_str())
                        }
                        _ => true,
                    });
                }
                m
            })
            .filter(|m| !m.content.is_empty())
            .collect();

        if !matches!(
            messages_for_provider.last().map(|m| &m.role),
            Some(Role::User)
        ) {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let (system_prompt, tools, toolshim_tools) = self.current_prompt_and_tools(session).await?;

        // The ephemeral turn-context block (time, working dir, turn budget);
        // injected into the provider view only, never persisted.
        let turns_taken = turns_taken_this_request(conversation);
        let conversation_for_provider = crate::agents::moim::inject_moim(
            &session.id,
            Conversation::new_unvalidated(messages_for_provider),
            &self.agent.extension_manager,
            turns_taken,
            self.max_turns,
        )
        .await;

        // The shared wrapper adds what a bare `provider.stream` lacks:
        // toolshim conversion, session-context scoping, the thinking-effort
        // default, and error enhancement.
        let stream = Agent::stream_response_from_provider(
            self.provider.clone(),
            self.model_config.clone(),
            &session.id,
            &system_prompt,
            conversation_for_provider.messages(),
            &tools,
            &toolshim_tools,
        )
        .await;

        let mut stream = match stream {
            Ok(stream) => stream,
            Err(err) => {
                return Ok(OperationResult::Applied(
                    self.error_outcome(&err, &emit).await,
                ))
            }
        };

        // Conversation::push handles merge logic — coalescing text, merging
        // thinking blocks by signature, deduping by message id, forwarding
        // inference metadata to the right prior message.
        let mut accumulator = Conversation::empty();
        let mut turn_usage: Option<ProviderUsage> = None;
        loop {
            tokio::select! {
                biased;
                _ = emit.cancelled() => break,
                next = stream.next() => {
                    let Some(result) = next else { break };
                    let (msg_opt, usage_opt) = match result {
                        Ok(chunk) => chunk,
                        // A mid-stream provider error: discard the partial
                        // assistant turn and append a tagged error message so a
                        // recovery op (or ExitOnError) handles it on the next
                        // iteration. The conversation never keeps a half-turn.
                        Err(err) => return Ok(OperationResult::Applied(self.error_outcome(&err, &emit).await)),
                    };
                    if let Some(usage) = usage_opt {
                        let enriched = self
                            .agent
                            .update_session_metrics(&session.id, self.schedule_id.clone(), &usage, false)
                            .await?;
                        emit.emit(AgentEvent::Usage(enriched.clone())).await;
                        turn_usage = Some(enriched);
                    }
                    if let Some(chunk) = msg_opt {
                        emit.emit(AgentEvent::Message(chunk.clone())).await;
                        accumulator.push(chunk);
                    }
                }
            }
        }

        if let Some(usage) = turn_usage {
            if let Some((message_id, message_usage)) = attach_turn_usage(&mut accumulator, &usage) {
                emit.emit(AgentEvent::MessageUsage {
                    message_id,
                    usage: message_usage,
                })
                .await;
            }
        }

        Ok(OperationResult::Applied(
            accumulator.into_iter().map(Into::into).collect(),
        ))
    }
}
