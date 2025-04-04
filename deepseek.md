# DeepSeek Model Integration Notes

## Overview
This document summarizes findings and solutions for integrating DeepSeek models via the HuggingFace provider in Goose.

## Issues Identified

1. **Tool Choice Parameter Required**
   - DeepSeek models require a `tool_choice` parameter when tools are provided in the request.
   - Unlike other models that support `"auto"` as a value, DeepSeek models require either:
     - `"none"` to disable automatic tool calling
     - A specific tool definition object to force using that tool

2. **Conversation Loop Limitations**
   - DeepSeek models have difficulty handling the full conversation loop with tool calling.
   - After receiving a tool response, the model encounters errors when attempting to continue the conversation.
   - Error: `"Invalid request. Please check the parameters and try again. Details: can only concatenate str (not \"dict\") to str"`
   - This appears to be a Python error in the HuggingFace API backend when processing tool results with DeepSeek models.

3. **Conversational Queries**
   - The model tries to use tools even for conversational queries where no tool is needed.
   - This can lead to inappropriate tool usage for simple questions.

## Solutions Implemented

1. **Tool Choice Parameter**
   - Added logic to detect when tools are present and add the appropriate `tool_choice` parameter.
   - For the initial query with shell tools, we use a specific tool choice to direct the model to use the shell.
   - For other tools or after tool responses, we use `"none"` to prevent errors.

2. **Tool Response Handling**
   - Modified the `complete` method to detect when we're handling a tool response.
   - For tool responses, we create a new conversation that:
     - Embeds the tool output directly in the system message
     - Uses a simple user message to ask for analysis of the output
     - Doesn't include any tools in the request
   - This approach avoids the API error by not using the tool response format at all.

3. **No-Operation Tool**
   - Added a "noop" tool for conversational queries where no actual tool is needed.
   - The noop tool is automatically added when no other tools are present.
   - This prevents the model from trying to use inappropriate tools for simple conversational queries.

## Current Status

- Basic tool calling works - the model can successfully use tools like `developer__shell`.
- After tool execution, the model provides a detailed analysis of the tool output.
- Conversational queries work properly without triggering unnecessary tool calls.
- The solution effectively works around the API limitations while providing a good user experience.

## Future Improvements

1. Make the tool_choice configurable via an environment variable.
2. Add better error handling for specific DeepSeek error messages.
3. Implement a fallback mechanism to use a different model for tool response handling.
4. Explore if newer DeepSeek model versions improve tool calling support.
5. Work with HuggingFace to improve their DeepSeek integration for full tool calling support.

## References

- HuggingFace API Documentation
- DeepSeek Model Documentation
- Error message: `"Invalid request. Please check the parameters and try again. Details: can only concatenate str (not \"dict\") to str"`