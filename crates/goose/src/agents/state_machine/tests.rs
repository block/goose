use anyhow::Result;
use futures::StreamExt;
use rmcp::model::Role;

use crate::agents::state_machine::test_helpers::{
    tool_response_text, ScriptedProvider, Step, TestExtensionClient, TestHarness, TestToolBehavior,
};
use crate::agents::tool_execution::DECLINED_RESPONSE;
use crate::agents::types::SessionConfig;
use crate::agents::{state_machine, AgentEvent};
use crate::config::GooseMode;
use crate::conversation::message::{ActionRequiredData, Message, MessageContent};
use crate::permission::Permission;
use std::sync::Arc;

#[tokio::test]
async fn bang_shell_runs_shell_without_llm() -> Result<()> {
    let harness = TestHarness::with_steps([Step::Text("should not be called".to_string())])
        .await
        .with_extension(TestExtensionClient::named(
            "developer",
            vec![("shell".to_string(), TestToolBehavior::Echo)],
        ))
        .await;

    let messages = harness.run("!echo hello", 10).await?;

    assert_eq!(messages.len(), 2, "events: {messages:#?}");
    assert_eq!(messages[0].role, Role::Assistant);
    assert!(messages[0].content.iter().any(|content| {
        matches!(
            content,
            MessageContent::ToolRequest(request)
                if request
                    .tool_call
                    .as_ref()
                    .is_ok_and(|tool_call| tool_call.name == "developer__shell"
                        && tool_call
                            .arguments
                            .as_ref()
                            .and_then(|args| args.get("command"))
                            .and_then(|command| command.as_str())
                            == Some("echo hello"))
        )
    }));
    assert_eq!(messages[1].role, Role::User);
    assert!(messages[1].is_tool_response());
    assert!(
        tool_response_text(&messages[1]).contains("\"command\":\"echo hello\""),
        "tool response: {}",
        tool_response_text(&messages[1])
    );
    assert_eq!(harness.provider.call_count(), 0);

    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 3, "persisted: {persisted:#?}");
    assert_eq!(persisted[0].role, Role::User);
    assert_eq!(persisted[0].as_concat_text(), "!echo hello");
    assert_eq!(persisted[1].role, Role::Assistant);
    assert!(persisted[1].is_tool_call());
    assert_eq!(persisted[2].role, Role::User);
    assert!(persisted[2].is_tool_response());

    Ok(())
}

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

    assert_eq!(messages.len(), 3, "events: {messages:#?}");
    assert_eq!(messages[0].role, Role::Assistant);
    assert!(messages[0].is_tool_call());
    assert_eq!(messages[1].role, Role::User);
    assert!(messages[1].is_tool_response());
    assert_eq!(messages[2].role, Role::Assistant);

    let resp_text = tool_response_text(&messages[1]);
    assert!(resp_text.contains("\"x\":1"), "tool response: {resp_text}");

    assert_eq!(harness.provider.call_count(), 2);

    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted.len(), 4);
    assert_eq!(persisted[0].role, Role::User);

    Ok(())
}

#[tokio::test]
async fn unknown_tool_is_returned_to_the_llm_as_an_error() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "missing__tool".to_string(),
            args: serde_json::json!({}),
        },
        Step::Text("recovered".to_string()),
    ])
    .await;

    let messages = harness.run("try the missing tool", 10).await?;

    assert_eq!(harness.provider.call_count(), 2);
    assert_eq!(messages.len(), 3, "events: {messages:#?}");
    assert!(messages[1].is_tool_response());
    assert!(
        tool_response_text(&messages[1]).contains("Tool 'missing__tool' is not available"),
        "tool response: {}",
        tool_response_text(&messages[1])
    );
    assert_eq!(messages[2].as_concat_text(), "recovered");

    Ok(())
}

#[tokio::test]
async fn stops_at_max_turns() -> Result<()> {
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

    assert_eq!(harness.provider.call_count(), 3);

    let limit = messages.last().expect("at least one message");
    assert_eq!(limit.role, Role::Assistant);
    assert!(
        limit.as_concat_text().contains("maximum number of actions"),
        "last message: {limit:#?}"
    );

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
async fn max_turns_counts_llm_calls_not_assistant_messages() -> Result<()> {
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let provider = Arc::new(ScriptedProvider::from_fn(move |_messages, _tools| {
        let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        vec![
            Message::assistant().with_text(format!("thinking about step {n}")),
            Message::assistant().with_tool_request(
                format!("call_{n}"),
                Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                    .with_arguments(serde_json::Map::new())),
            ),
        ]
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_default_extension()
        .await;

    let messages = harness.run("keep going", 3).await?;

    assert_eq!(harness.provider.call_count(), 3, "events: {messages:#?}");
    assert!(messages
        .last()
        .unwrap()
        .as_concat_text()
        .contains("maximum number of actions"));

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
async fn queued_steer_is_injected_between_turns() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_fn(|messages, _tools| {
        let saw_steer = messages
            .iter()
            .any(|m| m.as_concat_text().contains("actually, use blue"));
        vec![Message::assistant().with_text(if saw_steer {
            "switched to blue"
        } else {
            "starting"
        })]
    }));
    let harness = TestHarness::with_provider(provider).await;
    harness
        .agent
        .steer(
            &harness.session_id,
            Message::user().with_text("actually, use blue"),
        )
        .await;

    let messages = harness.run("paint it", 10).await?;

    assert_eq!(harness.provider.call_count(), 2, "events: {messages:#?}");
    assert_eq!(
        messages.last().unwrap().as_concat_text(),
        "switched to blue"
    );

    let persisted = harness.persisted_messages().await?;
    let steer = persisted
        .iter()
        .find(|m| m.as_concat_text() == "actually, use blue")
        .expect("persisted steer message");
    assert!(steer.metadata.steer);
    assert!(!harness.agent.has_pending_steers(&harness.session_id).await);

    Ok(())
}

#[tokio::test]
async fn unparseable_tool_call_gets_parse_error_response() -> Result<()> {
    use rmcp::model::{ErrorCode, ErrorData};

    let harness = TestHarness::with_steps([
        Step::Messages(vec![Message::assistant().with_tool_request(
            "bad_call",
            Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "unbalanced braces in arguments",
                None,
            )),
        )]),
        Step::Text("let me fix that".to_string()),
    ])
    .await
    .with_default_extension()
    .await;

    let messages = harness.run("do it", 10).await?;

    let tool_response = messages
        .iter()
        .find(|m| m.is_tool_response())
        .expect("a tool response");
    let text = tool_response_text(tool_response);
    assert!(text.contains("could not be parsed"), "response: {text}");
    assert!(text.contains("unbalanced braces"), "response: {text}");
    assert_eq!(harness.provider.call_count(), 2);
    assert_eq!(messages.last().unwrap().as_concat_text(), "let me fix that");

    Ok(())
}

#[tokio::test]
async fn turn_context_is_injected_into_provider_view() -> Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};

    let saw_turn_context = Arc::new(AtomicBool::new(false));
    let saw = saw_turn_context.clone();
    let provider = Arc::new(ScriptedProvider::from_fn(move |messages, _tools| {
        if messages
            .iter()
            .any(|m| m.as_concat_text().contains("<turn-context>"))
        {
            saw.store(true, Ordering::Relaxed);
        }
        vec![Message::assistant().with_text("ok")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    harness.run("hello", 10).await?;

    assert!(saw_turn_context.load(Ordering::Relaxed));

    let persisted = harness.persisted_messages().await?;
    assert!(!persisted
        .iter()
        .any(|m| m.as_concat_text().contains("<turn-context>")));

    Ok(())
}

#[tokio::test]
async fn old_tool_pairs_are_summarized_away() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("summary of the pair")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    let session_manager = harness.agent.config.session_manager.clone();
    session_manager
        .add_message(&harness.session_id, &Message::user().with_text("old work"))
        .await?;
    for n in 0..46 {
        let id = format!("call_{n}");
        session_manager
            .add_message(
                &harness.session_id,
                &Message::assistant().with_tool_request(
                    id.clone(),
                    Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                        .with_arguments(serde_json::Map::new())),
                ),
            )
            .await?;
        let mut response = Message::user();
        response.add_tool_response_with_metadata(
            id,
            Ok(rmcp::model::CallToolResult::success(vec![
                rmcp::model::Content::text("result"),
            ])),
            None,
        );
        session_manager
            .add_message(&harness.session_id, &response)
            .await?;
    }

    harness.run("carry on", 10).await?;

    assert_eq!(harness.provider.call_count(), 11);

    let persisted = harness.persisted_messages().await?;
    let visible_requests = persisted
        .iter()
        .filter(|m| m.is_agent_visible() && m.is_tool_call())
        .count();
    assert_eq!(visible_requests, 36, "persisted: {persisted:#?}");
    let summaries = persisted
        .iter()
        .filter(|m| {
            m.as_concat_text() == "summary of the pair"
                && m.is_agent_visible()
                && !m.is_user_visible()
        })
        .count();
    assert_eq!(summaries, 10);

    Ok(())
}

#[tokio::test]
async fn tool_pairs_from_the_current_turn_are_not_summarized() -> Result<()> {
    let turns = std::sync::atomic::AtomicUsize::new(0);
    let provider = Arc::new(ScriptedProvider::from_fn(move |_messages, tools| {
        if tools.is_empty() {
            return vec![Message::assistant().with_text("summary of the pair")];
        }

        let turn = turns.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if turn < 21 {
            vec![Message::assistant().with_tool_request(
                format!("call_{turn}"),
                Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                    .with_arguments(serde_json::Map::new())),
            )]
        } else {
            vec![Message::assistant().with_text("done")]
        }
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_default_extension()
        .await;

    harness.run("do a lot of work", 30).await?;

    assert_eq!(harness.provider.call_count(), 22);
    let persisted = harness.persisted_messages().await?;
    assert_eq!(
        persisted
            .iter()
            .filter(|message| message.is_agent_visible() && message.is_tool_call())
            .count(),
        21
    );
    assert!(!persisted
        .iter()
        .any(|message| message.as_concat_text() == "summary of the pair"));

    Ok(())
}

#[tokio::test]
async fn batched_tool_pairs_are_summarized_as_groups() -> Result<()> {
    // 14 turns of two parallel calls each: one assistant message with two
    // requests, one user message with both responses — the shape the machine
    // itself writes. Hiding is per message, so each pair must be summarized
    // once as a group; per-id summaries would double-summarize and
    // double-hide. The first batch of 10 ids covers 5 message pairs.
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("group summary")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    let session_manager = harness.agent.config.session_manager.clone();
    session_manager
        .add_message(&harness.session_id, &Message::user().with_text("old work"))
        .await?;
    for n in 0..14 {
        let ids = [format!("call_{n}a"), format!("call_{n}b")];
        let mut request = Message::assistant();
        let mut response = Message::user();
        for id in &ids {
            request = request.with_tool_request(
                id.clone(),
                Ok(rmcp::model::CallToolRequestParams::new("test__echo")
                    .with_arguments(serde_json::Map::new())),
            );
            response.add_tool_response_with_metadata(
                id.clone(),
                Ok(rmcp::model::CallToolResult::success(vec![
                    rmcp::model::Content::text("result"),
                ])),
                None,
            );
        }
        session_manager
            .add_message(&harness.session_id, &request)
            .await?;
        session_manager
            .add_message(&harness.session_id, &response)
            .await?;
    }

    harness.run("carry on", 10).await?;

    // 5 group summaries plus the actual turn.
    assert_eq!(harness.provider.call_count(), 6);

    let persisted = harness.persisted_messages().await?;
    let summaries = persisted
        .iter()
        .filter(|m| {
            m.as_concat_text() == "group summary" && m.is_agent_visible() && !m.is_user_visible()
        })
        .count();
    assert_eq!(summaries, 5, "persisted: {persisted:#?}");

    // No widowed content: a request and its response are hidden or visible
    // together.
    for message in persisted.iter().filter(|m| m.is_tool_response()) {
        let response_ids = message.get_tool_response_ids();
        let paired_visibility = persisted
            .iter()
            .find(|m| {
                m.get_tool_request_ids()
                    .intersection(&response_ids)
                    .next()
                    .is_some()
            })
            .map(|m| m.is_agent_visible());
        assert_eq!(paired_visibility, Some(message.is_agent_visible()));
    }

    Ok(())
}

#[tokio::test]
async fn compact_command_does_not_duplicate_the_command_message() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("a summary")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    let session_manager = harness.agent.config.session_manager.clone();
    session_manager
        .add_message(&harness.session_id, &Message::user().with_text("hello"))
        .await?;
    session_manager
        .add_message(
            &harness.session_id,
            &Message::assistant().with_text("hi there"),
        )
        .await?;

    let events = harness.run_events("/compact", 10).await?;

    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {events:#?}");

    // The compacted conversation retains the triggering message; the op must
    // not append a second copy.
    let persisted = harness.persisted_messages().await?;
    let command_count = persisted
        .iter()
        .filter(|m| m.as_concat_text().trim() == "/compact")
        .count();
    assert_eq!(command_count, 1, "persisted: {persisted:#?}");
    let command = persisted
        .iter()
        .find(|m| m.as_concat_text().trim() == "/compact")
        .unwrap();
    assert!(command.is_user_visible());
    assert!(!command.is_agent_visible());

    Ok(())
}

#[tokio::test]
async fn chat_mode_skips_tool_execution() -> Result<()> {
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "test__echo".to_string(),
            args: serde_json::json!({ "x": 1 }),
        },
        Step::Text("noted".to_string()),
    ])
    .await
    .with_default_extension()
    .await
    .with_goose_mode(GooseMode::Chat)
    .await;

    let messages = harness.run("try the tool", 10).await?;

    assert!(!messages.iter().any(|m| m
        .content
        .iter()
        .any(|c| matches!(c, MessageContent::ActionRequired(_)))));
    let tool_response = messages
        .iter()
        .find(|m| m.is_tool_response())
        .expect("a tool response");
    let text = tool_response_text(tool_response);
    assert!(text.contains("chat mode"), "tool response: {text}");
    assert!(!text.contains("\"x\":1"));
    assert_eq!(harness.provider.call_count(), 2);

    Ok(())
}

#[tokio::test]
async fn extension_added_mid_reply_refreshes_tools() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, tools| {
        if tools.iter().any(|tool| tool.name == "extra__echo") {
            vec![Message::assistant().with_text("extension ready".to_string())]
        } else {
            vec![Message::assistant().with_tool_request(
                "call_1",
                Ok(rmcp::model::CallToolRequestParams::new("test__addext")
                    .with_arguments(serde_json::Map::new())),
            )]
        }
    }));
    let harness = TestHarness::with_provider(provider)
        .await
        .with_extension(
            crate::agents::state_machine::test_helpers::TestExtensionClient::new(vec![(
                "addext".to_string(),
                crate::agents::state_machine::test_helpers::TestToolBehavior::AddExtension,
            )]),
        )
        .await;

    let messages = harness.run("install the extra extension", 10).await?;

    assert_eq!(harness.provider.call_count(), 2, "events: {messages:#?}");
    assert_eq!(messages.last().unwrap().as_concat_text(), "extension ready");

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
    use crate::recipe::{Recipe, Response};

    let harness = TestHarness::with_steps([
        Step::Text("thinking about it".to_string()),
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "recipe__final_output".to_string(),
            args: serde_json::json!({ "result": "42" }),
        },
        Step::Text("wrapped up".to_string()),
    ])
    .await;
    let recipe = Recipe::builder()
        .title("Structured output")
        .description("Return structured output")
        .instructions("Compute the answer")
        .response(Response {
            json_schema: Some(serde_json::json!({
                "type": "object",
                "properties": { "result": { "type": "string" } },
                "required": ["result"]
            })),
        })
        .build()
        .unwrap();
    harness
        .agent
        .config
        .session_manager
        .update(&harness.session_id)
        .recipe(Some(recipe))
        .apply()
        .await?;

    let messages = harness.run("compute the answer", 10).await?;

    assert_eq!(harness.provider.call_count(), 3, "events: {messages:#?}");

    let persisted = harness.persisted_messages().await?;
    assert!(persisted
        .iter()
        .any(|m| m.as_concat_text() == FINAL_OUTPUT_CONTINUATION_MESSAGE));
    assert_eq!(
        persisted.last().unwrap().as_concat_text(),
        r#"{"result":"42"}"#
    );

    Ok(())
}

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

    assert_eq!(calls.load(Ordering::SeqCst), 3, "events: {events:#?}");
    assert_eq!(env.invocations(), 3);

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
async fn stop_hook_is_notified_once_on_max_turns_exit() -> Result<()> {
    let env = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
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
        .await
        .with_hook_manager(env.hook_manager());

    let messages = harness.run("keep going", 2).await?;

    assert_eq!(harness.provider.call_count(), 2);
    assert_eq!(env.invocations(), 1, "events: {messages:#?}");

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
    let provider = Arc::new(ScriptedProvider::from_fn(|_messages, _tools| {
        vec![Message::assistant().with_text("ok")]
    }));
    let harness = TestHarness::with_provider(provider).await;

    harness.set_total_tokens(120_000).await;

    let events = harness.run_events("hello", 10).await?;

    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {events:#?}");

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

    assert_eq!(harness.provider.call_count(), 2);

    let reloaded = harness.reload().await?;
    assert_eq!(reloaded.usage.total_tokens, Some(15));

    Ok(())
}

#[tokio::test]
async fn compaction_operation_contributes_remaining_context_to_moim() -> Result<()> {
    let provider = Arc::new(ScriptedProvider::from_fn(|messages, _tools| {
        assert!(
            messages
                .iter()
                .any(|message| message.as_concat_text().contains("<compaction>")),
            "messages: {messages:#?}"
        );
        vec![Message::assistant().with_text("ok")]
    }));
    let harness = TestHarness::with_provider(provider).await;
    harness.set_total_tokens(60_000).await;

    harness.run("hello", 10).await?;

    assert_eq!(harness.provider.call_count(), 1);
    Ok(())
}

#[tokio::test]
async fn llm_turn_records_usage_on_session_and_message() -> Result<()> {
    let harness = TestHarness::with_steps([Step::Text("hi there".to_string())]).await;

    let events = harness.run_events("hello", 10).await?;

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
async fn usage_before_a_stream_error_is_recorded() -> Result<()> {
    use goose_providers::conversation::token_usage::{ProviderUsage, Usage as ProviderTokenUsage};
    use goose_providers::errors::ProviderError;

    let usage = ProviderUsage::new(
        "scripted-model".to_string(),
        ProviderTokenUsage::new(Some(10), Some(5), Some(15)),
    );
    let provider = Arc::new(ScriptedProvider::from_stream(vec![
        Ok((
            Some(Message::assistant().with_text("partial response")),
            Some(usage),
        )),
        Err(ProviderError::ServerError("boom".to_string())),
    ]));
    let harness = TestHarness::with_provider(provider).await;

    let events = harness.run_events("hello", 10).await?;

    let reloaded = harness.reload().await?;
    assert_eq!(reloaded.usage.total_tokens, Some(15));
    assert!(events.iter().any(
        |event| matches!(event, AgentEvent::Usage(usage) if usage.usage.total_tokens == Some(15))
    ));
    assert!(!events
        .iter()
        .any(|event| matches!(event, AgentEvent::MessageUsage { .. })));

    let persisted = harness.persisted_messages().await?;
    assert!(!persisted
        .iter()
        .any(|message| message.as_concat_text() == "partial response"));
    let error = persisted
        .iter()
        .find(|message| message.error_kind().is_some())
        .expect("a persisted error message");
    assert!(error.metadata.usage.is_none());

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

    let saw_error_event = events.iter().any(|e| {
        matches!(
            e,
            AgentEvent::Message(m) if m.error_kind() == Some(MessageErrorKind::Other)
        )
    });
    assert!(saw_error_event, "events: {events:#?}");

    let persisted = harness.persisted_messages().await?;
    let error = persisted
        .iter()
        .find(|m| m.error_kind() == Some(MessageErrorKind::Other))
        .expect("a persisted error message");
    assert!(error.is_user_visible());
    assert!(!error.is_agent_visible());

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
    assert_eq!(harness.reload().await?.usage.total_tokens, Some(0));

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
    harness.set_total_tokens(100).await;

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
    assert_eq!(harness.reload().await?.usage.total_tokens, Some(0));

    Ok(())
}

#[tokio::test]
async fn skill_slash_command_adds_the_skill_to_the_turn() -> Result<()> {
    let working_dir = tempfile::tempdir()?;
    let skill_dir = working_dir.path().join(".agents/skills/review");
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review code\n---\nReview $ARGUMENTS carefully.",
    )?;
    let provider = Arc::new(ScriptedProvider::from_fn(|messages, _tools| {
        assert!(messages.iter().any(|message| message
            .as_concat_text()
            .contains("Review src/lib.rs carefully.")));
        vec![Message::assistant().with_text("reviewed")]
    }));
    let harness = TestHarness::with_provider(provider).await;
    harness
        .agent
        .config
        .session_manager
        .update(&harness.session_id)
        .working_dir(working_dir.path().to_path_buf())
        .apply()
        .await?;

    harness.run("/review src/lib.rs", 10).await?;

    assert_eq!(harness.provider.call_count(), 1);
    let persisted = harness.persisted_messages().await?;
    assert_eq!(persisted[0].as_concat_text(), "/review src/lib.rs");
    assert!(!persisted[0].is_agent_visible());
    assert!(persisted[1]
        .as_concat_text()
        .contains("Review src/lib.rs carefully."));
    assert!(persisted[1].is_agent_visible());
    assert!(!persisted[1].is_user_visible());

    Ok(())
}

#[tokio::test]
async fn skill_operation_advertises_and_executes_load_skill() -> Result<()> {
    let working_dir = tempfile::tempdir()?;
    let skill_dir = working_dir.path().join(".agents/skills/review");
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review code\n---\nReview carefully.",
    )?;
    let harness = TestHarness::with_steps([
        Step::ToolCall {
            id: "call_1".to_string(),
            name: "load_skill".to_string(),
            args: serde_json::json!({ "name": "review" }),
        },
        Step::Text("done".to_string()),
    ])
    .await;
    harness
        .agent
        .config
        .session_manager
        .update(&harness.session_id)
        .working_dir(working_dir.path().to_path_buf())
        .apply()
        .await?;

    let messages = harness.run("use the review skill", 10).await?;

    assert_eq!(harness.provider.call_count(), 2);
    assert!(messages[1].is_tool_response());
    assert!(tool_response_text(&messages[1]).contains("Review carefully."));
    assert_eq!(messages.last().unwrap().as_concat_text(), "done");

    Ok(())
}

#[tokio::test]
async fn repeated_context_length_errors_stop_after_capped_retries() -> Result<()> {
    use crate::conversation::message::MessageErrorKind;
    use goose_providers::errors::ProviderError;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
async fn successful_turns_do_not_reset_the_compact_retry_budget() -> Result<()> {
    use crate::conversation::message::MessageErrorKind;
    use goose_providers::errors::ProviderError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fn = calls.clone();
    let provider = Arc::new(ScriptedProvider::from_fn_result(
        move |_messages, _tools| {
            let n = calls_for_fn.fetch_add(1, Ordering::SeqCst);
            match n {
                0 | 3 | 6 => Err(ProviderError::ContextLengthExceeded("too long".to_string())),
                1 | 4 => Ok(vec![Message::assistant().with_text("summary")]),
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
    assert_eq!(replaced, 2, "events: {events:#?}");

    assert_eq!(harness.provider.call_count(), 7);

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
async fn context_length_error_triggers_compaction_recovery() -> Result<()> {
    use goose_providers::errors::ProviderError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_fn = calls.clone();
    let provider = Arc::new(ScriptedProvider::from_fn_result(
        move |_messages, _tools| match calls_for_fn.fetch_add(1, Ordering::SeqCst) {
            0 => Err(ProviderError::ContextLengthExceeded("too long".to_string())),
            _ => Ok(vec![Message::assistant().with_text("recovered")]),
        },
    ));
    let harness = TestHarness::with_provider(provider).await;

    let events = harness.run_events("hello", 10).await?;

    let replaced = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {events:#?}");

    let persisted = harness.persisted_messages().await?;
    let last = persisted.last().expect("a persisted message");
    assert!(last.error_kind().is_none(), "tail still an error: {last:?}");
    let context_errors: Vec<_> = persisted
        .iter()
        .filter(|message| {
            message.error_kind()
                == Some(crate::conversation::message::MessageErrorKind::ContextLengthExceeded)
        })
        .collect();
    assert_eq!(context_errors.len(), 1);
    assert!(!context_errors[0].is_agent_visible());

    assert_eq!(calls.load(Ordering::SeqCst), 3);

    Ok(())
}
