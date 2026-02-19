//! Integration tests for LocalInferenceProvider.
//!
//! These tests require a downloaded GGUF model and are ignored by default.
//! Run with: cargo test -p goose --test local_inference_integration -- --ignored

use futures::StreamExt;
use goose::conversation::message::Message;
use goose::model::ModelConfig;
use goose::providers::create;
use std::time::Instant;

const TEST_MODEL: &str = "llama-3.2-1b";

#[tokio::test]
#[ignore]
async fn test_local_inference_stream_produces_output() {
    let model_config = ModelConfig::new(TEST_MODEL).expect("valid model config");
    let provider = create("local", model_config.clone(), Vec::new())
        .await
        .expect("provider creation should succeed");

    let system = "You are a helpful assistant. Be brief.";
    let messages = vec![Message::user().with_text("Say hello.")];

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

#[tokio::test]
#[ignore]
async fn test_local_inference_cold_and_warm_performance() {
    let model_config = ModelConfig::new(TEST_MODEL).expect("valid model config");
    let provider = create("local", model_config.clone(), Vec::new())
        .await
        .expect("provider creation should succeed");

    // Cold start (includes model loading)
    let messages = vec![Message::user().with_text("what is the capital of Moldova?")];
    let start = Instant::now();
    let (response, _usage) = provider
        .complete(&model_config, "test-session", "", &messages, &[])
        .await
        .expect("cold completion should succeed");
    let cold_elapsed = start.elapsed();

    let text = response.as_concat_text();
    assert!(!text.is_empty(), "cold start should produce a response");
    println!("Cold start: {cold_elapsed:.2?}, response length: {}", text.len());

    // Warm run (model already loaded)
    let messages2 = vec![Message::user().with_text("what is the capital of France?")];
    let start2 = Instant::now();
    let (response2, _usage2) = provider
        .complete(&model_config, "test-session", "", &messages2, &[])
        .await
        .expect("warm completion should succeed");
    let warm_elapsed = start2.elapsed();

    let text2 = response2.as_concat_text();
    assert!(!text2.is_empty(), "warm run should produce a response");
    println!("Warm run: {warm_elapsed:.2?}, response length: {}", text2.len());
    assert!(
        warm_elapsed < cold_elapsed,
        "warm run ({warm_elapsed:.2?}) should be faster than cold start ({cold_elapsed:.2?})"
    );
}

#[tokio::test]
#[ignore]
async fn test_local_inference_large_prompt() {
    let model_config = ModelConfig::new(TEST_MODEL).expect("valid model config");
    let provider = create("local", model_config.clone(), Vec::new())
        .await
        .expect("provider creation should succeed");

    // Build a large prompt (~3500 tokens) to exercise prefill performance
    let padding = "You are Goose, a highly capable AI assistant.\n".repeat(80);
    let prompt = format!("{padding}\nNow answer this: what is the capital of Moldova?");
    let messages = vec![Message::user().with_text(&prompt)];

    let start = Instant::now();
    let (response, _usage) = provider
        .complete(&model_config, "test-session", "", &messages, &[])
        .await
        .expect("large prompt completion should succeed");
    let elapsed = start.elapsed();

    let text = response.as_concat_text();
    assert!(!text.is_empty(), "large prompt should produce a response");
    println!(
        "Large prompt: {elapsed:.2?}, prompt ~{} chars, response length: {}",
        prompt.len(),
        text.len()
    );
}
