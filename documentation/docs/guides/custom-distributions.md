---
title: Custom Distributions
sidebar_position: 60
sidebar_label: Custom Distributions
---

# Custom Distributions

gosling is designed to be forked and customized. You can create your own "distro" of gosling preconfigured with specific providers, bundled extensions, custom branding, and tailored workflows for your organization or audience.

## What you can customize

| What You Want | Complexity |
|---------------|------------|
| Preconfigure a model/provider | Low |
| Add custom AI providers (declarative JSON, no code) | Low |
| Bundle custom MCP extensions | Medium |
| Modify system prompts | Low |
| Customize desktop branding (icons, names, colors) | Medium |
| Build a new UI via REST API or ACP | High |

## Getting started

The full guide lives in the repo root since you'll need to work at the code level to build a custom distribution:

👉 **[CUSTOM_DISTROS.md](https://github.com/repo-makeover/gosling/blob/main/CUSTOM_DISTROS.md)**

It covers:

- **Architecture overview** — how gosling's layers (UI → server → core) fit together
- **Configuration-only customization** — environment variables, `config.yaml`, `init-config.yaml`
- **Extension bundling** — adding MCP servers as built-in extensions
- **Custom branding** — replacing icons, app names, system prompts
- **Building new interfaces** — integrating via the REST API or Agent Client Protocol (ACP)
- **Custom AI providers** — declarative JSON providers or implementing the Provider trait
- **Licensing & contribution guidance** — staying compliant with Apache 2.0

## Quick example: ship gosling with a local model

The simplest custom distribution just sets environment defaults:

```bash
export GOSLING_PROVIDER=ollama
export GOSLING_MODEL=qwen3-coder:latest
```

Or create an `init-config.yaml` applied on first run:

```yaml
GOSLING_PROVIDER: ollama
GOSLING_MODEL: qwen3-coder:latest
```

See the [full guide](https://github.com/repo-makeover/gosling/blob/main/CUSTOM_DISTROS.md) for more scenarios including corporate API key distribution, audience-specific builds, and custom UIs.
