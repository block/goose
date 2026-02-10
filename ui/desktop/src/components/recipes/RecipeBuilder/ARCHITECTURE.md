# AI Recipe Builder - Architecture Summary

## Feature Overview

A conversational interface for creating Goose recipes through AI assistance, integrated into the existing RecipesView. Users can create recipes via natural language conversation, manually edit them in a form, test them in real-time, and save them to disk.

**Branch:** `lifei/ai-edit-toggle-recipe`
**Commits:** 4 (foundation → basic functionality → UI polish → better test chat)
**Scope:** 10 new files, +1221 lines

---

## Component Architecture

```
RecipeBuilder/
├── index.tsx                 — Main container, state orchestrator
├── RecipeBuilderHeader.tsx   — Nav toggle (Chat/Edit), Test, Save, Close buttons
├── RecipeBuilderChat.tsx     — AI conversation for recipe generation
├── RecipeBuilderEdit.tsx     — Form-based manual recipe editor
├── RecipeBuilderTest.tsx     — Live recipe testing in a split panel
├── recipeBuilderRecipe.ts    — System prompt instructing AI on recipe format
├── recipeExtractor.ts        — YAML extraction & parsing from AI output
├── types.ts                  — TypeScript interfaces
└── PLAN.md                   — Implementation plan
```

---

## How It Works

### Chat View
User describes a recipe conversationally. The AI outputs YAML, which is automatically extracted and parsed into a `Recipe` object via `recipeExtractor.ts`. Key functions:
- `extractYamlFromText()` — Regex-based extraction of last YAML code block
- `parseYamlToRecipe()` — Async YAML→Recipe conversion using `parseRecipeFromFile()`
- Extraction only runs when streaming completes (not mid-stream)

### Edit View
A TanStack React Form for manual recipe editing. Changes sync bidirectionally with Chat:
- If the user edits in the form, the next chat message prepends the updated YAML as context for the AI
- External updates (from Chat) reset the form without triggering change callbacks
- Bidirectional conversion: `recipeToFormData()` ↔ `formDataToRecipe()`

### Test Panel
Opens as a right-side split panel:
- Starts a real session using the recipe
- Handles parameter substitution via input modal
- Detects if the recipe changed since the test started (yellow warning banner + restart button)
- Session terminates when panel closes

### Save
Validates recipe has title + description, calls `saveRecipe()` API, refreshes the recipe list, shows toast notification.

---

## State Management

All state lives in `index.tsx` (single source of truth):

```typescript
currentView: 'chat' | 'edit'           // Which main view is shown
testPanelOpen: boolean                  // Whether test panel is visible
recipe: Recipe | null                   // The recipe being built
testRecipeSnapshot: Recipe | null       // Recipe version when test started
isSaving: boolean                       // Save operation in progress
```

Derived flags:
- `canTest` — True when recipe exists
- `canSave` — True when recipe has title AND description

---

## Data Flow

```
User Interactions
│
├─► Chat View Input
│   └─► AI Response (streaming)
│       └─► Extract YAML → Parse Recipe → setRecipe()
│
├─► Edit View Change
│   └─► Form submission → setRecipe()
│
├─► View Toggle (Chat ↔ Edit)
│   └─► Edit changes prepended as YAML context on next chat message
│
├─► Test Panel Toggle
│   └─► Open: capture snapshot → start session
│   └─► Change detected: show warning + restart button
│   └─► Close: terminate session
│
└─► Save Button
    └─► saveRecipe() API → reload list → toast
```

---

## Sync Rules

| Event | Action |
|-------|--------|
| AI outputs YAML in chat | Extract → parse → `setRecipe()` |
| User edits field in Edit | Immediately `setRecipe()` |
| User sends chat message (if edited in Edit) | Prepend current recipe as YAML context |
| Click "Test" | `setTestRecipeSnapshot(recipe)` for baseline |
| Recipe changes while test open | Show "Recipe changed" warning banner |
| Click "Restart" in test | Update snapshot, restart session |
| Close test panel | `stopAgent()`, clear snapshot |

---

## Loop Prevention (Ref Tracking)

Multiple refs prevent feedback loops between Chat ↔ Edit sync:

| Ref | Location | Purpose |
|-----|----------|---------|
| `lastAIRecipeRef` | RecipeBuilderChat | Prevents re-prepending same AI recipe |
| `lastExtractedYamlRef` | RecipeBuilderChat | Prevents re-processing same YAML |
| `isInternalUpdateRef` | RecipeBuilderEdit | Prevents form reset feedback loops |
| `lastExternalRecipeRef` | RecipeBuilderEdit | Tracks external changes to avoid unnecessary resets |

---

## Integration Point

`RecipesView.tsx` — Added an "AI Recipe Builder" button (Sparkles icon) in the header that toggles the builder as a full-screen overlay. Passes `onClose` and `onRecipeSaved` callbacks.

---

## Implementation Status

All 6 planned phases are complete:
1. **Foundation** — Structure, types, header, integration
2. **Edit View** — Form-based recipe creation with full sync
3. **Chat View** — AI conversation with YAML extraction and parsing
4. **Chat ↔ Edit Sync** — Bidirectional synchronization with context prepending
5. **Test Panel** — Full testing with change detection and restart
6. **Polish** — Loading states, error handling, animations
