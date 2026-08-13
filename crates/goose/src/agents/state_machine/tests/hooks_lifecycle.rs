use anyhow::Result;

use super::calculator_extension::{value, ADD};
use super::pipeline::{test_pipeline, MessageKind::Agent, MessageKind::ToolResponse, MAX_TURNS};
use crate::agents::final_output_tool::FINAL_OUTPUT_TOOL_NAME;
use crate::agents::state_machine::ops_stop_hook::DENIED;
use crate::agents::tool_execution::{CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};
use crate::config::permission::PermissionLevel;
use crate::config::GooseMode;
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

/// Builds a recipe whose structured response forces the model through
/// `recipe__final_output`, which `RecipeOperation` executes itself rather than
/// handing to `ToolExecutionOperation`.
fn final_output_recipe() -> crate::recipe::Recipe {
    crate::recipe::Recipe::builder()
        .title("Hook parity recipe")
        .description("Exercises the final-output hook lifecycle")
        .instructions("Return a structured answer")
        .response(crate::recipe::Response {
            json_schema: Some(serde_json::json!({
                "type": "object",
                "properties": { "answer": { "type": "string" } },
                "required": ["answer"]
            })),
        })
        .build()
        .expect("valid recipe")
}

/// recipe final-output parity: the call `RecipeOperation` executes directly still
/// emits `PreToolUse` and `PreToolUseResult`, correlated by one `tool_call_id`.
#[tokio::test]
async fn recipe_final_output_emits_pre_tool_use_and_result_with_matching_id() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
        ("PreToolUseResult", "", "result.sh", RECORD_RESULT_SCRIPT),
    ]);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    pipeline.set_recipe(final_output_recipe()).await?;
    api.on("produce the answer").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "answer": "done" }),
    );

    pipeline.run(["produce the answer"]).await?;

    let pres = env.payloads("pre.log");
    let results = env.payloads("result.log");
    assert_eq!(
        pres.len(),
        1,
        "PreToolUse must fire for recipe final output"
    );
    assert_eq!(
        results.len(),
        1,
        "PreToolUseResult must fire for recipe final output"
    );
    assert_eq!(pres[0]["tool_name"], FINAL_OUTPUT_TOOL_NAME);
    assert_eq!(results[0]["tool_name"], FINAL_OUTPUT_TOOL_NAME);
    assert_eq!(results[0]["event"], "PreToolUseResult");
    assert_eq!(results[0]["decision"], "allow");
    assert_eq!(
        pres[0]["tool_call_id"], results[0]["tool_call_id"],
        "PreToolUse and PreToolUseResult must carry the same tool_call_id"
    );
    assert!(pres[0]["tool_call_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    Ok(())
}

/// recipe final-output parity: the post-tool event fires once the call completes,
/// carrying the id the pre events carried.
#[tokio::test]
async fn recipe_final_output_emits_post_tool_event() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
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
    pipeline.set_recipe(final_output_recipe()).await?;
    api.on("produce the answer").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "answer": "done" }),
    );

    pipeline.run(["produce the answer"]).await?;

    let pres = env.payloads("pre.log");
    let posts = env.payloads("post.log");
    assert_eq!(pres.len(), 1);
    assert_eq!(
        posts.len(),
        1,
        "PostToolUse must fire for a successful recipe final output"
    );
    assert_eq!(posts[0]["event"], "PostToolUse");
    assert_eq!(posts[0]["tool_name"], FINAL_OUTPUT_TOOL_NAME);
    assert_eq!(
        posts[0]["tool_call_id"], pres[0]["tool_call_id"],
        "the post event must carry the same tool_call_id as the pre events"
    );
    Ok(())
}

/// recipe final-output parity: a denying hook stops the call. The final-output
/// tool never runs, so the recipe never reports a successful structured answer,
/// and no post event fires — the same shape a denied ordinary tool call has.
#[tokio::test]
async fn recipe_final_output_denied_by_hook_does_not_execute() -> Result<()> {
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
    pipeline.set_recipe(final_output_recipe()).await?;
    api.on("produce the answer").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "answer": "done" }),
    );
    api.on("denied by policy hook").reply("understood");
    // Denied, so the recipe never gets its structured answer and re-prompts to
    // the turn cap. Answer the compaction request that cap triggers, otherwise
    // the dummy API panics on an unmatched rule and buries the real assertions.
    api.on("Please summarize the conversation history")
        .reply("summary");

    let result = pipeline.run(["produce the answer"]).await?;

    let results = env.payloads("result.log");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["decision"], "deny");
    assert_eq!(results[0]["blocked_by"], "test-plugin");
    assert_eq!(results[0]["tool_name"], FINAL_OUTPUT_TOOL_NAME);

    assert!(
        env.payloads("post.log").is_empty(),
        "PostToolUse must not fire for a denied final-output call"
    );
    assert!(
        env.payloads("postfail.log").is_empty(),
        "PostToolUseFailure must not fire for a denied final-output call"
    );

    let produced_answer = result
        .conversation()
        .messages()
        .iter()
        .any(|message| message.as_concat_text().contains("\"answer\""));
    assert!(
        !produced_answer,
        "a denied final-output call must not execute the tool"
    );
    Ok(())
}

/// Writes a skill `SkillOperation` can load, and returns the tool arguments that
/// load it. `load_skill` is executed by `SkillOperation`, which is registered
/// ahead of `ToolExecutionOperation`, so it never reaches the hook wrapper.
fn install_skill(working_dir: &std::path::Path) -> serde_json::Value {
    let skill_dir = working_dir.join(".agents/skills/review");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review helper\n---\nSKILL_BODY_CONTENT\n",
    )
    .expect("skill file");
    serde_json::json!({ "name": "review" })
}

/// load_skill parity: the call `SkillOperation` executes directly still emits
/// `PreToolUse` and `PreToolUseResult`, correlated by one `tool_call_id`.
#[tokio::test]
async fn load_skill_emits_pre_tool_use_and_result_with_matching_id() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
        ("PreToolUseResult", "", "result.sh", RECORD_RESULT_SCRIPT),
    ]);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    let arguments = install_skill(pipeline.working_dir());
    api.on("use the skill").call("load_skill", arguments);
    api.on("SKILL_BODY_CONTENT").reply("skill loaded");

    pipeline.run(["use the skill"]).await?;

    let pres = env.payloads("pre.log");
    let results = env.payloads("result.log");
    assert_eq!(pres.len(), 1, "PreToolUse must fire for load_skill");
    assert_eq!(
        results.len(),
        1,
        "PreToolUseResult must fire for load_skill"
    );
    assert_eq!(pres[0]["tool_name"], "load_skill");
    assert_eq!(results[0]["tool_name"], "load_skill");
    assert_eq!(results[0]["event"], "PreToolUseResult");
    assert_eq!(results[0]["decision"], "allow");
    assert_eq!(
        pres[0]["tool_call_id"], results[0]["tool_call_id"],
        "PreToolUse and PreToolUseResult must carry the same tool_call_id"
    );
    assert!(pres[0]["tool_call_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    Ok(())
}

/// load_skill parity: the post-tool event fires once the skill load completes,
/// carrying the id the pre events carried.
#[tokio::test]
async fn load_skill_emits_post_tool_event() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
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
    let arguments = install_skill(pipeline.working_dir());
    api.on("use the skill").call("load_skill", arguments);
    api.on("SKILL_BODY_CONTENT").reply("skill loaded");

    pipeline.run(["use the skill"]).await?;

    let pres = env.payloads("pre.log");
    let posts = env.payloads("post.log");
    assert_eq!(pres.len(), 1);
    assert_eq!(
        posts.len(),
        1,
        "PostToolUse must fire for a successful load_skill"
    );
    assert_eq!(posts[0]["event"], "PostToolUse");
    assert_eq!(posts[0]["tool_name"], "load_skill");
    assert_eq!(
        posts[0]["tool_call_id"], pres[0]["tool_call_id"],
        "the post event must carry the same tool_call_id as the pre events"
    );
    Ok(())
}

/// load_skill parity: a denying hook stops the call. The skill body never
/// reaches the conversation and no post event fires — the same shape a denied
/// ordinary tool call has.
#[tokio::test]
async fn load_skill_denied_by_hook_does_not_execute() -> Result<()> {
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
    let arguments = install_skill(pipeline.working_dir());
    api.on("use the skill").call("load_skill", arguments);
    api.on("denied by policy hook").reply("understood");

    let result = pipeline.run(["use the skill"]).await?;

    let results = env.payloads("result.log");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["decision"], "deny");
    assert_eq!(results[0]["blocked_by"], "test-plugin");
    assert_eq!(results[0]["tool_name"], "load_skill");

    assert!(
        env.payloads("post.log").is_empty(),
        "PostToolUse must not fire for a denied load_skill call"
    );
    assert!(
        env.payloads("postfail.log").is_empty(),
        "PostToolUseFailure must not fire for a denied load_skill call"
    );

    let loaded_body = result
        .conversation()
        .messages()
        .iter()
        .any(|message| message.as_concat_text().contains("SKILL_BODY_CONTENT"));
    assert!(
        !loaded_body,
        "a denied load_skill call must not execute the skill load"
    );
    Ok(())
}

/// Unknown-tool parity: a valid unadvertised call still emits `PreToolUse` and
/// `PreToolUseResult`, correlated by one `tool_call_id`.
#[tokio::test]
async fn unknown_tool_emits_pre_tool_use_and_result_with_matching_id() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
        ("PreToolUseResult", "", "result.sh", RECORD_RESULT_SCRIPT),
    ]);
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_hook_manager(env.hook_manager());
    api.on("try the missing tool")
        .unadvertised_call("missing__tool", serde_json::json!({}));
    api.on("not available").reply("recovered");

    pipeline.run(["try the missing tool"]).await?;

    let pres = env.payloads("pre.log");
    let results = env.payloads("result.log");
    assert_eq!(pres.len(), 1, "PreToolUse must fire for an unknown tool");
    assert_eq!(
        results.len(),
        1,
        "PreToolUseResult must fire for an unknown tool"
    );
    assert_eq!(pres[0]["tool_name"], "missing__tool");
    assert_eq!(results[0]["tool_name"], "missing__tool");
    assert_eq!(results[0]["event"], "PreToolUseResult");
    assert_eq!(results[0]["decision"], "allow");
    assert_eq!(
        pres[0]["tool_call_id"], results[0]["tool_call_id"],
        "PreToolUse and PreToolUseResult must carry the same tool_call_id"
    );
    assert!(pres[0]["tool_call_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    Ok(())
}

/// Unknown-tool parity: the unavailable result is a failed tool outcome and
/// carries the same id as the pre event.
#[tokio::test]
async fn unknown_tool_emits_post_tool_failure_event() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
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
    api.on("try the missing tool")
        .unadvertised_call("missing__tool", serde_json::json!({}));
    api.on("not available").reply("recovered");

    pipeline.run(["try the missing tool"]).await?;

    let pres = env.payloads("pre.log");
    let post_failures = env.payloads("postfail.log");
    assert_eq!(pres.len(), 1);
    assert!(
        env.payloads("post.log").is_empty(),
        "PostToolUse must not fire for an unavailable tool"
    );
    assert_eq!(
        post_failures.len(),
        1,
        "PostToolUseFailure must fire for an unavailable tool"
    );
    assert_eq!(post_failures[0]["event"], "PostToolUseFailure");
    assert_eq!(post_failures[0]["tool_name"], "missing__tool");
    assert_eq!(
        post_failures[0]["tool_call_id"], pres[0]["tool_call_id"],
        "the post event must carry the same tool_call_id as the pre events"
    );
    Ok(())
}

/// Unknown-tool parity: a denying hook returns before the unknown-tool handler
/// creates its unavailable result, and no post event fires.
#[tokio::test]
async fn unknown_tool_denied_by_hook_does_not_resolve_as_unavailable() -> Result<()> {
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
    api.on("try the missing tool")
        .unadvertised_call("missing__tool", serde_json::json!({}));
    api.on("denied by policy hook").reply("understood");

    let result = pipeline.run(["try the missing tool"]).await?;

    let results = env.payloads("result.log");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["decision"], "deny");
    assert_eq!(results[0]["blocked_by"], "test-plugin");
    assert_eq!(results[0]["tool_name"], "missing__tool");
    assert!(
        env.payloads("post.log").is_empty(),
        "PostToolUse must not fire for a denied unknown tool"
    );
    assert!(
        env.payloads("postfail.log").is_empty(),
        "PostToolUseFailure must not fire for a denied unknown tool"
    );
    let tool_error = result
        .conversation()
        .messages()
        .iter()
        .flat_map(|message| &message.content)
        .find_map(|content| match content {
            MessageContent::ToolResponse(response) => response.tool_result.as_ref().err(),
            _ => None,
        })
        .expect("denied unknown tool response");
    assert!(tool_error.message.contains("denied by policy hook"));
    assert!(!tool_error.message.contains("is not available"));
    Ok(())
}

#[tokio::test]
async fn chat_mode_skips_recipe_final_output_without_tool_hooks() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
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
    let pipeline = pipeline
        .with_hook_manager(env.hook_manager())
        .with_goose_mode(GooseMode::Chat)
        .await
        .with_max_turns(1);
    pipeline.set_recipe(final_output_recipe()).await?;
    api.on("produce the answer").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "answer": "done" }),
    );

    let result = pipeline.run(["produce the answer"]).await?;

    result.assert_message(-2, ToolResponse, CHAT_MODE_TOOL_SKIPPED_RESPONSE);
    assert!(env.payloads("pre.log").is_empty());
    assert!(env.payloads("result.log").is_empty());
    assert!(env.payloads("post.log").is_empty());
    assert!(env.payloads("postfail.log").is_empty());
    Ok(())
}

#[tokio::test]
async fn chat_mode_skips_unknown_tool_without_tool_hooks() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
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
    let pipeline = pipeline
        .with_hook_manager(env.hook_manager())
        .with_goose_mode(GooseMode::Chat)
        .await
        .with_max_turns(1);
    api.on("try the missing tool")
        .unadvertised_call("missing__tool", serde_json::json!({}));

    let result = pipeline.run(["try the missing tool"]).await?;

    result.assert_message(-2, ToolResponse, CHAT_MODE_TOOL_SKIPPED_RESPONSE);
    assert!(env.payloads("pre.log").is_empty());
    assert!(env.payloads("result.log").is_empty());
    assert!(env.payloads("post.log").is_empty());
    assert!(env.payloads("postfail.log").is_empty());
    Ok(())
}

#[tokio::test]
async fn denied_unknown_tool_reports_policy_decline_without_tool_hooks() -> Result<()> {
    let env = RecordingHookEnv::new(&[
        ("PreToolUse", "", "pre.sh", RECORD_PRE_SCRIPT),
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
    let pipeline = pipeline
        .with_hook_manager(env.hook_manager())
        .with_goose_mode(GooseMode::Approve)
        .await;
    pipeline.set_permission("missing__tool", PermissionLevel::NeverAllow);
    api.on("try the missing tool")
        .unadvertised_call("missing__tool", serde_json::json!({}));
    api.on(DECLINED_RESPONSE).reply("understood");

    let result = pipeline.run(["try the missing tool"]).await?;

    result.assert_message(-2, ToolResponse, DECLINED_RESPONSE);
    assert!(env.payloads("pre.log").is_empty());
    assert!(env.payloads("result.log").is_empty());
    assert!(env.payloads("post.log").is_empty());
    assert!(env.payloads("postfail.log").is_empty());
    Ok(())
}
