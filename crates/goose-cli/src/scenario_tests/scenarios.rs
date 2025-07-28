#[cfg(test)]
mod tests {
    use crate::scenario_tests::message_generator::{image, text};
    /// This module contains tests for the scenario runner and various scenarios.
    /// You can set the GOOSE_TEST_PROVIDER to just run a specific provider.
    use crate::scenario_tests::mock_client::WEATHER_TYPE;
    use crate::scenario_tests::scenario_runner::run_scenario;
    use anyhow::Result;

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
}
