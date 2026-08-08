# AVCD Agent

AVCD Agent is Avocado Technology's native AI agent for code, workflows, and
automation. It includes a desktop app, CLI, and Agent Client Protocol (ACP)
server, with support for multiple model providers and Model Context Protocol
(MCP) extensions.

## Local development

Requirements: Docker Desktop, Make, Node.js 24.10 or newer, and pnpm 10.30 or
newer.

```bash
cp .env.local.example .env.local
make dev
make test-smoke
```

The ACP backend listens on `http://localhost:3000`. Run the Electron desktop
against that backend on the host:

```bash
make dev-ui
```

Use `make help` for the complete local workflow. Provider credentials can be
synced with `make pull-secrets` after configuring `INFISICAL_PROJECT_ID`.

## Keeping the fork current

The `origin` remote is the AVCD distribution and `upstream` is the AAIF
project. Keep rebranding isolated to the files described in
[`CUSTOM_DISTROS.md`](CUSTOM_DISTROS.md), then update from upstream:

```bash
git fetch upstream
git merge upstream/main
```

Resolve upstream changes without removing AVCD branding, the fork-owned
updater destination, or the disabled upstream telemetry configuration.

## Attribution and license

AVCD Agent is a modified distribution of
[goose](https://github.com/aaif-goose/goose), developed by the
[Agentic AI Foundation](https://aaif.io/) and its contributors. The project is
licensed under the Apache License 2.0. See [LICENSE](LICENSE) and
[NOTICE](NOTICE).

Upstream technical documentation remains available at
[goose-docs.ai](https://goose-docs.ai/).
