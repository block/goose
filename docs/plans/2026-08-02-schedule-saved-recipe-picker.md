# Plan: schedule-saved-recipe-picker

**Goal:** Let a desktop user create a schedule by selecting a recipe they already have saved, directly inside the ScheduleModal create flow — instead of only browsing a YAML file or pasting a deep link.

**Affected files:**
- `ui/desktop/src/components/schedule/ScheduleModal.tsx` — add a third source tab "Saved recipes"; lazy-load saved recipes via `listSavedRecipes()`; render a searchable `Select` (react-select) picker; on selection reuse the existing `setParsedRecipe` + `setScheduleIdFromTitle` path; show the existing "recipe parsed successfully" preview.
- `ui/desktop/src/components/schedule/__tests__/ScheduleModal.test.tsx` — add a test asserting the Saved-recipes source is present and selecting a recipe enables Create (mocks `listSavedRecipes`).
- i18n message JSON (if present) — add new keys for the tab label, placeholder, and empty/loading/error states.

**Approach:** Add `'saved'` to the existing `SourceType` union (`'file' | 'deeplink' | 'saved'`) and a third toggle button, matching the current two-button pill. When `'saved'` becomes active, lazily fetch `listSavedRecipes()` (from `../../recipe/recipe_management`, already a dependency of this file via `getStorageDirectory`) and render the reusable `Select` component (`components/ui/Select.tsx`, a searchable `react-select` wrapper) with one option per saved recipe (value = recipe id, label = title). On change, resolve the `RecipeManifest`, call `setParsedRecipe(manifest.recipe)` and `setScheduleIdFromTitle(title)`. The existing submit validation (`if (!parsedRecipe)`) and payload construction work unchanged. Reuse the green "recipe parsed successfully" preview block already used by the deeplink source.

**Edge cases:**
- Empty saved-recipe list → show an empty-state hint (no crash, Create stays disabled until a valid recipe exists).
- Load failure → show an inline error inside the saved tab; the other two sources remain usable.
- Edit mode → the source picker is already hidden in edit mode (only cron is editable), so no change needed.
- Reopening the modal → existing reset `useEffect` must also clear the saved selection; add the saved state to that reset.
- `RecipeDto` → `Recipe` assignability: `Recipe = RecipeDto & { optional fields }`, so `manifest.recipe` (a `Recipe`) is already the correct type.

**Test plan:**
- Unit: render modal in create mode, switch to "Saved recipes", assert the `Select` appears and that selecting a recipe (mocked `listSavedRecipes`) sets up `parsedRecipe` so the Create button enables and submit calls `onSubmit` with a `NewSchedulePayload`.
- Manual smoke: open Create Schedule, pick a saved recipe, confirm id auto-derives and Create is enabled; create and see the schedule appear in the list.

**Conventions to follow:**
- react-intl `defineMessages` + `useIntl` (already used in this file) for all new strings.
- Reuse `Select` primitive instead of a bespoke list.
- Match the existing toggle-pill styling for the new tab button.

**Open questions / risks:**
- Whether translation JSONs are build-enforced (if so, missing keys could fail `pnpm typecheck`/lint) — verify in i18n dir.

**Estimated size:** S (~60–90 LOC in ScheduleModal + test).
