# ApeMind Agent

ApeMind Agent is an ApeCloud desktop agent distribution based on upstream
[Goose](https://github.com/aaif-goose/goose). It runs on the user's own machine
and provides a local desktop entry point for agent workflows.

The first PoC focuses on desktop branding and safe packaging defaults while
keeping the fork shallow enough to continue tracking upstream.

## What This Fork Changes

- ApeMind Agent product name and desktop-facing copy.
- ApeCloud/ApeMind visual assets for desktop packaging.
- Build defaults that avoid telemetry, OTel, and AWS provider features unless
  explicitly enabled.

Packaging artifact names for desktop installers, CLI downloads, and Docker
images are tracked separately as distribution work.

## What Stays Upstream-Compatible

- Core agent loop.
- Tool execution behavior.
- Provider runtime internals.
- MCP runtime internals.
- Cargo crate names and repository path.

## Target Users

The main user entry point is the desktop app:

- macOS Desktop for internal team validation.
- Windows Desktop for customer validation.
- CLI for debugging and automation.
- Docker for distribution and automated smoke checks.

## Build Notes

The PoC CLI profile uses upstream feature flags and release settings:

```bash
source ./bin/activate-hermit >/tmp/hermit.log 2>&1
CARGO_PROFILE_RELEASE_LTO=true \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
CARGO_PROFILE_RELEASE_STRIP=true \
  cargo build --release --package goose-cli --bin goose \
  --no-default-features --features rustls-tls
```

This keeps AWS provider, telemetry, and OTel out of the default PoC build.

## Upstream

This project is a branded distribution of Goose, an Apache-2.0 licensed open
source project from the Agentic AI Foundation at the Linux Foundation. Keep the
upstream license and notices intact when distributing ApeMind Agent.
