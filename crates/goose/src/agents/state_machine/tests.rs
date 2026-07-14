use anyhow::Result;
use futures::StreamExt;
use rmcp::model::Role;

use crate::agents::state_machine::test_helpers::{
    tool_response_text, ScriptedProvider, Step, TestHarness,
};
use crate::agents::tool_execution::DECLINED_RESPONSE;
use crate::agents::types::SessionConfig;
use crate::agents::{state_machine, AgentEvent};
use crate::config::GooseMode;
use crate::conversation::message::{ActionRequiredData, Message, MessageContent};
use crate::permission::Permission;
use std::sync::Arc;

#[tokio::test]
async fn llm_requests_tool_then_replies() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "test__echo".to_string(),
            args: serde_json::json!({ "x": 1 }),
        },
        Step::Text("all done".to_string()),
    ])
    .await
    .with_default_extension()
    .await;

    let messages = harness.run("use the echo tool", 10).await?;

    // emitted: assistant(tool req) + user(tool resp) + assistant(text)
    assert_eq!(messages.len(), 3, "events: {messages:#?}");
    assert_eq!(messages[0].role, Role::Assistant);
    assert!(messages[0].is_tool_call());
    assert_eq!(messages[1].role, Role::User);
    assert!(messages[1].is_tool_response());
    assert_eq!(messages[2].role, Role::Assistant);

    // tool actually ran: echo returned the args as JSON text
    let resp_text = tool_response_text(&messages[1]);
    assert!(resp_text.contains("\"x\":1"), "tool response: {resp_text}");

    // provider was called twice (tool turn + final text turn)
    assert_eq!(harness.provider.call_count(), 2);

    // persisted conversation matches what was emitted (prompt + 3 above)
    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 4);
    assert_eq!(persisted[0].role, Role::User);

    Ok(())
}

#[tokio::test]
async fn stops_at_max_turns() -> Result<()> {
    // The provider never stops on its own — every turn calls a tool, whose
    // response re-triggers the LLM. Only the max-turns op can halt the loop.
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let provider = Arc::new(ScriptedProvider::from_fn(move |_messages, _tools| {
        let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        vec![Message::assistant().with_tool_request(
            format!("call_{n}"),
            Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                .with_arguments(serde_json::Map::new())),
        )]
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_default_extension()
        .await;

    let messages = harness.run("keep going", 3).await?;

    // 3 LLM turns, then the max-turns op halts before a 4th.
    assert_eq!(harness.provider.call_count(), 3);

    let limit = messages.last().expect("at least one message");
    assert_eq!(limit.role, Role::Assistant);
    assert!(
        limit.as_concat_text().contains("maximum number of actions"),
        "last message: {limit:#?}"
    );

    // The 3 tool-calling turns and the limit message are persisted, so a
    // reloaded transcript shows why the agent stopped.
    let persisted = harness.persisted_messages().await?;
    let tool_call_turns = persisted.iter().filter(|m| m.is_tool_call()).count();
    assert_eq!(tool_call_turns, 3);
    let last = persisted.last().expect("a persisted message");
    assert!(
        last.as_concat_text().contains("maximum number of actions"),
        "tail: {last:?}"
    );

    Ok(())
}

#[tokio::test]
async fn approve_mode_waits_for_tool_confirmation_before_execution() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "test__echo".to_string(),
            args: serde_json::json!({ "x": 1 }),
        },
        Step::Text("done".to_string()),
    ])
    .await
    .with_default_extension()
    .await
    .with_goose_mode(GooseMode::Approve)
    .await;

    let stream = state_machine::reply(
        &harness.agent,
        Message::user().with_text("use the echo tool"),
        SessionConfig {
            id: harness.session_id.clone(),
            schedule_id: None,
            max_turns: Some(10),
            retry_config: None,
        },
        None,
    )
    .await?;
    tokio::pin!(stream);

    let mut messages = Vec::new();
    let mut saw_confirmation = false;
    while let Some(event) = stream.next().await {
        let event = event?;
        if let AgentEvent::Message(message) = &event {
            if message.content.iter().any(|content| {
                matches!(
                    content,
                    MessageContent::ActionRequired(action)
                        if matches!(
                            action.data,
                            ActionRequiredData::ToolConfirmation { ref id, .. } if id == "call_1"
                        )
                )
            }) {
                saw_confirmation = true;
            }
            messages.push(message.clone());
        }
    }

    assert!(saw_confirmation, "messages: {messages:#?}");
    assert_eq!(harness.provider.call_count(), 1);

    let stream = state_machine::reply(
        &harness.agent,
        Message::user()
            .with_content(MessageContent::action_required_tool_confirmation_response(
                "call_1",
                Permission::AllowOnce,
            ))
            .with_visibility(false, false),
        SessionConfig {
            id: harness.session_id.clone(),
            schedule_id: None,
            max_turns: Some(10),
            retry_config: None,
        },
        None,
    )
    .await?;
    tokio::pin!(stream);

    while let Some(event) = stream.next().await {
        let event = event?;
        if let AgentEvent::Message(message) = event {
            messages.push(message);
        }
    }

    assert_eq!(harness.provider.call_count(), 2);
    assert!(messages.iter().any(|m| {
        m.role == Role::User && m.is_tool_response() && tool_response_text(m).contains("\"x\":1")
    }));

    Ok(())
}

#[tokio::test]
async fn denied_tool_confirmation_becomes_tool_response() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "test__echo".to_string(),
            args: serde_json::json!({ "x": 1 }),
        },
        Step::Text("done".to_string()),
    ])
    .await
    .with_default_extension()
    .await
    .with_goose_mode(GooseMode::Approve)
    .await;

    let stream = state_machine::reply(
        &harness.agent,
        Message::user().with_text("use the echo tool"),
        SessionConfig {
            id: harness.session_id.clone(),
            schedule_id: None,
            max_turns: Some(10),
            retry_config: None,
        },
        None,
    )
    .await?;
    tokio::pin!(stream);

    let mut messages = Vec::new();
    let mut saw_confirmation = false;
    while let Some(event) = stream.next().await {
        let event = event?;
        if let AgentEvent::Message(message) = &event {
            if message.content.iter().any(|content| {
                matches!(
                    content,
                    MessageContent::ActionRequired(action)
                        if matches!(
                            action.data,
                            ActionRequiredData::ToolConfirmation { ref id, .. } if id == "call_1"
                        )
                )
            }) {
                saw_confirmation = true;
            }
            messages.push(message.clone());
        }
    }

    assert!(saw_confirmation, "messages: {messages:#?}");
    let stream = state_machine::reply(
        &harness.agent,
        Message::user()
            .with_content(MessageContent::action_required_tool_confirmation_response(
                "call_1",
                Permission::DenyOnce,
            ))
            .with_visibility(false, false),
        SessionConfig {
            id: harness.session_id.clone(),
            schedule_id: None,
            max_turns: Some(10),
            retry_config: None,
        },
        None,
    )
    .await?;
    tokio::pin!(stream);

    while let Some(event) = stream.next().await {
        let event = event?;
        if let AgentEvent::Message(message) = event {
            messages.push(message);
        }
    }

    assert_eq!(harness.provider.call_count(), 2);
    assert!(messages.iter().any(|m| {
        m.role == Role::User
            && m.is_tool_response()
            && tool_response_text(m).contains(DECLINED_RESPONSE)
    }));

    Ok(())
}

#[tokio::test]
async fn goal_nudges_once_then_clears() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::Text("did some work".to_string()),
        Step::Text("goal is met".to_string()),
    ])
    .await;
    harness.agent.set_goal(Some("ship it".to_string())).await;

    let messages = harness.run("work on the goal", 10).await?;

    // The nudge bought exactly one extra turn, then the goal was cleared.
    assert_eq!(harness.provider.call_count(), 2, "events: {messages:#?}");
    assert!(harness.agent.get_goal().await.is_none());

    let persisted = harness.persisted_messages().await?;
    let nudge = persisted
        .iter()
        .find(|m| m.as_concat_text().contains("fully met"))
        .expect("a goal nudge message");
    assert!(!nudge.is_user_visible());
    assert!(nudge.is_agent_visible());
    assert_eq!(persisted.last().unwrap().as_concat_text(), "goal is met");

    Ok(())
}

#[tokio::test]
async fn grind_is_bounded_by_max_turns() -> Result<()> {
    // Grind nudges after every completed turn; the nudges are user-role
    // messages, so only the max-turns budget (which ignores machine-generated
    // user messages) can end the loop.
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("grinding")]
    }));
    let harness = TestHarness::with_provider(provider).await;
    harness
        .agent
        .set_grind(Some("never done".to_string()))
        .await;

    let messages = harness.run("go", 3).await?;

    assert_eq!(harness.provider.call_count(), 3);
    let last = messages.last().expect("at least one message");
    assert!(
        last.as_concat_text().contains("maximum number of actions"),
        "last: {last:#?}"
    );

    Ok(())
}

#[tokio::test]
async fn retry_resets_conversation_until_attempts_exhausted() -> Result<()> {
    use crate::agents::types::{RetryConfig, SuccessCheck};

    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("attempt")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    let stream = state_machine::reply(
        &harness.agent,
        Message::user().with_text("do the thing"),
        SessionConfig {
            id: harness.session_id.clone(),
            schedule_id: None,
            max_turns: Some(10),
            retry_config: Some(RetryConfig {
                max_retries: 1,
                checks: vec![SuccessCheck::Shell {
                    command: "exit 1".to_string(),
                }],
                on_failure: None,
                timeout_seconds: None,
                on_failure_timeout_seconds: None,
            }),
        },
        None,
    )
    .await?;
    tokio::pin!(stream);

    let mut replaced = 0;
    while let Some(event) = stream.next().await {
        if matches!(event?, AgentEvent::HistoryReplaced(_)) {
            replaced += 1;
        }
    }

    // First failure resets the conversation and retries; the second exhausts
    // the budget and appends the give-up message.
    assert_eq!(replaced, 1);
    assert_eq!(harness.provider.call_count(), 2);

    let persisted = harness.persisted_messages().await?;
    let last = persisted.last().expect("a persisted message");
    assert!(
        last.as_concat_text()
            .contains("Maximum retry attempts (1) exceeded"),
        "tail: {last:?}"
    );

    Ok(())
}

#[tokio::test]
async fn final_output_is_nudged_recorded_and_consumed() -> Result<()> {
    use crate::agents::final_output_tool::FINAL_OUTPUT_CONTINUATION_MESSAGE;
    use crate::recipe::Response;

    let harness = TestHarness::with_steps([
        // Turn 1 ends without calling the tool — the op must prod the model.
        Step::Text("thinking about it".to_string()),
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "recipe__final_output".to_string(),
            args: serde_json::json!({ "result": "42" }),
        },
        Step::Text("wrapped up".to_string()),
    ])
    .await;
    harness
        .agent
        .add_final_output_tool(Response {
            json_schema: Some(serde_json::json!({
                "type": "object",
                "properties": { "result": { "type": "string" } },
                "required": ["result"]
            })),
        })
        .await;

    let messages = harness.run("compute the answer", 10).await?;

    assert_eq!(harness.provider.call_count(), 3, "events: {messages:#?}");

    let persisted = harness.persisted_messages().await?;
    assert!(persisted
        .iter()
        .any(|m| m.as_concat_text() == FINAL_OUTPUT_CONTINUATION_MESSAGE));
    // The recorded output is appended as the closing assistant message.
    assert_eq!(
        persisted.last().unwrap().as_concat_text(),
        r#"{"result":"42"}"#
    );

    Ok(())
}

/// A hook plugin whose script logs each invocation to `hook.log` under its
/// plugin root. Keep the returned env alive for the duration of the test.
struct HookTestEnv {
    _temp_dir: tempfile::TempDir,
    plugin_dir: std::path::PathBuf,
}

impl HookTestEnv {
    fn new(event: &str, script: &str) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let plugin_dir = temp_dir.path().join("test-plugin");
        std::fs::create_dir_all(plugin_dir.join("hooks")).unwrap();
        std::fs::write(
            plugin_dir.join("hooks/hooks.json"),
            format!(
                r#"{{"hooks": {{"{event}": [{{"hooks": [{{"type": "command", "command": "sh ${{PLUGIN_ROOT}}/hook.sh"}}]}}]}}}}"#
            ),
        )
        .unwrap();
        std::fs::write(plugin_dir.join("hook.sh"), script).unwrap();
        Self {
            _temp_dir: temp_dir,
            plugin_dir,
        }
    }

    fn hook_manager(&self) -> crate::hooks::HookManager {
        use crate::plugins::discovery::{DiscoveredPlugin, PluginScope};
        crate::hooks::HookManager::from_plugins_for_test(vec![DiscoveredPlugin {
            name: "test-plugin".into(),
            root: self.plugin_dir.clone(),
            scope: PluginScope::Project,
        }])
    }

    fn invocations(&self) -> usize {
        std::fs::read_to_string(self.plugin_dir.join("hook.log"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}

const LOG_AND_ALLOW_SCRIPT: &str = "#!/bin/sh\necho ran >> \"$PLUGIN_ROOT/hook.log\"\nexit 0\n";
const LOG_AND_BLOCK_SCRIPT: &str =
    "#!/bin/sh\necho blocked >> \"$PLUGIN_ROOT/hook.log\"\necho \"not done yet\" >&2\nexit 2\n";

#[tokio::test]
async fn stop_hook_denial_retries_until_cap_overrides() -> Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let env = HookTestEnv::new("Stop", LOG_AND_BLOCK_SCRIPT);
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fn = calls.clone();
    let provider = Arc::new(ScriptedProvider::from_fn(move |_messages, _tools| {
        let n = calls_for_fn.fetch_add(1, Ordering::SeqCst);
        vec![Message::assistant().with_text(format!("response {n}"))]
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_hook_manager(env.hook_manager())
        .with_stop_hook_block_cap(2);

    let events = harness.run_events("hello", 10).await?;

    // Initial turn plus two honored retries; the third block hits the cap.
    assert_eq!(calls.load(Ordering::SeqCst), 3, "events: {events:#?}");
    assert_eq!(env.invocations(), 3);

    // Denial context is durable agent-facing state; the cap warning is a
    // persisted user-facing notification.
    let persisted = harness.persisted_messages().await?;
    let denials = persisted
        .iter()
        .filter(|m| m.as_concat_text().contains("blocked ending this turn"))
        .count();
    assert_eq!(denials, 2, "persisted: {persisted:#?}");
    let last = persisted.last().expect("a persisted message");
    assert!(
        last.content.iter().any(|c| matches!(
            c,
            MessageContent::SystemNotification(n) if n.msg.contains("GOOSE_STOP_HOOK_BLOCK_CAP")
        )),
        "tail: {last:?}"
    );

    Ok(())
}

#[tokio::test]
async fn stop_hook_allow_ends_turn_after_one_check() -> Result<()> {
    let env = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
    let harness = TestHarness::with_steps([Step::Text("done".to_string())])
        .await
        .with_hook_manager(env.hook_manager());

    let messages = harness.run("hello", 10).await?;

    assert_eq!(harness.provider.call_count(), 1);
    assert_eq!(env.invocations(), 1);
    assert_eq!(messages.last().unwrap().as_concat_text(), "done");

    Ok(())
}

#[tokio::test]
async fn session_start_hook_fires_once_per_session() -> Result<()> {
    let env = HookTestEnv::new("SessionStart", LOG_AND_ALLOW_SCRIPT);
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("ok")]
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_hook_manager(env.hook_manager());

    harness.run("first", 10).await?;
    harness.run("second", 10).await?;

    assert_eq!(env.invocations(), 1);
    assert_eq!(harness.provider.call_count(), 2);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_hook_denial_becomes_tool_error() -> Result<()> {
    let env = HookTestEnv::new("PreToolUse", LOG_AND_BLOCK_SCRIPT);
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "test__echo".to_string(),
            args: serde_json::json!({ "x": 1 }),
        },
        Step::Text("understood".to_string()),
    ])
    .await
    .with_default_extension()
    .await
    .with_hook_manager(env.hook_manager());

    let messages = harness.run("use the echo tool", 10).await?;

    assert_eq!(env.invocations(), 1);
    let tool_response = messages
        .iter()
        .find(|m| m.is_tool_response())
        .expect("a tool response");
    let text = tool_response_text(tool_response);
    assert!(
        text.contains("denied by policy hook"),
        "tool response: {text}"
    );
    // The denied tool never ran (echo would have returned the args as JSON).
    assert!(!text.contains("\"x\":1"));

    Ok(())
}

#[tokio::test]
async fn elicitation_blocks_tool_until_response_arrives() -> Result<()> {
    use crate::action_required_manager::ElicitationOutcome;
    use rmcp::model::ElicitationAction;

    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "test__elicit".to_string(),
            args: serde_json::json!({}),
        },
        Step::Text("thanks".to_string()),
    ])
    .await
    .with_default_extension()
    .await;

    let stream = state_machine::reply(
        &harness.agent,
        Message::user().with_text("ask me"),
        SessionConfig {
            id: harness.session_id.clone(),
            schedule_id: None,
            max_turns: Some(10),
            retry_config: None,
        },
        None,
    )
    .await?;
    tokio::pin!(stream);

    // Drive the stream; when the blocked tool's elicitation request shows up,
    // answer it the way Agent::reply's interception does — via the registry,
    // with the response message persisted — and keep draining.
    let mut messages = Vec::new();
    let mut answered_id = None;
    while let Some(event) = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next())
        .await
        .expect("stream stalled: elicitation never unblocked")
    {
        let event = event?;
        if let AgentEvent::Message(message) = event {
            let elicitation_id = message.content.iter().find_map(|content| match content {
                MessageContent::ActionRequired(action) => match &action.data {
                    ActionRequiredData::Elicitation { id, .. } => Some(id.clone()),
                    _ => None,
                },
                _ => None,
            });
            if let Some(id) = elicitation_id {
                let response_message = Message::user()
                    .with_generated_id()
                    .with_content(MessageContent::action_required_elicitation_response(
                        id.clone(),
                        serde_json::json!({ "answer": "blue" }),
                        ElicitationAction::Accept,
                    ))
                    .agent_only();
                crate::elicitation::complete_elicitation_with_message(
                    &harness.agent.config.session_manager,
                    &harness.session_id,
                    &id,
                    ElicitationOutcome::Accept(serde_json::json!({ "answer": "blue" })),
                    &response_message,
                )
                .await?;
                answered_id = Some(id);
            }
            messages.push(message);
        }
    }

    assert!(
        answered_id.is_some(),
        "no elicitation request was emitted: {messages:#?}"
    );

    // The tool unblocked with the accepted data and the turn completed.
    let tool_response = messages
        .iter()
        .find(|m| m.is_tool_response())
        .expect("a tool response");
    assert!(
        tool_response_text(tool_response).contains("blue"),
        "tool response: {tool_response:#?}"
    );
    assert_eq!(messages.last().unwrap().as_concat_text(), "thanks");
    assert_eq!(harness.provider.call_count(), 2);

    // The pending question and its answer are both durable conversation state.
    let persisted = harness.persisted_messages().await?;
    let request_position = persisted.iter().position(|m| {
        m.content.iter().any(|c| {
            matches!(
                c,
                MessageContent::ActionRequired(action)
                    if matches!(action.data, ActionRequiredData::Elicitation { .. })
            )
        })
    });
    let response_position = persisted.iter().position(|m| {
        m.content.iter().any(|c| {
            matches!(
                c,
                MessageContent::ActionRequired(action)
                    if matches!(action.data, ActionRequiredData::ElicitationResponse { .. })
            )
        })
    });
    assert!(
        request_position.is_some() && request_position < response_position,
        "persisted: {persisted:#?}"
    );

    Ok(())
}

#[tokio::test]
async fn stale_orphaned_tool_request_is_not_executed() -> Result<()> {
    // A crash mid-execution leaves an unanswered tool request behind. On the
    // next user prompt it must not be executed or re-approved, and the
    // provider must not see it (an unanswered tool call is a protocol error).
    // It stays in the transcript as history.
    let provider = Arc::new(ScriptedProvider::from_fn(|messages, _tools| {
        assert!(
            messages.iter().all(|m| m
                .content
                .iter()
                .all(|c| !matches!(c, MessageContent::ToolRequest(_)))),
            "provider saw an orphaned tool request: {messages:#?}"
        );
        vec![Message::assistant().with_text("fresh start")]
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_default_extension()
        .await;

    let session_manager = harness.agent.config.session_manager.clone();
    session_manager
        .add_message(
            &harness.session_id,
            &Message::user().with_text("old prompt"),
        )
        .await?;
    session_manager
        .add_message(
            &harness.session_id,
            &Message::assistant().with_tool_request(
                "orphan_1",
                Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                    .with_arguments(serde_json::Map::new())),
            ),
        )
        .await?;

    let messages = harness.run("are you there?", 10).await?;

    assert_eq!(harness.provider.call_count(), 1);
    assert_eq!(messages.len(), 1, "events: {messages:#?}");
    assert_eq!(messages[0].as_concat_text(), "fresh start");

    let persisted = harness.persisted_messages().await?;
    assert!(persisted.iter().any(|m| {
        m.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(request) if request.id == "orphan_1"))
    }));
    assert!(!persisted.iter().any(|m| m.is_tool_response()));

    Ok(())
}

#[tokio::test]
async fn compacts_when_over_token_threshold() -> Result<()> {
    // Every provider call (the compaction summary and the post-compaction LLM
    // turn) returns plain text, so the loop ends after one real turn.
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("ok")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    // 128k context * 0.8 threshold = 102_400; push well past it.
    harness.set_total_tokens(120_000).await;

    let events = harness.run_events("hello", 10).await?;

    // Compaction replaced the conversation exactly once.
    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {events:#?}");

    // The "Performing auto-compaction" notice was emitted.
    use crate::conversation::message::MessageContent;
    let saw_notice = events.iter().any(|e| {
        match e {
        AgentEvent::Message(m) => m.content.iter().any(|c| {
            matches!(c, MessageContent::SystemNotification(s) if s.msg.contains("auto-compaction"))
        }),
        _ => false,
    }
    });
    assert!(saw_notice, "events: {events:#?}");

    // Provider was called for the summary and then the post-compaction turn.
    assert_eq!(harness.provider.call_count(), 2);

    // The stale 120k total was replaced by real usage — first the compaction's
    // summary size, then the post-compaction turn's total (the scripted
    // provider reports 15) — so compaction doesn't re-trigger.
    let reloaded = harness.reload().await?;
    assert_eq!(reloaded.usage.total_tokens, Some(15));

    Ok(())
}

#[tokio::test]
async fn llm_turn_records_usage_on_session_and_message() -> Result<()> {
    let harness = TestHarness::with_steps([Step::Text("hi there".to_string())]).await;

    let events = harness.run_events("hello", 10).await?;

    // The scripted provider reports (10 in, 5 out, 15 total) per call.
    let reloaded = harness.reload().await?;
    assert_eq!(reloaded.usage.total_tokens, Some(15));
    assert_eq!(reloaded.usage.input_tokens, Some(10));
    assert_eq!(reloaded.usage.output_tokens, Some(5));

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::Usage(u) if u.usage.total_tokens == Some(15))),
        "events: {events:#?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::MessageUsage { .. })),
        "events: {events:#?}"
    );

    // The usage ledger is attached to the persisted assistant message.
    let persisted = harness.persisted_messages().await?;
    let assistant = persisted
        .iter()
        .find(|m| m.role == Role::Assistant)
        .expect("an assistant message");
    let usage = assistant.metadata.usage.as_ref().expect("message usage");
    assert_eq!(usage.total_tokens, Some(15));

    Ok(())
}

#[tokio::test]
async fn provider_error_is_persisted_and_yields() -> Result<()> {
    use crate::conversation::message::MessageErrorKind;
    use goose_providers::errors::ProviderError;

    let provider = Arc::new(ScriptedProvider::from_steps([Step::Error(
        ProviderError::ServerError("boom".to_string()),
    )]));
    let harness = TestHarness::with_provider(provider).await;

    let events = harness.run_events("hello", 10).await?;

    // The error surfaced as a message event (replacing the old notification).
    let saw_error_event = events.iter().any(|e| {
        matches!(
            e,
            AgentEvent::Message(m) if m.error_kind() == Some(MessageErrorKind::Other)
        )
    });
    assert!(saw_error_event, "events: {events:#?}");

    // It is durable conversation state, tagged, user-visible, agent-invisible.
    let persisted = harness.persisted_messages().await?;
    let error = persisted
        .iter()
        .find(|m| m.error_kind() == Some(MessageErrorKind::Other))
        .expect("a persisted error message");
    assert!(error.is_user_visible());
    assert!(!error.is_agent_visible());

    // The provider was called exactly once: ExitOnError yielded, no retry.
    assert_eq!(harness.provider.call_count(), 1);

    Ok(())
}

#[tokio::test]
async fn slash_command_yields_without_calling_provider() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_steps([Step::Text(
        "should not run".to_string(),
    )]));
    let harness = TestHarness::with_provider(provider).await;

    let messages = harness.run("/status", 10).await?;

    assert_eq!(harness.provider.call_count(), 0);
    assert_eq!(messages.len(), 2, "events: {messages:#?}");
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[0].as_concat_text(), "/status");
    assert_eq!(messages[1].role, Role::Assistant);
    assert!(
        messages[1].as_concat_text().contains("Provider:"),
        "response: {:#?}",
        messages[1]
    );

    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 2);
    assert!(persisted.iter().all(|m| m.is_user_visible()));
    assert!(persisted.iter().all(|m| !m.is_agent_visible()));

    Ok(())
}

#[tokio::test]
async fn unknown_slash_text_falls_through_to_provider() -> Result<()> {
    let harness = TestHarness::with_steps([Step::Text("saw it".to_string())]).await;

    let messages = harness.run("/not-a-command", 10).await?;

    assert_eq!(harness.provider.call_count(), 1);
    assert_eq!(messages.len(), 1, "events: {messages:#?}");
    assert_eq!(messages[0].as_concat_text(), "saw it");

    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0].as_concat_text(), "/not-a-command");
    assert!(persisted[0].is_user_visible());
    assert!(persisted[0].is_agent_visible());

    Ok(())
}

#[tokio::test]
async fn goal_slash_command_starts_turn_with_hidden_kickoff() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::Text("working on it".to_string()),
        Step::Text("all done".to_string()),
    ])
    .await;

    let messages = harness.run("/goal finish the migration", 10).await?;

    // The command kicks off a turn; the retry op then nudges once ("check
    // whether the goal has been fully met") for a second turn, and clears
    // the goal.
    assert_eq!(harness.provider.call_count(), 2);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[1].role, Role::Assistant);
    assert_eq!(messages[2].as_concat_text(), "working on it");
    assert!(harness.agent.get_goal().await.is_none());

    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 6, "persisted: {persisted:#?}");
    assert_eq!(persisted[0].as_concat_text(), "/goal finish the migration");
    assert!(persisted[0].is_user_visible());
    assert!(!persisted[0].is_agent_visible());
    assert!(persisted[1].is_user_visible());
    assert!(!persisted[1].is_agent_visible());
    assert!(persisted[2]
        .as_concat_text()
        .contains("finish the migration"));
    assert!(!persisted[2].is_user_visible());
    assert!(persisted[2].is_agent_visible());
    assert_eq!(persisted[3].as_concat_text(), "working on it");
    assert!(persisted[4].as_concat_text().contains("fully met"));
    assert_eq!(persisted[5].as_concat_text(), "all done");

    Ok(())
}

#[tokio::test]
async fn history_slash_command_replaces_history_and_yields() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_steps([Step::Text(
        "should not run".to_string(),
    )]));
    let harness = TestHarness::with_provider(provider).await;

    let events = harness.run_events("/clear", 10).await?;

    assert_eq!(harness.provider.call_count(), 0);
    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {events:#?}");

    let messages: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::Message(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 2, "events: {events:#?}");
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[0].as_concat_text(), "/clear");
    assert_eq!(messages[1].role, Role::Assistant);

    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 2);
    assert!(persisted.iter().all(|m| m.is_user_visible()));
    assert!(persisted.iter().all(|m| !m.is_agent_visible()));

    Ok(())
}

#[tokio::test]
async fn repeated_context_length_errors_stop_after_capped_retries() -> Result<()> {
    use crate::conversation::message::MessageErrorKind;
    use goose_providers::errors::ProviderError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Turns (even calls) always blow the context; compaction summaries (odd
    // calls) succeed but never help. Without a working retry cap this cycles
    // compact/retry forever.
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fn = calls.clone();
    let provider = Arc::new(ScriptedProvider::from_fn_result(
        move |_messages, _tools| match calls_for_fn.fetch_add(1, Ordering::SeqCst) % 2 {
            0 => Err(ProviderError::ContextLengthExceeded("too long".to_string())),
            _ => Ok(vec![Message::assistant().with_text("summary")]),
        },
    ));
    let harness = TestHarness::with_provider(provider).await;

    let events = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        harness.run_events("hello", 10),
    )
    .await
    .expect("retry cap did not stop the compact/retry cycle")?;

    // Failing turn, then two capped compact-and-retry cycles, then ExitOnError
    // yields: turn, summary, retry, summary, retry.
    assert_eq!(harness.provider.call_count(), 5, "events: {events:#?}");

    let persisted = harness.persisted_messages().await?;
    let last = persisted.last().expect("a persisted message");
    assert_eq!(
        last.error_kind(),
        Some(MessageErrorKind::ContextLengthExceeded),
        "tail: {last:?}"
    );

    Ok(())
}

#[tokio::test]
async fn successful_turns_reset_the_compact_retry_budget() -> Result<()> {
    use goose_providers::errors::ProviderError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Three context errors in one reply, but each compaction is followed by a
    // successful turn (a tool call that keeps the loop going). The failure
    // budget resets on success, so the third compaction must still happen —
    // the cap only stops *consecutive* failed compact-retry cycles.
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fn = calls.clone();
    let provider = Arc::new(ScriptedProvider::from_fn_result(
        move |_messages, _tools| {
            let n = calls_for_fn.fetch_add(1, Ordering::SeqCst);
            match n {
                0 | 3 | 6 => Err(ProviderError::ContextLengthExceeded("too long".to_string())),
                1 | 4 | 7 => Ok(vec![Message::assistant().with_text("summary")]),
                8 => Ok(vec![Message::assistant().with_text("done")]),
                _ => Ok(vec![Message::assistant().with_tool_request(
                    format!("call_{n}"),
                    Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                        .with_arguments(serde_json::Map::new())),
                )]),
            }
        },
    ));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_default_extension()
        .await;

    let events = harness.run_events("hello", 20).await?;

    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 3, "events: {events:#?}");

    // (error, summary, retry) three times, ending in "done": 9 calls.
    assert_eq!(harness.provider.call_count(), 9);

    let persisted = harness.persisted_messages().await?;
    let last = persisted.last().expect("a persisted message");
    assert!(last.error_kind().is_none(), "tail still an error: {last:?}");
    assert_eq!(last.as_concat_text(), "done");

    Ok(())
}

#[tokio::test]
async fn context_length_error_triggers_compaction_recovery() -> Result<()> {
    use goose_providers::errors::ProviderError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // First LLM call blows the context; after compaction replaces the
    // conversation, the retried call succeeds with plain text.
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fn = calls.clone();
    let provider = Arc::new(ScriptedProvider::from_fn_result(
        move |_messages, _tools| {
            match calls_for_fn.fetch_add(1, Ordering::SeqCst) {
                // call 0: the failing turn
                0 => Err(ProviderError::ContextLengthExceeded("too long".to_string())),
                // call 1: the compaction summary
                // call 2: the retried turn
                _ => Ok(vec![Message::assistant().with_text("recovered")]),
            }
        },
    ));
    let harness = TestHarness::with_provider(provider).await;

    let events = harness.run_events("hello", 10).await?;

    // Compaction replaced the conversation as part of recovery.
    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {events:#?}");

    // The turn ultimately succeeded; no error message lingers on the tail.
    let persisted = harness.persisted_messages().await?;
    let last = persisted.last().expect("a persisted message");
    assert!(last.error_kind().is_none(), "tail still an error: {last:?}");

    // Failing turn + compaction summary + retried turn = three provider calls.
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    Ok(())
}
