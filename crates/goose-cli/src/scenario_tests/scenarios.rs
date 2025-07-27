#[cfg(test)]
mod tests {
    use crate::scenario_tests::mock_client::WEATHER_TYPE;
    use crate::scenario_tests::scenario_runner::run_scenario;
    use anyhow::Result;

    #[tokio::test]
    async fn test_what_is_your_name() -> Result<()> {
        run_scenario("what_is_your_name", &["what is your name"], |result| {
            assert!(result.error.is_none());
            result.last_message()?.to_lowercase().contains("goose");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_weather_tool() -> Result<()> {
        run_scenario(
            "weather_tool",
            &["tell me what the weather is in Berlin, Germany"],
            |result| {
                assert!(result.error.is_none());

                let last_message = result.last_message()?;

                assert!(
                    last_message.contains("Berlin"),
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
}
