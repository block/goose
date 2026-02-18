// Simple test to measure LocalInferenceProvider performance
use goose::conversation::message::Message;
use goose::model::ModelConfig;
use goose::providers::base::Provider;
use goose::providers::local_inference::LocalInferenceProvider;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let config = ModelConfig::new("Llama-3.2-1B-Instruct")?;

    println!("Creating provider...");
    let provider = LocalInferenceProvider::from_env(config.clone(), vec![]).await?;

    // Test 1: First run (cold - includes model loading)
    println!("\n=== Test 1: Cold start (includes model loading) ===");
    println!("Testing with prompt: 'what is the capital of Moldova?'");
    let messages = vec![Message::user().with_text("what is the capital of Moldova?")];

    let start = Instant::now();
    let (response, _usage) = provider
        .complete(&config, "test-session", "", &messages, &[])
        .await?;
    let elapsed = start.elapsed();

    println!("\nResponse: {}", response.as_concat_text());
    println!("Time elapsed: {:.2?}", elapsed);

    let char_count = response.as_concat_text().len();
    let estimated_tokens = char_count / 4;
    let tokens_per_sec = estimated_tokens as f64 / elapsed.as_secs_f64();
    println!("Estimated speed: ~{:.1} tokens/sec", tokens_per_sec);

    // Test 2: Second run (warm - model already loaded)
    println!("\n=== Test 2: Warm run (model cached) ===");
    println!("Testing with prompt: 'what is the capital of France?'");
    let messages2 = vec![Message::user().with_text("what is the capital of France?")];

    let start2 = Instant::now();
    let (response2, _usage2) = provider
        .complete(&config, "test-session", "", &messages2, &[])
        .await?;
    let elapsed2 = start2.elapsed();

    println!("\nResponse: {}", response2.as_concat_text());
    println!("Time elapsed: {:.2?}", elapsed2);

    let char_count2 = response2.as_concat_text().len();
    let estimated_tokens2 = char_count2 / 4;
    let tokens_per_sec2 = estimated_tokens2 as f64 / elapsed2.as_secs_f64();
    println!("Estimated speed: ~{:.1} tokens/sec", tokens_per_sec2);

    // Test 3: Large prompt (~3500 tokens, under 4096 context limit)
    println!("\n=== Test 3: Large prompt (~3500 tokens) ===");

    // Create a realistic long prompt similar to what Goose would have
    // Including system instructions, tool definitions, examples, etc.
    let realistic_system = r#"
You are Goose, a highly capable AI programming assistant. You help developers write, debug, and maintain code.

Core Capabilities:
- Write production-quality code in any programming language
- Debug complex issues and provide fixes
- Refactor code for better maintainability
- Explain technical concepts clearly
- Review code and suggest improvements
- Design system architectures
- Write tests and documentation

Guidelines:
- Always prioritize correctness and clarity
- Follow best practices and idioms for the language
- Consider edge cases and error handling
- Write self-documenting code with clear variable names
- Add comments only when the logic isn't self-evident
- Prefer simple solutions over complex ones
- Test your code before suggesting it

Available Tools:
"#.repeat(3); // Stay well under limit

    let tool_definitions = r#"
Tool: read_file
Description: Read contents of a file from the filesystem
Parameters:
  - path (string, required): Absolute path to the file
  - encoding (string, optional): File encoding, defaults to utf-8
Returns: File contents as string
Example usage: read_file(path="/home/user/code.py")

Tool: write_file
Description: Write or overwrite a file on the filesystem
Parameters:
  - path (string, required): Absolute path to the file
  - content (string, required): Content to write to file
  - create_dirs (boolean, optional): Create parent directories if needed
Returns: Success confirmation
Example usage: write_file(path="/home/user/new.py", content="print('hello')")

Tool: list_directory
Description: List contents of a directory
Parameters:
  - path (string, required): Absolute path to directory
  - recursive (boolean, optional): Recursively list subdirectories
  - pattern (string, optional): Glob pattern to filter files
Returns: List of file and directory paths
Example usage: list_directory(path="/home/user/project", pattern="*.py")
"#
    .repeat(6); // Stay well under limit

    let examples = r#"
Example conversation:
User: Help me write a function to parse JSON
Assistant: I'll help you write a JSON parser. Here's a robust implementation:

```python
import json
from typing import Any, Optional

def parse_json(json_string: str) -> Optional[dict[str, Any]]:
    """Parse JSON string and return dict, or None if invalid."""
    try:
        return json.loads(json_string)
    except json.JSONDecodeError as e:
        print(f"Invalid JSON: {e}")
        return None
```

This handles errors gracefully and uses type hints for clarity.
"#
    .repeat(8); // Stay well under limit

    let full_prompt = format!(
        "{}\n\n{}\n\n{}\n\nNow answer this: what is the capital of Moldova?",
        realistic_system, tool_definitions, examples
    );

    let messages3 = vec![Message::user().with_text(&full_prompt)];

    let estimated_tokens = full_prompt.len() / 4;
    println!(
        "Prompt length: {} chars, estimated ~{} tokens (model limit: 4096)",
        full_prompt.len(),
        estimated_tokens
    );

    let start3 = Instant::now();
    let (response3, _usage3) = provider
        .complete(&config, "test-session", "", &messages3, &[])
        .await?;
    let elapsed3 = start3.elapsed();

    let response_text = response3.as_concat_text();
    println!(
        "\nResponse ({} chars): {}",
        response_text.len(),
        if response_text.len() > 200 {
            format!(
                "{}...",
                &response_text.chars().take(200).collect::<String>()
            )
        } else {
            response_text.clone()
        }
    );
    println!("Total time: {:.2?}", elapsed3);
    println!(
        "Estimated prefill speed: ~{:.1} tokens/sec",
        estimated_tokens as f64 / elapsed3.as_secs_f64()
    );

    Ok(())
}
