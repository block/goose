# goose-local-inference

On-device model inference for goose. Runs GGUF models through `llama.cpp` (via
`llama-cpp-2`), with an optional MLX backend on Apple silicon.

Reach it through [`goose-providers`](../goose-providers) with the
`local-inference` feature, which exposes `LocalInferenceProvider` as an ordinary
`Provider`. Depend on this crate directly only when you need model management.

## Features

Default is `[]` — CPU inference.

- `cuda`, `vulkan` — GPU acceleration via the corresponding `llama-cpp-2` backend.
- `mlx` — the MLX backend for Apple silicon.

## What it handles

- **Runtime and placement** — `InferenceRuntime` describes the machine;
  `available_inference_memory_bytes` and `recommend_local_model` pick a model that
  will actually fit.
- **Model lifecycle** — `is_model_loaded`, `loaded_model_ids`, and `evict_model`
  manage what's resident. `management`, `local_model_registry`, `hf_models`, and
  `paths` cover discovery, on-disk layout, and the Hugging Face catalog;
  `huggingface_auth` handles gated repos. Downloads go through
  [`goose-download-manager`](../goose-download-manager), re-exported here as
  `download_manager`.
- **Prompt formatting** — `prompt_template` applies the model's chat template;
  `builtin_chat_template_names()` lists the bundled ones.
- **Tool calling** — `native_tool_parsing` and `tool_parsing` extract tool calls
  from model output, and `tool_emulation` (toolshim) fills in for models with no
  native tool support.
- **Richer outputs** — `thinking_output` separates reasoning blocks from the
  answer; `multimodal` handles image input.
- **Config** — `config_resolver` and `provider_utils` resolve settings such as
  `LOCAL_LLM_MODEL`.

## Building

The `llama.cpp` backends compile native code, so a C/C++ toolchain is required,
plus the CUDA or Vulkan SDK when selecting those features.

```bash
cargo build -p goose-local-inference
cargo build -p goose-local-inference --features mlx
```
