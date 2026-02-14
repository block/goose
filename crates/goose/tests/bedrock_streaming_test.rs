use anyhow::Result;
use dotenvy::dotenv;
use futures::StreamExt;
use goose::conversation::message::Message;
use goose::providers::base::ProviderUsage;
use goose::providers::bedrock::BEDROCK_DEFAULT_MODEL;
use goose::providers::create_with_named_model;
use rmcp::model::Tool;
use rmcp::object;

fn has_bedrock_credentials() -> bool {
    // AWS_PROFILE alone is sufficient, or we need both ACCESS_KEY_ID and SECRET_ACCESS_KEY
    let has_profile = std::env::var("AWS_PROFILE").is_ok();
    let has_access_keys = std::env::var("AWS_ACCESS_KEY_ID").is_ok()
        && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok();
    has_profile || has_access_keys
}

#[tokio::test]
async fn test_bedrock_streaming_basic() -> Result<()> {
    dotenv().ok();

    if !has_bedrock_credentials() {
        eprintln!("Skipping Bedrock streaming test: no AWS credentials in environment");
        return Ok(());
    }

    // Check if streaming is supported
    // Note: Provider is registered as "aws_bedrock", not "bedrock"
    let provider = create_with_named_model("aws_bedrock", BEDROCK_DEFAULT_MODEL, vec![]).await?;

    assert!(
        provider.supports_streaming(),
        "Bedrock provider should support streaming"
    );

    // Test basic streaming
    let message = Message::user().with_text("Say hello in 3 words");
    let system_prompt = "You are a helpful assistant.";

    let mut stream = provider
        .stream("test-session-id", system_prompt, &[message], &[])
        .await?;

    let mut message_count = 0;
    let mut text_content = String::new();

    // Collect streamed messages
    while let Some(result) = stream.next().await {
        match result {
            Ok((Some(msg), usage_opt)) => {
                message_count += 1;

                // Print debug info
                println!(
                    "Stream chunk {}: role={:?}, content_items={}",
                    message_count,
                    msg.role,
                    msg.content.len()
                );

                // Collect text content
                for content in &msg.content {
                    if let goose::conversation::message::MessageContent::Text(text) = content {
                        text_content.push_str(&text.text);
                        println!("  Text chunk: {}", text.text);
                    }
                }

                // Print usage if available
                if let Some(usage) = usage_opt {
                    println!(
                        "  Usage - Input: {:?}, Output: {:?}",
                        usage.usage.input_tokens, usage.usage.output_tokens
                    );
                }
            }
            Ok((None, Some(usage))) => {
                println!(
                    "Final usage - Input: {:?}, Output: {:?}",
                    usage.usage.input_tokens, usage.usage.output_tokens
                );
            }
            Ok((None, None)) => {
                println!("Stream end marker received");
            }
            Err(e) => {
                eprintln!("Stream error: {:?}", e);
                return Err(e.into());
            }
        }
    }

    println!("\n=== Stream Summary ===");
    println!("Total chunks received: {}", message_count);
    println!("Final text: {}", text_content);

    assert!(
        message_count > 0,
        "Should receive at least one streamed message"
    );
    assert!(
        !text_content.is_empty(),
        "Should receive text content in stream"
    );

    Ok(())
}

#[tokio::test]
async fn test_bedrock_streaming_with_tools() -> Result<()> {
    dotenv().ok();

    if !has_bedrock_credentials() {
        eprintln!("Skipping Bedrock streaming-with-tools test: no AWS credentials in environment");
        return Ok(());
    }

    let provider = create_with_named_model("aws_bedrock", BEDROCK_DEFAULT_MODEL, vec![]).await?;

    // Create a simple tool
    let weather_tool = Tool::new(
        "get_weather",
        "Get the weather for a location",
        object!(
            {
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city name"
                    }
                }
            }
        ),
    );

    let message = Message::user().with_text("What's the weather in San Francisco?");
    let system_prompt = "You are a helpful weather assistant. Always use the get_weather tool to answer weather questions.";

    let mut stream = provider
        .stream(
            "test-session-id",
            system_prompt,
            &[message],
            &[weather_tool],
        )
        .await?;

    let mut received_tool_request = false;
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok((Some(msg), _)) => {
                message_count += 1;

                for content in &msg.content {
                    if matches!(
                        content,
                        goose::conversation::message::MessageContent::ToolRequest(_)
                    ) {
                        received_tool_request = true;
                        println!("✓ Received tool request in stream");
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Stream error: {:?}", e);
                return Err(e.into());
            }
        }
    }

    println!("Total chunks: {}", message_count);

    assert!(
        message_count > 0,
        "Streaming should return at least one chunk"
    );

    if !received_tool_request {
        println!(
            "Warning: no tool request seen in stream; provider may answer directly without tools"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_bedrock_streaming_vs_non_streaming_consistency() -> Result<()> {
    dotenv().ok();

    if !has_bedrock_credentials() {
        eprintln!(
            "Skipping Bedrock streaming vs non-streaming consistency test: no AWS credentials in environment"
        );
        return Ok(());
    }

    let provider = create_with_named_model("aws_bedrock", BEDROCK_DEFAULT_MODEL, vec![]).await?;

    let message = Message::user().with_text("What is 2+2?");
    let system_prompt = "Answer very briefly.";

    // Get non-streaming response
    let (non_stream_msg, non_stream_usage) = provider
        .complete(
            "test-session-id",
            system_prompt,
            std::slice::from_ref(&message),
            &[],
        )
        .await?;

    println!("Non-streaming response: {:?}", non_stream_msg.content);
    println!("Non-streaming usage: {:?}", non_stream_usage);

    // Get streaming response
    let mut stream = provider
        .stream("test-session-id", system_prompt, &[message], &[])
        .await?;

    let mut stream_text = String::new();
    let mut stream_usage: Option<ProviderUsage> = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok((Some(msg), _)) => {
                for content in &msg.content {
                    if let goose::conversation::message::MessageContent::Text(text) = content {
                        stream_text.push_str(&text.text);
                    }
                }
            }
            Ok((None, Some(usage))) => {
                stream_usage = Some(usage);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Stream error: {:?}", e);
                return Err(e.into());
            }
        }
    }

    println!("Streaming response: {}", stream_text);
    println!("Streaming usage: {:?}", stream_usage);

    // Both should produce text
    let non_stream_text = non_stream_msg
        .content
        .iter()
        .filter_map(|c| {
            if let goose::conversation::message::MessageContent::Text(t) = c {
                Some(t.text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    assert!(
        !non_stream_text.is_empty(),
        "Non-streaming response should have text"
    );
    assert!(
        !stream_text.is_empty(),
        "Streaming response should have text"
    );

    // Usage should be available
    assert!(
        non_stream_usage.usage.input_tokens.is_some(),
        "Non-streaming usage should have input tokens"
    );
    assert!(
        stream_usage.is_some(),
        "Streaming should provide usage information"
    );

    Ok(())
}
