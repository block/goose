# Experimental Features

Goose is an open source project that is constantly being improved, and new features are added regularly. Some of these features are considered experimental, meaning they are still in development and may not be fully stable or ready for production use. This guide covers how to enable and use experimental features in Goose, as well as how to provide feedback on them.

## Available Experimental Features

:::note
The list of experimental features may change as Goose development progresses. Some features may be promoted to stable features, while others might be modified or removed.This section will be updated with specific experimental features as they become available
:::

### Ollama Tool Shim

The Ollama tool shim is an experimental feature that enables tool calling capabilities for language models that don't natively support tool calling (like DeepSeek). It works by instructing the primary model to output json for intended tool usage, the interpretive model uses ollama structured outputs to translate the primary model's message into valid json, and then that json is translated into valid tool calls to be invoked.


#### How to use the Ollama Tool Shim

1. Make sure you have Ollama installed and running
2. For optimal performance, run the Ollama server with an increased context length:
   ```bash
   OLLAMA_CONTEXT_LENGTH=50000 ollama serve
   ```
   Note: This feature requires building Ollama from source as it hasn't been released yet.
3. Enable the tool shim by setting the `GOOSE_TOOLSHIM` environment variable:

   ```bash
   GOOSE_TOOLSHIM=1 
   ```

The default interpreter model is `Mistral` but you can override it using the `GOOSE_TOOLSHIM_MODEL` environment variable.

  ```bash
  GOOSE_TOOLSHIM=1 GOOSE_TOOLSHIM_MODEL=llama3.2 cargo run --bin goose session
  ```


## Feedback

If you encounter any issues with these features, check if the issue is already reported in the [GitHub issues](https://github.com/goose/goose/issues) or join the [Discord community](https://discord.gg/block-opensource) to share.