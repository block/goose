# AVCD Agent

AVCD Agent is Avocado Technology's native AI agent for code, workflows, and
automation. It includes a desktop app, CLI, and Agent Client Protocol (ACP)
server, with support for multiple model providers and Model Context Protocol
(MCP) extensions.

## Local development

Requirements: Docker Desktop, Make, Node.js 24.10 or newer, and pnpm 10.30 or
newer.

On macOS, the current upstream Electron Forge stack exits before writing the
bundle under Node 24. Use `nvm use 22.18.0` for `make package-ui`; development,
linting, and tests continue to use the declared Node 24 runtime.

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
synced after an Infisical organization administrator creates the `avcd-agent`
project:

```bash
export INFISICAL_PROJECT_ID="<project-uuid>"
make pull-secrets
```

The same project ID is used by `make upload-secrets`. Project creation is an
account-level action; no project ID or credential is committed to this fork.

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
