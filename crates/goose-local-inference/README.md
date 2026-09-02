# goose-local-inference

On-device model inference for goose. Supports `llama.cpp` (via `llama-cpp-2`)
and eredu as independently selectable backends. eredu loads both SafeTensors
and GGUF artifacts and provides checkpoint-native chat templates, semantic
streaming, constrained sampling, and native tool calls.

Reach it through [`goose-providers`](../goose-providers) with the
`local-inference` feature, which exposes `LocalInferenceProvider` as an ordinary
`Provider`. Depend on this crate directly only when you need model management.

## Features

The default is `llamacpp`.

- `llamacpp` — GGUF inference through llama.cpp.
- `eredu` — SafeTensors and GGUF inference through eredu.
- `cuda`, `vulkan` — optional llama.cpp accelerator support.

When both backends are compiled, GGUF models use llama.cpp by default and
SafeTensors models use eredu. Each model can override that selection in its
settings. llama.cpp cannot be selected for SafeTensors artifacts.

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
- **Prompt formatting and tools** — eredu owns checkpoint chat templates,
  semantic output parsing, constraints, and native tool calls. The llama.cpp
  backend retains its built-in/custom templates and emulated-tool fallback.
- **Richer outputs** — semantic reasoning is streamed as thinking output;
  llama.cpp also supports image input through an associated projection model.
- **Config** — `config_resolver` and `provider_utils` resolve settings such as
  `LOCAL_LLM_MODEL`.

## Building

The `llama.cpp` backends compile native code, so a C/C++ toolchain is required,
plus the CUDA or Vulkan SDK when selecting those features.

```bash
cargo build -p goose-local-inference
cargo build -p goose-local-inference --no-default-features --features eredu
cargo build -p goose-local-inference --features eredu
```
