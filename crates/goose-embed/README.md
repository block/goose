# goose-embed

Embeddable Rust SDK for the [goose](https://github.com/aaif-goose/goose) AI agent.

Use this crate to run goose's agent loop inside your own Rust program — with
your own provider, MCP extensions, recipes, and skills — without shelling out
to the `goose` binary or talking ACP over a subprocess.

## Status — Spike

This crate is an **API-discovery spike**, not a production-ready SDK. It is
implemented as a thin facade over the full [`goose`] crate, so depending on it
pulls the same dependency tree as depending on `goose` directly. The crate's
purpose is to validate the public API surface against real examples before a
slim `goose-core` crate is carved out of `goose`. See
[`docs/brainstorms/2026-05-28-goose-embed-rust-sdk.md`](../../docs/brainstorms/2026-05-28-goose-embed-rust-sdk.md)
for the roadmap.

If you need the embed story today and don't care about dep weight, this is
the right crate. If dep weight matters, wait for `goose-core`.

## Quick start

```rust,no_run
use futures::StreamExt;
use goose_embed::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let goose = Goose::builder()
        .provider("anthropic", "claude-sonnet-4")
        .working_dir(std::env::current_dir()?)
        .build()
        .await?;

    let mut stream = goose.reply("What is 2 + 2?").await?;
    while let Some(event) = stream.next().await {
        if let Ok(AgentEvent::Message(message)) = event {
            println!("{}", message.as_concat_text());
        }
    }
    Ok(())
}
```

Run the example with a real provider:

```bash
GOOSE_EMBED_PROVIDER=anthropic \
GOOSE_EMBED_MODEL=claude-sonnet-4 \
cargo run -p goose-embed --example hello_agent
```

## Concepts

### Builder

[`Goose::builder()`](crate::Goose::builder) returns a [`GooseBuilder`] with
chainable setters:

| Method | Purpose |
|---|---|
| `.provider(name, model)` | Pick the LLM provider and model. |
| `.extension(ExtensionConfig)` | Register a single MCP extension. |
| `.extensions(iter)` | Register many at once. |
| `.recipe(Recipe)` | Apply a recipe: extensions, response schema, and (optionally) provider/model from `settings`. |
| `.working_dir(path)` | Set the working directory the agent sees. |
| `.session_name(name)` | Override the default `goose-embed-<pid>` session name. |
| `.permission_decider(impl PermissionDecider)` | Install a custom permission policy. Defaults to [`AutoApprove`]. |
| `.max_turns(n)` | Cap the number of tool-loop turns per `reply()`. |

`.build()` is async and returns a [`Goose`] handle.

### Replying

[`Goose::reply(prompt)`](crate::Goose::reply) sends a prompt and returns a
[`ReplyStream`] that yields [`AgentEvent`]s as the agent produces them.
Tool-confirmation requests are intercepted automatically and routed through
the configured [`PermissionDecider`].

### Permission deciders

When the agent needs to confirm a tool call, the configured decider is
asked. Built-in impls:

| Type | Behavior |
|---|---|
| [`AutoApprove`] | Grants `AllowOnce` for everything. The default. |
| [`DenyAll`] | Denies every tool call. Useful as a safe default while wiring up real policy. |

Implement [`PermissionDecider`] yourself for anything more interesting:

```rust,ignore
use async_trait::async_trait;
use goose_embed::prelude::*;

struct AllowOnlyReads;

#[async_trait]
impl PermissionDecider for AllowOnlyReads {
    async fn decide(&self, request: PermissionRequest) -> Permission {
        if request.tool_name.contains("read") {
            Permission::AllowOnce
        } else {
            Permission::DenyOnce
        }
    }
}
```

### Recipes

Pass a [`Recipe`] to `.recipe(...)` and the builder will register its
extensions and (if you haven't set provider/model explicitly) read them from
`settings.goose_provider` / `settings.goose_model`. See
[`examples/with_recipe.rs`](examples/with_recipe.rs) for an end-to-end
example.

## What this crate does NOT do (v0)

- **Does not minimize dependencies.** Pulls everything `crates/goose` pulls.
  Use this crate when ergonomics matter more than binary size; wait for
  `goose-core` if you need a small dep graph.
- **Does not handle elicitations.** If the agent requests structured input
  via MCP elicitation, the request is logged as a warning and the agent
  stalls. Coming in v1.
- **Does not expose the embedded agent as an ACP server.** If you want
  other clients to connect to your composed agent over ACP, depend on
  `goose` directly and use `goose acp`. A `goose-acp-server` bridge crate
  may follow once `goose-core` exists.
- **Does not provide multi-language bindings.** Rust-only. Cross-language
  embedding goes through ACP via the existing `goose-sdk` crates.

## Roadmap

1. **This crate (today).** Validate the embed API surface.
2. **`goose-core`.** Carve out a lean crate containing only the agent loop,
   `Provider` trait, MCP utils, `Recipe`, `Skill`, `conversation`,
   `context_mgmt`, `permission`. Heavy deps (sqlx, scheduler, OAuth, AWS,
   candle) stay in the `goose` runtime crate.
3. **Re-emerge `goose-embed` on top of `goose-core`.** Same public API as
   today; orders-of-magnitude smaller dep graph.
4. **`goose-acp-server` bridge.** Optional crate for embedders that want to
   re-expose their composed agent over ACP.
