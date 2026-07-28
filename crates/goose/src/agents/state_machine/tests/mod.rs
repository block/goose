use anyhow::Result;
use rmcp::model::Role;
use serde_json::json;

use self::calculator_extension::{value, ADD, REQUEST_VALUE};
use self::pipeline::{run_machine, test_pipeline, MAX_TURNS};
use crate::agents::tool_execution::DECLINED_RESPONSE;
use crate::agents::{state_machine, AgentEvent};
use crate::config::GooseMode;
use crate::conversation::message::{ActionRequiredData, Message, MessageContent};
use crate::conversation::Conversation;
use crate::permission::Permission;

mod calculator_extension;
mod dummy_api;
mod pipeline;
mod provider_lifecycle;
mod tool_lifecycle;

fn tool_response_text(message: &Message) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            MessageContent::ToolResponse(response) => Some(match response.tool_result.as_ref() {
                Ok(result) => result
                    .content
                    .iter()
                    .filter_map(|content| content.as_text().map(|text| text.text.clone()))
                    .collect::<String>(),
                Err(error) => error.message.to_string(),
            }),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn bang_shell_requests_the_shell_tool() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;

    let result = pipeline.run(["!echo hello"]).await?;
    let conversation = result.rendered_conversation();
    assert!(conversation
        .iter()
        .any(|line| line == r#"toolcall: shell({"command":"echo hello"})"#));
    assert!(conversation
        .iter()
        .any(|line| line.starts_with("toolresponse: Tool 'shell' is not available")));
    assert_eq!(api.call_count(), 0);

    Ok(())
}

#[tokio::test]
async fn stops_at_max_turns() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("keep going").call(ADD, value(1));

    let result = pipeline.run(["keep going"]).await?;
    assert_eq!(api.call_count(), MAX_TURNS as usize);
    assert_eq!(pipeline.calculator_total(), MAX_TURNS as i64 - 1);
    let calls = api.calls();
    let first_budgeted_call = MAX_TURNS.div_ceil(2) as usize;
    assert!(calls[..first_budgeted_call]
        .iter()
        .all(|call| !call.input_contains("<turn-budget>")));
    assert!(calls[first_budgeted_call..]
        .iter()
        .all(|call| call.input_contains("<turn-budget>")));

    let conversation = result.rendered_conversation();
    assert_eq!(
        conversation.last().unwrap(),
        &format!("agent: {}", state_machine::MAX_TURNS_MESSAGE)
    );
    assert_eq!(
        conversation
            .iter()
            .filter(|line| line.starts_with("toolcall:"))
            .count(),
        MAX_TURNS as usize
    );
    assert!(conversation
        .iter()
        .all(|line| !line.contains("<turn-budget>")));

    Ok(())
}

#[tokio::test]
async fn approve_mode_waits_for_tool_confirmation_before_execution() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_goose_mode(GooseMode::Approve).await;
    api.on("add one").call(ADD, value(1));
    api.on("result: 1").reply("done");

    let awaiting_confirmation = pipeline.run(["add one"]).await?;
    assert!(awaiting_confirmation
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .iter()
        .any(|message| message.content.iter().any(|content| {
            matches!(
                content,
                MessageContent::ActionRequired(action)
                    if matches!(
                        action.data,
                        ActionRequiredData::ToolConfirmation { ref id, .. }
                            if id == "dummy-tool-call-1"
                    )
            )
        })));
    assert_eq!(api.call_count(), 1);
    assert_eq!(pipeline.calculator_total(), 0);

    pipeline
        .session_manager
        .add_message(
            &pipeline.session_id,
            &Message::user()
                .with_content(MessageContent::action_required_tool_confirmation_response(
                    "dummy-tool-call-1",
                    Permission::AllowOnce,
                ))
                .with_visibility(false, false),
        )
        .await?;
    run_machine(&pipeline).await?;
    let result = pipeline.session().await?;

    assert_eq!(api.call_count(), 2);
    assert!(result
        .conversation
        .unwrap_or_default()
        .messages()
        .iter()
        .any(|message| message.is_tool_response() && tool_response_text(message) == "result: 1"));
    assert_eq!(pipeline.calculator_total(), 1);

    Ok(())
}

#[tokio::test]
async fn denied_tool_confirmation_becomes_tool_response() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_goose_mode(GooseMode::Approve).await;
    api.on("add one").call(ADD, value(1));
    api.on(DECLINED_RESPONSE).reply("done");

    let awaiting_confirmation = pipeline.run(["add one"]).await?;
    assert!(awaiting_confirmation
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .iter()
        .any(|message| message.content.iter().any(|content| {
            matches!(
                content,
                MessageContent::ActionRequired(action)
                    if matches!(
                        action.data,
                        ActionRequiredData::ToolConfirmation { ref id, .. }
                            if id == "dummy-tool-call-1"
                    )
            )
        })));
    pipeline
        .session_manager
        .add_message(
            &pipeline.session_id,
            &Message::user()
                .with_content(MessageContent::action_required_tool_confirmation_response(
                    "dummy-tool-call-1",
                    Permission::DenyOnce,
                ))
                .with_visibility(false, false),
        )
        .await?;
    run_machine(&pipeline).await?;
    let result = pipeline.session().await?;

    assert_eq!(api.call_count(), 2);
    assert_eq!(pipeline.calculator_total(), 0);
    assert!(result
        .conversation
        .unwrap_or_default()
        .messages()
        .iter()
        .any(|message| message.is_tool_response()
            && tool_response_text(message).contains(DECLINED_RESPONSE)));

    Ok(())
}

#[tokio::test]
async fn queued_steer_is_injected_between_turns() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("paint it").reply("starting");
    api.on("actually, use blue").reply("switched to blue");
    pipeline
        .steer(Message::user().with_text("actually, use blue"))
        .await;

    let result = pipeline.run(["paint it"]).await?;
    assert_eq!(api.call_count(), 2);
    let rendered = result.rendered_conversation();
    assert_eq!(
        rendered.last().map(String::as_str),
        Some("agent: switched to blue")
    );

    let conversation = result.session.conversation.unwrap_or_default();
    let steer = conversation
        .messages()
        .iter()
        .find(|m| m.as_concat_text() == "actually, use blue")
        .expect("persisted steer message");
    assert!(steer.metadata.steer);
    assert!(!pipeline.has_pending_steers().await);

    Ok(())
}

#[tokio::test]
async fn tool_pairs_are_summarized_after_the_current_turn() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("do a lot of work").call(ADD, value(1));
    api.on("result:").call(ADD, value(1));
    api.on("result: 21").reply("done");
    api.on_system("summarize a tool call & response pair")
        .reply("summary of the pair");
    api.on("carry on").reply("carried on");

    let current_turn = pipeline.run(["do a lot of work"]).await?;
    assert_eq!(api.call_count(), 22);
    assert_eq!(
        current_turn
            .rendered_conversation()
            .last()
            .map(String::as_str),
        Some("agent: done")
    );
    let persisted = current_turn
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .to_vec();
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

    let next_turn = pipeline.run(["carry on"]).await?;
    assert_eq!(
        next_turn.rendered_conversation().last().map(String::as_str),
        Some("agent: carried on")
    );
    let persisted = next_turn
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .to_vec();
    assert_eq!(
        persisted
            .iter()
            .filter(|message| message.is_agent_visible() && message.is_tool_call())
            .count(),
        11,
        "persisted: {persisted:#?}"
    );
    assert_eq!(
        persisted
            .iter()
            .filter(|message| {
                message.as_concat_text() == "summary of the pair"
                    && message.is_agent_visible()
                    && !message.is_user_visible()
            })
            .count(),
        10
    );

    Ok(())
}

#[tokio::test]
async fn batched_tool_pairs_are_summarized_as_groups() -> Result<()> {
    // 14 turns of two parallel calls each: one assistant message with two
    // requests, one user message with both responses — the shape the machine
    // itself writes. Hiding is per message, so each pair must be summarized
    // once as a group; per-id summaries would double-summarize and
    // double-hide. The first batch of 10 ids covers 5 message pairs.
    let (pipeline, api) = test_pipeline().await?;
    api.on("result").reply("group summary");
    api.on("carry on").reply("done");

    let session_manager = pipeline.session_manager.clone();
    session_manager
        .add_message(&pipeline.session_id, &Message::user().with_text("old work"))
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
            .add_message(&pipeline.session_id, &request)
            .await?;
        session_manager
            .add_message(&pipeline.session_id, &response)
            .await?;
    }

    let result = pipeline.run(["carry on"]).await?;

    // 5 group summaries plus the actual turn.
    assert_eq!(api.call_count(), 6);
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: done")
    );

    let persisted = result
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .to_vec();
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
    let (pipeline, api) = test_pipeline().await?;
    api.on("Please summarize the conversation history")
        .reply("a summary");

    let session_manager = pipeline.session_manager.clone();
    session_manager
        .add_message(&pipeline.session_id, &Message::user().with_text("hello"))
        .await?;
    session_manager
        .add_message(
            &pipeline.session_id,
            &Message::assistant().with_text("hi there"),
        )
        .await?;

    let result = pipeline.run(["/compact"]).await?;

    let replaced = result
        .events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {:#?}", result.events);

    // The compacted conversation retains the triggering message; the op must
    // not append a second copy.
    let persisted = result
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .to_vec();
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
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_goose_mode(GooseMode::Chat).await;
    api.on("try the tool").call(ADD, value(1));
    api.on("chat mode").reply("noted");

    let result = pipeline.run(["try the tool"]).await?;
    let rendered = result.rendered_conversation();
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("toolresponse:") && line.contains("chat mode")));
    assert_eq!(rendered.last().map(String::as_str), Some("agent: noted"));
    let conversation = result.session.conversation.unwrap_or_default();
    assert!(!conversation.messages().iter().any(|m| {
        m.content
            .iter()
            .any(|c| matches!(c, MessageContent::ActionRequired(_)))
    }));
    assert_eq!(pipeline.calculator_total(), 0);
    Ok(())
}

#[tokio::test]
async fn extension_added_mid_reply_refreshes_tools() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("install the extra extension").call(
        crate::agents::platform_extensions::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE,
        json!({
            "action": "enable",
            "extension_name": "analyze"
        }),
    );
    api.on("installed successfully").reply("extension ready");

    let result = pipeline.run(["install the extra extension"]).await?;

    assert_eq!(api.call_count(), 2);
    assert!(api.calls()[1].advertises_tool("analyze"));
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: extension ready")
    );

    Ok(())
}

#[tokio::test]
async fn goal_nudges_once_then_clears() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("work on the goal").reply("did some work");
    api.on("fully met").reply("goal is met");
    pipeline.set_goal(Some("ship it".to_string())).await;

    let result = pipeline.run(["work on the goal"]).await?;

    assert_eq!(api.call_count(), 2, "events: {:#?}", result.events);
    assert!(pipeline.get_goal().await.is_none());

    let rendered = result.rendered_conversation();
    let persisted = result
        .session
        .conversation
        .unwrap_or_default()
        .messages()
        .to_vec();
    let nudge = persisted
        .iter()
        .find(|m| m.as_concat_text().contains("fully met"))
        .expect("a goal nudge message");
    assert!(!nudge.is_user_visible());
    assert!(nudge.is_agent_visible());
    assert_eq!(
        rendered.last().map(String::as_str),
        Some("agent: goal is met")
    );

    Ok(())
}

#[tokio::test]
async fn grind_is_bounded_by_max_turns() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("go").reply("grinding");
    api.on("never done").reply("grinding");
    pipeline.set_grind(Some("never done".to_string())).await;

    let result = pipeline.run(["go"]).await?;
    assert_eq!(api.call_count(), MAX_TURNS as usize);
    assert_eq!(
        result.rendered_conversation().last().unwrap(),
        &format!("agent: {}", state_machine::MAX_TURNS_MESSAGE)
    );

    Ok(())
}

#[tokio::test]
async fn retry_resets_conversation_until_attempts_exhausted() -> Result<()> {
    use crate::agents::types::{RetryConfig, SuccessCheck};
    use crate::recipe::Recipe;

    let (pipeline, api) = test_pipeline().await?;
    api.on("do the thing").reply("attempt");
    let retry = RetryConfig {
        max_retries: 1,
        checks: vec![SuccessCheck::Shell {
            command: "exit 1".to_string(),
        }],
        on_failure: None,
        timeout_seconds: None,
        on_failure_timeout_seconds: None,
    };
    let recipe = Recipe::builder()
        .title("retry test")
        .description("retry test")
        .prompt("do the thing")
        .retry(retry)
        .build()
        .expect("valid recipe");
    pipeline
        .session_manager
        .update(&pipeline.session_id)
        .recipe(Some(recipe))
        .apply()
        .await?;

    let result = pipeline.run(["do the thing"]).await?;
    let replaced = result
        .events
        .iter()
        .filter(|event| matches!(event, AgentEvent::HistoryReplaced(_)))
        .count();

    assert_eq!(replaced, 1);
    assert_eq!(api.call_count(), 2);

    assert!(result
        .rendered_conversation()
        .last()
        .is_some_and(|line| line.contains("Maximum retry attempts (1) exceeded")));

    Ok(())
}

#[tokio::test]
async fn final_output_is_nudged_recorded_and_consumed() -> Result<()> {
    use crate::agents::final_output_tool::FINAL_OUTPUT_CONTINUATION_MESSAGE;
    use crate::recipe::{Recipe, Response};

    let (pipeline, api) = test_pipeline().await?;
    api.on("compute the answer").reply("thinking about it");
    api.on(FINAL_OUTPUT_CONTINUATION_MESSAGE)
        .call("recipe__final_output", json!({ "result": "42" }));
    let recipe = Recipe::builder()
        .title("Structured output")
        .description("Return structured output")
        .instructions("Compute the answer")
        .response(Response {
            json_schema: Some(json!({
                "type": "object",
                "properties": { "result": { "type": "string" } },
                "required": ["result"]
            })),
        })
        .build()
        .unwrap();
    pipeline
        .session_manager
        .update(&pipeline.session_id)
        .recipe(Some(recipe))
        .apply()
        .await?;

    let result = pipeline.run(["compute the answer"]).await?;

    assert_eq!(api.call_count(), 2, "events: {:#?}", result.events);

    let rendered = result.rendered_conversation();
    assert!(rendered
        .iter()
        .any(|line| line.contains(FINAL_OUTPUT_CONTINUATION_MESSAGE)));
    assert_eq!(
        rendered.last().map(String::as_str),
        Some(r#"agent: {"result":"42"}"#)
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
    let env = HookTestEnv::new("Stop", LOG_AND_BLOCK_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline
        .with_hook_manager(env.hook_manager())
        .with_stop_hook_block_cap(2);
    api.on("hello").reply("response");
    api.on("blocked ending this turn").reply("response");

    let result = pipeline.run(["hello"]).await?;

    assert_eq!(api.call_count(), 3);
    assert_eq!(env.invocations(), 3);

    let conversation = result.session.conversation.unwrap_or_default();
    let denials = conversation
        .messages()
        .iter()
        .filter(|m| m.as_concat_text().contains("blocked ending this turn"))
        .count();
    assert_eq!(denials, 2);
    let last = conversation.last().expect("a persisted message");
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
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("hello").reply("done");

    let result = pipeline.run(["hello"]).await?;

    assert_eq!(api.call_count(), 1);
    assert_eq!(env.invocations(), 1);
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: done")
    );

    Ok(())
}

#[tokio::test]
async fn stop_hook_does_not_run_on_max_turns_exit() -> Result<()> {
    let env = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("keep going").call(ADD, value(1));

    pipeline.run(["keep going"]).await?;

    assert_eq!(api.call_count(), MAX_TURNS as usize);
    assert_eq!(pipeline.calculator_total(), MAX_TURNS as i64 - 1);
    assert_eq!(env.invocations(), 0);

    Ok(())
}

#[tokio::test]
async fn session_start_hook_fires_once_per_session() -> Result<()> {
    let env = HookTestEnv::new("SessionStart", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("first").reply("ok");
    api.on("second").reply("ok");

    pipeline.run(["/status"]).await?;
    assert_eq!(env.invocations(), 0);

    pipeline.run(["first", "second"]).await?;

    assert_eq!(env.invocations(), 1);
    assert_eq!(api.call_count(), 2);

    Ok(())
}

#[tokio::test]
async fn user_prompt_submit_hook_fires_once_for_an_agent_turn() -> Result<()> {
    let env = HookTestEnv::new("UserPromptSubmit", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("result: 1").reply("done");

    pipeline.run(["/status"]).await?;
    assert_eq!(env.invocations(), 0);

    pipeline.run(["add one"]).await?;
    assert_eq!(api.call_count(), 2);
    assert_eq!(pipeline.calculator_total(), 1);
    assert_eq!(env.invocations(), 1);

    Ok(())
}

#[tokio::test]
async fn pre_tool_use_hook_denial_becomes_tool_error() -> Result<()> {
    let env = HookTestEnv::new("PreToolUse", LOG_AND_BLOCK_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("denied by policy hook").reply("understood");
    let result = pipeline.run(["add one"]).await?;

    assert_eq!(env.invocations(), 1);
    assert!(result
        .rendered_conversation()
        .iter()
        .any(|line| line.starts_with("toolresponse:") && line.contains("denied by policy hook")));
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: understood")
    );
    assert_eq!(pipeline.calculator_total(), 0);

    Ok(())
}

#[tokio::test]
async fn elicitation_blocks_tool_until_response_arrives() -> Result<()> {
    use crate::action_required_manager::ElicitationOutcome;
    use rmcp::model::ElicitationAction;

    let (pipeline, api) = test_pipeline().await?;
    api.on("ask me").call(REQUEST_VALUE, json!({}));
    api.on("result: 7").reply("thanks");
    pipeline
        .session_manager
        .add_message(&pipeline.session_id, &Message::user().with_text("ask me"))
        .await?;

    let cancel = tokio_util::sync::CancellationToken::new();
    let machine = pipeline.machine(cancel.clone());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let emit = state_machine::Emitter::new(tx, cancel);
    let mut answered_id = None;
    for _ in 0..100 {
        let session = pipeline.session().await?;
        let step = machine.step(&session, emit.clone());
        tokio::pin!(step);
        let mut result = loop {
            tokio::select! {
                result = &mut step => break result?,
                Some(event) = rx.recv() => {
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
                                    value(7),
                                    ElicitationAction::Accept,
                                ))
                                .agent_only();
                            crate::elicitation::complete_elicitation_with_message(
                                &pipeline.session_manager,
                                &pipeline.session_id,
                                &id,
                                ElicitationOutcome::Accept(value(7)),
                                &response_message,
                            )
                            .await?;
                            answered_id = Some(id);
                        }
                    }
                }
            }
        };
        let Some(ref mut result) = result else {
            break;
        };
        machine
            .apply(pipeline.session_manager.as_ref(), &session, result, &emit)
            .await?;
        if result.yield_to_client {
            break;
        }
    }

    assert!(answered_id.is_some());

    let session = pipeline.session().await?;
    let conversation = session.conversation.unwrap_or_default();
    let tool_response = conversation
        .messages()
        .iter()
        .find(|m| m.is_tool_response())
        .expect("a tool response");
    assert!(
        tool_response_text(tool_response).contains("result: 7"),
        "tool response: {tool_response:#?}"
    );
    assert_eq!(conversation.last().unwrap().as_concat_text(), "thanks");
    assert_eq!(pipeline.calculator_total(), 7);
    assert_eq!(api.call_count(), 2);

    let request_position = conversation.messages().iter().position(|m| {
        m.content.iter().any(|c| {
            matches!(
                c,
                MessageContent::ActionRequired(action)
                    if matches!(action.data, ActionRequiredData::Elicitation { .. })
            )
        })
    });
    let response_position = conversation.messages().iter().position(|m| {
        m.content.iter().any(|c| {
            matches!(
                c,
                MessageContent::ActionRequired(action)
                    if matches!(action.data, ActionRequiredData::ElicitationResponse { .. })
            )
        })
    });
    assert!(request_position.is_some());
    assert!(response_position.is_some());

    Ok(())
}

#[tokio::test]
async fn compacts_when_over_token_threshold() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("fill the context").reply("filled");
    api.on("Please summarize the conversation history")
        .reply("summary");
    api.on("Your context was compacted").reply("ok");

    let context = format!(
        "fill the context {}",
        "x".repeat(pipeline.context_limit() * 78 / 100)
    );
    let filled = pipeline.run([&context]).await?;
    let filled_usage = filled.session.usage.total_tokens.unwrap();

    let result = pipeline.run(["hello"]).await?;

    let replaced = result
        .events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1);

    use crate::conversation::message::MessageContent;
    let saw_notice = result.events.iter().any(|e| {
        match e {
        AgentEvent::Message(m) => m.content.iter().any(|c| {
            matches!(c, MessageContent::SystemNotification(s) if s.msg.contains("auto-compaction"))
        }),
        _ => false,
    }
    });
    assert!(saw_notice);

    assert_eq!(api.call_count(), 3);
    assert!(result
        .session
        .usage
        .total_tokens
        .is_some_and(|tokens| tokens < filled_usage));
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: ok")
    );

    Ok(())
}

#[tokio::test]
async fn replacing_conversation_recalculates_context_usage() -> Result<()> {
    let (pipeline, _api) = test_pipeline().await?;
    pipeline.set_total_tokens(100).await;

    let conversation = Conversation::new_unvalidated(vec![
        Message::user().with_text("keep this"),
        Message::assistant().with_text("and this"),
    ]);
    let token_counter = crate::token_counter::create_token_counter()
        .await
        .map_err(anyhow::Error::msg)?;
    let expected = conversation
        .messages()
        .iter()
        .map(|message| token_counter.count_chat_tokens("", std::slice::from_ref(message), &[]))
        .sum::<usize>() as i32;
    let session = pipeline.session().await?;
    let mut result = state_machine::StepResult {
        effects: vec![conversation.into()],
        yield_to_client: false,
    };
    let machine =
        state_machine::StateMachine::new(Vec::new(), tokio_util::sync::CancellationToken::new());
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let emit = state_machine::Emitter::new(tx, tokio_util::sync::CancellationToken::new());

    machine
        .apply(
            pipeline.session_manager.as_ref(),
            &session,
            &mut result,
            &emit,
        )
        .await?;

    assert_eq!(pipeline.session().await?.usage.total_tokens, Some(expected));
    Ok(())
}

#[tokio::test]
async fn response_usage_survives_a_conversation_replacement_in_the_same_step() -> Result<()> {
    use goose_providers::conversation::token_usage::{ProviderUsage, Usage as ProviderTokenUsage};

    let (pipeline, _api) = test_pipeline().await?;
    pipeline.set_total_tokens(100).await;

    let replacement = Conversation::new_unvalidated(vec![Message::user().with_text("new context")]);
    let response = Message::assistant().with_text("response after replacement");
    let usage = ProviderUsage::new(
        "scripted-model".to_string(),
        ProviderTokenUsage::new(Some(10), Some(5), Some(15)),
    );
    let session = pipeline.session().await?;
    let mut result = state_machine::StepResult {
        effects: vec![
            replacement.into(),
            state_machine::StateEffect::RecordUsage(usage),
            response.into(),
        ],
        yield_to_client: false,
    };
    let machine =
        state_machine::StateMachine::new(Vec::new(), tokio_util::sync::CancellationToken::new());
    let (tx, _rx) = tokio::sync::mpsc::channel(4);
    let emit = state_machine::Emitter::new(tx, tokio_util::sync::CancellationToken::new());

    machine
        .apply(
            pipeline.session_manager.as_ref(),
            &session,
            &mut result,
            &emit,
        )
        .await?;

    let reloaded = pipeline.session().await?;
    assert_eq!(reloaded.usage.total_tokens, Some(15));
    assert_eq!(
        reloaded
            .conversation
            .and_then(|conversation| conversation.last().cloned())
            .and_then(|message| message.metadata.usage)
            .and_then(|usage| usage.total_tokens),
        Some(15)
    );
    Ok(())
}

#[tokio::test]
async fn compaction_operation_contributes_remaining_context_to_moim() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("fill the context").reply("filled");
    api.on("hello").reply("ok");

    let context = format!(
        "fill the context {}",
        "x".repeat(pipeline.context_limit() / 2)
    );
    pipeline.run([&context]).await?;
    pipeline.run(["hello"]).await?;

    assert_eq!(api.call_count(), 2);
    assert!(api.calls()[1].input_contains("<compaction>"));
    Ok(())
}

#[tokio::test]
async fn llm_turn_records_usage_on_session_and_message() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("hello").reply("hi there");

    let result = pipeline.run(["hello"]).await?;
    let input_tokens = api.calls()[0].input_tokens();
    let output_tokens = "hi there".chars().count() as i32;
    let total_tokens = input_tokens + output_tokens;

    assert_eq!(result.session.usage.total_tokens, Some(total_tokens));
    assert_eq!(result.session.usage.input_tokens, Some(input_tokens));
    assert_eq!(result.session.usage.output_tokens, Some(output_tokens));

    assert!(
        result.events.iter().any(
            |e| matches!(e, AgentEvent::Usage(u) if u.usage.total_tokens == Some(total_tokens))
        ),
        "events: {:#?}",
        result.events
    );
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, AgentEvent::MessageUsage { .. })),
        "events: {:#?}",
        result.events
    );

    let conversation = result.session.conversation.unwrap_or_default();
    let assistant = conversation
        .messages()
        .iter()
        .find(|m| m.role == Role::Assistant)
        .expect("an assistant message");
    let usage = assistant.metadata.usage.as_ref().expect("message usage");
    assert_eq!(usage.total_tokens, Some(total_tokens));

    Ok(())
}

#[tokio::test]
async fn usage_before_a_stream_error_is_recorded() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("hello")
        .reply("partial response")
        .server_error("boom");

    let result = pipeline.run(["hello"]).await?;
    let total_tokens = api.calls()[0].input_tokens() + "partial response".chars().count() as i32;

    assert_eq!(result.session.usage.total_tokens, Some(total_tokens));
    assert!(result.events.iter().any(
        |event| matches!(event, AgentEvent::Usage(usage) if usage.usage.total_tokens == Some(total_tokens))
    ));
    assert!(!result
        .events
        .iter()
        .any(|event| matches!(event, AgentEvent::MessageUsage { .. })));

    assert!(!result
        .rendered_conversation()
        .iter()
        .any(|line| line == "agent: partial response"));
    let conversation = result.session.conversation.unwrap_or_default();
    let error = conversation
        .messages()
        .iter()
        .find(|message| message.error_kind().is_some())
        .expect("a persisted error message");
    assert!(error.metadata.usage.is_none());

    Ok(())
}

#[tokio::test]
async fn provider_error_is_persisted_and_yields() -> Result<()> {
    use crate::conversation::message::MessageErrorKind;

    let (pipeline, api) = test_pipeline().await?;
    api.on("hello").server_error("boom");

    let result = pipeline.run(["hello"]).await?;
    assert!(result
        .rendered_conversation()
        .iter()
        .any(|line| line.starts_with("error:") && line.contains("boom")));
    let conversation = result.session.conversation.unwrap_or_default();
    let error = conversation
        .messages()
        .iter()
        .find(|m| m.error_kind() == Some(MessageErrorKind::Other))
        .expect("a persisted error message");
    assert!(error.is_user_visible());
    assert!(!error.is_agent_visible());

    assert_eq!(api.call_count(), 1);

    Ok(())
}

#[tokio::test]
async fn slash_command_yields_without_calling_provider() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;

    let result = pipeline.run(["/status"]).await?;

    assert_eq!(api.call_count(), 0);
    let rendered = result.rendered_conversation();
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("agent:") && line.contains("Provider:")));
    let conversation = result.session.conversation.unwrap_or_default();
    assert!(conversation.messages().iter().all(|m| m.is_user_visible()));
    assert!(conversation
        .messages()
        .iter()
        .all(|m| !m.is_agent_visible()));
    Ok(())
}

#[tokio::test]
async fn unknown_slash_text_falls_through_to_provider() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("/not-a-command").reply("saw it");

    let result = pipeline.run(["/not-a-command"]).await?;

    assert_eq!(api.call_count(), 1);
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: saw it")
    );
    let conversation = result.session.conversation.unwrap_or_default();
    let command = conversation
        .messages()
        .iter()
        .find(|message| message.as_concat_text() == "/not-a-command")
        .expect("persisted user message");
    assert!(command.is_user_visible());
    assert!(command.is_agent_visible());

    Ok(())
}

#[tokio::test]
async fn goal_slash_command_starts_turn_with_hidden_kickoff() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("Start working toward this goal now")
        .reply("working on it");
    api.on("fully met").reply("all done");

    let result = pipeline.run(["/goal finish the migration"]).await?;

    assert_eq!(api.call_count(), 2);
    assert!(pipeline.get_goal().await.is_none());

    let rendered = result.rendered_conversation();
    assert_eq!(rendered.last().map(String::as_str), Some("agent: all done"));
    let conversation = result.session.conversation.unwrap_or_default();
    let persisted = conversation.messages();
    let command = persisted
        .iter()
        .find(|message| message.as_concat_text() == "/goal finish the migration")
        .expect("persisted goal command");
    assert!(command.is_user_visible());
    assert!(!command.is_agent_visible());
    persisted
        .iter()
        .find(|message| {
            message.as_concat_text().contains("finish the migration")
                && !message.is_user_visible()
                && message.is_agent_visible()
        })
        .expect("hidden goal kickoff");

    Ok(())
}

#[tokio::test]
async fn history_slash_command_replaces_history_and_yields() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    pipeline.set_total_tokens(100).await;

    let result = pipeline.run(["/clear"]).await?;

    assert_eq!(api.call_count(), 0);
    let replaced = result
        .events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1);

    let conversation = result.session.conversation.unwrap_or_default();
    let persisted = conversation.messages();
    assert_eq!(persisted.len(), 2);
    assert!(persisted.iter().all(|m| m.is_user_visible()));
    assert!(persisted.iter().all(|m| !m.is_agent_visible()));
    assert_eq!(result.session.usage.total_tokens, Some(0));

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
    let (pipeline, api) = test_pipeline().await?;
    api.on("Review src/lib.rs carefully.").reply("reviewed");
    pipeline
        .session_manager
        .update(&pipeline.session_id)
        .working_dir(working_dir.path().to_path_buf())
        .apply()
        .await?;

    let result = pipeline.run(["/review src/lib.rs"]).await?;

    assert_eq!(api.call_count(), 1);
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: reviewed")
    );
    let conversation = result.session.conversation.unwrap_or_default();
    let persisted = conversation.messages();
    let command = persisted
        .iter()
        .find(|message| message.as_concat_text() == "/review src/lib.rs")
        .expect("persisted skill command");
    assert!(!command.is_agent_visible());
    let skill = persisted
        .iter()
        .find(|message| {
            message
                .as_concat_text()
                .contains("Review src/lib.rs carefully.")
        })
        .expect("injected skill");
    assert!(skill.is_agent_visible());
    assert!(!skill.is_user_visible());

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
    let (pipeline, api) = test_pipeline().await?;
    api.on("use the review skill")
        .call("load_skill", json!({ "name": "review" }));
    api.on("Review carefully.").reply("done");
    pipeline
        .session_manager
        .update(&pipeline.session_id)
        .working_dir(working_dir.path().to_path_buf())
        .apply()
        .await?;

    let result = pipeline.run(["use the review skill"]).await?;

    assert_eq!(api.call_count(), 2);
    let rendered = result.rendered_conversation();
    assert!(rendered
        .iter()
        .any(|line| line.starts_with("toolresponse:") && line.contains("Review carefully.")));
    assert_eq!(rendered.last().map(String::as_str), Some("agent: done"));

    Ok(())
}

#[tokio::test]
async fn repeated_context_length_errors_stop_after_capped_retries() -> Result<()> {
    use crate::conversation::message::MessageErrorKind;

    let (pipeline, api) = test_pipeline().await?;
    api.on("hello").context_limit_error("too long");
    api.on("Please summarize the conversation history")
        .reply("summary");
    api.on("Your context was compacted")
        .context_limit_error("too long");

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), pipeline.run(["hello"]))
        .await
        .expect("retry cap did not stop the compact/retry cycle")?;

    assert_eq!(api.call_count(), 5);

    let conversation = result.session.conversation.unwrap_or_default();
    let last = conversation.last().expect("a persisted message");
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

    let (pipeline, api) = test_pipeline().await?;
    api.on("hello").context_limit_error("too long");
    api.on("Please summarize the conversation history")
        .reply("summary");
    api.on("Your context was compacted").call(ADD, value(1));
    api.on("result:").context_limit_error("too long");

    let result = pipeline.run(["hello"]).await?;

    let replaced = result
        .events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 2);
    assert_eq!(
        api.call_count(),
        7,
        "events: {:#?}\nconversation: {:#?}",
        result.events,
        result.session.conversation
    );

    let conversation = result.session.conversation.unwrap_or_default();
    let last = conversation.last().expect("a persisted message");
    assert_eq!(
        last.error_kind(),
        Some(MessageErrorKind::ContextLengthExceeded),
        "tail: {last:?}"
    );

    Ok(())
}

#[tokio::test]
async fn context_length_error_triggers_compaction_recovery() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("Please summarize the conversation history")
        .reply("summary");
    api.on("Your context was compacted").reply("recovered");
    pipeline
        .session_manager
        .add_message(
            &pipeline.session_id,
            &Message::user().with_text(format!(
                "old context {}",
                "x".repeat(pipeline.context_limit() * 96 / 100)
            )),
        )
        .await?;
    pipeline
        .session_manager
        .add_message(
            &pipeline.session_id,
            &Message::assistant().with_text("old response"),
        )
        .await?;

    let result = pipeline.run(["hello"]).await?;

    let replaced = result
        .events
        .iter()
        .filter(|e| matches!(e, AgentEvent::HistoryReplaced(_)))
        .count();
    assert_eq!(replaced, 1, "events: {:#?}", result.events);
    assert_eq!(
        result.rendered_conversation().last().map(String::as_str),
        Some("agent: recovered")
    );

    let conversation = result.session.conversation.unwrap_or_default();
    let persisted = conversation.messages();
    let last = conversation.last().expect("a persisted message");
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
    assert_eq!(api.call_count(), 3);

    Ok(())
}
