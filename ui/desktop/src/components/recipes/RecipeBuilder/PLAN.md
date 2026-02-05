# AI Recipe Builder - Requirements & Implementation Plan

## Overview

A conversational interface for creating Goose recipes with AI assistance, manual editing, and testing capabilities.

---

## Layout

```
┌─────────────────────────────────────────────────────────┐
│  [Chat] [Edit]                        [Test] [Save] [X] │
├────────────────────────────┬────────────────────────────┤
│                            │                            │
│   Main View                │   Test Panel               │
│   (Chat OR Edit - swap)    │   (optional, toggle)       │
│                            │                            │
└────────────────────────────┴────────────────────────────┘
```

### Header Buttons

| Button | Behavior |
|--------|----------|
| [Chat] | Switch main view to Chat (active style when selected) |
| [Edit] | Switch main view to Edit form (active style when selected) |
| [Test] | Toggle test panel on/off. Disabled until recipe exists |
| [Save] | Save recipe. Disabled until recipe has title + description |
| [X] | Close builder, return to RecipesView |

---

## State

```typescript
currentView: 'chat' | 'edit'           // Which main view is shown
testPanelOpen: boolean                  // Whether test panel is visible
recipe: Recipe | null                   // Single source of truth
testSessionId: string | null            // Active test session
testRecipeSnapshot: Recipe | null       // Recipe version test is running
```

---

## Sync Rules

| Event | Action |
|-------|--------|
| AI outputs YAML in chat | Extract recipe → update `recipe` state |
| User edits field in Edit view | Update `recipe` state immediately |
| User sends chat message (if recipe was edited) | Prepend current `recipe` as YAML context |
| User clicks "Start Test" | Copy `recipe` to `testRecipeSnapshot`, start session |
| `recipe` changes while test open | Show "Recipe changed. [Restart]" warning |
| User closes test panel | Terminate test session |

---

## User Flows

### 1. Entry
- Opens in Chat view, full width
- [Edit] enabled (can create manually)
- [Test] and [Save] disabled (no recipe yet)

### 2. Create via Chat
- User chats with AI
- AI outputs YAML recipe
- `recipe` state populated from extraction
- [Test] and [Save] become enabled

### 3. Create via Edit
- User clicks [Edit]
- Empty form shown
- User fills required fields (title, description)
- `recipe` state populated
- [Test] and [Save] become enabled

### 4. Switch Chat ↔ Edit
- Click [Chat] or [Edit] to swap main view
- Both views read/write same `recipe` state
- Edit view always shows latest recipe (AI or user edits)

### 5. Test
- Click [Test] → Panel opens on right side
- Shows "Start Test" button initially
- Click "Start Test" → Session starts with current recipe
- Recipe changes → Warning banner with [Restart] button
- Click [Test] again → Panel closes, session terminated

### 6. Save
- Click [Save] → Recipe saved to disk
- Shows success toast
- Calls `onRecipeSaved` callback

---

## File Structure

```
RecipeBuilder/
├── index.tsx                 # Main container, state management, layout
├── RecipeBuilderHeader.tsx   # Header with view toggle and action buttons
├── RecipeBuilderChat.tsx     # Chat view with AI conversation
├── RecipeBuilderEdit.tsx     # Edit view with form fields
├── RecipeBuilderTest.tsx     # Test panel component
├── recipeBuilderRecipe.ts    # Recipe that guides AI to build recipes
├── recipeExtractor.ts        # Extract recipe from AI YAML output
├── types.ts                  # TypeScript types
└── PLAN.md                   # This file
```

---

## Implementation Phases

### Phase 1: Foundation ✅ COMPLETE
- [x] Create folder structure + types.ts
- [x] Create index.tsx - empty shell with state
- [x] Create RecipeBuilderHeader.tsx with buttons
- [x] Add entry point in RecipesView

### Phase 2: Edit View ✅ COMPLETE
- [x] Wire up Edit view using RecipeFormFields
- [x] Connect form to `recipe` state
- [x] Handle empty state (no recipe yet)
- [x] Enable Save when recipe is valid

### Phase 3: Chat View ✅ COMPLETE
- [x] Create recipeBuilderRecipe.ts (AI guidance)
- [x] Create recipeExtractor.ts (YAML parsing)
- [x] Create RecipeBuilderChat.tsx
- [x] Initialize AI session on mount
- [x] Extract recipe from AI output → update state

### Phase 4: Chat ↔ Edit Sync ✅ COMPLETE
- [x] Switch Chat → Edit shows current recipe in form
- [x] Switch Edit → Chat syncs edits on next message
- [x] Prepend recipe YAML when user sends message after editing

### Phase 5: Test Panel ✅ COMPLETE
- [x] Create RecipeBuilderTest.tsx
- [x] Start test session with current recipe
- [x] Detect recipe changes, show warning
- [x] Restart test with updated recipe
- [x] Close panel terminates session

### Phase 6: Polish ✅ COMPLETE
- [x] Loading states (LoadingGoose in chat and test)
- [x] Error handling (error displays in chat and test)
- [x] Animations/transitions (CSS transitions for panel resize)
- [x] Edge cases (empty recipe, panel toggle cleanup)

---

## Decisions Made

| Question | Decision |
|----------|----------|
| Edit always shows latest (AI or user)? | Yes - single source of truth |
| Warn on unsaved changes when closing? | No (not implementing now) |
| Test session on panel close? | Terminate |
| Edit view with no recipe? | Show empty form, user can create manually |
| Reuse existing edit UI? | Yes - use RecipeFormFields from shared/ |
| Track builder/test sessions? | Yes - add to activeSessions, appear in Sessions view |
