# ToolShim Module

The ToolShim module provides a reusable component for interpreting and augmenting LLM outputs with tool calls, regardless of whether the underlying model natively supports tool/function calling.

## Overview

ToolShim addresses the challenge of working with models that don't natively support tools by:

1. Taking the text output from any LLM
2. Sending it to a separate "interpreter" model (which can be the same or different model)
3. Using structured output to extract tool call intentions
4. Converting those intentions back into proper tool calls
5. Augmenting the original message with the extracted tool calls

## Key Components

### ToolInterpreter Trait

The core of ToolShim is the `ToolInterpreter` trait, which defines the interface for any model that can interpret text and extract tool calls:

```rust
#[async_trait::async_trait]
pub trait ToolInterpreter {
    async fn interpret_to_tool_calls(&self, content: &str, tools: &[Tool]) -> Result<Vec<ToolCall>, ProviderError>;
}
```

### Implementations

The module provides an implementation for Ollama:

- `OllamaInterpreter`: Uses Ollama's structured output API to interpret tool calls

### Helper Functions

- `augment_message_with_tool_calls`: A utility function that takes any message, extracts text content, sends it to an interpreter, and adds any detected tool calls back to the message.
- `process_interpreter_response`: Processes the structured output response from an interpreter and extracts tool calls
- `default_system_prompt`: Provides the default system prompt for tool call interpretation
- `default_format_schema`: Provides the default JSON schema for structured output

## Usage Example

```rust
// Create an interpreter 
let interpreter = OllamaInterpreter::new("http://localhost:11434".to_string());

// Get a response from any LLM
let llm_message = my_llm.generate("What's the weather like in New York?").await?;

// Define available tools
let tools = vec![
    Tool::new(
        "weather",
        "Get weather information for a location",
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City or location"
                }
            },
            "required": ["location"]
        }),
    ),
];

// Augment the message with tool calls
let augmented_message = augment_message_with_tool_calls(&interpreter, llm_message, &tools).await?;
```

### Enhanced Provider

You can create enhanced providers that automatically use ToolShim as shown with the included `EnhancedOllamaProvider`:

```rust
let enhanced_provider = EnhancedOllamaProvider::new(
    ModelConfig::new("llama3"), 
    true // enable tool shim
)?;

// Use the provider as normal
let (message, usage) = enhanced_provider.complete(
    "You are a helpful assistant.", 
    &messages, 
    &tools
).await?;

// The message will automatically have tool calls added if detected
```

## Benefits

- **Modular**: Can be used with any provider implementation
- **Extensible**: Easy to add new interpreter implementations for different providers
- **Reusable**: Common logic is abstracted away from specific provider implementations
- **Separation of Concerns**: Keeps the base providers simple while adding tool functionality where needed

## Implementation Notes

- The interpreter model can be the same as the main model or a different one
- For optimal results, use an interpreter model that's good at JSON generation
- The system prompt and schema can be customized to fit specific needs
- The module handles various response formats to maximize compatibility