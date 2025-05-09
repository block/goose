use anyhow::Result;
use dotenv::dotenv;
use goose_llm::extractors::generate_session_name;
use goose_llm::message::Message;
use goose_llm::providers::errors::ProviderError;

#[tokio::test]
async fn test_generate_session_name_success() -> Result<(), ProviderError> {
    // Load .env for Databricks credentials
    dotenv().ok();
    let has_creds =
        std::env::var("DATABRICKS_HOST").is_ok() && std::env::var("DATABRICKS_TOKEN").is_ok();
    if !has_creds {
        println!("Skipping generate_session_name test – Databricks creds not set");
        return Ok(());
    }

    // Build a few messages with at least two user messages
    let messages = vec![
        Message::user().with_text("Hello, how are you?"),
        Message::assistant().with_text("I’m fine, thanks!"),
        Message::user().with_text("What’s the weather in New York tomorrow?"),
    ];

    let name = generate_session_name(&messages).await?;
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
    );

    Ok(())
}

#[tokio::test]
async fn test_generate_session_name_no_user() {
    // No user messages → expect ExecutionError
    let messages = vec![
        Message::assistant().with_text("System starting…"),
        Message::assistant().with_text("All systems go."),
    ];

    let err = generate_session_name(&messages).await;
    assert!(
        matches!(err, Err(ProviderError::ExecutionError(_))),
        "Expected ExecutionError when there are no user messages, got: {:?}",
        err
    );
}
