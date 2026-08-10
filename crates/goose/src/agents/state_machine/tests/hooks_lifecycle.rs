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

/// Plugin fixture that can register several events at once, each with its own
/// matcher and script, and read back the JSON payloads a script recorded.
struct RecordingHookEnv {
    _temp_dir: tempfile::TempDir,
    plugin_dir: std::path::PathBuf,
}

/// (event name, matcher or "" for none, script file name, script body)
type HookSpec<'a> = (&'a str, &'a str, &'a str, &'a str);

impl RecordingHookEnv {
    fn new(specs: &[HookSpec<'_>]) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let plugin_dir = temp_dir.path().join("test-plugin");
        std::fs::create_dir_all(plugin_dir.join("hooks")).unwrap();
        let entries: Vec<String> = specs
            .iter()
            .map(|(event, matcher, script, _)| {
                let matcher = if matcher.is_empty() {
                    String::new()
                } else {
                    format!(r#""matcher": "{matcher}", "#)
                };
                format!(
                    r#""{event}": [{{{matcher}"hooks": [{{"type": "command", "command": "sh ${{PLUGIN_ROOT}}/{script}"}}]}}]"#
                )
            })
            .collect();
        std::fs::write(
            plugin_dir.join("hooks/hooks.json"),
            format!(r#"{{"hooks": {{{}}}}}"#, entries.join(", ")),
        )
        .unwrap();
        for (_, _, script, body) in specs {
            std::fs::write(plugin_dir.join(script), body).unwrap();
        }
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

    fn payloads(&self, log: &str) -> Vec<serde_json::Value> {
        std::fs::read_to_string(self.plugin_dir.join(log))
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

const RECORD_PRE_SCRIPT: &str =
    "#!/bin/sh\ncat >> \"$PLUGIN_ROOT/pre.log\"\nprintf '\\n' >> \"$PLUGIN_ROOT/pre.log\"\nexit 0\n";
const RECORD_RESULT_SCRIPT: &str =
    "#!/bin/sh\ncat >> \"$PLUGIN_ROOT/result.log\"\nprintf '\\n' >> \"$PLUGIN_ROOT/result.log\"\nexit 0\n";
const RECORD_POST_SCRIPT: &str =
    "#!/bin/sh\ncat >> \"$PLUGIN_ROOT/post.log\"\nprintf '\\n' >> \"$PLUGIN_ROOT/post.log\"\nexit 0\n";
const RECORD_POST_FAILURE_SCRIPT: &str =
    "#!/bin/sh\ncat >> \"$PLUGIN_ROOT/postfail.log\"\nprintf '\\n' >> \"$PLUGIN_ROOT/postfail.log\"\nexit 0\n";
const DENY_AND_RECORD_SCRIPT: &str =
    "#!/bin/sh\ncat >> \"$PLUGIN_ROOT/pre.log\"\nprintf '\\n' >> \"$PLUGIN_ROOT/pre.log\"\necho \"blocked by test policy\" >&2\nexit 2\n";

/// deny-invisible: the tool never dispatches, neither post event fires, and a
/// PreToolUseResult subscriber still sees the denial with blocked_by and reason.
#[tokio::test]
async fn pre_tool_use_result_observes_denial_that_post_hooks_never_see() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", DENY_AND_RECORD_SCRIPT),
        ("PreToolUseResult", "", "result.sh", RECORD_RESULT_SCRIPT),
        ("PostToolUse", "", "post.sh", RECORD_POST_SCRIPT),
        (
            "PostToolUseFailure",
            "",
            "postfail.sh",
            RECORD_POST_FAILURE_SCRIPT,
        ),
    ]);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("denied by policy hook").reply("understood");

    pipeline.run(["add one"]).await?;

    assert_eq!(pipeline.calculator_total(), 0, "tool must not dispatch");
    assert!(
        env.payloads("post.log").is_empty(),
        "PostToolUse must not fire for a denied call"
    );
    assert!(
        env.payloads("postfail.log").is_empty(),
        "PostToolUseFailure must not fire for a denied call"
    );

    let results = env.payloads("result.log");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["event"], "PreToolUseResult");
    assert_eq!(results[0]["decision"], "deny");
    assert_eq!(results[0]["policy_evaluated"], true);
    assert_eq!(results[0]["blocked_by"], "test-plugin");
    assert_eq!(results[0]["reason"], "blocked by test policy");
    assert!(results[0]["tool_call_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    Ok(())
}

/// repeated identical calls: two calls with the same name and input in one
/// session correlate to their outcomes by tool_call_id, not by name plus input.
#[tokio::test]
async fn repeated_identical_calls_correlate_by_tool_call_id() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
        ("PreToolUseResult", "", "result.sh", RECORD_RESULT_SCRIPT),
        ("PostToolUse", "", "post.sh", RECORD_POST_SCRIPT),
    ]);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("result: 1").call(ADD, value(1));
    api.on("result: 2").reply("done");

    pipeline.run(["add one"]).await?;
    assert_eq!(pipeline.calculator_total(), 2);

    let pres = env.payloads("pre.log");
    let results = env.payloads("result.log");
    let posts = env.payloads("post.log");
    assert_eq!(pres.len(), 2);
    assert_eq!(results.len(), 2);
    assert_eq!(posts.len(), 2);

    for payloads in [&pres, &results, &posts] {
        assert_eq!(payloads[0]["tool_name"], payloads[1]["tool_name"]);
        assert_eq!(payloads[0]["tool_input"], payloads[1]["tool_input"]);
    }

    let ids: Vec<&str> = results
        .iter()
        .map(|payload| payload["tool_call_id"].as_str().unwrap())
        .collect();
    assert_ne!(
        ids[0], ids[1],
        "identical name and input must still carry distinct ids"
    );

    for (index, id) in ids.iter().enumerate() {
        assert_eq!(
            pres[index]["tool_call_id"], results[index]["tool_call_id"],
            "PreToolUse and PreToolUseResult must carry one id per call"
        );
        assert_eq!(
            posts
                .iter()
                .filter(|payload| payload["tool_call_id"] == *id)
                .count(),
            1,
            "each call must pair with exactly one outcome by id"
        );
    }
    Ok(())
}

/// no matching hook: a PreToolUse rule is registered but its matcher does not
/// match, so nothing runs and the event reports allow with policy_evaluated false.
#[tokio::test]
async fn pre_tool_use_result_reports_allow_and_unevaluated_when_no_hook_matches() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        (
            "PreToolUse",
            "a_tool_name_that_never_matches",
            "pre.sh",
            DENY_AND_RECORD_SCRIPT,
        ),
        ("PreToolUseResult", "", "result.sh", RECORD_RESULT_SCRIPT),
    ]);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("add one").call(ADD, value(1));
    api.on("result: 1").reply("done");

    pipeline.run(["add one"]).await?;
    assert_eq!(pipeline.calculator_total(), 1, "tool must still run");

    assert!(
        env.payloads("pre.log").is_empty(),
        "the non-matching rule must not run"
    );
    let results = env.payloads("result.log");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["decision"], "allow");
    assert_eq!(results[0]["policy_evaluated"], false);
    assert!(results[0].get("blocked_by").is_none());
    assert!(results[0].get("reason").is_none());
    Ok(())
}
