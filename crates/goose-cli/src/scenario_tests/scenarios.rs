#[cfg(test)]
mod tests {
    use crate::scenario_tests::run_test_scenario;
    use anyhow::Result;

    #[tokio::test]
    async fn test_basic_greeting() -> Result<()> {
        let result = run_test_scenario("basic_greeting", &["hello", "goodbye"]).await?;

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
    }
}
