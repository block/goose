/// Spike test for OpenAI Responses API with gpt-5.1-codex
///
/// This is a standalone program to validate the Responses API works
/// before integrating it into the provider system.
///
/// Run with: cargo run --example test_responses_api
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    id: String,
    object: String,
    created_at: i64,
    status: String,
    model: String,
    output: Vec<OutputItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum OutputItem {
    Reasoning {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<Vec<String>>,
    },
    Message {
        id: String,
        status: String,
        role: String,
        content: Vec<ContentBlock>,
    },
    FunctionCall {
        id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        name: String,
        arguments: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum ContentBlock {
    OutputText {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Vec<Value>>,
    },
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ReasoningInfo {
    effort: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    input_tokens: i32,
    output_tokens: i32,
    total_tokens: i32,
}

async fn test_simple_request() -> Result<()> {
    println!("=== Testing Simple Responses API Request ===\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let payload = json!({
        "model": "gpt-5.1-codex",
        "input": "Write a simple hello world function in Python",
        "instructions": "You are a helpful coding assistant. Provide clear, concise code."
    });

    println!("Request payload:");
    println!("{}\n", serde_json::to_string_pretty(&payload)?);

    let response = client
        .post("https://api.openai.com/v1/responses")
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    println!("Response status: {}\n", status);

    let response_text = response.text().await?;
    println!("Raw response:");
    println!("{}\n", response_text);

    if status.is_success() {
        let parsed: Response = serde_json::from_str(&response_text)?;
        println!("Parsed response:");
        println!("  ID: {}", parsed.id);
        println!("  Model: {}", parsed.model);
        println!("  Status: {}", parsed.status);
        println!("  Output items: {}", parsed.output.len());

        for item in &parsed.output {
            match item {
                OutputItem::Reasoning { id, .. } => {
                    println!("    - Reasoning block (id: {})", id);
                }
                OutputItem::Message {
                    id, role, content, ..
                } => {
                    println!("    - Message (id: {}, role: {})", id, role);
                    for content_block in content {
                        match content_block {
                            ContentBlock::OutputText { text, .. } => {
                                println!("      Text: {}", text);
                            }
                            ContentBlock::ToolCall { name, .. } => {
                                println!("      Tool call: {}", name);
                            }
                        }
                    }
                }
                OutputItem::FunctionCall {
                    id,
                    name,
                    arguments,
                    ..
                } => {
                    println!(
                        "    - Function call (id: {}, name: {}, args: {})",
                        id, name, arguments
                    );
                }
            }
        }

        if let Some(reasoning) = parsed.reasoning {
            println!("  Reasoning effort: {}", reasoning.effort);
        }
        if let Some(usage) = parsed.usage {
            println!(
                "  Usage: input={}, output={}, total={}",
                usage.input_tokens, usage.output_tokens, usage.total_tokens
            );
        }
    }

    Ok(())
}

async fn test_with_tools() -> Result<()> {
    println!("\n=== Testing Responses API with Tools ===\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let payload = json!({
        "model": "gpt-5.1-codex",
        "input": "What's 25 * 17? Use the calculator.",
        "instructions": "You are a helpful assistant with access to tools.",
        "tools": [
            {
                "type": "function",
                "name": "calculator",
                "description": "Perform basic arithmetic operations",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"],
                            "description": "The operation to perform"
                        },
                        "a": {
                            "type": "number",
                            "description": "First number"
                        },
                        "b": {
                            "type": "number",
                            "description": "Second number"
                        }
                    },
                    "required": ["operation", "a", "b"]
                }
            }
        ]
    });

    println!("Request payload:");
    println!("{}\n", serde_json::to_string_pretty(&payload)?);

    let response = client
        .post("https://api.openai.com/v1/responses")
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    println!("Response status: {}\n", status);

    let response_text = response.text().await?;
    println!("Raw response:");
    println!("{}\n", response_text);

    if status.is_success() {
        let parsed: Response = serde_json::from_str(&response_text)?;
        println!("Parsed response:");
        println!("  ID: {}", parsed.id);
        println!("  Model: {}", parsed.model);
        println!("  Output items: {}", parsed.output.len());

        for item in &parsed.output {
            match item {
                OutputItem::Reasoning { id, .. } => {
                    println!("    - Reasoning block (id: {})", id);
                }
                OutputItem::Message {
                    id, role, content, ..
                } => {
                    println!("    - Message (id: {}, role: {})", id, role);
                    for content_block in content {
                        match content_block {
                            ContentBlock::OutputText { text, .. } => {
                                println!("      Text: {}", text);
                            }
                            ContentBlock::ToolCall { name, input, .. } => {
                                println!("      Tool call: {} with input: {}", name, input);
                            }
                        }
                    }
                }
                OutputItem::FunctionCall {
                    id,
                    name,
                    arguments,
                    ..
                } => {
                    println!(
                        "    - Function call (id: {}, name: {}, args: {})",
                        id, name, arguments
                    );
                }
            }
        }
    }

    Ok(())
}

async fn test_structured_input() -> Result<()> {
    println!("\n=== Testing Responses API with Structured Input ===\n");

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Using structured input similar to messages format
    let payload = json!({
        "model": "gpt-5.1-codex",
        "input": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Write a function to calculate fibonacci numbers"
                    }
                ]
            }
        ],
        "instructions": "You are an expert programmer. Write clean, efficient code."
    });

    println!("Request payload:");
    println!("{}\n", serde_json::to_string_pretty(&payload)?);

    let response = client
        .post("https://api.openai.com/v1/responses")
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    println!("Response status: {}\n", status);

    let response_text = response.text().await?;
    println!("Raw response:");
    println!("{}\n", response_text);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("OpenAI Responses API Spike Test for gpt-5.1-codex\n");
    println!("================================================\n");

    // Test 1: Simple request
    if let Err(e) = test_simple_request().await {
        eprintln!("Simple request test failed: {}", e);
    }

    // Test 2: Request with tools
    if let Err(e) = test_with_tools().await {
        eprintln!("Tools test failed: {}", e);
    }

    // Test 3: Structured input
    if let Err(e) = test_structured_input().await {
        eprintln!("Structured input test failed: {}", e);
    }

    println!("\n=== Spike Test Complete ===");

    Ok(())
}
