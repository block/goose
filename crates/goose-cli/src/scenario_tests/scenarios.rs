#[cfg(test)]
mod tests {
    use crate::scenario_tests::run_multi_provider_scenario;
    use anyhow::Result;

    #[tokio::test]
    async fn test_basic_greeting() -> Result<()> {
        run_multi_provider_scenario("multi_basic_greeting", &["hello", "goodbye"], |result| {
            assert!(result
                .message_contents()
                .iter()
                .any(|msg| msg.contains("Hello")));
            assert!(result
                .message_contents()
                .iter()
                .any(|msg| msg.contains("Goodbye")));
            assert!(result.error.is_none());
            Ok(())
        })
        .await
    }
}
