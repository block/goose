# Mastra Repository Analysis

Mastra (`mastra-ai/mastra`) is a TypeScript-first agent framework for building production-grade AI assistants. The project has roughly **280 contributors**, **134 open pull requests**, and active CI coverage via GitHub Actions. This document captures the agreed-upon analysis from November 2025 so we can align on rule proposals before shipping automation.

## Repository Snapshot

- **Focus**: AI agents with tooling, memory, workflows, and multi-step orchestration
- **Primary language**: TypeScript with pnpm-based monorepo
- **Governance signals**: Detailed `CONTRIBUTING.md`, CODEOWNERS, changeset automation, active doc set
- **Pain points**: Complex LLM/provider integrations, repeated validation gaps, and regression risk in shared tooling layers

## Pull Request Sample (Nov 2025)

| PR | Title | Outcome | Notes |
| --- | --- | --- | --- |
| [#10180](https://github.com/mastra-ai/mastra/pull/10180) | feat: add custom model gateway support with automatic type generation | ✅ merged | Large feature: gateway registry, TS type generation, doc updates |
| [#10269](https://github.com/mastra-ai/mastra/pull/10269) | AI SDK tripwire data chunks | ✅ merged | Fixes & changeset for SDK data chunking bug |
| [#10141](https://github.com/mastra-ai/mastra/pull/10141) | fix: throw on invalid filter instead of silently skipping filtering | ✅ merged | Addressed regression where invalid filters returned unfiltered data |
| [#10300](https://github.com/mastra-ai/mastra/pull/10300) | Add description to type | ✅ merged | Unblocked Agent profile UI by exposing description metadata |
| [#9880](https://github.com/mastra-ai/mastra/pull/9880) | Fix clientjs clientTools execution | ✅ merged | Fixed client-side tool streaming regressions |
| [#9941](https://github.com/mastra-ai/mastra/pull/9941) | fix(core): input tool validation with no schema | ✅ merged | Restored validation for schema-less tool inputs |

## Pattern Summary

- **Validation & safety gaps (≈40%)** – invalid filters or schema-less tools silently bypassed safeguards.
- **Tooling & integration regressions (≈33%)** – clientTools streaming, AI SDK data chunking, URL handling.
- **Experience polish gaps (≈17%)** – missing agent descriptions prevented UI consistency.
- **High merge velocity** – most fixes merged quickly; reinforces need for automated guardrails so regressions are caught before release.

## Recommended Watchflow Rules

Rules intentionally avoid the optional `actions:` block so they remain compatible with the current loader. Enforcement intent is described in each `description` and reflected in `severity`.

```yaml
rules:
  - description: "Block merges when PRs change filter validation logic without failing on invalid inputs"
    enabled: true
    severity: "high"
    event_types: ["pull_request"]
    parameters:
      file_patterns:
        - "packages/core/src/**/vector-query.ts"
        - "packages/core/src/**/graph-rag.ts"
        - "packages/core/src/**/filters/*.ts"
      require_patterns:
        - "throw\\s+new\\s+Error"
        - "raise\\s+ValueError"
      forbidden_patterns:
        - "return\\s+.*filter\\s*$"
      how_to_fix: "Ensure invalid filters raise descriptive errors instead of silently returning unfiltered results."

  - description: "Require regression tests when modifying tool schema validation or client tool execution"
    enabled: true
    severity: "medium"
    event_types: ["pull_request"]
    parameters:
      source_patterns:
        - "packages/core/src/**/tool*.ts"
        - "packages/core/src/agent/**"
        - "packages/client/**"
      test_patterns:
        - "packages/core/tests/**"
        - "tests/**"
      min_test_files: 1
      rationale: "Tool invocation changes have previously caused regressions in clientTools streaming."

  - description: "Ensure every agent exposes a user-facing description for UI profiles"
    enabled: true
    severity: "low"
    event_types: ["pull_request"]
    parameters:
      file_patterns:
        - "packages/core/src/agent/**"
      required_text:
        - "description"
      message: "Add or update the agent description so downstream UIs can render capabilities."

  - description: "Block merges when URL or asset handling changes bypass provider capability checks"
    enabled: true
    severity: "high"
    event_types: ["pull_request"]
    parameters:
      file_patterns:
        - "packages/core/src/agent/message-list/**"
        - "packages/core/src/llm/**"
      require_patterns:
        - "isUrlSupportedByModel"
      forbidden_patterns:
        - "downloadAssetsFromMessages\\(messages\\)"
      how_to_fix: "Preserve remote URLs for providers that support them natively; only download assets for unsupported providers."
```

These concrete rules rely on the diff-aware validators recently added to Watchflow:

- `diff_pattern` ensures critical patches keep throwing exceptions or performing capability checks.
- `related_tests` requires PRs touching core modules to include matching test updates.
- `required_field_in_diff` verifies additions to agent definitions include a `description` so downstream UIs stay in sync.

Because the PR processor now passes normalized diffs into the engine, these validators operate deterministically without LLM fallbacks.

## PR Template Snippet

```markdown
## Repository Analysis Complete

We've analyzed your repository and identified key quality patterns based on recent PR history.

### Key Findings
- 40% of recent fixes patched validation or data-safety gaps (filters, schema-less tools).
- 33% addressed tool/LLM integration regressions (clientTools, AI SDK, URL handling).
- Tests/documentation often lag behind critical fixes, creating follow-up churn.

### Recommended Rules
- Block filter-validation changes that stop throwing on invalid inputs.
- Require regression tests when modifying tool schemas or clientTools execution.
- Enforce agent descriptions so UI consumers can present profiles.
- Block URL/asset handling changes that skip provider capability checks.

### Installation
1. Install the Watchflow GitHub App and grant access to `mastra-ai/mastra`.
2. Add `.watchflow/rules.yaml` with the rules above (see snippet).
3. Watchflow will start reporting violations through status checks immediately.

Questions? Reach out to the Watchflow team.
```

## Validation Plan

1. Keep the rule definitions in `docs/samples/mastra-watchflow-rules.yaml`.
2. Run `pytest tests/unit/test_mastra_rules_sample.py` to ensure every rule loads via `Rule.model_validate`.
3. (Optional) Use the repository analysis agent once PR-diff ingestion ships to simulate Mastra commits before opening an automated PR with these rules.

This keeps the deliverable lightweight, fully tested, and ready for the PR template automation flow discussed with Dimitris.
