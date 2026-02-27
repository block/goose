# Desktop UI E2E Journeys

This repo has Playwright+Electron E2E tests under:

- `ui/desktop/tests/e2e/*.spec.ts`

## Goals

We maintain multiple scenarios that reflect real user journeys, rather than a single monolithic E2E test. Each journey should:

- Navigate through a representative UI flow (pages + key actions)
- Be as environment-agnostic as possible
- Prefer stable selectors (`data-testid`, ARIA roles/names)
- Keep assertions focused on **navigation + core UI availability** (avoid depending on specific provider/model availability unless the scenario explicitly requires it)

## Running

From `ui/desktop/`:

```bash
npm run test-e2e:journeys
```

Contrast audit (axe-core, optional):

```bash
npm run test-e2e:contrast
```

## Existing journeys

### Ungated journeys (should run in any environment)

- `journey-onboarding-welcome.spec.ts`
  - First-run onboarding (Welcome page). Validates the “choose a model provider” UI is present.

- `journey-settings-providers.spec.ts`
  - Opens `#/configure-providers`.
  - This route is intentionally **not** guarded by `ProviderGuard`.

### Provider-gated journeys (require a configured provider + `RUN_E2E_PROVIDER_JOURNEYS=true`)

- `journey-create-session.spec.ts`
  - Chat happy path (open `#/pair` and send a message).

- `journey-settings-models.spec.ts`
  - Settings → Models → open/close “Switch models”.

- `journey-evaluate.spec.ts`
  - Navigate to Evaluate.

- `journey-evaluate-datasets.spec.ts`
  - Evaluate → Datasets: create a dataset via UI.

- `journey-evaluate-runs.spec.ts`
  - Evaluate → Run History: opens run details if present.
  - Optionally creates a run if `RUN_E2E_EVAL_RUNS=true`.

- `journey-monitoring.spec.ts`
  - Navigate to Monitoring.

- `journey-monitoring-details.spec.ts`
  - Monitoring: switches to Live + Tool Analytics tabs.
  - Accepts either “loaded” or “error” UI (environment dependent).

- `journey-workflows.spec.ts`
  - Navigate Recipes → Pipelines → Scheduler.

- `journey-extensions.spec.ts`
  - Navigate to Extensions.

## Conventions

### Hash routes

The desktop app uses a `HashRouter`, so routes are `#/pair`, `#/settings`, etc.

### First-run modals

Journeys should call `bootstrapFirstRunUI(page)` early to dismiss optional first-run UX (telemetry opt-out, choose-model modal, announcements).

### Provider guard

Most routes are wrapped by `ProviderGuard` and redirect to `/welcome` when no provider is configured.

Journeys that require a configured provider should skip if they detect `#/welcome`.
