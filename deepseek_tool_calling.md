# DeepSeek Tool Calling Implementation

## Overview

This document describes the implementation of tool calling support for DeepSeek models in the Goose project. The implementation embeds tools directly in the system prompt and handles the special format used by DeepSeek models for tool calls.

## Key Components

### 1. Tool Definition in System Prompt

For DeepSeek models, tools are embedded directly in the system prompt following the format shown in the Hugging Face example:

```
You are a helpful Assistant.

## Tools

### Function

You have the following functions available:

- `get_current_weather`:
```json
{
    "name": "get_current_weather",
    "description": "Get the current weather in a given location",
    "parameters": {
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "The city and state, e.g. San Francisco, CA"
            },
            "unit": {
                "type": "string",
                "enum": [
                    "celsius",
                    "fahrenheit"
                ]
            }
        },
        "required": [
            "location"
        ]
    }
}
```
```

### 2. Custom Response Parsing

DeepSeek models return tool calls in a special format with markers like `<｜tool▁calls▁begin｜>` and `<｜tool▁call▁end｜>`. Our implementation includes a custom parser that:

1. Detects these special markers
2. Extracts the function name and arguments
3. Converts them to the standard Goose tool call format

### 3. Implementation Details

The implementation consists of three main functions:

1. `is_deepseek_model()`: Detects when we're working with a DeepSeek model
2. `create_system_prompt_with_tools()`: Embeds tools in the system prompt for DeepSeek models
3. `parse_deepseek_response()`: Parses the special format used by DeepSeek models for tool calls

### 4. Known Limitations

As noted in the `deepseek.md` file, DeepSeek models have difficulty handling the full conversation loop with tool calling. After receiving a tool response, the model encounters errors when attempting to continue the conversation:

```
Error: "Invalid request. Please check the parameters and try again. Details: can only concatenate str (not \"dict\") to str"
```

This appears to be a Python error in the HuggingFace API backend when processing tool results with DeepSeek models.

## Usage

To use the DeepSeek model with tool calling:

```bash
GOOSE_PROVIDER=huggingface GOOSE_MODEL=deepseek-ai/DeepSeek-V3-0324 cargo run --bin goose run -t "run ls -la"
```

## Future Improvements

1. Implement a better solution for handling tool responses to avoid the API error
2. Add support for multiple tool calls in a single response
3. Improve error handling and recovery
4. Work with HuggingFace to improve their DeepSeek integration for full tool calling support