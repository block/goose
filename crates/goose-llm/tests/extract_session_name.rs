use anyhow::Result;
use dotenv::dotenv;
use goose_llm::extractors::generate_session_name;
use goose_llm::message::Message;
use goose_llm::providers::errors::ProviderError;
use rstest::rstest;

fn should_run_test() -> Result<(), String> {
    dotenv().ok();
    if std::env::var("DATABRICKS_HOST").is_err() {
        return Err("Missing DATABRICKS_HOST".to_string());
    }
    if std::env::var("DATABRICKS_TOKEN").is_err() {
        return Err("Missing DATABRICKS_TOKEN".to_string());
    }
    Ok(())
}

async fn _generate_session_name(
    model_name: &str,
    messages: &[Message],
) -> Result<String, ProviderError> {
    let provider_name = "databricks";
    let provider_config = serde_json::json!({
        "host": std::env::var("DATABRICKS_HOST").expect("Missing DATABRICKS_HOST"),
        "token": std::env::var("DATABRICKS_TOKEN").expect("Missing DATABRICKS_TOKEN"),
    });

    let model_config = goose_llm::ModelConfig::new(model_name.to_string());
    generate_session_name(provider_name, provider_config, model_config, messages).await
}

#[rstest]
#[case("claude-3-5-haiku")]
#[case("goose-gpt-4-1")]
#[case("goose-gemini-2-5-pro")]
#[tokio::test]
async fn test_generate_session_name_success(#[case] model_name: &str) {
    if should_run_test().is_err() {
        println!("Skipping...");
        return;
    }

    // Build a few messages with at least two user messages
    let messages = vec![
        Message::user().with_text("Hello, how are you?"),
        Message::assistant().with_text("I’m fine, thanks!"),
        Message::user().with_text("What’s the weather in New York tomorrow?"),
    ];

    let name = _generate_session_name(model_name, &messages)
        .await
        .expect("Failed to generate session name");

    println!("Generated session name: {:?}", name);

    // Should be non-empty and at most 4 words
    let name = name.trim();
    assert!(!name.is_empty(), "Name must not be empty");
    let word_count = name.split_whitespace().count();
    assert!(
        word_count <= 4,
        "Name must be 4 words or less, got {}: {}",
        word_count,
        name
    )
}

#[rstest]
#[case("claude-3-5-haiku")]
#[case("goose-gpt-4-1")]
#[case("goose-gemini-2-5-pro")]
#[tokio::test]
async fn test_generate_session_name_no_user(#[case] model_name: &str) {
    if should_run_test().is_err() {
        println!("Skipping 'test_generate_session_name_no_user'. Databricks creds not set");
        return;
    }

    // No user messages → expect ExecutionError
    let messages = vec![
        Message::assistant().with_text("System starting…"),
        Message::assistant().with_text("All systems go."),
    ];

    let err = _generate_session_name(model_name, &messages).await;
    assert!(
        matches!(err, Err(ProviderError::ExecutionError(_))),
        "Expected ExecutionError when there are no user messages, got: {:?}",
        err
    );
}
