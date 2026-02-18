//! Integration tests for LocalInferenceProvider.
//!
//! These tests require a downloaded GGUF model and are ignored by default.
//! Run with: cargo test -p goose --test local_inference_integration -- --ignored

use futures::StreamExt;
use goose::model::ModelConfig;
use goose::providers::base::Provider;
use goose::providers::create;

const TEST_MODEL: &str = "llama-3.2-1b";

#[tokio::test]
#[ignore]
async fn test_local_inference_stream_produces_output() {
    let model_config = ModelConfig::new(TEST_MODEL).expect("valid model config");
    let provider = create("local", model_config.clone(), Vec::new())
        .await
        .expect("provider creation should succeed");

    let system = "You are a helpful assistant. Be brief.";
    let messages = vec![goose::conversation::message::Message::user().with_text("Say hello.")];

    let mut stream = provider
        .stream(&model_config, "test-session", system, &messages, &[])
        .await
        .expect("stream should start");

    let mut got_text = false;
    let mut got_usage = false;

    while let Some(result) = stream.next().await {
        let (msg, usage) = result.expect("stream item should be Ok");
        if msg.is_some() {
            got_text = true;
        }
        if let Some(u) = usage {
            got_usage = true;
            let usage_inner = u.usage;
            assert!(
                usage_inner.input_tokens.unwrap_or(0) > 0,
                "should have input tokens"
            );
            assert!(
                usage_inner.output_tokens.unwrap_or(0) > 0,
                "should have output tokens"
            );
        }
    }

    assert!(got_text, "stream should produce at least one text message");
    assert!(got_usage, "stream should produce usage info");
}
