# shadcn/ui Pattern Normalization Complete

## Summary

Successfully normalized all shadcn/ui CSS patterns to goose's semantic naming system. The codebase now has ONE consistent naming convention, ready for MCP standard migration.

---

## What Was Fixed

### Issue 1: shadcn Base Patterns → Goose Semantic

| shadcn Pattern | Uses | Goose Pattern | Status |
|----------------|------|---------------|--------|
| `bg-background` | 10 | `bg-default` | ✅ Fixed |
| `border-border` | 8 | `border-default` | ✅ Fixed |
| `text-foreground` | 8 | `text-default` | ✅ Fixed |
| `text-muted-foreground` | 18 | `text-muted` | ✅ Fixed |

### Issue 2: Wrong Usage

| Wrong Pattern | Uses | Correct Pattern | Status |
|---------------|------|-----------------|--------|
| `bg-border-default` | 10 | `bg-muted` | ✅ Fixed (dividers) |

### Issue 3: Undefined Variables

| Undefined | Uses | Replaced With | Status |
|-----------|------|---------------|--------|
| `text-placeholder` | 8 | `text-muted` + added `--placeholder` to CSS | ✅ Fixed |
| `border-secondary` | 1 | `border-default` | ✅ Fixed |
| `border-dark` | 1 | `border-default` | ✅ Fixed |
| `border-muted` | 1 | `border-default` | ✅ Fixed |
| `text-error` | 1 | `text-danger` | ✅ Fixed |

### Issue 4: Tailwind Default Naming

| Tailwind Default | Uses | Goose Semantic | Status |
|------------------|------|----------------|--------|
| `text-destructive` | 6 | `text-danger` | ✅ Fixed |
| `border-destructive` | 1 | `border-danger` | ✅ Fixed |

---

## Total Changes

- **Files Modified:** 132 files
- **Lines Changed:** 906 insertions, 912 deletions (net -6 lines)
- **Total Fixes:** ~71 individual replacements

---

## What is shadcn/ui?

**NOT a package you install** - it's a **copy-paste component collection**:
- Built on Radix UI (accessible primitives)
- Styled with Tailwind CSS
- Uses a specific CSS variable convention

### shadcn/ui CSS Convention

Expects these **base variables**:
```css
--background  /* simple, generic */
--foreground
--border
--muted
--accent
```

### Why We Normalized Instead of Adding Compatibility

**❌ Compatibility Layer Would Have Created:**
```tsx
// 3 ways to do the same thing!
<div className="bg-background" />  // shadcn way
<div className="bg-default" />     // goose way
<div className="bg-primary" />     // MCP way (future)
```

**✅ One Consistent System:**
```tsx
// Now: ONE way
<div className="bg-default" />

// Future MCP migration: CLEAN path
<div className="bg-primary" />
```

---

## CSS Variables Added

Added `--placeholder` to support Tailwind's `placeholder:` modifier:

```css
/* :root */
--placeholder: var(--color-neutral-400);

/* .dark */
--placeholder: var(--color-neutral-400);
```

Used in:
- `ui/input.tsx` - `placeholder:text-placeholder`
- `ChatInput.tsx` - `placeholder:text-placeholder`

---

## Components Affected

### UI Components (shadcn/ui based)
- `ui/Pill.tsx` - `bg-background`, `border-border` → `bg-default`, `border-default`
- `ui/sidebar.tsx` - `bg-background` → `bg-default`
- `ui/input.tsx` - `file:text-foreground` → `file:text-default`
- `ui/scroll-area.tsx` - `bg-border`, `bg-border-dark` → `bg-muted`
- `ui/separator.tsx` - `bg-border-default` → `bg-muted`
- `ui/button.tsx` - `border-destructive` → `border-danger`
- `ui/tabs.tsx` - `text-muted-foreground` → `text-muted`

### Application Components
- `MessageQueue.tsx` - Multiple shadcn patterns normalized
- `ErrorBoundary.tsx` - `bg-background`, `text-destructive` → goose semantic
- `ToolCallArguments.tsx` - `text-placeholder` → `text-muted`
- `MentionPopover.tsx` - `border-muted` → `border-default`
- `CostTracker.tsx` - `bg-border-default` dividers → `bg-muted`
- `ChatInput.tsx` - `bg-border-default` dividers → `bg-muted`
- `StandaloneAppView.tsx` - `var(--text-error)` → `var(--text-danger)`

### Settings Components
- `DefaultProviderSetupForm.tsx` - `border-secondary` → `border-default`
- `LocalModelManager.tsx` - `text-destructive` → `text-danger`

### Session Components
- `SessionItem.tsx` - `text-muted-foreground` → `text-muted`

### Recipe Components
- `RecipesView.tsx` - `text-muted-foreground` → `text-muted`

---

## Current Goose Semantic Usage

After normalization:

| Class | Uses | Purpose |
|-------|------|---------|
| `bg-default` | 126 | Primary background |
| `bg-muted` | 129 | Subtle/secondary background |
| `bg-medium` | 16 | Medium emphasis background |
| `bg-accent` | 15 | Brand accent background |
| `text-default` | 303 | Primary text color |
| `text-muted` | 338 | De-emphasized text |
| `text-inverse` | 19 | Inverted text (dark on light, light on dark) |
| `text-on-accent` | 17 | Text on accent backgrounds |
| `text-danger` | 11 | Error/danger text |
| `border-default` | 161 | Standard borders |
| `border-strong` | 14 | Emphasized borders |
| `border-accent` | 3 | Brand accent borders |
| `border-danger` | 5 | Error borders |

---

## Benefits

### 1. One Consistent System ✅
- No confusion about which pattern to use
- All developers use the same naming
- Easier onboarding

### 2. Clean MCP Migration Path ✅
- Single source of truth
- Simple find/replace: `bg-default` → `bg-primary`
- No overlapping systems to reconcile

### 3. Better Semantics ✅
- `bg-default` is clearer than `bg-background`
- `text-danger` is more semantic than `text-destructive`
- Consistent with design system thinking

### 4. Maintainability ✅
- Fewer patterns to document
- Easier to refactor
- Clear ownership of variables

---

## Lessons Learned

### 1. Copy-Paste Components Need Customization
shadcn/ui components are **meant to be customized** - not used as-is. We should have adapted them to goose's conventions from the start.

### 2. Design System Consistency Matters
Having two naming systems (shadcn + goose) created confusion and bugs. Better to normalize early.

### 3. Compatibility Layers Are Tech Debt
Adding `--background` would have been a quick fix but created long-term maintenance burden.

### 4. Semantic Naming > Generic Naming
`bg-default` is more meaningful than `bg-background`. `text-danger` is clearer than `text-destructive`.

---

## Next Steps: MCP Migration

With normalization complete, MCP migration will be straightforward:

### Current → MCP Mapping

```bash
# Backgrounds
bg-default → bg-primary
bg-muted → bg-secondary
bg-medium → bg-tertiary

# Text
text-default → text-primary
text-muted → text-secondary

# Borders
border-default → border-primary
border-strong → border-secondary
```

### MCP Variables to Add

1. **Typography System** (missing)
   - Font sizes: xs, sm, md, lg (text + heading)
   - Line heights for each size
   - Font weights: normal, medium, semibold, bold

2. **Border System** (missing)
   - Border radius: xs, sm, md, lg, xl, full
   - Border width: regular

3. **Shadow System** (expand)
   - Current: 1 shadow (`--shadow-default`)
   - MCP needs: hairline, sm, md, lg

4. **Ring Colors** (expand)
   - Current: 1 ring variant
   - MCP needs: 7 semantic variants for focus states

5. **State Variants** (new)
   - Ghost states (transparent/minimal)
   - Disabled states

---

## Verification

### ✅ Zero Issues Remaining

```bash
bg-background:       0 ✅
border-border:       0 ✅
bg-border-default:   0 ✅
text-foreground:     0 ✅
text-destructive:    0 ✅
border-destructive:  0 ✅
text-error:          0 ✅
border-secondary:    0 ✅
border-dark:         0 ✅
border-muted:        0 ✅
```

### ✅ One Consistent System

All components now use goose semantic naming:
- `bg-default`, `bg-muted`, `bg-medium`
- `text-default`, `text-muted`, `text-inverse`
- `border-default`, `border-strong`, `border-accent`

---

## Conclusion

✅ **shadcn/ui normalization complete**

The goose codebase now has:
- ✅ One consistent CSS naming convention
- ✅ Zero shadcn/ui legacy patterns
- ✅ Clean base for MCP migration
- ✅ Better semantic clarity
- ✅ Easier maintenance

**Ready for MCP standard migration!**
