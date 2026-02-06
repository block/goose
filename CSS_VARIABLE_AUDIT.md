# CSS Variable Usage Audit - Post-Cleanup

## Summary

After the initial cleanup, there are **still undefined variables** being used in the codebase. The analysis reveals a complex picture with multiple naming conventions.

---

## Tailwind v4 Naming Pattern

Tailwind v4 generates utility classes from CSS custom properties defined in the `@theme inline` section:

### Pattern:
```
CSS: --color-text-default
↓
Tailwind class: text-text-default
```

The class name = utility type (`text`) + variable name after `--color-` prefix (`text-default`)

---

## Currently Defined Variables (from @theme inline)

### Text Colors (defined = 7)
✅ `--color-text-default` → `text-text-default` (112 uses)
✅ `--color-text-muted` → `text-text-muted` (200 uses)
✅ `--color-text-inverse` → `text-text-inverse` (14 uses)
✅ `--color-text-accent` → `text-text-accent` (1 use)
✅ `--color-text-on-accent` → `text-on-accent` (16 uses)
✅ `--color-text-danger` → `text-text-danger` (3 uses)
✅ `--color-text-success` → `text-text-success` (0 uses)
✅ `--color-text-warning` → `text-text-warning` (1 use)
✅ `--color-text-info` → `text-text-info` (1 use)

### Background Colors (defined = 7)
✅ `--color-background-default` → `bg-background-default` (115 uses)
✅ `--color-background-muted` → `bg-background-muted` (82 uses)
✅ `--color-background-medium` → `bg-background-medium` (15 uses)
✅ `--color-background-inverse` → `bg-background-inverse` (5 uses)
✅ `--color-background-accent` → `bg-background-accent` (12 uses)
✅ `--color-background-danger` → `bg-background-danger` (10 uses)
✅ `--color-background-info` → `bg-background-info` (1 use)
✅ `--color-background-card` → `bg-background-card` (3 uses)

### Border Colors (defined = 4)
✅ `--color-border-default` → `border-border-default` (25 uses)
✅ `--color-border-strong` → `border-border-strong` (6 uses)
✅ `--color-border-accent` → `border-border-accent` (3 uses)
✅ `--color-border-danger` → `border-border-danger` (0 uses)
✅ `--color-border-info` → `border-border-info` (1 use)

---

## UNDEFINED Variables Still Being Used

### Text (5 undefined)
❌ `text-text-subtle` (12 uses) - no `--color-text-subtle` defined
❌ `textProminentInverse` (2 uses) - non-standard camelCase
❌ `textPlaceholder` (9 uses) - non-standard camelCase

### Background (6 undefined)
❌ `bg-background-defaultInverse` (2 uses) - camelCase, not defined
❌ `bg-background-primary` (1 use) - not defined
❌ `bg-background-panel` (1 use) - not defined
❌ `bg-background-light` (1 use) - not defined
❌ `bg-background-dark` (1 use) - not defined
❌ `bgStandardInverse` (2 uses) - non-standard camelCase

### Border (2 undefined)
❌ `border-border-muted` (5 uses) - no `--color-border-muted` defined
❌ `border-border-focus` (2 uses) - no `--color-border-focus` defined

**Total: 42 uses of undefined variables**

---

## Alternative Class Usage (Non-Tailwind v4 Pattern)

Some code uses shortened class names that may work via Tailwind's default utilities:

- `text-default` (176 uses) - instead of `text-text-default`
- `border-default` (62 uses) - instead of `border-border-default`
- `text-muted` (27 uses) - instead of `text-text-muted`
- `text-inverse` (2 uses) - instead of `text-text-inverse`
- `text-danger` (5 uses) - instead of `text-text-danger`
- `text-info` (3 uses) - instead of `text-text-info`

These may be:
1. Tailwind generating classes from the `:root` variables (e.g., `--text-default`)
2. Custom utility classes
3. Inconsistent usage that should be standardized

---

## Issues to Fix

### Priority 1: Undefined Variables (42 uses)

These MUST be fixed:

| Variable | Uses | Suggested Fix |
|----------|------|---------------|
| `text-text-subtle` | 12 | Add `--color-text-subtle` OR replace with `text-text-muted` |
| `border-border-muted` | 5 | Add `--color-border-muted` OR replace with `border-border-default` |
| `border-border-focus` | 2 | Add `--color-border-focus` OR replace with `border-border-strong` |
| `textProminentInverse` | 2 | Replace with `text-text-inverse` |
| `textPlaceholder` | 9 | Add `--color-text-placeholder` or use existing |
| `bgStandardInverse` | 2 | Replace with `bg-background-inverse` |
| `bg-background-defaultInverse` | 2 | Replace with `bg-background-inverse` |
| `bg-background-primary` | 1 | Replace with `bg-background-default` |
| `bg-background-panel` | 1 | Replace with `bg-background-card` |
| `bg-background-light` | 1 | Replace with `bg-background-muted` (light mode specific) |
| `bg-background-dark` | 1 | Replace with `bg-background-medium` (dark mode specific) |

### Priority 2: Naming Inconsistency (268 uses)

Standardize on Tailwind v4 pattern:

| Current | Uses | Should Be |
|---------|------|-----------|
| `text-default` | 176 | `text-text-default` or define properly |
| `border-default` | 62 | `border-border-default` or define properly |
| `text-muted` | 27 | `text-text-muted` or define properly |
| `text-inverse` | 2 | `text-text-inverse` or define properly |
| `text-danger` | 5 | `text-text-danger` or define properly |

---

## Recommendations

### Option 1: Complete Tailwind v4 Compliance
- Add missing `--color-*` variables to `@theme inline`
- Replace all shortened names with full Tailwind v4 pattern
- Standardize on `text-text-*`, `bg-background-*`, `border-border-*`

### Option 2: Simplify to Direct Variables
- Keep `:root` variables like `--text-default`, `--background-muted`
- Remove `@theme inline` section or simplify it
- Use shorter class names consistently: `text-default`, `bg-muted`, `border-default`
- Less redundant, more readable

### Option 3: Hybrid Approach
- Keep Tailwind v4 for theme-able variables (backgrounds, text, borders)
- Add missing variables for gaps
- Accept some inconsistency where it exists

---

## Current Usage Statistics

### Total CSS Class Usage
- **Background classes:** 248 uses
  - Defined: 233 uses (94%)
  - Undefined: 8 uses (3%)
  - Alternative syntax: 7 uses (3%)

- **Text classes:** 461 uses
  - Defined (Tailwind v4 pattern): 238 uses (52%)
  - Alternative syntax: 213 uses (46%)
  - Undefined: 10 uses (2%)

- **Border classes:** 93 uses
  - Defined: 38 uses (41%)
  - Alternative syntax: 48 uses (52%)
  - Undefined: 7 uses (7%)

---

## Next Steps Decision Needed

Before proceeding with MCP migration, we need to decide:

1. **Which naming convention to use?**
   - Full Tailwind v4 (`text-text-default`) - more verbose but explicit
   - Shortened (`text-default`) - cleaner but less clear what's custom vs. Tailwind default

2. **Should we add the missing variables or replace with existing ones?**
   - Add: `--color-text-subtle`, `--color-border-muted`, etc.
   - Replace: Use existing similar variables

3. **How to handle the camelCase variables?**
   - `textProminentInverse`, `textPlaceholder`, `bgStandardInverse`
   - These seem to be older conventions that should be migrated

**Recommendation:**
- Fix all undefined variables (42 uses)
- Standardize on ONE naming pattern before MCP migration
- Add missing semantic variables that make sense (`text-subtle`, `border-muted`)
