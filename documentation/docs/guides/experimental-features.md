# Experimental Features

Goose is an open source project that is constantly being improved, and new features are added regularly. Some of these features are considered experimental, meaning they are still in development and may not be fully stable or ready for production use. This guide covers how to enable and use experimental features in Goose, as well as how to provide feedback on them.

:::note
Experimental features are new capabilities that are still being tested and refined. While they can provide additional functionality, they may be less stable than standard features.
:::

## Enabling Experimental Features

To access experimental features, run:

```sh
goose configure
```

Select "Goose Settings" from the menu:

```sh
┌ goose-configure
│
◆ What would you like to configure?
| ○ Configure Providers
| ○ Add Extension
| ○ Toggle Extensions
| ○ Remove Extension
// highlight-next-line
| ● Goose Settings (Set the Goose Mode, Tool Output, Experiment and more)
└
```

Then select "Toggle Experiment" option in the menu:
   ```sh
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Goose Settings 
   │
   ◆  What setting would you like to configure?
   │  ○ Goose Mode 
   │  ○ Tool Output 
   // highlight-next-line
   │  ● Toggle Experiment (Enable or disable an experiment feature)
   └  
   ```

## Available Experimental Features

:::note
The list of experimental features may change as Goose development progresses. Some features may be promoted to stable features, while others might be modified or removed.This section will be updated with specific experimental features as they become available
:::

### Smart Approve

The `GOOSE_SMART_APPROVE` experimental feature when enabled allows Goose to review any commands about to be run to determine their sensitivity. Commands with high sensitivity will require approval from the user before it's executed.

Here's an example when a `write` command is about to be executed

```sh
─── text_editor | developer ──────────────────────────
path: ~/Documents/block/goose/test.txt
command: write
file_text: This is a test file.

// highlight-start
◇  Goose would like to call the above tool, do you approve?
│  Yes 
// highlight-end
│
### /Users/yingjiehe/Documents/block/goose/test.txt

This is a test file.


I've created a file named "test.txt" with some simple content. Let me verify that the file was created by checking its contents:

─── text_editor | developer ──────────────────────────
path: ~/Documents/block/goose/test.txt
command: view
```

### Ollama Tool Shim

The Ollama tool shim is an experimental feature that enables tool calling capabilities for language models that don't natively support tool calling (like DeepSeek). It works by using Ollama models to interpret the primary model's responses and convert them into valid tool calls.


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

## Feedback

If you encounter any issues with these features, check if the issue is already reported in the [GitHub issues](https://github.com/goose/goose/issues) or join the [Discord community](https://discord.gg/block-opensource) to share.