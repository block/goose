# Experimental Features

This guide covers experimental features in Goose that are available but may not be fully stable or could change in future releases.

## Ollama Tool Shim

The Ollama tool shim is an experimental feature that enables tool calling capabilities for language models that don't natively support tool calling (like DeepSeek). It works by using Ollama models to interpret the primary model's responses and convert them into valid tool calls.

### How it Works

1. The primary model (e.g., DeepSeek) is instructed to output JSON for intended tool usage
2. An Ollama-based interpretive model translates the primary model's message into valid JSON
3. The JSON is then converted into valid tool calls to be invoked

### Setup Requirements

1. Make sure you have Ollama installed and running
2. For optimal performance, run the Ollama server with an increased context length:
   ```bash
   OLLAMA_CONTEXT_LENGTH=50000 ollama serve
   ```
   Note: This feature requires building Ollama from source as it hasn't been released yet.

### Usage

Enable the tool shim by setting the `GOOSE_TOOLSHIM` environment variable:

```bash
GOOSE_TOOLSHIM=1 cargo run --bin goose session
```

### Configuration

- Default interpreter model: Mistral
- Override the interpreter model using `GOOSE_TOOLSHIM_MODEL`:
  ```bash
  GOOSE_TOOLSHIM=1 GOOSE_TOOLSHIM_MODEL=llama3.2 cargo run --bin goose session
  ```

### Recommended Configuration

The most effective combination currently is:
- Primary model: DeepSeek-R1 via OpenRouter
- Tool shim enabled with default settings

### Known Limitations

- Requires Ollama to be installed and running
- Increased context length feature requires building Ollama from source
- May introduce slight latency due to the additional interpretation step