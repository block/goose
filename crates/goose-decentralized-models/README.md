# goose-decentralized-models

Share and discover LLM models via Nostr.

## Quick Start

```bash
# 1. Initialize (creates config + generates Nostr keys)
cargo run -p goose-decentralized-models -- init

# 2. Edit config if needed
#    ~/.config/goose/decentralized-models.json

# 3. Publish your models (expires in 1 hour)
cargo run -p goose-decentralized-models -- publish

# 4. Discover models from others
cargo run -p goose-decentralized-models -- discover

# 5. Discover and launch goose with a remote model
cargo run -p goose-decentralized-models -- run
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Create config + generate Nostr keys |
| `publish` | Publish models to Nostr (idempotent, updates in place) |
| `discover` | Find models others have published |
| `list` | Show your published models |
| `unpublish <name>` | Remove a specific model |
| `show-key` | Display your Nostr public key |
| `run` | Discover a model and launch goose with it |

## Run Command

The `run` command discovers models published on Nostr and launches goose connected to the first available one:

```bash
# Use first available model
cargo run -p goose-decentralized-models -- run

# Prefer a specific model (falls back to first available)
cargo run -p goose-decentralized-models -- run --model qwen3
```

This sets `OLLAMA_HOST` to the discovered endpoint and launches `goose --provider ollama --model <model>`.

## Config

Located at `~/.config/goose/decentralized-models.json`:

```json
{
  "relays": ["wss://relay.damus.io", "wss://nos.lol"],
  "models": [
    {
      "name": "qwen3-coder:latest",
      "display_name": "Qwen3 Coder",
      "context_size": 32000
    }
  ],
  "ollama_endpoint": "http://localhost:11434",
  "advertise_endpoint": {
    "host": "YOUR_PUBLIC_IP",
    "port": 11434,
    "https": false
  },
  "ttl_seconds": 3600
}
```

## How It Works

1. Models are published as Nostr events (Kind 31990, replaceable)
2. Publishing is idempotent — re-publishing updates the existing event
3. Discovery only shows models published in the last 30 minutes
4. Endpoints are OpenAI-compatible (proxied through Ollama)

## Files

- Config: `~/.config/goose/decentralized-models.json`
- Keys: `~/.config/goose/nostr-key.nsec` (0600 permissions)
