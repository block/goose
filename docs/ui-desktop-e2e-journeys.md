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

- `journey-onboarding-welcome.spec.ts`
  - First-run onboarding (Welcome page). Should run even without a configured provider.

- `journey-create-session.spec.ts`
  - Chat happy path (create session + send message).
  - Skips automatically if the app redirects to `/welcome` (no provider configured).

- `journey-settings-models.spec.ts`
  - Settings → Models → open/close “Switch models”.
  - Skips if no provider configured.

- `journey-settings-providers.spec.ts`
  - Settings → Providers (via `#/configure-providers`).
  - Light assertions (provider inventory is environment-specific).

- `journey-evaluate.spec.ts`
  - Navigate to Evaluate.
  - Skips if no provider configured.

- `journey-monitoring.spec.ts`
  - Navigate to Monitoring.
  - Skips if no provider configured.

- `journey-workflows.spec.ts`
  - Navigate Recipes → Pipelines → Scheduler.
  - Skips if no provider configured.

- `journey-extensions.spec.ts`
  - Navigate to Extensions.
  - Skips if no provider configured.

## Conventions

### Hash routes

The desktop app uses a `HashRouter`, so routes are `#/pair`, `#/settings`, etc.

### First-run modals

Journeys should call `bootstrapFirstRunUI(page)` early to dismiss optional first-run UX (telemetry opt-out, choose-model modal, announcements).

### Provider guard

Most routes are wrapped by `ProviderGuard` and redirect to `/welcome` when no provider is configured.

Journeys that require a configured provider should skip if they detect `#/welcome`.
