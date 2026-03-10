---
title: Human-in-the-Loop Workflow Recipes
description: Build approval-gated Goose recipes that stay broadly useful and locally testable
---

Goose recipes are often used as task starters, but they also work well for **approval-gated workflows** where the agent should investigate first, produce an artifact, and only continue after an explicit human decision.

This pattern is useful when the task is high-trust or hard to reverse, such as:

- reviewing a risky shell command before execution
- preparing a release recommendation before signoff
- summarizing a proposed change before any file edits

For external approval systems, see the [gotoHuman MCP tutorial](/docs/mcp/gotohuman-mcp). The recipes in this guide focus on **local-first**, **maintainer-testable** workflows that only require Goose's built-in tools.

## Pattern

A mergeable human-in-the-loop recipe should usually follow this sequence:

1. Inspect local state and gather evidence.
2. Write a review artifact to a deterministic file path.
3. Present a concise summary to the user.
4. Ask for an explicit approval phrase such as `APPROVE` or `REJECT`.
5. Only perform the gated action after approval.

This keeps the workflow safe and makes it easy for maintainers to evaluate the recipe without credentials, SaaS accounts, or organization-specific context.

## Local Testability

When contributing approval-gated recipes to the cookbook, optimize for local verification:

- Prefer built-in extensions like `developer`.
- Write a report file with a predictable name such as `command_review.md`.
- Keep approval inputs explicit and easy to inspect.
- Avoid remote APIs, paid services, and browser logins.
- Add a simple `retry.checks` rule so success can be validated automatically.

Even if a reviewer does not continue past the approval gate, they can still verify that the recipe created the expected review artifact and followed the workflow correctly.

## Example Recipes

This repository includes two local-first examples:

- [Risky Command Review](https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/risky-command-review.yaml)
- [Release Signoff](https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/release-signoff.yaml)

## Suggested Validation Flow

Validate the recipe schema first:

```bash
goose recipe validate documentation/src/pages/recipes/data/recipes/risky-command-review.yaml
goose recipe validate documentation/src/pages/recipes/data/recipes/release-signoff.yaml
```

Then run a local review flow against a small sample workspace or repository:

```bash
goose run \
  --recipe documentation/src/pages/recipes/data/recipes/risky-command-review.yaml \
  --params 'workspace_path=/path/to/workspace' \
  --params 'command=touch approved.txt' \
  --params 'objective=Create a marker file only after explicit review'
```

```bash
goose run \
  --recipe documentation/src/pages/recipes/data/recipes/release-signoff.yaml \
  --params 'repo_path=/path/to/repo' \
  --params 'release_context=v1.2.3 release candidate for local verification'
```

In both cases, the expected first checkpoint is the generated report file:

- `/path/to/workspace/command_review.md`
- `/path/to/repo/release_signoff.md`

That makes the workflow easy to review even before the approval step is completed.
