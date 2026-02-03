# Configuration Guide

This guide covers how to configure Watchflow rules. Rules are **description + event_types + parameters**; the engine matches parameter keys to built-in conditions. No custom code—just YAML in `.watchflow/rules.yaml` on the default branch.

**Pro tip:** Test rule ideas at [watchflow.dev](https://watchflow.dev). Use “Evaluate” for feasibility and suggested YAML, or run repo analysis to get a full suggested config. When you land from the app install flow (`?installation_id=...&repo=owner/repo`), no PAT is required.

---

## Rule structure

Each rule has:

| Field | Required | Description |
|-------|----------|-------------|
| `description` | Yes | Short human-readable description (used in check runs and comments). |
| `enabled` | No | Default `true`. Set `false` to disable without deleting. |
| `severity` | No | `low` \| `medium` \| `high` \| `critical`. Drives presentation, not logic. |
| `event_types` | Yes | Events this rule runs on: `pull_request`, `push`, `deployment`, etc. |
| `parameters` | Yes | Key-value map. **Keys determine which condition runs** (e.g. `require_linked_issue`, `max_lines`). |

The loader reads `.watchflow/rules.yaml` from the repo default branch and builds `Rule` objects with condition instances from the **condition registry**. Parameter names must match what the conditions expect; see below.

---

## Parameter reference (supported logic)

### Pull request conditions

**Linked issue**

```yaml
parameters:
  require_linked_issue: true
```

PR must reference an issue (e.g. “Fixes #123”) in title or body.

**Title pattern**

```yaml
parameters:
  title_pattern: "^feat|^fix|^docs|^style|^refactor|^test|^chore|^perf|^ci|^build|^revert"
```

PR title must match the regex (e.g. conventional commits).

**Description length**

```yaml
parameters:
  min_description_length: 50
```

PR body length must be ≥ N characters.

**Required labels**

```yaml
parameters:
  required_labels: ["Type/Bug", "Type/Feature", "Status/Review"]
```

PR must have all of these labels.

**Min approvals**

```yaml
parameters:
  min_approvals: 2
```

At least N approvals required.

**Max PR size (lines)**

```yaml
parameters:
  max_lines: 500
```

Total additions + deletions must be ≤ N. The loader also accepts `max_changed_lines` as an alias.

**CODEOWNERS: require owners as reviewers**

```yaml
parameters:
  require_code_owner_reviewers: true
```

For every file changed, the corresponding CODEOWNERS entries must be in the requested reviewers (users or teams). If CODEOWNERS is missing, the condition skips (no violation).

**CODEOWNERS: path must have owner**

```yaml
parameters:
  require_path_has_code_owner: true
```

Every changed path must have at least one owner in CODEOWNERS. If CODEOWNERS is missing, the condition skips.

**Critical paths / code owners**

```yaml
parameters:
  critical_owners: []   # or list of path patterns if supported
```

Changes to critical paths require code-owner review. (See registry for exact semantics.)

**Protected branches**

```yaml
parameters:
  protected_branches: ["main", "master"]
```

Blocks targeting these branches (e.g. merge without going through PR flow as configured).

### Push conditions

**No force push**

```yaml
parameters:
  no_force_push: true
```

Reject force pushes. Typically used with `event_types: ["push"]`.

### File conditions

**Max file size**

```yaml
parameters:
  max_file_size_mb: 1
```

No single file in the PR may exceed N MB.

**File pattern**

```yaml
parameters:
  pattern: "tests/.*\\.py$|test_.*\\.py$"
  condition_type: "files_match_pattern"
```

Changed files must (or must not) match the pattern; exact behavior depends on condition.

### Time and deployment

**Allowed hours, days, weekend** — See condition registry and examples in repo for `allowed_hours`, `timezone`, `days`, and deployment-related parameters.

---

## Example rules

**Linked issue + PR size + CODEOWNERS reviewers**

```yaml
rules:
  - description: "PRs must reference a linked issue (e.g. Fixes #123)"
    enabled: true
    severity: high
    event_types: ["pull_request"]
    parameters:
      require_linked_issue: true

  - description: "PR total lines changed must not exceed 500"
    enabled: true
    severity: medium
    event_types: ["pull_request"]
    parameters:
      max_lines: 500

  - description: "When a PR modifies paths with CODEOWNERS, those owners must be added as reviewers"
    enabled: true
    severity: high
    event_types: ["pull_request"]
    parameters:
      require_code_owner_reviewers: true
```

**Title pattern + description length**

```yaml
rules:
  - description: "PR titles must follow conventional commit format; descriptions must be at least 50 chars"
    enabled: true
    severity: medium
    event_types: ["pull_request"]
    parameters:
      title_pattern: "^feat|^fix|^docs|^style|^refactor|^test|^chore"
      min_description_length: 50
```

**No force push to main**

```yaml
rules:
  - description: "No direct pushes to main - all changes must go through PRs"
    enabled: true
    severity: critical
    event_types: ["push"]
    parameters:
      no_force_push: true
```

---

## Severity levels

- **low** — Informational; no blocking.
- **medium** — Warning; often acknowledgable.
- **high** — Blocking unless acknowledged (when the rule allows).
- **critical** — Blocking; acknowledgment may not be allowed depending on rule.

Severity affects how violations are presented in check runs and comments; it does not change the condition logic.

---

## Event types

- **pull_request** — PR opened, updated, synchronized, etc.
- **push** — Pushes to branches (use `no_force_push` for branch protection).
- **deployment** / **deployment_status** / **deployment_review** — Deployment protection and time-based deploy rules.
- **issue_comment** — Used for parsing `@watchflow acknowledge` and similar commands.

---

## Where rules are read from

Rules are loaded from **`.watchflow/rules.yaml` on the repo default branch** (e.g. `main`) via the GitHub API using the installation token. So:

- Changes to `.watchflow/rules.yaml` take effect after merge to the default branch.
- No local clone or filesystem access is required for evaluation; PR data and CODEOWNERS content are fetched by the enricher.

---

## Best practices

1. **Start small** — Enable one or two rules (e.g. `require_linked_issue`, `require_code_owner_reviewers`), then add more.
2. **Use watchflow.dev** — Run repo analysis or feasibility to get suggested YAML that uses the correct parameter names.
3. **Version control** — Keep `.watchflow/rules.yaml` in the repo and review rule changes in PRs.
4. **Acknowledgment** — Use `@watchflow acknowledge "reason"` for legitimate one-off exceptions; don’t use it to bypass policy routinely.

For the full list of conditions and parameter names, see [Features](../features.md) and the source: `src/rules/conditions/` and `src/rules/registry.py`.
