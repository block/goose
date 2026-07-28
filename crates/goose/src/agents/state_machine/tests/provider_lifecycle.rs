use anyhow::Result;

use super::calculator_extension::{ADD, ADD_VALUES, named_values, value};
use super::dummy_api::ProviderFeatures;
use super::pipeline::MessageKind::{Agent, Thinking, ToolCall, ToolResponse};
use super::pipeline::test_pipeline_with;
use crate::conversation::message::Message;

#[tokio::test]
async fn provider_lifecycle() -> Result<()> {
    let (mut pipeline, api) = test_pipeline_with(ProviderFeatures {
        reports_usage: false,
        preserves_thinking: true,
    })
    .await?;
    pipeline
        .set_system_prompt_override("CUSTOM_SYSTEM_PROMPT")
        .await;

    api.on("inspect this image and add one")
        .reasoning("I should inspect the image before calculating.")
        .reply("The image is suitable. I will add one.")
        .call(ADD, value(1));
    api.on("result: 1").reply("The total is 1.");

    let image_data = "aW1hZ2UtZGF0YQ==";
    let result = pipeline
        .run_message(
            Message::user()
                .with_text("inspect this image and add one")
                .with_image(image_data, "image/png"),
        )
        .await?;
    result.assert_message(
        2,
        Thinking,
        "I should inspect the image before calculating.",
    );
    result.assert_message(3, Agent, "The image is suitable. I will add one.");
    result.assert_message(4, ToolCall, ADD);
    result.assert_message(5, ToolResponse, "result: 1");
    result.assert_message(-1, Agent, "The total is 1.");

    let calls = api.calls();
    assert!(calls[0].input_has_image("image/png", image_data));
    assert!(
        calls[..2]
            .iter()
            .all(|call| call.system_contains("CUSTOM_SYSTEM_PROMPT"))
    );
    assert_eq!(
        calls[1].input_occurrences("I should inspect the image before calculating."),
        1
    );
    assert_eq!(
        calls[1].input_occurrences("The image is suitable. I will add one."),
        1
    );
    assert_eq!(calls[1].input_occurrences(ADD), 1);
    let schema = calls[0].tool_schema(ADD_VALUES).expect("add_values schema");
    assert!(schema.get("additionalProperties").is_some());
    assert!(
        schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .is_none_or(serde_json::Map::is_empty)
    );

    api.on("add named values")
        .call(ADD_VALUES, named_values([("left", 2), ("right", 3)]));
    api.on("result: 6").reply("The total is 6.");
    let result = pipeline.run(["add named values"]).await?;
    result.assert_message(-2, ToolResponse, "result: 6");
    result.assert_message(-1, Agent, "The total is 6.");

    let conversation = result.session.conversation.as_ref().unwrap();
    let first_total = result
        .session
        .usage
        .total_tokens
        .expect("estimated session usage");
    assert!(first_total > 0);
    assert_eq!(
        conversation
            .last()
            .and_then(|message| message.metadata.usage.as_ref())
            .and_then(|usage| usage.total_tokens),
        Some(first_total)
    );

    api.on("return no choices").no_choices();
    let result = pipeline.run(["return no choices"]).await?;
    result.assert_message(-1, Agent, "model returned an empty response");

    api.on("after no choices")
        .reply("recovered from no choices");
    let result = pipeline.run(["after no choices"]).await?;
    result.assert_message(-1, Agent, "recovered from no choices");

    api.on("return an empty reply").reply("");
    let result = pipeline.run(["return an empty reply"]).await?;
    result.assert_message(-1, Agent, "model returned an empty response");

    api.on("after empty reply")
        .reply("recovered from empty reply");
    let result = pipeline.run(["after empty reply"]).await?;
    result.assert_message(-1, Agent, "recovered from empty reply");

    api.on("return an empty server error").empty_server_error();
    let result = pipeline.run(["return an empty server error"]).await?;
    let error = result
        .rendered_conversation()
        .last()
        .cloned()
        .expect("persisted provider error");
    assert!(error.starts_with("error:") && error.contains("500"));

    api.on("after server error")
        .reply("recovered from server error");
    let result = pipeline.run(["after server error"]).await?;
    result.assert_message(-1, Agent, "recovered from server error");
    assert!(
        result
            .session
            .usage
            .total_tokens
            .is_some_and(|total| total > first_total)
    );
    assert!(
        api.calls()
            .iter()
            .all(|call| call.system_contains("CUSTOM_SYSTEM_PROMPT"))
    );

    pipeline.clear_system_prompt_override().await;
    pipeline = pipeline.with_model("gpt-4.1").await;
    api.on("use the standard prompt")
        .reply("The standard prompt is active.");
    let result = pipeline.run(["use the standard prompt"]).await?;
    result.assert_message(-1, Agent, "The standard prompt is active.");
    let call = api.calls().last().cloned().expect("provider request");
    assert!(call.uses_model("gpt-4.1"));
    assert!(call.system_contains("general-purpose AI agent called goose"));

    Ok(())
}
