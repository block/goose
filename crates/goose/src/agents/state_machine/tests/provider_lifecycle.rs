use anyhow::Result;

use super::dummy_api::ProviderFeatures;
use super::pipeline::MessageKind::Agent;
use super::pipeline::test_pipeline_with;
use super::test_pipeline;

#[tokio::test]
async fn missing_provider_usage_is_estimated() -> Result<()> {
    let (pipeline, api) = test_pipeline_with(ProviderFeatures {
        reports_usage: false,
        ..Default::default()
    })
    .await?;

    api.on("first turn").reply("first reply");
    let result = pipeline.run(["first turn"]).await?;
    result.assert_message(-1, Agent, "first reply");
    let first_total = result
        .session
        .usage
        .total_tokens
        .expect("estimated session usage");
    assert!(first_total > 0);
    assert_eq!(
        result
            .session
            .conversation
            .as_ref()
            .and_then(|conversation| conversation.last())
            .and_then(|message| message.metadata.usage.as_ref())
            .and_then(|usage| usage.total_tokens),
        Some(first_total)
    );

    api.on("second turn").reply("still working");
    let result = pipeline.run(["second turn"]).await?;
    result.assert_message(-1, Agent, "still working");
    assert!(
        result
            .session
            .usage
            .total_tokens
            .is_some_and(|total| total > first_total)
    );

    Ok(())
}

#[tokio::test]
async fn provider_failures_end_the_turn_and_later_turns_recover() -> Result<()> {
    let (pipeline, api) = test_pipeline().await?;

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
    let conversation = result.rendered_conversation();
    let error = conversation.last().expect("persisted provider error");
    assert!(
        error.starts_with("error:") && error.contains("500"),
        "provider error: {error}"
    );

    api.on("after server error")
        .reply("recovered from server error");
    let result = pipeline.run(["after server error"]).await?;
    result.assert_message(-1, Agent, "recovered from server error");

    Ok(())
}
