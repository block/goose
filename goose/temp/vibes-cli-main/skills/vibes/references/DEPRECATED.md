# Deprecated API Patterns

This document contains deprecated API patterns that should NOT be used in new code. These are kept for reference when migrating older apps.

## Old use-vibes / use-fireproof API (Pre-fireproof-clerk)

The `@necrodome/fireproof-clerk` package has a DIFFERENT API than the old `use-vibes` package.

**NEVER generate code with these patterns:**

```jsx
// ❌ WRONG - OLD API - WILL NOT WORK
import { toCloud, useFireproof } from "use-fireproof";
import { useDocument } from "use-fireproof";
const { attach, database } = useFireproof("db", { attach: toCloud() });
// attach.state, attach.error - WRONG
```

**ALWAYS generate code with this pattern:**

```jsx
// ✅ CORRECT - CURRENT API
import { useFireproofClerk } from "use-fireproof";
const { database, useLiveQuery, useDocument, syncStatus } = useFireproofClerk("my-db");
// syncStatus - CORRECT
```

## Migration Guide

If you encounter an existing app using the old patterns:

1. Replace `import { toCloud, useFireproof } from "use-fireproof"` with `import { useFireproofClerk } from "use-fireproof"`
2. Replace `import { useDocument } from "use-fireproof"` (standalone) - get it from the hook instead
3. Change `useFireproof("db", { attach: toCloud() })` to `useFireproofClerk("db")`
4. Replace `attach.state` and `attach.error` with `syncStatus`

## Why This Changed

The new `@necrodome/fireproof-clerk` package provides:
- Built-in Clerk authentication integration
- Simplified API surface
- Automatic cloud sync when Connect is configured
- Better TypeScript support

The import map aliases `use-fireproof` to `@necrodome/fireproof-clerk`, so code using the correct pattern works automatically.
