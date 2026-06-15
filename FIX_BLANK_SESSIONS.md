# Fix: Prevent Blank Chat Histories from Duplicate Session Creation

## Problem

Users are experiencing blank chat histories in the session list. Investigation revealed:

- **38% of user sessions** (40 out of 105) have 0 messages
- Sessions are created in **duplicate pairs** at the exact same timestamp
- One session gets used, the other is abandoned after 12-22 seconds
- All blank sessions have the default name "New Chat"
- Pattern started around March 2026

## Root Cause

**React useEffect race condition in `ui/desktop/src/App.tsx`**

The session creation `useEffect` has `extensionsList` in its dependency array:

```typescript
useEffect(() => {
  // Create session logic...
}, [
  initialMessage,
  recipeDeeplinkFromConfig,
  recipeIdFromConfig,
  resumeSessionId,
  setSearchParams,
  extensionsList,  // ← PROBLEM: This changes during initialization
]);
```

### What happens:

1. User opens a new chat
2. `useEffect` runs and starts creating a session
3. While session is being created, `extensionsList` loads/changes
4. `useEffect` runs **again** due to `extensionsList` change
5. **Second session is created** at nearly the same timestamp
6. First session completes and gets used
7. Second session is abandoned (blank, 0 messages)

The `isCreatingSession` guard doesn't prevent this because:
- It's not in the dependency array (intentionally, to avoid stale closures)
- React can batch state updates, causing both effects to see `isCreatingSession=false`

## Solution

### 1. Primary Fix: Remove extensionsList from useEffect dependencies

**File**: `ui/desktop/src/App.tsx`

Remove `extensionsList` from the dependency array. The extensions list is captured at the time of session creation but shouldn't trigger re-creation when it changes during initialization.

```typescript
useEffect(() => {
  // Session creation logic...
  // extensionsList is used but not a dependency
}, [
  initialMessage,
  recipeDeeplinkFromConfig,
  recipeIdFromConfig,
  resumeSessionId,
  setSearchParams,
  // extensionsList removed from here
]);
```

**Why this is safe:**
- `extensionsList` is only used to pass extension configs to the new session
- We want to capture the extensions at the moment the effect runs
- We don't want to re-create sessions when extensions load/change
- The session will use whatever extensions were available at creation time

### 2. Cleanup Script: Remove Existing Blank Sessions

**File**: `scripts/cleanup-blank-sessions.sh`

A safe cleanup script that:
- Identifies sessions with 0 messages
- Shows details before deletion
- Creates a backup before making changes
- Requires explicit confirmation

**Usage:**
```bash
./scripts/cleanup-blank-sessions.sh
```

## Testing

### Verify the fix:

1. **Before fix**: Open multiple new chats quickly
   - Check database: `sqlite3 ~/.local/share/goose/sessions/sessions.db "SELECT created_at, COUNT(*) FROM sessions GROUP BY created_at HAVING COUNT(*) > 1"`
   - Should see duplicate timestamps

2. **After fix**: Same test
   - Should see no duplicate timestamps
   - Each new chat creates exactly one session

### Test the cleanup script:

```bash
cd /tmp/goose
./scripts/cleanup-blank-sessions.sh
```

Expected output:
- Shows count of blank sessions
- Lists details of blank sessions
- Warns about duplicate timestamps
- Creates backup before deletion
- Reports number of sessions deleted

## Verification

### Check for blank sessions:

```sql
SELECT s.id, s.name, s.created_at, COUNT(m.id) as message_count
FROM sessions s 
LEFT JOIN messages m ON s.id = m.session_id 
WHERE s.session_type = 'user' 
GROUP BY s.id 
HAVING message_count = 0
ORDER BY s.created_at DESC;
```

### Check for duplicate timestamps (race condition indicator):

```sql
SELECT created_at, COUNT(*) as count, GROUP_CONCAT(id) as session_ids
FROM sessions 
WHERE session_type = 'user'
GROUP BY created_at 
HAVING count > 1
ORDER BY created_at DESC;
```

## Impact

- **Prevents** new blank sessions from being created
- **Cleans up** existing blank sessions (optional, via script)
- **Improves** user experience by removing clutter from session list
- **No breaking changes** - only removes unused sessions

## Future Improvements

1. **Add session creation debouncing** - Additional safety layer
2. **Add telemetry** - Track duplicate session creation attempts
3. **Database constraint** - Consider UNIQUE constraint on (created_at, working_dir)
4. **Stabilize extensionsList** - Use useMemo in ConfigContext to prevent unnecessary re-renders

## Files Changed

- `ui/desktop/src/App.tsx` - Remove extensionsList from useEffect dependencies
- `scripts/cleanup-blank-sessions.sh` - New cleanup script (optional)

## Migration Path

1. Deploy the fix to prevent new blank sessions
2. Users can optionally run cleanup script to remove existing blank sessions
3. No forced migration needed - blank sessions are harmless, just clutter
