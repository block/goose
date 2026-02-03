# Welcome to Watchflow

<div class="grid cards" markdown>

-   :fontawesome-solid-rocket: __[Quick Start](getting-started/quick-start.md)__

    Get up and running in minutes

-   :fontawesome-solid-shield: __[Features](features.md)__

    Supported conditions and capabilities

-   :fontawesome-solid-chart-line: __[Benchmarks](benchmarks.md)__

    Real-world impact and results

-   :fontawesome-solid-cog: __[Configuration](getting-started/configuration.md)__

    Rule reference and parameter details

</div>

## What is Watchflow?

Watchflow is a **rule engine for GitHub** that runs where you already work: no new dashboards, no “AI-powered” hand-waving. You define rules in `.watchflow/rules.yaml`; we evaluate them on every PR and push and surface violations as check runs and PR comments. Think of it as the **immune system** for your repo—consistent enforcement so maintainers don’t have to chase reviewers or guess what’s allowed.

We built it for teams that still care about traceability, CODEOWNERS, and review quality. Rules are description + event types + parameters; the engine matches parameters to built-in conditions (linked issues, PR size, code owner reviewers, title patterns, branch protection, and more). Optional repo analysis suggests rules from your PR history; you keep full control.

### The problem we solve

- **Static branch protection** can’t enforce “CODEOWNERS must be requested” or “PR must reference an issue.”
- **Generic governance tools** add another abstraction layer and another place to look.
- **Maintainers** end up manually checking the same things on every PR.

### What Watchflow does

- **Lives in the repo** — one config file, version-controlled, next to CODEOWNERS and workflows.
- **Deterministic rule evaluation** — condition-based checks on PR/push; no LLM in the hot path for enforcement.
- **Maintainer-first** — check runs and comments in GitHub; acknowledgments in-thread with `@watchflow acknowledge "reason"`.
- **Optional intelligence** — repo analysis and feasibility checks when you want suggestions; enforcement stays rule-driven.

## Key features

- **Condition-based rules** — `require_linked_issue`, `max_lines`, `require_code_owner_reviewers`, `no_force_push`, title patterns, approvals, labels, and more.
- **CODEOWNERS-aware** — Require owners for modified paths to be requested as reviewers; or require every changed path to have an owner.
- **Webhook-native** — Uses GitHub delivery IDs so handler and processor both run; comments and check runs stay in sync.
- **Install-flow friendly** — When no rules file exists, we post a welcome comment with a link to watchflow.dev (installation_id + repo) so you can run analysis and create a rules PR without a PAT.

## Quick example

Instead of hoping everyone remembers to request CODEOWNERS:

```yaml
rules:
  - description: "When a PR modifies paths with CODEOWNERS, those owners must be added as reviewers"
    enabled: true
    severity: high
    event_types: ["pull_request"]
    parameters:
      require_code_owner_reviewers: true
```

Watchflow checks modified files, resolves owners from CODEOWNERS, and ensures they’re in the requested reviewers list. One rule, no custom code.

## Get started

- [Quick Start](getting-started/quick-start.md) — Install the app, add `.watchflow/rules.yaml`, verify.
- [Configuration](getting-started/configuration.md) — Full parameter reference and examples.
- [Features](features.md) — Supported conditions and capabilities.

## Community

- **GitHub**: [warestack/watchflow](https://github.com/warestack/watchflow)
- **Discussions**: [GitHub Discussions](https://github.com/warestack/watchflow/discussions)
- **Issues**: [GitHub Issues](https://github.com/warestack/watchflow/issues)

---

*Watchflow: the immune system for your repo. Rules in YAML, enforcement in GitHub.*
