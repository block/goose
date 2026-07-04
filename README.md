<div align="center">

<img src="assets/gosling_logo_asset_pack/derived/gosling-wordmark-light.svg" alt="gosling logo" width="240">

# gosling

_a lighter goose — your native open source AI agent for code, workflows, and everything in between_

<p align="center">
  <a href="https://opensource.org/licenses/Apache-2.0"
    ><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg"></a>
</p>
</div>

gosling is a general-purpose AI agent that runs on your machine. Not just for code — use it for research, writing, automation, data analysis, or anything you need to get done.

A native desktop app for macOS, Linux, and Windows. A full CLI for terminal workflows. An API to embed it anywhere. Built in Rust for performance and portability.

gosling works with 15+ providers — Anthropic, OpenAI, Google, Ollama, OpenRouter, Azure, Bedrock, and more. Use API keys or your existing Claude, ChatGPT, or Gemini subscriptions via ACP. Connect to 70+ extensions via the [Model Context Protocol](https://modelcontextprotocol.io/) open standard.

## Provenance

gosling **v0.0.1** is a fork of [goose](https://github.com/aaif-goose/goose) **v1.38**, the open source AI agent from the [Agentic AI Foundation (AAIF)](https://aaif.io/) at the Linux Foundation. All credit for the underlying agent framework goes to the goose project and its contributors. gosling is licensed under the same Apache 2.0 license and is not endorsed by or affiliated with the goose project or AAIF.

## Vision

gosling aims to be a **lighter version of goose**: the same trusted agent core with a smaller footprint, a simpler surface, and faster iteration. The goal is an agent you can install next to (or instead of) goose that stays lean — fewer moving parts, quicker startup, and an easier codebase to remix for custom distributions.

## Footprint & performance vs. goose

Comparison performed **2026-07-04** between release builds of `goose-cli` from `goose` v1.41.0 (commit `181cbbe`) and `gosling` v1.40.0 (commit `5b7d039`), same host, matched Cargo feature flags (`code-mode` excluded from both — its `v8-goose` static-lib download is blocked by this environment's network policy, symmetrically for both builds).

| | goose | gosling | Δ |
|---|---|---|---|
| Cargo.lock packages | 1251 | 1065 | -186 (-15%) |
| Binary size (stripped) | 151 MB | 117 MB | -22% |
| `target/release` build dir | 3.8 GB | 2.4 GB | -37% |
| Build time (wall) | 17m12s | 11m26s | -33% |
| Runtime shared libs (`ldd`) | libstdc++, libgcc_s, libm, libc | libgcc_s, libm, libc | no libstdc++ |
| `--version` cold start | 8.4ms avg / 24.0 MB peak RSS | 6.1ms avg / 17.7 MB peak RSS | -27% time, -26% mem |
| `doctor` cold start | 8.8ms avg / 28.9 MB peak RSS | 6.3ms avg / 22.0 MB peak RSS | -29% time, -24% mem |

The gap traces almost entirely to the local-inference stack (candle, llama.cpp, MLX, Hugging Face downloads — 148 crates) that gosling extracts: gosling also drops the `recipe`, `schedule`, `gateway`, and `local-models` CLI subcommands. Core agent/session/MCP functionality is unchanged between the two. Actual LLM conversation/tool-calling throughput wasn't benchmarked (no provider configured in the comparison environment) and isn't expected to differ, since that path is dominated by the provider API in both.

## What's new in gosling

- **New name, new mark** — the goose branding has been replaced by gosling: a fresh flying-gosling logo across the desktop app, tray, docs, and installers.
- **Runs side by side with goose** — gosling is fully deconflicted from an existing goose install:
  - separate config/data/state directories (`~/.config/goose` vs `~/.config/gosling`, etc.)
  - separate OS keyring service (`gosling`) for provider credentials
  - its own `gosling://` deep-link scheme (goose keeps `goose://`; gosling still accepts `goose://` session share links for interop)
  - its own app identity (`Gosling.app` / `Gosling.exe` / `Gosling` packages) and updater feed
  - single-instance behavior is preserved per app: one running Goose and one running Gosling, each guarded by its own instance lock
- **Provenance in the app** — Help → About shows that this is Gosling v0.0.1, a fork of goose v1.38.

## Get started

Build the desktop app or CLI from source:

```bash
source bin/activate-hermit
cargo build --release          # CLI
just run-ui                    # desktop app
```

See [BUILDING_LINUX.md](BUILDING_LINUX.md), [BUILDING_DOCKER.md](BUILDING_DOCKER.md), and [ui/desktop/README.md](ui/desktop/README.md) for platform-specific instructions.

## Quick links

- [Documentation source](documentation/) — the gosling docs site
- [Custom Distributions](CUSTOM_DISTROS.md) — build your own distro with preconfigured providers, extensions, and branding
- [Contributing](CONTRIBUTING.md)

## Upstream compatibility notes

- CLI command names and binaries are renamed from goose's (`gosling`, `goslingd` instead of `goose`, `goosed`); scripts and docs that shell out to `goose`/`goosed` need updating.
- Environment variables and project files are renamed too (`GOSLING_*` instead of `GOOSE_*`; `.goslinghints`/`.gosling/` instead of `.goosehints`/`.goose/`) — see "Runs side by side with goose" above for why, and for the narrow spots (DB migration, `gosling://`/`goose://` share links) that do keep reading the old names.

## a little gosling humor 🐥

> Why did the developer switch from goose to gosling?
>
> They wanted the same migrations with less honking! 🚀
