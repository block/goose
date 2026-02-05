# CSS Variables Cleanup Plan (Pre-MCP Migration)

## Executive Summary

Before migrating to MCP standard variables, we need to clean up the existing goose design system:
- **Fix 9 undefined variables** (318 total uses)
- **Remove 6 unused variables** (dead code)
- **Consolidate 3 redundant variables**
- **Define 1 missing variable**

This cleanup will ensure a clean foundation for the MCP migration.

---

## Phase 1: Fix Undefined Variables (CRITICAL - 318 uses)

These classes are used throughout the codebase but NOT defined in main.css:

### High Priority (200+ uses combined)

| Undefined Variable | Uses | Correct Variable | Reason |
|-------------------|------|------------------|--------|
| `text-text-standard` | 52 | `text-default` | Standard body text, redundant prefix |
| `text-text-prominent` | 11 | `text-default` | Headings/titles, emphasis via font-weight not color |
| `bg-background-subtle` | 11 | `bg-background-muted` | Subtle backgrounds, maps to existing muted |
| `border-border-subtle` | 25 | `border-default` | Standard borders, no "subtle" variant exists |

### Medium Priority (error states - 15 uses)

| Undefined Variable | Uses | Correct Variable | Reason |
|-------------------|------|------------------|--------|
| `bg-background-error` | 5 | `bg-background-danger` | Design system uses "danger" not "error" |
| `text-text-error` | ~3 | `text-danger` | Companion to bg-background-error |
| `border-border-error` | ~2 | `border-danger` | Companion to bg-background-error |

### Low Priority (9 uses)

| Undefined Variable | Uses | Correct Variable | Reason |
|-------------------|------|------------------|--------|
| `text-text-on-accent` | 9 | `text-on-accent` | Variable exists, redundant prefix |
| `bg-background-hover` | 4 | `bg-background-muted` OR refactor to `hover:` modifier | Used for hover states |

### Special Case (defined in CSS but not in :root)

| Undefined Variable | Uses | Correct Variable | Action |
|-------------------|------|------------------|--------|
| `var(--text-prominent-inverse)` | 2 | `var(--text-inverse)` | Toast button text, should use existing inverse |

**Total Impact:** 318 instances across 103 files

---

## Phase 2: Remove Unused Variables (Dead Code)

These variables are defined in main.css but NEVER used:

### Can Be Removed Immediately

| Variable | Defined | Uses | Action |
|----------|---------|------|--------|
| `--background-strong` | ✅ Line 69 | 0 | **DELETE** - No usages found |
| `--border-input` | ✅ Line 77 | 0 | **DELETE** - border-default used instead |
| `--background-success` | ✅ Line 72 | 0 | **DELETE** - Never used in components |
| `--background-warning` | ✅ Line 74 | 0 | **DELETE** - Never used in components |
| `--border-success` | ✅ Line 80 | 0 | **DELETE** - Never used |
| `--border-warning` | ✅ Line 81 | 0 | **DELETE** - Never used |

**Total to Remove:** 6 variables (12 definitions with light/dark modes)

---

## Phase 3: Consolidate Redundant Variables

### Background Variables Analysis

| Variable | Uses | Value (Light) | Value (Dark) | Status |
|----------|------|---------------|--------------|--------|
| `--background-default` | 110 | `#ffffff` | `#22252a` | ✅ Keep - Primary |
| `--background-app` | 3 | `#ffffff` | `#22252a` | ⚠️ Same as default |
| `--background-card` | 3 | `#ffffff` | `#22252a` | ⚠️ Same as default |

**Recommendation:** Keep all three for semantic clarity, but recognize they're aliases:
- `background-app` - Body/root background
- `background-default` - Standard container background
- `background-card` - Card component background

These may diverge during MCP migration (card might need subtle elevation), so keep them separate.

---

## Phase 4: Variables to Keep (Currently Used)

These are defined and actively used - keep as-is:

### Heavily Used (50+ uses)
- ✅ `--text-muted` (178 uses)
- ✅ `--background-default` (110 uses)
- ✅ `--text-default` (96 uses)
- ✅ `--background-muted` (59 uses)

### Moderately Used (5-50 uses)
- ✅ `--border-default` (24 uses)
- ✅ `--text-inverse` (10 uses)
- ✅ `--background-medium` (9 uses) - Used for hover states in sidebar
- ✅ `--background-accent` (8 uses) - Brand color
- ✅ `--border-strong` (7 uses) - Focus/hover states on inputs
- ✅ `--background-inverse` (5 uses)
- ✅ `--text-danger` (5 uses)
- ✅ `--border-danger` (5 uses)

### Low Use But Necessary (1-4 uses)
- ✅ `--border-accent` (3 uses)
- ✅ `--background-card` (3 uses)
- ✅ `--background-danger` (2 uses)
- ✅ `--text-accent` (1 use)
- ✅ `--text-warning` (1 use) - Used in ToolCallWithResponse
- ✅ `--text-info` (1 use) - Used in ToolCallWithResponse
- ✅ `--background-info` (1 use)

### Typography
- ✅ `--font-sans` (Cash Sans)
- ✅ `--font-mono` (monospace)

### UI-Specific (Keep)
- ✅ All `--sidebar-*` variables (8 variants)
- ✅ `--ring` (focus ring)
- ✅ `--shadow-default` (drop shadow)

---

## Phase 5: Additional Issues Found

### Issue 1: Naming Inconsistency

**Current Pattern:**
- Tailwind classes use redundant prefixes: `text-text-*`, `bg-background-*`, `border-border-*`
- CSS variables use clean names: `--text-*`, `--background-*`, `--border-*`

**Inconsistency Example:**
- CSS: `--text-default`
- Tailwind: `text-text-default` ❌ (redundant)
- Should be: `text-default` ✅

This is caused by Tailwind v4's `@theme` configuration. Check if this is intentional or a configuration issue.

### Issue 2: Missing Variable Definition

- `--text-prominent-inverse` - Used but not defined
- Should be added OR replaced with `--text-inverse`

---

## Implementation Strategy

### Step 1: Add Missing Definitions (Quick Fix)

Add to `:root` section of main.css:

```css
:root {
  /* ... existing ... */

  /* Define previously implicit variables */
  --text-prominent-inverse: var(--text-inverse); /* For toast buttons */
}
```

### Step 2: Global Find & Replace (Automated)

Run these replacements across `ui/desktop/src/**/*.{tsx,ts}`:

```bash
# High priority (200+ combined uses)
text-text-standard       → text-default
text-text-prominent      → text-default
bg-background-subtle     → bg-background-muted
border-border-subtle     → border-default

# Error states
bg-background-error      → bg-background-danger
text-text-error          → text-danger
border-border-error      → border-danger

# Redundant prefix
text-text-on-accent      → text-on-accent

# Hover states (need manual review)
bg-background-hover      → bg-background-muted
```

### Step 3: Manual Review Cases

**bg-background-hover special cases:**
- `ApiKeyTester.tsx:76` - Used as border color → Should be `border-default`
- `ProviderGuard.tsx:329` - Used as border color → Should be `border-default`
- Other uses - Refactor to use `hover:bg-background-medium`

### Step 4: Remove Unused Variables

Delete from main.css (both :root and .dark sections):

```css
/* DELETE THESE */
--background-strong: var(--color-neutral-300);
--border-input: var(--color-neutral-100);
--background-success: var(--color-green-200);
--background-warning: var(--color-yellow-200);
--border-success: var(--color-green-200);
--border-warning: var(--color-yellow-200);
```

### Step 5: Fix text-prominent-inverse

In main.css, replace:

```css
/* OLD */
.Toastify__close-button {
  color: var(--text-prominent-inverse) !important;
}

/* NEW */
.Toastify__close-button {
  color: var(--text-inverse) !important;
}
```

### Step 6: Verify

Run these checks:

```bash
# Check for remaining undefined variables
grep -r "text-text-\|bg-background-subtle\|border-border-" ui/desktop/src --include="*.tsx"

# Verify no regressions
npm run build
npm run test
```

---

## Risk Assessment

### Low Risk
- ✅ Removing unused variables (no impact)
- ✅ Fixing undefined variables (currently broken, can only improve)
- ✅ Consolidating redundant names (semantic aliases)

### Medium Risk
- ⚠️ `bg-background-hover` refactor - Need to test hover states
- ⚠️ Global replacements - Need comprehensive testing

### High Risk
- ❌ None - These are all bug fixes and cleanup

---

## Testing Checklist

After implementing changes:

- [ ] **Sidebar** - Hover states work correctly (uses background-medium)
- [ ] **Forms** - Input focus states visible (uses border-strong)
- [ ] **Cards** - Background colors consistent
- [ ] **Error states** - Red backgrounds/borders/text display correctly
- [ ] **Toast notifications** - Close button visible and styled
- [ ] **Apps view** - Tags and badges have subtle backgrounds
- [ ] **Schedule views** - Error messages display with correct styling
- [ ] **Recipes** - Modal borders and form inputs look correct
- [ ] **Provider cards** - Hover states functional
- [ ] **Dark mode** - All changes work in dark theme

---

## Files Requiring Most Changes

Based on undefined variable usage:

1. `ui/desktop/src/components/ui/RecipeWarningModal.tsx` (text-text-standard)
2. `ui/desktop/src/components/ProviderGuard.tsx` (multiple undefined)
3. `ui/desktop/src/components/schedule/ScheduleDetailView.tsx` (error states)
4. `ui/desktop/src/components/schedule/ScheduleModal.tsx` (error states)
5. `ui/desktop/src/components/apps/AppsView.tsx` (bg-background-subtle)
6. `ui/desktop/src/components/UserMessage.tsx` (text-text-prominent, border-border-subtle)
7. `ui/desktop/src/components/settings/app/TelemetrySettings.tsx` (text-text-standard)
8. `ui/desktop/src/styles/main.css` (text-prominent-inverse)

---

## Post-Cleanup Status

**After Phase 1-5 Complete:**

### Variables Summary
- ✅ **Defined & Used:** 25 variables (clean)
- ✅ **Undefined Issues:** 0 (fixed 9)
- ✅ **Unused Variables:** 0 (removed 6)
- ✅ **Design System:** Consistent and maintainable

### Ready for MCP Migration
With a clean foundation, the MCP migration can proceed with:
- Clear 1:1 mappings from goose → MCP
- No undefined variables to confuse the process
- No dead code to maintain
- Consistent naming patterns

---

## Estimated Effort

- **Phase 1-2:** 2-3 hours (automated find/replace + validation)
- **Phase 3:** 1 hour (manual review of hover states)
- **Phase 4:** 30 minutes (remove unused variables)
- **Phase 5:** 1 hour (testing and verification)

**Total:** ~5 hours for complete cleanup
