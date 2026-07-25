use anyhow::Result;
use serde_json::json;

use super::calculator_extension::{value, ADD};
use super::pipeline::MessageKind::{Agent, ToolResponse};
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
    api.on("make a broken tool call").call(ADD, json!({}));
    api.on("missing field").reply("The calculation failed");
    api.on("are you there").reply("still here");

    let result = pipeline
        .run(["make a broken tool call", "are you there"])
        .await?;

    result.assert_message(2, ToolResponse, "");
    result.assert_message(-1, Agent, "still here");
    Ok(())
}

#[tokio::test]
async fn recover_from_missing_tool() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("try the missing tool")
        .call("missing__tool", json!({}));
    api.on("Tool 'missing__tool' is not available")
        .reply("recovered");

    let result = pipeline.run(["try the missing tool"]).await?;
    assert_eq!(api.call_count(), 2);

    result.assert_message(2, ToolResponse, "Tool 'missing__tool' is not available");
    result.assert_message(-1, Agent, "recovered");

    Ok(())
}

#[tokio::test]
async fn unparseable_tool_call_gets_parse_error_response() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;
    api.on("do it").malformed_tool_call(ADD, r#"{"value":"#);
    api.on("could not be parsed").reply("let me fix that");

    let result = pipeline.run(["do it"]).await?;
    result.assert_message(2, ToolResponse, "could not be parsed");
    assert_eq!(api.call_count(), 2);
    result.assert_message(-1, Agent, "let me fix that");

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
