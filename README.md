# Avocado Work

Avocado Work is Avocado Technology's native AI agent for code, workflows, and
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
make dev # creates .env.local from the example when it is missing
make test-smoke
```

Avocado Work uses OpenRouter by default, with the same curated 13-model catalog
as avcd-ai. Add the shared credential to `.env.local` before starting a chat:

```bash
# Same secret value as OPENROUTER_KEY in avcd-ai; goose uses this variable name.
OPENROUTER_API_KEY="<openrouter-key>"
```

The default model is `deepseek/deepseek-v4-flash`. Verify the catalog, Docker
configuration, generated desktop environment, and (when a key is present)
provider connectivity with:

```bash
make validate-openrouter
```

The ACP backend listens on `http://localhost:3000`. Run the Electron desktop
against that backend on the host:

```bash
nvm use          # reads .nvmrc (Node 24.16.0)
make dev-ui
```

Provider and model environment variables override persisted configuration in
the local Docker volume. If an old keyring or secret-storage setting still
interferes, reset only the local development state:

```bash
make dev-down
docker volume rm avcd-agent_avcd-agent-config
make dev
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
[`CUSTOM_DISTROS.md`](CUSTOM_DISTROS.md) and the living skill
[`.cursor/skills/architecture/avcd-agent-custom-distro/SKILL.md`](.cursor/skills/architecture/avcd-agent-custom-distro/SKILL.md),
then update from upstream:

```bash
git fetch upstream
git merge upstream/main
```

Resolve upstream changes without removing AVCD branding, the fork-owned
updater destination, or the disabled upstream telemetry configuration.

## Attribution and license

Avocado Work is a modified distribution of
[goose](https://github.com/aaif-goose/goose), developed by the
[Agentic AI Foundation](https://aaif.io/) and its contributors. The project is
licensed under the Apache License 2.0. See [LICENSE](LICENSE) and
[NOTICE](NOTICE).

Upstream technical documentation remains available at
[goose-docs.ai](https://goose-docs.ai/).

## Desktop data directory

After the Avocado Work rebrand, Electron stores settings under
`~/Library/Application Support/Avocado Work` on macOS (previously
`AVCD Agent`). Local sessions and settings do not migrate automatically.

