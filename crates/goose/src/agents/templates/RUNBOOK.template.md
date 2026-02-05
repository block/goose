# Project Runbook

**Goal:** [Single sentence describing what this accomplishes]

**Preconditions:**
- [ ] [Required software/versions]
- [ ] [Required environment variables]
- [ ] [Required permissions/access]

---

## Steps

### 1. [Step Name]
**Description:** [What this step does and why]

**Command:**
```bash
[actual command to run]
```

**Expected Result:** [What success looks like]

**If Fails:** [What to do if this step fails]

**Requires:**
- [Any specific precondition for this step]

---

### 2. [Next Step]
**Description:** [What this does]

**Command:**
```bash
[command]
```

**Expected Result:** [Success criteria]

**If Fails:** [Failure handling]

---

### 3. [Continue for all steps...]

---

## Verification

After all steps complete:

1. **Build Check**
   ```bash
   cargo build --release
   ```
   Expected: Clean build with no errors

2. **Test Check**
   ```bash
   cargo test --workspace
   ```
   Expected: All tests passing

3. **Lint Check**
   ```bash
   ./scripts/clippy-lint.sh
   ```
   Expected: No clippy warnings

---

## Rollback

If execution fails and needs rollback:

```bash
git reset --hard HEAD
cargo clean
```

---

## Notes

[Any additional context, gotchas, or important information]
