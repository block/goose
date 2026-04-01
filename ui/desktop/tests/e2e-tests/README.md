# Agent-Driven E2E Tests

Replayable end-to-end tests for the Goose desktop app, created via natural language and replayed deterministically using [agent-browser](https://github.com/anthropics/agent-browser).

## Setup

Create `~/.config/goose/e2e.env` with your provider config:

```bash
GOOSE_PROVIDER=anthropic
GOOSE_MODEL=claude-haiku-4-5-20251001
ANTHROPIC_API_KEY=your-api-key-here
```

## Running Tests

- **Run all tests** (builds goosed, generates API types, runs in parallel):
  ```bash
  just e2e
  ```
- **Run a specific test** by regex:
  ```bash
  just e2e-setup
  bash ui/desktop/tests/e2e-tests/scripts/e2e-run-all.sh --only settings*
  ```
- **Skip a test**: rename with `.skip` (e.g., `settings-dark-mode.skip.batch.json`)
- **Stop all sessions** and clean up:
  ```bash
  bash ui/desktop/tests/e2e-tests/scripts/e2e-stop.sh
  ```

## Creating Tests

Describe the test scenario with steps in Goose. The agent will load the `create-e2e-test` skill to explore the app, record actions, and produce a `.batch.json` file and test scenario file.

## Debugging Failures

Use the `debug_e2e_failures` recipe to diagnose test failures from local runs or CI artifacts:

```bash
# Debug all failures from the latest local run
workflow_recipes/debug_e2e_failures/run.sh

# Debug a specific test
workflow_recipes/debug_e2e_failures/run.sh --test-name settings-dark-mode

# Debug from CI artifacts
workflow_recipes/debug_e2e_failures/run.sh --results-url https://github.com/block/goose/actions/runs/<run-id>/artifacts/<artifact-id>
```
