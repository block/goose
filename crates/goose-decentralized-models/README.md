# Goose Decentralized Models

Share and discover LLM models via Nostr in a peer-to-peer, permissionless way.

## Using the Decentralized Provider

The easiest way to use decentralized models is through goose's built-in `decentralized` provider:

```bash
# Use the decentralized provider
goose configure
# Select "decentralized" as the provider

# Or set via environment
export GOOSE_PROVIDER=decentralized
goose session

# Filter by model name (uses standard GOOSE_MODEL)
export GOOSE_PROVIDER=decentralized
export GOOSE_MODEL=qwen3
goose session

# Filter by geography
export DECENTRALIZED_GEO=US
goose session

# Filter by cost (max cost per token)
export DECENTRALIZED_MAX_COST=0.0
goose session

# Filter by minimum context size
export DECENTRALIZED_MIN_CONTEXT=32000
goose session

# Use custom relays (comma-separated)
export DECENTRALIZED_RELAYS=wss://relay.damus.io,wss://nos.lol
goose session
```

### Configuration Options

| Environment Variable | Description |
|---------------------|-------------|
| `GOOSE_MODEL` | Filter by model name (e.g., "qwen3", "llama") |
| `DECENTRALIZED_GEO` | Filter by geographic region (e.g., "US", "EU") |
| `DECENTRALIZED_MAX_COST` | Maximum cost per token (use 0.0 for free models) |
| `DECENTRALIZED_MIN_CONTEXT` | Minimum context window size required |
| `DECENTRALIZED_RELAYS` | Custom Nostr relays (comma-separated) |
| `DECENTRALIZED_TIMEOUT` | Request timeout in seconds (default: 600) |

## CLI Tools

For publishing your own models or manual discovery:

```bash
# Initialize config and keys
cargo run -p goose-decentralized-models -- init

# Publish your models
cargo run -p goose-decentralized-models -- publish

# Discover available models
cargo run -p goose-decentralized-models -- discover

# Run goose with a discovered model (legacy method)
cargo run -p goose-decentralized-models -- run
cargo run -p goose-decentralized-models -- run --model qwen3
```

## Library Usage

```rust
use goose_decentralized_models::{discover_model, ModelFilter};

// Simple - first available model
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
- **Backend abstraction** - Support backends beyond Ollama (vLLM, llama.cpp, etc.)
- **Publisher proxy** - Track usage, authenticate clients, enforce limits (see below)
- **Payment integration** - Automatic payment negotiation and settlement

### Publisher Proxy

A proxy service running on the publisher side to track usage, authenticate clients, and optionally enforce rate limits.

```
Clients ──▶ Proxy (public) ──▶ LLM Backend (private)
              │
              ▼
          Usage DB
```

#### Open Questions

**1. Deployment model** - Who runs this and how?

- People sharing via the goose app directly (simple, single-user)
- Dedicated hosting/infrastructure for scaled serving (multi-tenant, high availability)
- Unknown at this stage - design should accommodate both

**2. Client authentication** - How do we identify clients?

| Option | Pros | Cons |
|--------|------|------|
| Nostr signatures | Leverages existing identity, decentralized | Adds request overhead, client complexity |
| API keys | Simple, familiar | Out-of-band key exchange needed |
| None/open | Zero friction | No accountability, no payment possible |

Likely need multiple options - open for free models, authenticated for paid.

**3. Usage tracking** - What to record?

- Input/output token counts (required for billing)
- Request timestamps and latency
- Model used
- Client identifier (pubkey or key ID)
- Session/conversation grouping?

**4. Token counting** - How do we count accurately?

- Parse from backend response (`usage` field if OpenAI-compatible)
- Requires tokenizer if backend doesn't report (model-dependent)
- Fallback: estimate from response length (inaccurate)

**5. Storage** - Where does usage data live?

- Local (SQLite/file) - simple, portable
- Publish to Nostr - decentralized, auditable
- External service - for scaled deployments
- Likely: local primary, optional Nostr publishing for transparency

**6. Rate limiting / capacity**

- Per-client quotas (requests, tokens, cost)
- Global capacity limits
- Publish availability status (available/busy/overloaded) back to Nostr?

### Tunnelling

For publishers behind NAT/firewalls who can't expose a public endpoint.

#### Open Questions

**1. Tunnel mechanism**

| Option | Pros | Cons |
|--------|------|------|
| WebRTC | P2P, NAT traversal built-in, widely supported | Complex setup, STUN/TURN servers needed |
| Nostr relays as transport | Already have relay infrastructure | Not designed for high-bandwidth streaming |
| Cloudflare Tunnel / ngrok | Zero config, reliable | Centralized dependency |
| Custom relay servers | Full control | Need to run infrastructure |
| WireGuard/VPN | Proven, secure | Requires coordination, not P2P friendly |

**2. Discovery integration**

- How does client know to use tunnel vs direct connection?
- Publish tunnel endpoint in model listing?
- Fallback: try direct, then tunnel?

**3. Relay infrastructure**

- Who runs relay/TURN servers?
- Community operated? Publisher provided? Decentralized incentives?

### Payment Integration

Automatic payment negotiation between consumer and publisher.

#### Open Questions

**1. Discovery-time agreement** - When a client discovers a paid model:

- Client sees advertised price in model listing
- Client agrees to terms before first request? Or implicit by using?
- How is agreement recorded? Nostr event?

**2. Payment mechanisms** - How does money move?

| Option | Pros | Cons |
|--------|------|------|
| Lightning/Bitcoin | Decentralized, instant, micropayments, Nostr-aligned | Wallet integration, volatility |
| USDC/stablecoins | Stable value, familiar | Chain fees (unless L2), wallet needed |
| Low-cost chains (Base, Solana, etc.) | Cheap transactions | Wallet integration per chain |
| Prepaid credits | Simple accounting | Requires trust or escrow |
| Streaming payments (Superfluid, etc.) | Pay-as-you-go | Complex, chain-specific |

**3. Wallet integration**

- Client needs wallet to pay, publisher needs wallet to receive
- Embedded wallet vs external wallet connection?
- Multi-chain support or pick one to start?

**4. Tracking and reconciliation**

- Client tracks their own usage locally
- Publisher tracks and can provide usage reports
- Nostr as shared ledger for disputes?
- Automatic settlement at thresholds or intervals

**5. Trust model** (future consideration)

- Can client verify token counts reported by publisher?
- Can publisher trust client will pay?
- Escrow/deposits for new relationships?
- Reputation system based on payment history?

### Protocol / Wire Format

**Open Questions:**

- Custom headers for auth (`X-Nostr-Pubkey`, `X-Nostr-Sig`, `X-Payment-Token`)?
- Transparent pass-through vs. custom protocol wrapper?
- How does client signal willingness to pay / payment method?
- Usage query endpoint for clients to check balance/history?
