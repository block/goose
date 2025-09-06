---
title: From Zero to MacOS, Ollama, Goose, ToolCalling end to end
sidebar_position: 2
sidebar_label: MacOS Ollama Tool Calling e2e
---

Ollama & Goose Tool Calling can be configured on MacOS, but is closer to experimental than some other providers.

This guide provides step by step install + configuration, with realistic expectations what will work.

#### MacOS
This guide was written on an M4 Max MacBook with 128GB, but attempts to minimize requirements:
- Attempt to increase compatibility with other hardware configurations
- Attempt to maximize responsiveness on all hardware configurations

#### Ollama

1. Make sure you have [Ollama](https://ollama.com/download) installed and running.
 This guide is using the following version from [Homebrew](https://formulae.brew.sh/formula/ollama):
   ```
   ollama --version
   ollama version is 0.11.7
   ```
   At the time of writing, the precise Ollama version has not appeared to be important, so long as
 it supports the below listed env vars.

2. To minimize the number of changes we must make elsewhere, it is important that Ollama is started with at least the
 following configuration. Here we show and implement that configuration as env vars:
   ```
   OLLAMA_CONTEXT_LENGTH=32768   # without this as a minimum, Tool Calling likely won't not work at all
   OLLAMA_FLASH_ATTENTION="1"    # perf optimization mentioned by Homebrew. See Ollama docs for more info
   OLLAMA_KV_CACHE_TYPE="q8_0"   # perf optimization mentioned by Homebrew. See Ollama docs for more info
   ```
   More concisely:
   ```
   OLLAMA_CONTEXT_LENGTH=32768 OLLAMA_FLASH_ATTENTION="1" OLLAMA_KV_CACHE_TYPE="q8_0" ollama serve
   ```


#### Models

1. The default Goose Ollama Tool Shim model is `mistral-nemo`. This guide instead recommends:
   `michaelneale/deepseek-r1-goose:latest` NEEDS an approach to PINNING
   ```bash
   ollama pull michaelneale/deepseek-r1-goose:latest
   ```

2. The default Goose Ollama model is `qwen2.5`. This guide instead recommends:
   `qwen2.5-coder:32b` NEEDS an approach to PINNING
   ```bash
   ollama pull qwen2.5-coder:32b
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
