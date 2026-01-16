# Next Steps to Create PR

## 1. Fork the Repository

1. Go to https://github.com/block/goose
2. Click the "Fork" button in the top right
3. This will create `https://github.com/YOUR_USERNAME/goose`

## 2. Add Your Fork as a Remote

```bash
cd ~/Development/goose-session-name-fix
git remote add fork https://github.com/YOUR_USERNAME/goose.git
```

## 3. Push Your Branch

```bash
git push -u fork fix/session-name-ui-sync
```

## 4. Create the Pull Request

1. Go to https://github.com/block/goose/pulls
2. Click "New pull request"
3. Click "compare across forks"
4. Set:
   - **base repository**: `block/goose`
   - **base branch**: `main`
   - **head repository**: `YOUR_USERNAME/goose`
   - **compare branch**: `fix/session-name-ui-sync`
5. Click "Create pull request"
6. Copy the content from `PR_DESCRIPTION.md` into the PR description
7. Submit!

## Summary of Changes

**Branch**: `fix/session-name-ui-sync`
**Files Modified**: 
- `ui/desktop/src/hooks/useChatStream.ts` (added 19 lines)

**What it does**:
- Automatically refreshes session name in UI after replies complete
- Only checks if name is still "New session X" (stops after name is set)
- Maximum 3-4 API calls per session, then zero overhead
- Fixes the UX issue where users had to close/reopen sessions to see names

**Performance**:
- Extremely cheap: ~2-3KB data, <5ms DB queries
- Self-limiting: stops checking after name is updated
- No user-perceived latency
- See detailed performance analysis in PR_DESCRIPTION.md
