# Experimental Features

Goose is an open source project that is constantly being improved, and new features are added regularly. Some of these features are considered experimental, meaning they are still in development and may not be fully stable or ready for production use. This guide covers how to enable and use experimental features in Goose, as well as how to provide feedback on them.


## Available Experimental Features

:::note
The list of experimental features may change as Goose development progresses. Some features may be promoted to stable features, while others might be modified or removed.This section will be updated with specific experimental features as they become available
:::

### Ollama Tool Shim

The Ollama tool shim is an experimental feature that enables tool calling capabilities for language models that don't natively support tool calling (like DeepSeek). It works by instructing the primary model to output json for intended tool usage, the interpretive model uses ollama structured outputs to translate the primary model's message into valid json, and then that json is translated into valid tool calls to be invoked.


#### How to use the Ollama Tool Shim

1. Make sure you have [Ollama](https://ollama.com/download) installed and running
2. The default interpreter model is `mistral-nemo`, if you want to proceed with this, you have to pull it from ollama server by running:

   ```bash
   ollama pull mistral-nemo
   ```
3. If you want to use a different model, make sure to pull it first from the Ollama server. Then override the default interpreter model using the `GOOSE_TOOLSHIM_OLLAMA_MODEL` environment variable. For example, to use the `llama3.2` model, run:

   ```bash
   ollama pull llama3.2
   ```
   Then,

   ```bash
   GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2 
   ```

4. For optimal performance, run the Ollama server with an increased context length:
   ```bash
   OLLAMA_CONTEXT_LENGTH=32768 ollama serve
   ```

5. Enable the tool shim by setting the `GOOSE_TOOLSHIM` environment variable:

   ```bash
   GOOSE_TOOLSHIM=1 
   ```

Start a new Goose session with your tool shim preferences:

  ```bash
  GOOSE_TOOLSHIM=1 GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2 cargo run --bin goose session
  ```


## Feedback

If you encounter any issues with these features, check if the issue is already reported in the [GitHub issues](https://github.com/goose/goose/issues) or join the [Discord community](https://discord.gg/block-opensource) to share.