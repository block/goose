use anyhow::Result;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage as ProviderTokenUsage};
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock};
use serde_json::json;

use super::calculator_extension::{value, ADD};
use super::dummy_api::ProviderFeatures;
use super::pipeline::{self, test_pipeline, MessageKind::Agent};
use crate::agents::state_machine;
use crate::agents::state_machine::ops_compaction::MAX_CONTEXT_ERROR_COMPACTIONS;
use crate::context_mgmt::{compute_tool_call_cutoff, TOOLCALL_SUMMARIZATION_BATCH_SIZE};
use crate::conversation::message::{Message, MessageErrorKind};
use crate::conversation::Conversation;

const SUMMARIZE_HISTORY: &str = "Please summarize the conversation history";
const SUMMARIZE_TOOL_PAIR: &str = "summarize a tool call & response pair";

#[tokio::test]
async fn proactive_and_manual_compaction_continue_with_replaced_usage() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("fill the context").reply("filled");
    api.on("check the budget").reply("budget checked");
    api.on(SUMMARIZE_HISTORY).reply("summary");
    api.on("Your context was compacted")
        .reply("continued after compaction");
    api.on("after manual compaction").reply("still working");

    let half_full = format!(
        "fill the context {}",
        "x".repeat(pipeline.context_limit() / 2)
    );
    pipeline.run([half_full.as_str()]).await?;
    let budget = pipeline.run(["check the budget"]).await?;
    budget.assert_message(-1, Agent, "budget checked");
    assert!(api.calls().last().unwrap().input_contains("<compaction>"));

    let filled_usage = (pipeline.context_limit() as f64 * 0.81) as i32;
    pipeline.set_total_tokens(filled_usage).await;
    let compacted = pipeline.run(["continue"]).await?;
    compacted.assert_message(-1, Agent, "continued after compaction");
    compacted.assert_emitted("Performing auto-compaction");
    assert_eq!(compacted.history_replacements(), 1);
    assert!(compacted
        .session
        .usage
        .total_tokens
        .is_some_and(|tokens| tokens < filled_usage));

    let first_manual = pipeline.run(["/compact"]).await?;
    let second_manual = pipeline.run(["/compact"]).await?;
    first_manual.assert_emitted("Compaction complete");
    assert_eq!(first_manual.history_replacements(), 1);
    assert_eq!(second_manual.history_replacements(), 1);

    let commands = second_manual
        .conversation()
        .messages()
        .iter()
        .filter(|message| message.as_concat_text().trim() == "/compact")
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 2);
    assert!(commands
        .iter()
        .all(|message| message.is_user_visible() && !message.is_agent_visible()));

    let continued = pipeline.run(["after manual compaction"]).await?;
    continued.assert_message(-1, Agent, "still working");

    pipeline.set_total_tokens(100).await;
    let cleared = pipeline.run(["/clear"]).await?;
    assert_eq!(cleared.history_replacements(), 1);
    assert_eq!(cleared.conversation().messages().len(), 2);
    assert!(cleared
        .conversation()
        .messages()
        .iter()
        .all(|message| message.is_user_visible() && !message.is_agent_visible()));
    assert_eq!(cleared.session.usage.total_tokens, Some(0));

    let (pipeline, _api) = test_pipeline().await?;
    pipeline.set_total_tokens(100).await;
    let machine =
        state_machine::StateMachine::new(Vec::new(), tokio_util::sync::CancellationToken::new());
    let (tx, _rx) = tokio::sync::mpsc::channel(4);
    let emit = state_machine::Emitter::new(tx, tokio_util::sync::CancellationToken::new());
    let apply = async |effects: Vec<state_machine::GooseEffect>| -> Result<()> {
        let session = pipeline.session().await?;
        let mut result = state_machine::StepResult {
            effects,
            applied_step: None,
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

    let replacement = Conversation::new_unvalidated([
        Message::user().with_text("keep this"),
        Message::assistant().with_text("and this"),
    ]);
    apply(vec![replacement.into()]).await?;
    let recounted = pipeline.session().await?.usage.total_tokens;
    assert!(recounted.is_some_and(|tokens| tokens > 0 && tokens < 100));

    let replacement = Conversation::new_unvalidated([Message::user().with_text("new context")]);
    let usage = ProviderUsage::new(
        "scripted-model".to_string(),
        ProviderTokenUsage::new(Some(10), Some(5), Some(15)),
    );
    apply(vec![
        replacement.into(),
        state_machine::GooseEffect::RecordUsage(usage),
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
async fn tokenless_provider_compacts_estimated_context() -> Result<()> {
    let (pipeline, api) = pipeline::test_pipeline_with(ProviderFeatures {
        reports_usage: false,
        ..ProviderFeatures::default()
    })
    .await?;
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(200)),
        )
        .await;
    let large_context = (0..500)
        .map(|index| format!("token-{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    pipeline
        .seed([Message::user().with_text(large_context)])
        .await?;
    assert!(pipeline.session().await?.usage.total_tokens.is_none());

    api.on(SUMMARIZE_HISTORY).reply("summary");
    api.on("Your context was compacted")
        .reply("continued after estimated compaction");

    let compacted = pipeline.run(["continue"]).await?;
    compacted.assert_message(-1, Agent, "continued after estimated compaction");
    compacted.assert_emitted("Performing auto-compaction");
    assert_eq!(compacted.history_replacements(), 1);

    Ok(())
}

#[tokio::test]
async fn a_failed_compact_command_reports_the_error_and_keeps_working() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("do some work").reply("worked");
    api.on(SUMMARIZE_HISTORY).server_error("summarizer offline");
    pipeline.run(["do some work"]).await?;

    let failed = pipeline.run(["/compact"]).await?;
    assert_eq!(failed.history_replacements(), 0);
    failed.assert_message(-1, Agent, "summarizer offline");
    assert!(failed
        .conversation()
        .messages()
        .iter()
        .any(|message| message.as_concat_text() == "worked"));

    api.on("still there?").reply("still here");
    let recovered = pipeline.run(["still there?"]).await?;
    recovered.assert_message(-1, Agent, "still here");

    Ok(())
}

#[tokio::test]
async fn context_owning_provider_has_no_compaction_operation() -> Result<()> {
    let (pipeline, api) = pipeline::test_pipeline_with(ProviderFeatures {
        manages_own_context: true,
        ..ProviderFeatures::default()
    })
    .await?;
    api.on("continue").reply("continued");
    pipeline
        .set_total_tokens((pipeline.context_limit() as f64 * 0.81) as i32)
        .await;

    let continued = pipeline.run(["continue"]).await?;
    continued.assert_message(-1, Agent, "continued");
    assert_eq!(continued.history_replacements(), 0);
    assert_eq!(api.calls().len(), 1);

    for command in ["clear", "compact"] {
        let input = format!("/{command}");
        api.on(&input).reply(format!("provider handled /{command}"));
        let handled = pipeline.run([input.as_str()]).await?;
        handled.assert_message(-1, Agent, &format!("provider handled /{command}"));
        assert_eq!(handled.history_replacements(), 0);
    }
    assert_eq!(api.calls().len(), 3);

    Ok(())
}

#[tokio::test]
async fn text_that_looks_like_a_context_error_does_not_compact() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("false alarm")
        .reply("the provider said context length exceeded, but this is ordinary text");

    let false_alarm = pipeline.run(["false alarm"]).await?;
    false_alarm.assert_message(-1, Agent, "context length exceeded");
    assert_eq!(false_alarm.history_replacements(), 0);

    Ok(())
}

#[tokio::test]
async fn a_context_error_compacts_and_the_session_survives_a_failed_retry() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("real error").context_limit_error("too long");
    api.on(SUMMARIZE_HISTORY).reply("summary");
    api.on("Your context was compacted")
        .server_error("provider unavailable");

    let failed_after_compaction = pipeline.run(["real error"]).await?;
    assert_eq!(failed_after_compaction.history_replacements(), 1);
    assert_eq!(
        failed_after_compaction
            .conversation()
            .last()
            .and_then(Message::error_kind),
        Some(MessageErrorKind::Other)
    );

    api.on("try again").reply("recovered on the next turn");
    let recovered = pipeline.run(["try again"]).await?;
    recovered.assert_message(-1, Agent, "recovered on the next turn");

    Ok(())
}

#[tokio::test]
async fn repeated_context_errors_stop_compacting() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("keep overflowing").context_limit_error("too long");
    api.on(SUMMARIZE_HISTORY).reply("summary");
    api.on("Your context was compacted")
        .context_limit_error("too long");

    let mut kickoff = Message::user().with_text("keep overflowing");
    kickoff.created = 1;
    let capped = pipeline.run_message(kickoff).await?;

    assert_eq!(capped.history_replacements(), MAX_CONTEXT_ERROR_COMPACTIONS);
    assert_eq!(api.call_count(), 1 + 2 * MAX_CONTEXT_ERROR_COMPACTIONS);
    assert_eq!(
        capped.conversation().last().and_then(Message::error_kind),
        Some(MessageErrorKind::ContextLengthExceeded)
    );

    Ok(())
}

#[tokio::test]
async fn tool_pairs_are_compacted_only_after_the_current_turn() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let cutoff = compute_tool_call_cutoff(pipeline.context_limit(), pipeline::COMPACTION_THRESHOLD);
    let boundary = cutoff + TOOLCALL_SUMMARIZATION_BATCH_SIZE;

    api.on("do a lot of work").call(ADD, value(1));
    api.on("result:").call(ADD, value(1));
    api.on(format!("result: {}", boundary - 1))
        .reply("first batch done");
    api.on("reach the boundary").call(ADD, value(1));
    api.on(format!("result: {boundary}"))
        .reply("at the boundary");
    api.on("cross the boundary").call(ADD, value(1));
    api.on(format!("result: {}", boundary + 1))
        .reply("all work done");
    api.on_system(SUMMARIZE_TOOL_PAIR)
        .reply("summary of the pair");
    api.on("carry on").reply("carried on");

    let current_turn = pipeline
        .run([
            "do a lot of work",
            "reach the boundary",
            "cross the boundary",
        ])
        .await?;
    current_turn.assert_message(-1, Agent, "all work done");
    assert_eq!(
        current_turn
            .conversation()
            .messages()
            .iter()
            .filter(|message| message.is_agent_visible() && message.is_tool_call())
            .count(),
        boundary + 1
    );
    assert!(!current_turn
        .conversation()
        .messages()
        .iter()
        .any(|message| message.as_concat_text() == "summary of the pair"));

    let calls_before = api.call_count();
    let next_turn = pipeline.run(["carry on"]).await?;
    next_turn.assert_message(-1, Agent, "carried on");

    // The batch is one provider call per pair plus the turn's own inference. A
    // pair whose summary call fails is left alone, so check that every call was
    // made before reading the counts it produced.
    assert_eq!(
        api.call_count() - calls_before,
        TOOLCALL_SUMMARIZATION_BATCH_SIZE + 1,
        "expected a summary request per pair"
    );
    let summaries = next_turn
        .conversation()
        .messages()
        .iter()
        .filter(|message| {
            message.as_concat_text() == "summary of the pair"
                && message.is_agent_visible()
                && !message.is_user_visible()
        })
        .count();
    let visible_tool_calls = next_turn
        .conversation()
        .messages()
        .iter()
        .filter(|message| message.is_agent_visible() && message.is_tool_call())
        .count();
    assert_eq!(
        (summaries, visible_tool_calls),
        (
            TOOLCALL_SUMMARIZATION_BATCH_SIZE,
            boundary + 1 - TOOLCALL_SUMMARIZATION_BATCH_SIZE
        ),
        "summaries and the tool calls they replaced disagree"
    );

    Ok(())
}

#[tokio::test]
async fn parallel_and_failed_tool_pairs_are_compacted_as_complete_messages() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let cutoff = compute_tool_call_cutoff(pipeline.context_limit(), pipeline::COMPACTION_THRESHOLD);
    let calls_per_message = 2;
    let batches = 2;
    let summarized_messages = batches * TOOLCALL_SUMMARIZATION_BATCH_SIZE / calls_per_message;
    let pairs =
        (cutoff + batches * TOOLCALL_SUMMARIZATION_BATCH_SIZE).div_ceil(calls_per_message) + 1;

    api.on_system(SUMMARIZE_TOOL_PAIR).reply("pair summary");
    api.on("carry on").reply("done");

    pipeline
        .seed([Message::user().with_text("old work")])
        .await?;
    for n in 0..pairs {
        let ids = [format!("call_{n}a"), format!("call_{n}b")];
        let mut request = Message::assistant();
        let mut response = Message::user();
        for id in &ids {
            request = request.with_tool_request(
                id.clone(),
                Ok(CallToolRequestParams::new(ADD).with_arguments(serde_json::Map::new())),
            );
            let result = if n == 0 {
                CallToolResult::error(vec![ContentBlock::text("failed calculation")])
            } else {
                CallToolResult::success(vec![ContentBlock::text("result")])
            };
            response.add_tool_response_with_metadata(id.clone(), Ok(result), None);
        }
        pipeline.seed([request, response]).await?;
    }

    let result = pipeline.run(["carry on"]).await?;
    result.assert_message(-1, Agent, "done");
    assert_eq!(api.call_count(), summarized_messages + 1);

    let persisted = result.conversation();
    assert_eq!(
        persisted
            .messages()
            .iter()
            .filter(|message| {
                message.as_concat_text() == "pair summary"
                    && message.is_agent_visible()
                    && !message.is_user_visible()
            })
            .count(),
        summarized_messages
    );
    let failed_pair = persisted
        .messages()
        .iter()
        .filter(|message| {
            message.get_tool_request_ids().contains("call_0a")
                || message.get_tool_response_ids().contains("call_0a")
        })
        .collect::<Vec<_>>();
    assert_eq!(failed_pair.len(), 2);
    assert!(failed_pair
        .iter()
        .all(|message| message.is_user_visible() && !message.is_agent_visible()));
    for message in persisted
        .messages()
        .iter()
        .filter(|message| message.is_tool_response())
    {
        let response_ids = message.get_tool_response_ids();
        let paired_visibility = persisted
            .messages()
            .iter()
            .find(|request| {
                request
                    .get_tool_request_ids()
                    .intersection(&response_ids)
                    .next()
                    .is_some()
            })
            .map(Message::is_agent_visible);
        assert_eq!(paired_visibility, Some(message.is_agent_visible()));
    }

    Ok(())
}

#[tokio::test]
async fn a_small_model_compacts_a_large_tool_result_out_of_the_conversation() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    let pipeline = pipeline.with_model("gpt-3.5-turbo").await;
    let large_result = "x".repeat(2_000);

    let request = Message::assistant().with_tool_request(
        "large-result",
        Ok(CallToolRequestParams::new(ADD).with_arguments(serde_json::Map::new())),
    );
    let mut response = Message::user();
    response.add_tool_response_with_metadata(
        "large-result",
        Ok(CallToolResult::success(vec![ContentBlock::text(
            large_result.clone(),
        )])),
        None,
    );
    pipeline
        .seed([Message::user().with_text("old work"), request, response])
        .await?;
    let filled_usage = (pipeline.context_limit() as f64 * 0.85) as i32;
    pipeline.set_total_tokens(filled_usage).await;

    api.on(SUMMARIZE_HISTORY).reply("large work summarized");
    api.on("Your context was compacted").reply("continued");

    let compacted = pipeline.run(["continue"]).await?;
    compacted.assert_message(-1, Agent, "continued");
    assert_eq!(compacted.history_replacements(), 1);
    assert!(compacted
        .session
        .usage
        .total_tokens
        .is_some_and(|tokens| tokens < filled_usage));

    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("summarization request");
    assert!(summarization.system_contains(&large_result));
    assert!(!api.calls().last().unwrap().input_contains(&large_result));
    assert!(!compacted
        .conversation()
        .agent_visible_messages()
        .iter()
        .any(|message| message.as_concat_text().contains(&large_result)));

    Ok(())
}

#[tokio::test]
async fn a_large_tool_result_alone_triggers_the_mid_turn_check() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    // Six parallel shell calls each land a ~10k-character (≈5k-token)
    // truncated preview, so the conversation grows by ~31k tokens in one
    // step — far past the 19.2k threshold — while the request that produced
    // the calls was under it. Only a mid-turn check that recounts the
    // conversation, rather than trusting the preceding request's usage, can
    // see the turn cross the threshold. gpt-4.1's ~1M canonical limit keeps
    // the API's rejection wall far above every request, summarization
    // included.
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(24_000)),
        )
        .await;
    pipeline
        .add_extension_with_tools("developer", &["shell"])
        .await?;
    pipeline.set_permission(
        "shell",
        crate::config::permission::PermissionLevel::AlwaysAllow,
    );

    let huge_output = json!({ "command": "awk 'BEGIN{for(i=0;i<60000;i++)printf \"x \"}'" });
    api.on("read the huge outputs").calls([
        ("big-1", "shell", huge_output.clone()),
        ("big-2", "shell", huge_output.clone()),
        ("big-3", "shell", huge_output.clone()),
        ("big-4", "shell", huge_output.clone()),
        ("big-5", "shell", huge_output.clone()),
        ("big-6", "shell", huge_output),
    ]);
    api.on(SUMMARIZE_HISTORY).reply("huge outputs summarized");
    api.on("Your context was compacted")
        .reply("continued after the huge outputs");

    let run = pipeline.run(["read the huge outputs"]).await?;

    run.assert_message(-1, Agent, "continued after the huge outputs");
    assert_eq!(run.history_replacements(), 1);
    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("summarization request");
    assert!(
        summarization.input_tokens() > 60_000,
        "the summarization request must carry the huge tool results, i.e. the check ran mid-turn"
    );
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}

#[tokio::test]
async fn a_tool_loop_that_crosses_the_threshold_compacts_mid_turn() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;

    // The kickoff starts well under the threshold (0.8 * the model's 128k
    // canonical limit), so the only check that can see this turn grow is one
    // that runs after a tool response lands. Padding rides in the assistant
    // reply beside each tool call, growing the context past the trigger after
    // a few round-trips but below the API's hard 128k rejection.
    let padding = |round: usize| format!("padding {round} {}", "x".repeat(24_000));

    api.on("grow the context")
        .reasoning("looping")
        .reply(padding(0))
        .call(ADD, value(1));
    for round in 1..=4 {
        api.on(format!("result: {round}"))
            .reasoning("looping")
            .reply(padding(round))
            .call(ADD, value(1));
    }
    api.on(SUMMARIZE_HISTORY).reply("summary of the tool loop");
    api.on("Your context was compacted").reply("loop finished");

    let run = pipeline.run(["grow the context"]).await?;

    run.assert_message(-1, Agent, "loop finished");
    assert_eq!(
        run.history_replacements(),
        1,
        "the tool loop should compact exactly once"
    );
    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("summarization request");
    assert!(
        summarization.system_contains("result:"),
        "summarization ran while tool output was still in context, i.e. mid-turn"
    );
    assert_eq!(
        api.context_limit_rejections(),
        0,
        "compaction must fire before the provider rejects an oversized request"
    );
    assert!(
        !run.conversation()
            .messages()
            .iter()
            .any(|message| message.error_kind() == Some(MessageErrorKind::ContextLengthExceeded)),
        "a reactive compaction would have left a context-length error behind"
    );

    Ok(())
}

#[tokio::test]
async fn a_context_owning_provider_loops_tools_without_compacting() -> Result<()> {
    let (pipeline, api) = pipeline::test_pipeline_with(ProviderFeatures {
        manages_own_context: true,
        ..ProviderFeatures::default()
    })
    .await?;

    api.on("grow the context").call(ADD, value(1));
    api.on("result: 1").call(ADD, value(1));
    api.on("result: 2").reply("loop finished");

    // Past the threshold for the whole run, turn boundary included.
    pipeline
        .set_total_tokens((pipeline.context_limit() as f64 * 0.9) as i32)
        .await;

    let run = pipeline.run(["grow the context"]).await?;

    run.assert_message(-1, Agent, "loop finished");
    assert_eq!(run.history_replacements(), 0);
    assert!(!api
        .calls()
        .iter()
        .any(|call| call.input_contains(SUMMARIZE_HISTORY)));
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}

#[tokio::test]
async fn a_tool_result_added_to_the_provider_baseline_crosses_the_threshold() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    // Three parallel shell calls land ~15k tokens of truncated output. The
    // recounted conversation (~15.5k) and the preceding request's usage (the
    // serialized request, whose system overhead the conversation count cannot
    // see) each stay under the 19.2k threshold, so trusting only the larger
    // of the two misses the crossing; the unsent tool growth must be added to
    // the provider baseline for the mid-turn check to see it. gpt-4.1's ~1M
    // canonical limit keeps the API's rejection wall far above every request.
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(24_000)),
        )
        .await;
    pipeline
        .add_extension_with_tools("developer", &["shell"])
        .await?;
    pipeline.set_permission(
        "shell",
        crate::config::permission::PermissionLevel::AlwaysAllow,
    );

    let huge_output = json!({ "command": "awk 'BEGIN{for(i=0;i<60000;i++)printf \"x \"}'" });
    api.on("grow past the baseline").calls([
        ("big-1", "shell", huge_output.clone()),
        ("big-2", "shell", huge_output.clone()),
        ("big-3", "shell", huge_output),
    ]);
    api.on(SUMMARIZE_HISTORY)
        .reply("baseline growth summarized");
    api.on("Your context was compacted")
        .reply("continued past the baseline");

    let run = pipeline.run(["grow past the baseline"]).await?;

    let producing_request = api.calls().first().cloned().expect("producing request");
    assert!(
        producing_request.input_tokens() < 19_200,
        "the producing request must stay under the threshold, got {}",
        producing_request.input_tokens()
    );

    run.assert_message(-1, Agent, "continued past the baseline");
    assert_eq!(
        run.history_replacements(),
        1,
        "the unsent tool growth must be added to the provider baseline"
    );
    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("summarization request");
    assert!(
        summarization.system_contains("x x x"),
        "the summarization request must carry the tool results, i.e. the check ran mid-turn"
    );
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_steer_drained_before_the_check_still_recounts_tool_growth() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    // The steer operation runs before compaction and appends the steer as
    // the last User message, so the check that follows it sees a User
    // boundary. Three shell calls grow the context by ~15k tokens of unsent
    // output; the producing request stays under the 19.2k threshold, and so
    // does the recounted conversation alone — only the unsent growth added
    // to the provider baseline crosses it. A check that only recounts at a
    // Tool boundary sends the grown context on. The delayed calculator call
    // holds the tool step open so the steer is queued mid-execution.
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(24_000)),
        )
        .await;
    pipeline
        .add_extension_with_tools("developer", &["shell"])
        .await?;
    pipeline.set_permission(
        "shell",
        crate::config::permission::PermissionLevel::AlwaysAllow,
    );

    let huge_output = json!({ "command": "awk 'BEGIN{for(i=0;i<60000;i++)printf \"x \"}'" });
    api.on("grow then steer").calls([
        ("big-1", "shell", huge_output.clone()),
        ("big-2", "shell", huge_output.clone()),
        ("big-3", "shell", huge_output.clone()),
        (
            "slow-1",
            ADD,
            super::calculator_extension::delayed_value(7, 1_500),
        ),
    ]);
    api.on(SUMMARIZE_HISTORY).reply("steered work summarized");
    api.on("Your context was compacted")
        .reply("continued after the steer");

    let run = pipeline.run(["grow then steer"]);
    let steer = async {
        while pipeline.tool_contexts().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        pipeline
            .steer(Message::user().with_text("redirect the work"))
            .await;
    };
    let (result, ()) = tokio::join!(run, steer);
    let result = result?;

    let producing_request = api.calls().first().cloned().expect("producing request");
    assert!(
        producing_request.input_tokens() < 19_200,
        "the producing request must stay under the threshold, got {}",
        producing_request.input_tokens()
    );

    result.assert_message(-1, Agent, "continued after the steer");
    assert_eq!(
        result.history_replacements(),
        1,
        "the tool growth behind the steer must still be recounted"
    );
    assert_eq!(
        api.call_count(),
        3,
        "compaction must fire before the grown context is sent back to the provider"
    );
    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("summarization request");
    assert!(
        summarization.system_contains("redirect the work"),
        "the summarization request must carry the drained steer"
    );
    assert!(!pipeline.has_pending_steers().await);
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}

#[tokio::test]
async fn a_failed_mid_turn_compaction_continues_the_turn() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    // Three shell calls grow the context past the 19.2k threshold, but the
    // summarization request fails. The pass must continue to the next
    // inference instead of yielding the compaction error: the request that
    // follows may still fit, and the reactive path owns the outcome if it
    // does not. Rules match newest-first, so the summarization failure is
    // registered last while the recovery reply only matches a request that
    // already carries the tool results.
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(24_000)),
        )
        .await;
    pipeline
        .add_extension_with_tools("developer", &["shell"])
        .await?;
    pipeline.set_permission(
        "shell",
        crate::config::permission::PermissionLevel::AlwaysAllow,
    );

    let huge_output = json!({ "command": "awk 'BEGIN{for(i=0;i<60000;i++)printf \"x \"}'" });
    api.on("recover from a failed compaction").calls([
        ("big-1", "shell", huge_output.clone()),
        ("big-2", "shell", huge_output.clone()),
        ("big-3", "shell", huge_output),
    ]);
    api.on("x x x x").reply("recovered without compaction");
    api.on(SUMMARIZE_HISTORY).server_error("summarizer offline");

    let run = pipeline.run(["recover from a failed compaction"]).await?;

    run.assert_message(-1, Agent, "recovered without compaction");
    assert_eq!(
        run.history_replacements(),
        0,
        "the failed compaction must not replace the history"
    );
    assert!(api
        .calls()
        .iter()
        .any(|call| call.input_contains(SUMMARIZE_HISTORY)));
    assert!(run.conversation().messages().iter().all(|message| !message
        .as_concat_text()
        .contains("Ran into this error trying to compact")));
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}

#[tokio::test]
async fn a_stale_token_count_floors_on_the_full_conversation() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    // The session metadata reports 1,000 tokens while the seeded history
    // holds far more against the 19.2k threshold, and the bulk of it sits
    // before the newest assistant message. Only a recount whose total covers
    // the whole conversation can floor the stale metadata; one that stopped
    // counting at the newest assistant message would send the oversized
    // history on. gpt-4.1's ~1M canonical limit keeps the API's rejection
    // wall far above every request.
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(24_000)),
        )
        .await;
    pipeline
        .seed([
            Message::user().with_text("x ".repeat(22_000)),
            Message::assistant().with_text("done"),
        ])
        .await?;
    pipeline.set_total_tokens(1_000).await;

    api.on(SUMMARIZE_HISTORY).reply("stale history summarized");
    api.on("Your context was compacted")
        .reply("continued past the stale count");

    let run = pipeline.run(["continue the stale session"]).await?;

    run.assert_message(-1, Agent, "continued past the stale count");
    assert_eq!(
        run.history_replacements(),
        1,
        "the recounted conversation must floor the stale metadata"
    );
    assert_eq!(
        api.call_count(),
        2,
        "compaction must precede the first inference"
    );
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_large_steer_after_a_text_only_reply_rechecks_the_threshold() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    // The first reply is text-only and stays under the 19.2k threshold, so
    // the steer is what pushes past it: the check that runs after the steer
    // operation must recount and compact before the grown context is sent
    // back. The steer is sized from the first request's reported usage so
    // the crossing comes from the unsent growth added to the baseline.
    let pipeline = pipeline
        .with_model_config(
            goose_providers::model::ModelConfig::new("gpt-4.1").with_context_limit(Some(24_000)),
        )
        .await;

    let text_only = api
        .on("reply without tools")
        .hold_reply("a text-only answer");
    api.on(SUMMARIZE_HISTORY).reply("steer growth summarized");
    api.on("Your context was compacted")
        .reply("continued after the steer");

    let run = pipeline.run(["reply without tools"]);
    let steer = async {
        text_only.entered().await;
        let first = api.calls().first().cloned().expect("text-only request");
        let threshold = (pipeline.context_limit() as f64 * pipeline::COMPACTION_THRESHOLD) as i32;
        // Size the steer so the reported baseline plus its unsent growth
        // crosses with ~1k to spare, while the steer alone stays far enough
        // under the threshold that compaction (which preserves the most
        // recent text-only user message verbatim) is not re-triggered by the
        // retained steer afterwards.
        let steer_tokens = threshold - first.input_tokens() + 1_000;
        assert!(
            steer_tokens > 5_000,
            "first request unexpectedly large: {}",
            first.input_tokens()
        );
        let padding = steer_tokens.min(9_000) as usize;
        pipeline
            .steer(Message::user().with_text(format!("redirect the work {}", "x ".repeat(padding))))
            .await;
        text_only.release();
    };
    let (result, ()) = tokio::join!(run, steer);
    let result = result?;

    result.assert_message(-1, Agent, "continued after the steer");
    assert_eq!(
        result.history_replacements(),
        1,
        "the drained steer must be re-checked against the threshold"
    );
    assert_eq!(
        api.call_count(),
        3,
        "compaction must fire before the grown context is sent back to the provider"
    );
    let summarization = api
        .calls()
        .into_iter()
        .find(|call| call.input_contains(SUMMARIZE_HISTORY))
        .expect("summarization request");
    assert!(
        summarization.system_contains("redirect the work"),
        "the summarization request must carry the drained steer"
    );
    assert_eq!(api.context_limit_rejections(), 0);

    Ok(())
}
