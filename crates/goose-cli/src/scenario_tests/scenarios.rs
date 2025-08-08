//! This module contains tests for the scenario runner and various scenarios.
//! You can set the GOOSE_TEST_PROVIDER to just run a specific provider.

#[cfg(test)]
mod tests {
    use crate::scenario_tests::message_generator::{image, text};
    use crate::scenario_tests::mock_client::WEATHER_TYPE;
    use crate::scenario_tests::scenario_runner::run_scenario;
    use anyhow::Result;
    use goose::conversation::message::Message;

    #[tokio::test]
    async fn test_what_is_your_name() -> Result<()> {
        run_scenario(
            "what_is_your_name",
            text("what is your name"),
            None,
            |result| {
                assert!(result.error.is_none());
                assert!(
                    result.last_message()?.to_lowercase().contains("goose"),
                    "Response should contain 'goose': {}",
                    result.last_message()?
                );
                Ok(())
            },
        )
        .await
    }

    #[tokio::test]
    async fn test_weather_tool() -> Result<()> {
        // Google tells me it only knows about the weather in the US, so we skip it.
        run_scenario(
            "weather_tool",
            text("tell me what the weather is in Berlin, Germany"),
            Some(&["Google"]),
            |result| {
                assert!(result.error.is_none());

                let last_message = result.last_message()?.to_lowercase();

                assert!(
                    last_message.contains("berlin"),
                    "Last message should contain 'Berlin': {}",
                    last_message
                );
                assert!(
                    last_message.contains(WEATHER_TYPE),
                    "Last message should contain '{}': {}",
                    WEATHER_TYPE,
                    last_message
                );

                Ok(())
            },
        )
        .await
    }

    #[tokio::test]
    async fn test_image_analysis() -> Result<()> {
        // Google says it doesn't know about images, the other providers complain about
        // the image format, so we only run this for OpenAI and Anthropic.
        run_scenario(
            "image_analysis",
            image("What do you see in this image?", "test_image"),
            Some(&["Google", "azure_openai", "groq"]),
            |result| {
                assert!(result.error.is_none());
                let last_message = result.last_message()?;
                assert!(!last_message.is_empty());
                Ok(())
            },
        )
        .await
    }

    #[tokio::test]
    async fn test_context_length_exceeded_error() -> Result<()> {
        run_scenario(
            "context_length_exceeded",
            Box::new(|provider| {
                let model_config = provider.get_model_config();
                let context_length = model_config.context_limit.unwrap_or(300_000);
                // "hello " is only one token in most models, since the hello and space often
                // occur together in the training data.
                let large_message = "hello ".repeat(context_length + 100);
                Message::user().with_text(&large_message)
            }),
            Some(&["OpenAI"]),
            |result| {
                // this is unfortunate; we don't seem to actually catch the errors in this path,
                // but instead eat it:
                assert_eq!(
                    result.messages.len(),
                    0,
                    "Expected no messages due to compaction"
                );
                Ok(())
            },
        )
        .await
    }
}
