#[cfg(test)]
mod tests {
    use crate::scenario_tests::run_multi_provider_scenario;
    use anyhow::Result;

    #[tokio::test]
    async fn test_what_is_your_name() -> Result<()> {
        run_multi_provider_scenario("what_is_your_name", &["what is your name"], |result| {
            assert!(result
                .message_contents()
                .iter()
                .any(|msg| msg.to_lowercase().contains("goose")));
            assert!(result.error.is_none());
            Ok(())
        })
        .await
    }
}
