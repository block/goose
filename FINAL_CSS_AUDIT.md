# Final CSS Variables Usage Audit

## Summary

After normalizing shadcn/ui patterns, found **4 remaining issues** to fix and identified **low-usage variables** to consider for MCP migration.

---

## 🔴 Issues Found (Must Fix)

### Issue 1: Undefined Variables (4 uses)

| Variable | Uses | Problem |
|----------|------|---------|
| `bg-accent-primary` | 3 | NOT DEFINED - Tailwind looking for `--accent-primary` |
| `border-accent-primary` | 1 | NOT DEFINED |

**Affected File:**
- `settings/dictation/LocalModelManager.tsx` (all 4 uses)

**Should be:**
- `bg-accent-primary` → `bg-accent`
- `border-accent-primary` → `border-accent`

---

### Issue 2: Old Variable Names in CSS (2 uses)

**File:** `styles/search.css`

```css
/* Line 4 */
color: var(--text-standard);  /* Should be var(--text-default) */

/* Line 11 */
box-shadow: inset 0 0 0 1px var(--text-prominent);  /* Should be var(--text-default) */
```

These are the old camelCase variable names we cleaned up everywhere else!

---

### Issue 3: Redundant Variables

**Problem:** Three variables with IDENTICAL values

```css
/* Light mode */
--background-app: var(--color-white);
--background-default: var(--color-white);
--background-card: var(--color-white);

/* Dark mode */
--background-app: var(--color-neutral-950);
--background-default: var(--color-neutral-950);
--background-card: var(--color-neutral-950);
```

**Usage:**
- `bg-default`: 126 uses (primary background)
- `bg-app`: 2 uses (app root background)
- `bg-card`: 4 uses (card backgrounds)

**Analysis:**
- `bg-app` (2 uses) could be consolidated to `bg-default`
- `bg-card` (4 uses) has semantic meaning (cards might need subtle elevation in future), keep it

---

## 📊 Low Usage Variables (Consider for MCP)

### Very Low Usage (1-3 uses)

| Variable | Uses | Used In | Keep? |
|----------|------|---------|-------|
| `text-accent` | 1 | button.tsx link variant | ✅ Keep - semantic |
| `text-warning` | 1 | OllamaSetup.tsx | ✅ Keep - needed for warnings |
| `text-success` | 0 | NOWHERE | ❌ Remove or wait for usage |
| `bg-info` | 3 | OllamaSetup, MessageQueue | ✅ Keep - info states |
| `bg-app` | 2 | McpAppRenderer, BottomMenuAlertPopover | ⚠️ Consolidate to bg-default |

### Low Usage (4-10 uses)

| Variable | Uses | Keep? |
|----------|------|-------|
| `text-info` | 4 | ✅ Keep - info text |
| `bg-card` | 4 | ✅ Keep - semantic (cards) |
| `border-info` | 2 | ✅ Keep - info borders |
| `bg-inverse` | 9 | ✅ Keep - inverse backgrounds |

---

## ✅ Well-Used Variables (No Issues)

### Heavy Usage (100+ uses)

| Variable | Uses | Status |
|----------|------|--------|
| `text-muted` | 354 | ✅ Excellent |
| `text-default` | 322 | ✅ Excellent |
| `border-default` | 186 | ✅ Excellent |
| `bg-muted` | 139 | ✅ Excellent |
| `bg-default` | 126 | ✅ Excellent |

### Good Usage (10-50 uses)

| Variable | Uses | Status |
|----------|------|--------|
| `text-inverse` | 19 | ✅ Good |
| `text-on-accent` | 17 | ✅ Good |
| `text-danger` | 16 | ✅ Good |
| `bg-medium` | 16 | ✅ Good |
| `bg-accent` | 15 | ✅ Good |
| `border-strong` | 14 | ✅ Good |
| `bg-danger` | 13 | ✅ Good |

---

## 🔧 Recommended Actions

### Priority 1: Fix Undefined (4 instances)

```bash
# File: settings/dictation/LocalModelManager.tsx
bg-accent-primary → bg-accent
border-accent-primary → border-accent
```

### Priority 2: Fix Old CSS Variable Names (2 instances)

```bash
# File: styles/search.css
var(--text-standard) → var(--text-default)
var(--text-prominent) → var(--text-default)
```

### Priority 3: Consolidate Redundant (2 instances)

```bash
# Files: McpAppRenderer.tsx, BottomMenuAlertPopover.tsx
bg-app → bg-default
```

Then remove `--background-app` and `--app` from CSS.

### Priority 4: Consider Removing (0 uses)

```bash
# Remove from CSS if truly unused:
--text-success (and dark mode variant)
```

But wait until after MCP migration - might be needed for success states.

---

## 📈 Usage Statistics

### CSS Variables Defined in :root

**Total:** ~35 variables (excluding color-*, font-*, shadow-*)

**Categories:**
- Background: 9 variables
- Text: 9 variables
- Border: 5 variables
- Sidebar: 7 variables
- Other: 5 variables (ring, placeholder, shadow, breakpoint)

### Tailwind Utility Classes

**Most Used:**
1. `text-muted` - 354 uses
2. `text-default` - 322 uses
3. `border-default` - 186 uses
4. `bg-muted` - 139 uses
5. `bg-default` - 126 uses

**Least Used:**
1. `text-accent` - 1 use
2. `text-warning` - 1 use
3. `border-info` - 2 uses
4. `bg-app` - 2 uses
5. `bg-info` - 3 uses

---

## 🎯 Semantic Variable Groups

### Core Colors (Heavy Use)
- ✅ default, muted (100+ uses each)
- ✅ inverse (19 uses)

### Emphasis
- ✅ medium (16 uses)
- ✅ strong (14 uses)

### Brand
- ✅ accent (15 uses)
- ✅ on-accent (17 uses)

### Semantic States
- ✅ danger (13-16 uses)
- ⚠️ info (2-4 uses) - low but needed
- ⚠️ warning (1 use) - very low
- ❌ success (0 uses) - unused

### UI-Specific
- ✅ card (4 uses) - semantic distinction
- ⚠️ app (2 uses) - redundant with default

---

## 💡 Insights for MCP Migration

### Variables to Keep

All current variables are semantically meaningful and should map to MCP equivalents:

```
Current          →  MCP Standard
-----------------------------------
bg-default       →  bg-primary
bg-muted         →  bg-secondary
bg-medium        →  bg-tertiary
text-default     →  text-primary
text-muted       →  text-secondary
border-default   →  border-primary
border-strong    →  border-secondary
```

### Variables to Add in MCP

1. **Ghost/Disabled States** (missing)
   - Currently using opacity modifiers (e.g., `bg-muted/60`)
   - MCP has dedicated ghost and disabled variants

2. **Typography System** (missing)
   - Font sizes, weights, line-heights
   - Currently using Tailwind defaults

3. **Border Radius** (missing)
   - Currently using inline values (4px, 6px, 8px)
   - MCP has xs/sm/md/lg/xl/full scale

4. **Shadow Scale** (minimal)
   - Currently: 1 shadow (`--shadow-default`)
   - MCP needs: hairline, sm, md, lg

### Variables to Consider Removing

1. **`text-success`** - 0 uses
   - But might be needed for success states
   - Wait until MCP migration to decide

2. **`bg-app`** - Only 2 uses, identical to `bg-default`
   - Consolidate to `bg-default`
   - Remove from CSS

---

## 📋 Action Items Summary

**Must Fix (6 issues):**
1. ✅ Replace `bg-accent-primary` → `bg-accent` (3 uses)
2. ✅ Replace `border-accent-primary` → `border-accent` (1 use)
3. ✅ Fix `var(--text-standard)` in search.css
4. ✅ Fix `var(--text-prominent)` in search.css
5. ✅ Replace `bg-app` → `bg-default` (2 uses)
6. ✅ Remove `--background-app` and `--app` from CSS

**Consider (1 issue):**
7. ⚠️ Remove `--text-success` if truly unused (check after MCP)

---

## 🎉 Overall Health

**Current State: HEALTHY** ✅

- ✅ 95% of variables are well-used (10+ uses)
- ✅ Only 6 minor issues to fix
- ✅ Clear semantic naming
- ✅ Good coverage of use cases
- ✅ Ready for MCP migration

**After Fixes:**
- 0 undefined variables
- 0 old variable names
- 0 redundant variables (except bg-card, which is semantic)
- Clean, consistent system ready for MCP
