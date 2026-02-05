# Remaining CSS Issues Found

After comprehensive scan, found **10 categories of issues** that need to be fixed:

---

## Issue 1: `bg-background` (10 uses)

**Problem:** Incomplete class name, tries to reference `--background` which doesn't exist.

**Files affected:**
- `ui/Pill.tsx` (2 uses)
- `ui/sidebar.tsx` (3 uses)
- `MessageQueue.tsx` (4 uses)
- `ErrorBoundary.tsx` (1 use)

**Should be:** `bg-default` (for standard background)

---

## Issue 2: `border-border` (5 uses)

**Problem:** Tries to reference `--border` which doesn't exist.

**Files affected:**
- `ui/Pill.tsx` (2 uses)
- `MessageQueue.tsx` (3 uses)

**Should be:** `border-default`

---

## Issue 3: `bg-border-default` (10 uses)

**Problem:** Using a border color as a background color (completely wrong).

**Files affected:**
- `ui/separator.tsx` (1 use)
- `bottom_menu/CostTracker.tsx` (4 uses)
- `ChatInput.tsx` (1 use)

**Context:** These are vertical divider lines using `bg-border-default` for coloring

**Should be:** Create proper divider with `border-default` or use `bg-muted` for subtle lines

---

## Issue 4: `text-placeholder` (8 uses)

**Problem:** NOT DEFINED in CSS anywhere.

**Files affected:**
- `ToolCallArguments.tsx` (5 uses)
- Other components using placeholder text

**Should be:** Define `--placeholder` in CSS and add to @theme, OR use `text-muted`

---

## Issue 5: `text-error` (1 use)

**Problem:** Should use our semantic naming.

**Files affected:**
- `apps/StandaloneAppView.tsx` (inline style: `var(--text-error, #ef4444)`)

**Should be:** `text-danger` or `var(--text-danger)`

---

## Issue 6: `text-destructive` (5 uses)

**Problem:** Using Tailwind's default naming, should use our semantic names.

**Files affected:**
- `settings/dictation/LocalModelManager.tsx` (2 uses)
- `MessageQueue.tsx` (2 uses)
- `ErrorBoundary.tsx` (1 use)

**Should be:** `text-danger`

---

## Issue 7: `border-destructive` (2 uses)

**Problem:** Using Tailwind's default naming.

**Files affected:**
- `ui/button.tsx` (in cva definition)

**Should be:** `border-danger`

---

## Issue 8: `border-secondary` (1 use)

**Problem:** Undefined variable.

**Files affected:**
- `settings/providers/modal/subcomponents/forms/DefaultProviderSetupForm.tsx`

**Should be:** `border-default` or define `border-secondary`

---

## Issue 9: `border-dark` (1 use)

**Problem:** Undefined variable.

**Files affected:**
- `ui/scroll-area.tsx`

**Should be:** `border-default` or `border-strong`

---

## Issue 10: `border-muted` (1 use)

**Problem:** Undefined variable.

**Files affected:**
- `MentionPopover.tsx`

**Should be:** `border-default` (we have bg-muted and text-muted but not border-muted)

---

## Summary

| Issue | Count | Action |
|-------|-------|--------|
| `bg-background` | 10 | Replace with `bg-default` |
| `border-border` | 5 | Replace with `border-default` |
| `bg-border-default` | 10 | Fix dividers properly |
| `text-placeholder` | 8 | Define in CSS or use `text-muted` |
| `text-error` | 1 | Replace with `text-danger` |
| `text-destructive` | 5 | Replace with `text-danger` |
| `border-destructive` | 2 | Replace with `border-danger` |
| `border-secondary` | 1 | Replace with `border-default` |
| `border-dark` | 1 | Replace with `border-default` |
| `border-muted` | 1 | Replace with `border-default` |

**Total:** 44 more fixes needed

---

## Root Cause

The issue is that Tailwind CSS utilities like `bg-background` try to reference CSS custom properties by name. So:
- `bg-background` → looks for `var(--background)`
- `border-border` → looks for `var(--border)`
- `text-destructive` → Tailwind's built-in color

But we don't have these generic variables, we have semantic ones like:
- `--background-default`, `--background-muted`, etc.
- `--border-default`, `--border-strong`, etc.

---

## Recommended Fix Strategy

### Option 1: Add Generic Aliases (Quick)
Add to :root and @theme:
```css
--background: var(--background-default);
--border: var(--border-default);
--text: var(--text-default);
--placeholder: var(--text-muted); /* new */
```

### Option 2: Replace All Usage (Clean)
Replace all instances with specific semantic names:
- `bg-background` → `bg-default`
- `border-border` → `border-default`
- `text-destructive` → `text-danger`
- etc.

**Recommendation:** Option 2 (Replace) - More explicit and semantic

---

## Special Case: Dividers

The `bg-border-default` issue is specifically for vertical dividers:
```tsx
<div className="w-px h-4 bg-border-default mx-2" />
```

This should be either:
```tsx
<div className="w-px h-4 bg-muted mx-2" />
// OR
<div className="h-4 border-l border-default mx-2" />
```

---

## Priority

1. **High:** `bg-border-default` (wrong usage)
2. **High:** `text-placeholder` (undefined)
3. **Medium:** `bg-background`, `border-border` (incomplete)
4. **Medium:** `text-destructive`, `border-destructive` (wrong naming)
5. **Low:** Other border variants (1 use each)
