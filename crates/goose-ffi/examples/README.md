# Goose FFI Examples

This directory contains examples demonstrating how to use the Goose FFI interface with the new FFI-friendly agent design in both Python and Kotlin.

## Prerequisites

1. Build the Goose FFI library:
   ```bash
   cd ../../..  # Go to project root
   cargo build
   ```

2. Set up environment variables:
   ```bash
   export DATABRICKS_API_KEY="your-api-key"
   export DATABRICKS_HOST="your-databricks-host"
   ```

## Python Examples

### 1. goose_agent.py - Basic FFI Usage

This example demonstrates the basic usage of the FFI-friendly agent:
- Creating an agent with a provider
- Using the ReplyState system
- Handling tool approvals
- Processing conversations

Run it with:
```bash
python3 goose_agent.py
```

### 2. test_ffi.py - Comprehensive Tests

This script contains unit tests for the FFI implementation:
- Basic conversation flow
- Tool approval workflow
- Error handling
- Memory management

Run the tests with:
```bash
python3 test_ffi.py
```

### 3. end_to_end_example.py - Complete Frontend Tools Demo

This example shows a complete end-to-end flow with frontend tools:
- Calculator tool implementation
- Weather tool implementation
- Conversation management
- Tool execution and approval

Run it with:
```bash
python3 end_to_end_example.py
```

## Kotlin Example

### GooseExample.kt - Kotlin FFI Usage

This example demonstrates how to use the FFI interface from Kotlin using JNA:
- Loading the native library
- Creating an agent
- Using the ReplyState system
- Handling tool approvals

Run it with:
```bash
./gradlew run
```

Or if you don't have Gradle installed:
```bash
gradle wrapper
./gradlew run
```

## Running All Examples

Use the provided script to run all examples:
```bash
./run_examples.sh
```

## Key Concepts

### ReplyState

The `ReplyState` object manages the conversation flow through different states:
- `READY`: Initial state
- `WAITING_FOR_PROVIDER`: Waiting for LLM response
- `MESSAGE_YIELDED`: A message is available
- `WAITING_FOR_TOOL_APPROVAL`: Tool requests need approval
- `PROCESSING_TOOLS`: Processing tool results
- `COMPLETED`: Conversation finished
- `ERROR`: An error occurred

### Tool Handling

Tools are handled through a request/approval flow:
1. Agent requests a tool
2. Frontend receives tool request
3. Frontend approves/denies the tool
4. If approved, frontend executes the tool
5. Results are sent back to the agent

### Frontend Tools

Frontend tools are tools that execute on the client side:
- Calculator: Performs arithmetic operations
- Weather: Provides mock weather data
- Custom tools can be added easily

## Common Issues

1. **Library not found**: Make sure you've built the project and the library path is correct
2. **Environment variables**: Ensure DATABRICKS_API_KEY and DATABRICKS_HOST are set
3. **Memory leaks**: Always free resources using the provided free functions
4. **Tool approval**: Tools must be approved before execution

## Architecture

The FFI-friendly design uses a state machine pattern instead of async streams:

```
User Input -> Create ReplyState -> Start Conversation
                                        |
                                        v
                              Process States in Loop
                                        |
                    +-------------------+-------------------+
                    |                   |                   |
            MESSAGE_YIELDED    WAITING_FOR_TOOL    PROCESSING_TOOLS
                    |                   |                   |
            Display Message      Approve/Deny         Execute Tool
                    |                   |                   |
                    +-------------------+-------------------+
                                        |
                                        v
                                    COMPLETED
```

This design makes it easy to use from languages that don't support Rust's async streams.

## Flow Diagrams

See [flow_diagram.md](flow_diagram.md) for detailed sequence and state diagrams.