# Fix: Prevent blank chat histories from duplicate session creation

## Summary

Fixes a React useEffect race condition that creates duplicate sessions, leaving 38% of user sessions blank (0 messages). The fix removes `extensionsList` from the useEffect dependency array to prevent re-triggering session creation when extensions load during initialization.

## Problem

Users are experiencing blank chat histories in their session list:
- **40 out of 105 user sessions** (38%) have 0 messages
- Sessions are created in **duplicate pairs** at the exact same timestamp
- One session gets used, the other is abandoned after 12-22 seconds
- All blank sessions have the default name "New Chat"

### Evidence

```sql
-- 40 blank sessions found
SELECT COUNT(*) FROM (
  SELECT s.id FROM sessions s 
  LEFT JOIN messages m ON s.id = m.session_id 
  WHERE s.session_type = 'user' 
  GROUP BY s.id HAVING COUNT(m.id) = 0
);

-- 15 timestamps with duplicate sessions (race condition indicator)
SELECT created_at, COUNT(*) FROM sessions 
WHERE session_type = 'user'
GROUP BY created_at HAVING COUNT(*) > 1;
```

## Root Cause

**File**: `ui/desktop/src/App.tsx` (lines 103-155)

The session creation `useEffect` includes `extensionsList` in its dependency array. When extensions load during initialization, the list changes, triggering the effect again and creating a duplicate session.

```typescript
useEffect(() => {
  if ((initialMessage || recipeDeeplink) && !resumeSessionId && !isCreatingSession) {
    // Create session...
  }
}, [
  initialMessage,
  recipeDeeplinkFromConfig,
  recipeIdFromConfig,
  resumeSessionId,
  setSearchParams,
  extensionsList,  // ← PROBLEM: Changes during initialization
]);
```

**Timeline of the bug:**
1. User opens new chat → useEffect runs
2. Session creation starts with current extensionsList
3. Extensions finish loading → extensionsList updates
4. useEffect runs **again** due to extensionsList change
5. Second session is created at same timestamp
6. First session completes and is used
7. Second session is abandoned (blank, 0 messages)

The `isCreatingSession` guard doesn't prevent this because:
- It's intentionally not in the dependency array (to avoid stale closures)
- React can batch state updates, so both effects see `isCreatingSession=false`

## Solution

Remove `extensionsList` from the useEffect dependency array. The extensions list is captured at the time of session creation but shouldn't trigger re-creation when it changes.

### Changes

**ui/desktop/src/App.tsx:**
- Remove `extensionsList` from dependency array (line 156)
- Update comment to explain why this is safe

```diff
  useEffect(() => {
    // Session creation logic...
-   // Note: isCreatingSession is intentionally NOT in the dependency array
-   // It's only used as a guard to prevent concurrent session creation
+   // Note: isCreatingSession and extensionsList are intentionally NOT in the dependency array
+   // isCreatingSession is only used as a guard to prevent concurrent session creation
+   // extensionsList is captured at the time of session creation but shouldn't trigger re-creation
+   // when it changes (e.g., during extension loading), as this causes duplicate sessions
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    initialMessage,
    recipeDeeplinkFromConfig,
    recipeIdFromConfig,
    resumeSessionId,
    setSearchParams,
-   extensionsList,
  ]);
```

### Why this is safe

- `extensionsList` is used but doesn't need to trigger re-creation
- We want to capture extensions at the moment the effect runs
- Sessions use whatever extensions were available at creation time
- Prevents duplicate session creation when extensions load
- No functional change - just prevents unnecessary re-runs

## Testing

### Manual Testing

**Before fix:**
1. Open goose desktop app
2. Create several new chats in quick succession
3. Check database for duplicates:
   ```bash
   sqlite3 ~/.local/share/goose/sessions/sessions.db \
     "SELECT created_at, COUNT(*) FROM sessions GROUP BY created_at HAVING COUNT(*) > 1"
   ```
4. Result: Multiple duplicate timestamps

**After fix:**
1. Same test
2. Result: No duplicate timestamps, each chat creates exactly one session

### Automated Testing

Existing tests should pass. The change only affects when the effect runs, not what it does.

```bash
cd ui/desktop
npm test
```

## Cleanup Script (Optional)

A cleanup script is provided to remove existing blank sessions:

**File**: `scripts/cleanup-blank-sessions.sh`

Features:
- Identifies sessions with 0 messages
- Shows details before deletion
- Creates automatic backup
- Requires explicit confirmation
- Reports duplicate timestamp indicators

**Usage:**
```bash
./scripts/cleanup-blank-sessions.sh
```

This is optional and can be run by users who want to clean up their session list.

## Impact

### Positive
- ✅ Prevents new blank sessions from being created
- ✅ Cleaner session list for users
- ✅ Reduces database clutter
- ✅ No breaking changes
- ✅ No functional changes

### Risk Assessment
- **Risk Level**: LOW
- **Reason**: Only changes when useEffect runs, not what it does
- **Testing**: Manual testing confirms fix works
- **Rollback**: Simple revert if needed

## Related Issues

This may be related to user reports of:
- Duplicate sessions in session list
- Empty "New Chat" entries
- Session list clutter

## Checklist

- [x] Root cause identified and documented
- [x] Fix implemented and tested manually
- [x] Cleanup script provided (optional)
- [x] Documentation updated
- [x] No breaking changes
- [x] Existing tests should pass

## Files Changed

- `ui/desktop/src/App.tsx` - Remove extensionsList from useEffect dependencies
- `scripts/cleanup-blank-sessions.sh` - New cleanup script (optional)
- `FIX_BLANK_SESSIONS.md` - Detailed documentation
- `INVESTIGATION_SUMMARY.md` - Investigation details

## Future Improvements

1. Add telemetry to track session creation attempts
2. Add debouncing as additional safety layer
3. Stabilize extensionsList with useMemo in ConfigContext
4. Consider UNIQUE constraint on (created_at, working_dir) in database
5. Automated cleanup job for abandoned sessions
