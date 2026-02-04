# Goose Decentralized Models

Share and discover LLM models via Nostr.

## Quick Start

```bash
# Initialize config and keys
cargo run -p goose-decentralized-models -- init

# Publish your models
cargo run -p goose-decentralized-models -- publish

# Discover available models
cargo run -p goose-decentralized-models -- discover

# Run goose with a discovered model
cargo run -p goose-decentralized-models -- run
cargo run -p goose-decentralized-models -- run --model qwen3
```

## Library Usage

```rust
use goose_decentralized_models::{discover_model, ModelFilter};

// Simple - first free model
let model = discover_model(None, None).await?;

// With filtering
let filter = ModelFilter::new()
    .model("qwen")
    .geo("US")
    .min_context(32000);
let model = discover_model_filtered(None, &filter).await?;
```

## Roadmap

**Not yet implemented:**

- **Tunnelling** - Secure tunnel protocol for connecting to models behind NAT/firewalls
- **Usage metering** - Track usage for cost/billing, publish availability status (busy/overloaded)
- **Tunnel protocol** - Define wire protocol for secure model access
- **Backend abstraction** - Support backends beyond Ollama (vLLM, llama.cpp, etc.)
- **Negotiation** - Handshake for secure connection establishment and payment agreement
