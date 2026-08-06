use anyhow::Result;

use super::calculator_extension::{value, ADD};
use super::pipeline::{test_pipeline, MessageKind::Agent, MessageKind::ToolResponse, MAX_TURNS};
use crate::agents::state_machine::ops_stop_hook::DENIED;
use crate::conversation::message::{MessageContent, SystemNotificationType};

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
async fn stop_hooks_allow_block_and_skip_non_stop_exits() -> Result<()> {
    let allowed = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(allowed.hook_manager());
    api.on("hello").reply("done");

    let result = pipeline.run(["hello"]).await?;
    result.assert_message(-1, Agent, "done");
    assert_eq!(api.call_count(), 1);
    assert_eq!(allowed.invocations(), 1);

    let blocked = HookTestEnv::new("Stop", LOG_AND_BLOCK_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline
        .with_hook_manager(blocked.hook_manager())
        .with_stop_hook_block_cap(2);
    api.on("hello").reply("response");
    api.on("blocked ending this turn").reply("response");

    let (_, result, _) = pipeline.run_reconstructing_each_step("hello").await?;
    assert_eq!(api.call_count(), 3);
    assert_eq!(blocked.invocations(), 3);
    assert_eq!(
        result
            .conversation()
            .messages()
            .iter()
            .filter(|message| message
                .as_concat_text()
                .contains("blocked ending this turn"))
            .count(),
        2
    );
    assert_eq!(
        result
            .conversation()
            .messages()
            .iter()
            .filter(|message| message
                .metadata
                .operation_note("stop_hook", DENIED)
                .is_some())
            .count(),
        2
    );
    assert!(result.conversation().last().is_some_and(|message| {
        message.content.iter().any(|content| {
            matches!(
                content,
                MessageContent::SystemNotification(notification)
                    if notification.notification_type == SystemNotificationType::InlineMessage
                        && notification.msg.contains("GOOSE_STOP_HOOK_BLOCK_CAP")
            )
        })
    }));

    let maxed = HookTestEnv::new("Stop", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(maxed.hook_manager());
    api.on("keep going").call(ADD, value(1));

    pipeline.run(["keep going"]).await?;
    assert_eq!(api.call_count(), MAX_TURNS as usize);
    assert_eq!(pipeline.calculator_total(), MAX_TURNS as i64 - 1);
    assert_eq!(maxed.invocations(), 0);

    Ok(())
}

#[tokio::test]
async fn session_prompt_and_tool_hooks_fire_at_their_boundaries() -> Result<()> {
    let session_start = HookTestEnv::new("SessionStart", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(session_start.hook_manager());
    api.on("first").reply("ok");
    api.on("second").reply("ok");

    pipeline.run(["/status"]).await?;
    assert_eq!(session_start.invocations(), 1);
    pipeline.run(["first", "second"]).await?;
    assert_eq!(session_start.invocations(), 1);
    assert_eq!(api.call_count(), 2);

    let prompt_submit = HookTestEnv::new("UserPromptSubmit", LOG_AND_ALLOW_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(prompt_submit.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("result: 1").reply("done");

    pipeline.run(["/status"]).await?;
    assert_eq!(prompt_submit.invocations(), 1);
    let (_, result, _) = pipeline.run_reconstructing_each_step("add one").await?;
    result.assert_message(-1, Agent, "done");
    assert_eq!(api.call_count(), 2);
    assert_eq!(prompt_submit.invocations(), 2);

    let pre_tool = HookTestEnv::new("PreToolUse", LOG_AND_BLOCK_SCRIPT);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(pre_tool.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("denied by policy hook").reply("understood");

    let result = pipeline.run(["add one"]).await?;
    result.assert_message(-2, ToolResponse, "denied by policy hook");
    result.assert_message(-1, Agent, "understood");
    assert_eq!(pre_tool.invocations(), 1);
    assert_eq!(pipeline.calculator_total(), 0);

    Ok(())
}
