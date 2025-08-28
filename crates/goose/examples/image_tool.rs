use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use dotenvy::dotenv;
use goose::conversation::message::Message;
use goose::providers::{
    bedrock::BedrockProvider, databricks::DatabricksProvider, openai::OpenAiProvider,
};
use mcp_core::tool::ToolCall;
use rmcp::model::{Content, Tool};
use rmcp::object;
use serde_json::json;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Create providers
    let providers: Vec<Box<dyn goose::providers::base::Provider + Send + Sync>> = vec![
        Box::new(DatabricksProvider::default()),
        Box::new(OpenAiProvider::default()),
        Box::new(BedrockProvider::default()),
    ];

    for provider in providers {
        // Read and encode test image
        let image_data = fs::read("crates/goose/examples/test_assets/test_image.png")?;
        let base64_image = BASE64.encode(image_data);

        // Create a message sequence that includes a tool response with both text and image
        let messages = vec![
            Message::user().with_text("Read the image at ./test_image.png please"),
            Message::assistant().with_tool_request(
                "000",
                Ok(ToolCall::new(
                    "view_image",
                    json!({"path": "./test_image.png"}),
                )),
            ),
            Message::user()
                .with_tool_response("000", Ok(vec![Content::image(base64_image, "image/png")])),
        ];

        // Get a response from the model about the image
        let input_schema = object!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "default": null,
                    "description": "The path to the image"
                },
            }
        });
        let (response, usage) = provider
            .complete(
                "You are a helpful assistant. Please describe any text you see in the image.",
                &messages,
                &[Tool::new("view_image", "View an image", input_schema)],
            )
            .await?;

        // Print the response and usage statistics
        println!("\nResponse from AI:");
        println!("---------------");
        for content in response.content {
            println!("{:?}", content);
        }
        println!("\nToken Usage:");
        println!("------------");
        println!("Input tokens: {:?}", usage.usage.input_tokens);
        println!("Output tokens: {:?}", usage.usage.output_tokens);
        println!("Total tokens: {:?}", usage.usage.total_tokens);
    }

    Ok(())
}
