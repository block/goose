# Quick Start Guide

Get Watchflow running in a few minutes: install the app, add `.watchflow/rules.yaml`, and verify with a PR or push. No new dashboards—everything stays in GitHub.

---

## What you get

- **Rule evaluation** on every PR and push against your YAML rules.
- **Check runs** and **PR comments** when rules are violated (or when no rules file exists, a welcome comment with a link to set one up).
- **Acknowledgment** in-thread: `@watchflow acknowledge "reason"` where the rule allows it.
- **One config file** — `.watchflow/rules.yaml` on the default branch; rules are loaded from there via the GitHub API.

---

## Prerequisites

- **GitHub repo** where you have admin (or can install a GitHub App).
- **A few minutes** to install the app and add a rules file.

---

## Step 1: Install the GitHub App

1. Go to [Watchflow GitHub App](https://github.com/apps/watchflow).
2. Click **Install** and choose the org/repos you want to protect.
3. Grant the requested permissions (webhooks, repo content for rules and PR data).

Watchflow will start receiving webhooks. If there’s no `.watchflow/rules.yaml` yet, the first PR will get a **welcome comment** with a link to [watchflow.dev](https://watchflow.dev) (including `installation_id` and `repo`) so you can run repo analysis and create a rules PR **without entering a PAT**.

---

## Step 2: Add rules

**Option A — From the welcome comment (no PAT)**

1. Open a PR (or any PR) and find the Watchflow welcome comment.
2. Click the link to **watchflow.dev/analyze?installation_id=…&repo=owner/repo**.
3. Run repo analysis; review suggested rules and click **Create PR** to add `.watchflow/rules.yaml` to a branch.

**Option B — Manual**

Create `.watchflow/rules.yaml` in the repo root on the default branch, for example:

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

  - description: "No direct pushes to main - all changes via PRs"
    enabled: true
    severity: critical
    event_types: ["push"]
    parameters:
      no_force_push: true
```

Parameter names must match the [supported conditions](configuration.md); see [Configuration](configuration.md) for the full reference.

---

## Step 3: Verify

1. **Open a PR** (or push to a protected branch if you use `no_force_push`).
2. Check **GitHub Checks** for the Watchflow check run (pass / fail / neutral).
3. If a rule is violated, you should see a **PR comment** with the violation and remediation hint.
4. Where the rule allows it, reply with:
   `@watchflow acknowledge "Documentation-only change, no code impact"`
   (or `@watchflow ack "…"`).

---

## Comment commands

| Command | Purpose |
|--------|--------|
| `@watchflow acknowledge "reason"` / `@watchflow ack "reason"` | Record an acknowledgment for a violation (when the rule allows it). |
| `@watchflow evaluate "rule in plain English"` | Ask whether a rule is feasible and get suggested YAML. |
| `@watchflow help` | List commands. |

---

## Next steps

- **Tune rules** — [Configuration](configuration.md) for parameter reference and examples.
- **See supported logic** — [Features](../features.md) for all conditions and capabilities.
- **Architecture** — [Concepts / Overview](../concepts/overview.md) for flow and components.

---

*Watchflow: the immune system for your repo. Rules in YAML, enforcement in GitHub.*
