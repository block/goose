# Features

Watchflow is a rule engine for GitHub: rules in YAML, enforcement on PR and push, check runs and comments in-repo. This page summarizes supported logic and capabilities in a **maintainer-first**, tech-forward way—no marketing fluff.

## Supported conditions (rule logic)

Rules are **description + event_types + parameters**. The engine matches **parameter keys** to built-in conditions. You don’t specify “validator” by name; the loader infers it from the parameters.

### Pull request

| Parameter | Condition | Description |
|-----------|-----------|-------------|
| `require_linked_issue: true` | RequireLinkedIssueCondition | PR must reference an issue (e.g. Fixes #123). |
| `title_pattern: "<regex>"` | TitlePatternCondition | PR title must match regex (e.g. conventional commits). |
| `min_description_length: N` | MinDescriptionLengthCondition | PR body length ≥ N characters. |
| `required_labels: ["A", "B"]` | RequiredLabelsCondition | PR must have these labels. |
| `min_approvals: N` | MinApprovalsCondition | At least N approvals. |
| `max_lines: N` | MaxPrLocCondition | Total additions + deletions ≤ N. (Loader accepts `max_changed_lines` as alias.) |
| `require_code_owner_reviewers: true` | RequireCodeOwnerReviewersCondition | Owners for modified paths (from CODEOWNERS) must be requested as reviewers. |
| `critical_owners: []` | CodeOwnersCondition | Changes to critical paths require code-owner review. |
| `require_path_has_code_owner: true` | PathHasCodeOwnerCondition | Every changed path must have an owner in CODEOWNERS. |
| `protected_branches: ["main"]` | ProtectedBranchesCondition | Block direct merge/target to these branches. |

### Push

| Parameter | Condition | Description |
|-----------|-----------|-------------|
| `no_force_push: true` | NoForcePushCondition | Reject force pushes. |

### Files

| Parameter | Condition | Description |
|-----------|-----------|-------------|
| `max_file_size_mb: N` | MaxFileSizeCondition | No single file > N MB. |
| `pattern` + `condition_type: "files_match_pattern"` | FilePatternCondition | Changed files must (or must not) match pattern. |

### Time and deployment

| Parameter | Condition | Description |
|-----------|-----------|-------------|
| `allowed_hours`, `timezone` | AllowedHoursCondition | Restrict when actions can run. |
| `days` | DaysCondition | Restrict by day. |
| Weekend / deployment | WeekendCondition, WorkflowDurationCondition | Deployment and workflow rules. |

### Team

| Parameter | Condition | Description |
|-----------|-----------|-------------|
| `team: "<name>"` | AuthorTeamCondition | Event author must be in the given team. |

---

## Repository analysis → one-click rules PR

- **Endpoint**: `POST /api/v1/rules/recommend` with `repo_url`; optional `installation_id` (from install link) or user token for private repos and higher rate limits.
- **Behavior**: Analyzes repo structure and recent PR history; returns suggested rules (YAML) and a PR plan. No PAT required when you hit the link from the app install flow (`?installation_id=...&repo=owner/repo`).
- **Create PR**: `POST /api/v1/rules/recommend/proceed-with-pr` creates a branch and PR that adds `.watchflow/rules.yaml`. Auth: Bearer token or `installation_id` in body.

Suggested rules use the **same parameter names** as above so they work out of the box when you merge the PR.

---

## Welcome comment when no rules file

When `.watchflow/rules.yaml` is missing and a PR is opened, Watchflow:

1. Creates a **neutral check run** (“Rules not configured”).
2. Posts a **welcome comment** with:
   - Link to [watchflow.dev/analyze](https://watchflow.dev/analyze)?installation_id=…&repo=owner/repo (no PAT needed from install flow).
   - Short “manual setup” instructions and a minimal YAML example.
   - Note that rules are read from the default branch.

So maintainers get one clear next step instead of a silent skip.

---

## Webhook and task dedup

- Each delivery is identified by **`X-GitHub-Delivery`**; we store it on `WebhookEvent` as `delivery_id`.
- **Task ID** = `hash(event_type + delivery_id + func_qualname)` when `delivery_id` is present so the “run handler” and “run processor” tasks are **both** executed per delivery. That fixes “nothing delivered to the PR as comment” when the processor was previously skipped as a duplicate.

---

## Comment commands

| Command | Purpose |
|--------|--------|
| `@watchflow acknowledge "reason"` / `@watchflow ack "reason"` | Record an acknowledgment for a violation (when the rule allows it). |
| `@watchflow evaluate "rule in plain English"` | Ask whether a rule is feasible and get suggested YAML. |
| `@watchflow help` | List commands. |

---

## GitHub integration

- **GitHub App** — Install per org/repo; we use installation tokens for API access and webhooks.
- **Webhooks** — `pull_request`, `push`; we also support `issue_comment` for acknowledgments, and deployment/workflow events for time-based and deploy rules.
- **Check runs** — Violations show up as failed/neutral check runs with a summary and link to the rules file.
- **PR comments** — Violation summary and remediation hints; acknowledgment replies parsed in-thread.

---

## Rate limiting and auth

- **Repo analysis** — Public repos: anonymous allowed (rate limit per IP). Private repos or higher limits: send `installation_id` (from install link) or Bearer token.
- **Proceed-with-PR** — Requires either Bearer token or `installation_id` in the request body.

---

## What we don’t do (by design)

- **No “natural language” enforcement** — Rules are YAML with fixed parameter names; the engine doesn’t interpret freeform text in the hot path.
- **No custom code in repo** — All logic is in conditions; you only edit YAML.
- **No separate dashboard** — Everything stays in GitHub: check runs, comments, CODEOWNERS, branch protection.

For enterprise features (team management, Slack/Linear/Jira, SOC2), see [Warestack](https://www.warestack.com/).
