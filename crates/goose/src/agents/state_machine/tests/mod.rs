use anyhow::Result;
use rmcp::model::Role;
use serde_json::json;

use self::calculator_extension::{value, ADD};
use self::pipeline::{test_pipeline, MAX_TURNS};
use crate::agents::{state_machine, AgentEvent};
use crate::conversation::message::{Message, MessageContent};

mod calculator_extension;
mod compaction_lifecycle;
mod dummy_api;
mod pipeline;
mod provider_lifecycle;
mod recipe_scheduling_lifecycle;
mod reconstruction_isolation_lifecycle;
mod steering_lifecycle;
mod tool_lifecycle;

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
async fn replacing_the_conversation_recounts_usage_unless_the_turn_reports_it() -> Result<()> {
    use goose_providers::conversation::token_usage::{ProviderUsage, Usage as ProviderTokenUsage};

    use crate::conversation::Conversation;

    let (pipeline, _api) = test_pipeline().await?;
    pipeline.set_total_tokens(100).await;

    let machine =
        state_machine::StateMachine::new(Vec::new(), tokio_util::sync::CancellationToken::new());
    let (tx, _rx) = tokio::sync::mpsc::channel(4);
    let emit = state_machine::Emitter::new(tx, tokio_util::sync::CancellationToken::new());

    let apply = async |effects: Vec<state_machine::StateEffect>| -> Result<()> {
        let session = pipeline.session().await?;
        let mut result = state_machine::StepResult {
            effects,
            yield_to_client: false,
        };
        machine
            .apply(
                pipeline.session_manager.as_ref(),
                &session,
                &mut result,
                &emit,
            )
            .await
    };

    let replacement = Conversation::new_unvalidated(vec![
        Message::user().with_text("keep this"),
        Message::assistant().with_text("and this"),
    ]);
    apply(vec![replacement.into()]).await?;
    let recounted = pipeline.session().await?.usage.total_tokens;
    assert!(recounted.is_some_and(|tokens| tokens > 0 && tokens < 100));

    let replacement = Conversation::new_unvalidated(vec![Message::user().with_text("new context")]);
    let usage = ProviderUsage::new(
        "scripted-model".to_string(),
        ProviderTokenUsage::new(Some(10), Some(5), Some(15)),
    );
    apply(vec![
        replacement.into(),
        state_machine::StateEffect::RecordUsage(usage),
        Message::assistant()
            .with_text("response after replacement")
            .into(),
    ])
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
async fn prompt_and_skill_lifecycle() -> Result<()> {
    use self::pipeline::MessageKind::{Agent, ToolResponse, User};

    let (pipeline, api) = test_pipeline().await?;
    std::fs::write(
        pipeline.working_dir().join("AGENTS.md"),
        "ROOT_PROJECT_INSTRUCTION",
    )?;

    api.on("first turn").reply("first reply");
    pipeline.run(["first turn"]).await?;
    assert!(api.calls()[0].system_contains("ROOT_PROJECT_INSTRUCTION"));
    assert!(!api.calls()[0].system_contains("HOT_SKILL_INSTRUCTION"));

    let skill_dir = pipeline.working_dir().join(".agents/skills/review");
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review newly added code\n---\n\
         HOT_SKILL_INSTRUCTION: Review $ARGUMENTS carefully.",
    )?;
    std::fs::write(skill_dir.join("guide.txt"), "SUPPORTING_FILE_CONTENT")?;

    let provider_calls = api.call_count();
    let listed = pipeline.run(["/skills"]).await?;
    assert_eq!(api.call_count(), provider_calls);
    listed.assert_message(-1, Agent, "review");

    api.on("use the new skill")
        .call("load_skill", json!({ "name": "review" }));
    api.on("HOT_SKILL_INSTRUCTION")
        .call("load_skill", json!({ "name": "review/guide.txt" }));
    api.on("SUPPORTING_FILE_CONTENT").reply("skill loaded");

    let result = pipeline.run(["use the new skill"]).await?;
    let calls = api.calls();
    assert!(calls[provider_calls].system_contains("Review newly added code"));
    assert!(!calls[provider_calls].system_contains("HOT_SKILL_INSTRUCTION"));
    result.assert_message(-4, ToolResponse, "HOT_SKILL_INSTRUCTION");
    result.assert_message(-2, ToolResponse, "SUPPORTING_FILE_CONTENT");
    result.assert_message(-1, Agent, "skill loaded");

    api.on("Review src/lib.rs carefully.").reply("reviewed");
    let result = pipeline.run(["/review src/lib.rs"]).await?;
    result.assert_message(-1, Agent, "reviewed");

    let provider_calls = api.call_count();
    let result = pipeline.run(["/prompts calculator"]).await?;
    assert_eq!(api.call_count(), provider_calls);
    result.assert_message(-1, Agent, "explain_addition");

    api.on("Why?").reply("Two pairs contain four items.");
    let result = pipeline.run(["/prompt explain_addition"]).await?;
    result.assert_message(-4, User, "What is two plus two?");
    result.assert_message(-3, Agent, "Four.");
    result.assert_message(-2, User, "Why?");
    result.assert_message(-1, Agent, "Two pairs contain four items.");
    let call = api.calls().last().cloned().expect("provider request");
    assert_eq!(call.input_occurrences("What is two plus two?"), 1);
    assert_eq!(call.input_occurrences("Four."), 1);
    assert_eq!(call.input_occurrences("Why?"), 1);

    Ok(())
}
