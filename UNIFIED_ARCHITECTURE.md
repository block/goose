# Goose Client/Server

## Where we are

Goose currently ships three Rust binaries that each wrap the same core Agent in different ways:

- **`goose`** (CLI) — in-process agent with full lifecycle management. No server or protocol. Used in interactive mode, to run recipes and in some headless situations.
- **`goosed`** (server) — wraps the Agent behind a bespoke REST+SSE HTTP API with 103 endpoints. The desktop app spawns `goosed agent` as a child process. Mobile clients and Slack bots connect to it over the network.
- **`goose-acp-server`** — wraps the Agent behind the [Agent Client Protocol](https://agentclientprotocol.org) (ACP), an open standard for agent communication. Powers integration with editors like Zed and the JetBrains family. Also has a PoC TUI implementation.

## One binary

We should replace all this with one binary that speaks one protocol:

- Run interactively, is a CLI that talks ACP to an in-process agent.
- Run as `goose serve`, exposes the same ACP interface over HTTP and WebSocket for desktop, mobile, bots, and any other client.

One binary is easier to test, version and distribute. Can be installed with `brew install goose` or even `cargo install`. CI pipelines, containers, and embedded devices get a single static artifact. It should be runnable on a Raspberry Pi Zero, over a serial console and on any type of container.

## ACP as the goose protocol

ACP is an open protocol for agent-client communication built on JSON-RPC. Its core covers the basic agent lifecycle. ACP also offers custom methods that we can use to implement the rest of the goosed API. We can continue generating an API based on our Rust implementation and publish it as `@gooseprotocol/sdk` or similar.

Any features we implement as part of this extended protocol that make sense beyond goose, we can bring to the ACP community to see if they belong in the official protocol. The main things currently not covered are around configuration, recipes, scheduling and desktop-specific features (like dictation).

## Goose UIs

This way we'll end up with three categories of UIs: the basic CLI, first-party UIs and community UIs.

The `goose` binary can be run as a basic CLI similar to what we have; the goal should be to have something that starts instantly and runs anywhere. It's a line-oriented CLI that can adapt to do slightly better depending on the terminal that it runs in. It can borrow some ideas from the Rust TUI work, but there's no ambition to be a full-blown TUI.

First-party UIs are the UIs the goose team develops. Currently this comprises the desktop and the mobile client. We should add a decent TUI (we have a prototype in TypeScript) and an Android client for mobile.

Once we publish the goose protocol as an SDK, community UIs become straightforward to build. Any developer can connect to `goose serve` and get the full feature surface — not just the ACP baseline, but configuration, recipes, scheduling, and everything else goose offers. The protocol is the contract; the SDK makes it easy to use.

## The goose CLI: a static binary that runs anywhere

The `goose` binary is both the reference ACP client and the server. It should be statically linkable and run on a Raspberry Pi Zero, in an Alpine container, over a serial console, or on a developer's MacBook.

Not a full-screen TUI. A line-oriented CLI that enhances itself when it detects a capable terminal:

```
$ goose
  goose 🪿 session: a1b2c3

> fix the failing test in auth.rs

⠋ thinking...

I'll look at the test file first.

  ┃ developer: text_editor — view auth.rs        ✓
  ┃ developer: shell — cargo test auth            ✗

The test `test_token_refresh` is failing because...

  ┃ developer: text_editor — str_replace auth.rs  ✓
  ┃ developer: shell — cargo test auth            ✓

Fixed. The issue was...

>
```

Graceful degradation:

- Good terminal → color, spinners, syntax-highlighted code blocks, inline tool status
- Basic terminal → plain text, ASCII markers, no color
- Piped / non-interactive → clean text output, JSON mode available

The current CLI already has the right rendering approach using `console`, `rustyline`, `indicatif`, and `bat`. We can borrow from the [Rust TUI work](https://github.com/block/goose/pull/5831) (markdown rendering, component patterns) as long as everything degrades gracefully — no alternate screen, no mouse handling, no assumptions about terminal capabilities.

Static linking means the binary has no runtime dependencies. No Node.js, no Python, no shared libraries. Build with `cross` for `aarch64-unknown-linux-musl` and it runs on a Pi Zero. Heavy optional dependencies like `bat` (syntax highlighting, ~5MB) can be feature-flagged for minimal builds.

## Community frontends and integrations

ACP is an open protocol. `goose serve` speaks it. Anyone can build a frontend.

The ecosystem we want:

```
goose serve (ACP server)
     │
     ├── Desktop app (Electron) ──── shipped by us
     ├── Mobile app (iOS/RN) ─────── shipped by us
     ├── goose CLI ────────────────── shipped by us
     │
     ├── VS Code extension ────────── community
     ├── Web UI ───────────────────── community
     ├── Slack/Discord bot ────────── community
     ├── Emacs/Neovim plugin ──────── community
     └── Custom automation ────────── community
```

**For TypeScript/JavaScript developers:** We'll publish a package (e.g. `@gooseprotocol/sdk`) on npm that builds on the ACP SDK and includes typed support for goose's custom methods — configuration, recipes, scheduling, and the rest. Community frontends get the full feature surface, not just the ACP baseline. Our `ui/text` React/Ink TUI serves as a reference implementation showing how to use it.

**For Python developers:** An ACP client library on PyPI (installable via `pip` or runnable via `uvx`) would let the Python community build goose frontends and automation without touching Rust or TypeScript.

**The pattern is proven.** MCP (Model Context Protocol) showed that an open protocol for tool integration lets the community build hundreds of extensions that make goose more capable. ACP does the same thing for the client side — an open protocol for agent interaction lets the community build frontends, integrations, and automation that make goose more accessible.

The key enablers:

1. **`goose serve` is easy to run** — one command, one binary, listens on a port
2. **ACP is documented** — the protocol spec, the custom method schemas, the TypeScript SDK
3. **Reference implementations exist** — the CLI (Rust), the text TUI (TypeScript), the desktop (Electron)
4. **The protocol is extensible** — custom methods let goose-specific features coexist with the standard
