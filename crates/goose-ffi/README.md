# Goose FFI

Foreign Function Interface (FFI) for the Goose AI agent framework, allowing integration with other programming languages.

## Overview

The Goose FFI library provides C-compatible bindings for the Goose AI agent framework, enabling you to:

- Create and manage Goose agents from any language with C FFI support
- Configure and use the Databricks AI provider for now but is extensible to other providers as needed
- Send messages to agents and receive responses

## Building

To build the FFI library, you'll need Rust and Cargo installed. Then run:

```bash
# Build the library in debug mode
cargo build --package goose_ffi

# Build the library in release mode (recommended for production)
cargo build --release --package goose_ffi
```

This will generate a dynamic library (.so, .dll, or .dylib depending on your platform) in the `target` directory, and automatically generate the C header file in the `include` directory.

You can also build cross-platform binaries using cross command. For example to build for linux x86_64 architecture from Mac would require running

```bash
CROSS_BUILD_OPTS="--platform linux/amd64 --no-cache" CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build -p goose-ffi --release --target x86_64-unknown-linux-gnu --no-default-features
```
Note that this works only for gnu linux as it requires glibc.

## Generated C Header

The library uses cbindgen to automatically generate a C header file (`goose_ffi.h`) during the build process. This header contains all the necessary types and function declarations to use the library from C or any language with C FFI support.

## Examples

The FFI library includes examples in multiple languages to demonstrate how to use it.

### Python Example

The `examples/goose_agent.py` demonstrates using the FFI library from Python with ctypes. It shows:

1. How to create a proper Python wrapper around the Goose FFI interface
2. Loading the shared library dynamically based on platform
3. Setting up C-compatible structures
4. Creating an object-oriented API for easier use

Note: Tool callback functionality shown in earlier versions is not currently available and will be implemented in a future release.

To run the Python example:

```bash
# First, build the FFI library
cargo build --release --package goose_ffi

# Then set the environment variables & run the example
DATABRICKS_HOST=... DATABRICKS_API_KEY=... python crates/goose-ffi/examples/goose_agent.py
```

You need to have Python 3.6+ installed with the `ctypes` module (included in standard library).


```
> Tell me about the Eiffel Tower
```

The agent will respond with information about the Eiffel Tower.

## Using from Other Languages

The Goose FFI library can be used from many programming languages with C FFI support, including:

- Python (via ctypes or cffi)
- JavaScript/Node.js (via node-ffi)
- Ruby (via fiddle)
- C#/.NET (via P/Invoke)
- Go (via cgo)
- Java / Kotlin (via JNA or JNI)

Check the documentation for FFI support in your language of choice for details on how to load and use a C shared library.

## Provider Configuration

The FFI interface uses a provider type enumeration to specify which AI provider to use:

```c
// C enum (defined in examples/simple_agent.c)
typedef enum {
    PROVIDER_DATABRICKS = 0,  // Databricks AI provider
} ProviderType;
```

```python
# Python enum (defined in examples/goose_agent.py)
class ProviderType(IntEnum):
    DATABRICKS = 0  # Databricks AI provider
```

Currently, only the Databricks provider (provider_type = 0) is supported. If you attempt to use any other provider type, an error will be returned.

### Environment-based Configuration

The library supports configuration via environment variables, which makes it easier to use in containerized or CI/CD environments without hardcoding credentials:

#### Databricks Provider (type = 0)

```
DATABRICKS_API_KEY=dapi...     # Databricks API key
DATABRICKS_HOST=...            # Databricks host URL (e.g., "https://your-workspace.cloud.databricks.com")
```

These environment variables will be used automatically if you don't provide the corresponding parameters when creating an agent.

## Thread Safety

The FFI library is designed to be thread-safe. Each agent instance is independent, and tools callbacks are handled in a thread-safe manner. However, the same agent instance should not be used from multiple threads simultaneously without external synchronization.

## Error Handling

Functions that can fail return either null pointers or special result structures that indicate success or failure. Always check return values and clean up resources using the appropriate free functions.

## Memory Management

The FFI interface handles memory allocation and deallocation. Use the provided free functions (like `goose_free_string` and `goose_free_async_result`) to release resources when you're done with them.
