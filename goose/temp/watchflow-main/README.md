# Watchflow

[![Works with GitHub](https://img.shields.io/badge/Works%20with-GitHub-1f1f23?style=for-the-badge&logo=github)](https://github.com/warestack/watchflow)

GitHub governance that runs where you already work. No new dashboards, no “AI-powered” fluff—just rules in YAML, evaluated on every PR and push, with check runs and comments that maintainers actually read.

Watchflow is the governance layer for your repo: it enforces the policies you define (CODEOWNERS, approvals, linked issues, PR size, title patterns, branch protection) so you don’t have to chase reviewers or guess what’s allowed. Built for teams that still care about traceability and review quality.

---

## Why Watchflow?

Static branch protection can’t see *who* changed *what* or whether CODEOWNERS were actually requested. Generic “AI governance” tools add another layer of abstraction and another place to look. We wanted something that:

- **Lives in the repo** — `.watchflow/rules.yaml` next to your CODEOWNERS and workflows
- **Uses the same mental model** — conditions, parameters, event types; no “natural language” magic that doesn’t map to code
- **Fits the maintainer workflow** — check runs, PR comments, acknowledgments in-thread
- **Scales with your stack** — GitHub App, webhooks, one config file

So we built Watchflow: a rule engine that evaluates PRs and pushes against your rules, posts violations as check runs and comments, and lets developers acknowledge with a reason when the rule doesn’t fit the case. Optional repo analysis suggests rules from your PR history; you keep full control.

---

## How it works

1. **Install the GitHub App** and point it at your repos.
2. **Add `.watchflow/rules.yaml`** (or get a suggested one from [watchflow.dev](https://watchflow.dev) using repo analysis).
3. On **pull_request** and **push** events, Watchflow loads rules, enriches with PR data (files, reviews, CODEOWNERS), runs **condition-based evaluation** (no LLM in the hot path for rule checks).
4. Violations show up as **check runs** and **PR comments**; developers can reply with `@watchflow acknowledge "reason"` where the rule allows it.

Rules are **description + event_types + parameters**. The engine matches parameters to built-in conditions (e.g. `require_linked_issue`, `max_lines`, `require_code_owner_reviewers`, `no_force_push`). No custom code in the repo—just YAML.

---

## Supported logic (conditions)

| Area | Condition / parameter | Event | What it does |
| ------ | ------------------------ | ------ | ---------------- |
| **PR** | `require_linked_issue: true` | pull_request | PR must reference an issue (e.g. Fixes #123). |
| **PR** | `title_pattern: "^feat\\|^fix\\|..."` | pull_request | PR title must match regex. |
| **PR** | `min_description_length: 50` | pull_request | Body length ≥ N characters. |
| **PR** | `required_labels: ["Type/Bug", "Status/Review"]` | pull_request | PR must have these labels. |
| **PR** | `min_approvals: 2` | pull_request | At least N approvals. |
| **PR** | `max_lines: 500` | pull_request | Total additions + deletions ≤ N (alias: `max_changed_lines`). |
| **PR** | `require_code_owner_reviewers: true` | pull_request | Owners for modified paths (CODEOWNERS) must be requested as reviewers. |
| **PR** | `critical_owners: []` / code owners | pull_request | Changes to critical paths require code-owner review. |
| **PR** | `require_path_has_code_owner: true` | pull_request | Every changed path must have an owner in CODEOWNERS. |
| **PR** | `protected_branches: ["main"]` | pull_request | Block direct targets to these branches. |
| **Push** | `no_force_push: true` | push | Reject force pushes. |
| **Files** | `max_file_size_mb: 1` | pull_request | No single file > N MB. |
| **Files** | `pattern` + `condition_type: "files_match_pattern"` | pull_request | Changed files must (or must not) match glob/regex. |
| **Time** | `allowed_hours`, `days`, weekend | deployment / workflow | Restrict when actions can run. |
| **Deploy** | `environment`, approvals | deployment | Deployment protection. |

Rules are read from the **default branch** (e.g. `main`). Each webhook delivery is deduplicated by `X-GitHub-Delivery` so handler and processor both run; comments and check runs stay in sync.

---

## Rule format

```yaml
rules:
  - description: "PRs must reference a linked issue (e.g. Fixes #123)"
    enabled: true
    severity: high
    event_types: ["pull_request"]
    parameters:
      require_linked_issue: true

  - description: "When a PR modifies paths with CODEOWNERS, those owners must be added as reviewers"
    enabled: true
    severity: high
    event_types: ["pull_request"]
    parameters:
      require_code_owner_reviewers: true

  - description: "PR total lines changed must not exceed 500"
    enabled: true
    severity: medium
    event_types: ["pull_request"]
    parameters:
      max_lines: 500

  - description: "No direct pushes to main - all changes via PRs"
    enabled: true
    severity: critical
    event_types: ["push"]
    parameters:
      no_force_push: true
```

Severity drives how violations are presented; `event_types` limit which events the rule runs on. Parameters are fixed per condition—see the [configuration guide](docs/getting-started/configuration.md) for the full set.

---

## Quick start

1. **Install** — [Watchflow GitHub App](https://github.com/apps/watchflow), select repos.
2. **Configure** — Add `.watchflow/rules.yaml` in the repo root (or use [watchflow.dev](https://watchflow.dev) to generate one from repo analysis; use `?installation_id=...&repo=owner/repo` from the app install flow so no PAT is required).
3. **Verify** — Open a PR or push; check runs and comments will reflect your rules. Use `@watchflow acknowledge "reason"` where acknowledgments are allowed.

Detailed steps: [Quick Start](docs/getting-started/quick-start.md). Configuration reference: [Configuration](docs/getting-started/configuration.md).

---

## Comment commands

| Command | Purpose |
| -------- | -------- |
| `@watchflow acknowledge "reason"` / `@watchflow ack "reason"` | Record an acknowledgment for a violation (when the rule allows it). |
| `@watchflow evaluate "rule in plain English"` | Ask whether a rule is feasible and get suggested YAML. |
| `@watchflow help` | List commands. |

---

## API and repo analysis

- **`POST /api/v1/rules/recommend`** — Analyze a repo (structure, PR history) and return suggested rules. Accepts `repo_url`; optional `installation_id` (from install link) or user token for private repos and higher rate limits.
- **`POST /api/v1/rules/recommend/proceed-with-pr`** — Create a PR that adds `.watchflow/rules.yaml` from recommended rules. Auth: Bearer token or `installation_id` in body.

When no `.watchflow/rules.yaml` exists and a PR is opened, Watchflow posts a **welcome comment** with a link to watchflow.dev (including `installation_id` and `repo`) so maintainers can run analysis and create a rules PR without entering a PAT.

---

## Docs and support

- [Quick Start](docs/getting-started/quick-start.md)
- [Configuration](docs/getting-started/configuration.md)
- [Features](docs/features.md)
- [Development](DEVELOPMENT.md)

Issues and discussions: [GitHub](https://github.com/warestack/watchflow). For enterprise features (team management, Slack/Linear/Jira, SOC2), see [Warestack](https://www.warestack.com/).

---

## License

MIT — see [LICENSE](LICENSE).
