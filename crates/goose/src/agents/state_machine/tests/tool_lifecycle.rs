use anyhow::Result;
use serde_json::json;

use super::calculator_extension::{value, ADD, DIVIDE};
use super::pipeline::MessageKind::{Agent, ToolCall, ToolResponse};
use super::pipeline::MAX_TURNS;
use super::test_pipeline;
use crate::conversation::message::{Message, MessageContent};

#[tokio::test]
async fn basic_tool_calling() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("add one").call(ADD, value(1));
    api.on("result: 1").reply("The total is 1");
    api.on("hello").reply("hi there!");

    let result = pipeline.run(["add one", "hello"]).await?;

    result.assert_message(2, ToolResponse, "");
    result.assert_message(3, Agent, "The total is 1");
    result.assert_message(-1, Agent, "hi there!");
    assert_eq!(api.call_count(), 3);
    Ok(())
}

#[tokio::test]
async fn recover_from_faulty_call() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("make a broken tool call")
        .malformed_tool_call(ADD, "");
    api.on("missing field").reply("The calculation failed");
    api.on("divide by zero").call(DIVIDE, value(0));
    api.on("calculator operation failed")
        .reply("The calculator failed");
    api.on("add after the failures").call(ADD, value(1));
    api.on("result: 1").reply("still here");

    let result = pipeline
        .run([
            "make a broken tool call",
            "divide by zero",
            "add after the failures",
        ])
        .await?;

    result.assert_message(2, ToolResponse, "missing field");
    result.assert_message(6, ToolResponse, "calculator operation failed");
    result.assert_message(10, ToolResponse, "result: 1");
    result.assert_message(-1, Agent, "still here");
    assert_eq!(pipeline.calculator_total(), 1);
    Ok(())
}

#[tokio::test]
async fn recover_from_missing_tool() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("try the missing tool")
        .call("missing__tool", json!({}));
    api.on("Available tools").call(ADD, value(2));
    api.on("result: 2").reply("recovered");

    let result = pipeline.run(["try the missing tool"]).await?;
    assert_eq!(api.call_count(), 3);

    result.assert_message(2, ToolResponse, "Tool 'missing__tool' is not available");
    assert!(api.calls()[1].input_contains(ADD));
    result.assert_message(3, ToolCall, ADD);
    result.assert_message(4, ToolResponse, "result: 2");
    result.assert_message(-1, Agent, "recovered");
    assert_eq!(pipeline.calculator_total(), 2);

    Ok(())
}

#[tokio::test]
async fn malformed_tool_calls_recover_and_remain_bounded() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("do it").malformed_tool_call(ADD, r#"{"value":"#);
    api.on("could not be parsed").reply("let me fix that");

    let result = pipeline.run(["do it"]).await?;
    result.assert_message(2, ToolResponse, "could not be parsed");
    assert_eq!(api.call_count(), 2);
    result.assert_message(-1, Agent, "let me fix that");

    api.on("repeat malformed forever")
        .malformed_tool_call(ADD, r#"{"value":"#);
    let calls_before = api.call_count();
    let result = pipeline.run(["repeat malformed forever"]).await?;

    assert_eq!(api.call_count() - calls_before, MAX_TURNS as usize);
    result.assert_message(-1, Agent, crate::agents::state_machine::MAX_TURNS_MESSAGE);

    Ok(())
}

#[tokio::test]
async fn multiple_tool_calls_are_paired_and_executed_once() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    pipeline.synchronize_calculator(2);
    api.on("run several calculations").calls([
        ("first", ADD, value(1)),
        ("duplicate", ADD, value(2)),
        ("duplicate", ADD, value(100)),
    ]);
    api.on("result: 3").reply("done");

    let result = pipeline.run(["run several calculations"]).await?;

    result.assert_message(1, ToolCall, ADD);
    result.assert_message(2, ToolCall, ADD);
    result.assert_message(3, ToolResponse, "");
    result.assert_message(4, ToolResponse, "");
    result.assert_message(-1, Agent, "done");
    assert_eq!(pipeline.calculator_total(), 3);

    let conversation = result.session.conversation.unwrap_or_default();
    let mut request_ids = conversation
        .messages()
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|content| match content {
            MessageContent::ToolRequest(request) => Some(request.id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut response_ids = conversation
        .messages()
        .iter()
        .flat_map(Message::get_tool_response_ids)
        .collect::<Vec<_>>();
    request_ids.sort();
    response_ids.sort();
    assert_eq!(request_ids, ["duplicate", "first"]);
    assert_eq!(response_ids, request_ids);

    Ok(())
}

#[tokio::test]
async fn stale_orphaned_tool_request_is_not_executed() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("are you there?").reply("fresh start");
    let session_manager = pipeline.session_manager.clone();
    session_manager
        .add_message(
            &pipeline.session_id,
            &Message::user().with_text("old prompt"),
        )
        .await?;
    session_manager
        .add_message(
            &pipeline.session_id,
            &Message::assistant().with_tool_request(
                "orphan_1",
                Ok(rmcp::model::CallToolRequestParams::new("shell")
                    .with_arguments(serde_json::Map::new())),
            ),
        )
        .await?;

    let result = pipeline.run(["are you there?"]).await?;

    assert_eq!(api.call_count(), 1);
    assert!(!api.calls()[0].input_contains("orphan_1"));

    result.assert_message(-1, Agent, "fresh start");
    let conversation = result.session.conversation.unwrap_or_default();
    assert!(conversation.messages().iter().any(|m| {
        m.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(request) if request.id == "orphan_1"))
    }));
    assert!(!conversation.messages().iter().any(|m| m.is_tool_response()));

    Ok(())
}
