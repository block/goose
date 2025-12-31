# Goose Utility Binaries

This directory contains standalone utility scripts for Goose.

## consolidate-messages

**Purpose:** One-time migration to fix fragmented chat histories.

### Background

Before the streaming consolidation fix (commit e53a12eef1), assistant messages were saved as separate database rows for each streamed chunk. This caused chat histories to display with unwanted line breaks when viewing saved sessions.

The consolidation fix prevents new messages from being fragmented, but existing broken histories need to be repaired.

### What It Does

- Scans all sessions in the database
- Finds consecutive assistant text-only messages
- Merges them into single messages
- Deletes the fragmented pieces
- Reports the number of fragments consolidated

### Safety

- **Safe to run multiple times** - idempotent operation
- **Preserves message ordering** - uses `created_timestamp`
- **Never touches**:
  - User messages
  - Messages with tool calls
  - Non-consecutive assistant messages
- **Transactional** - all changes committed together or rolled back

### Usage

```bash
# From the repository root
cd /path/to/goose

# Run the migration
cargo run --bin consolidate-messages

# Or build and run separately
cargo build --bin consolidate-messages
./target/debug/consolidate-messages
```

### Output Example

```
🔧 Consolidating Fragmented Messages
=====================================

This will merge consecutive assistant text messages that were
fragmented during streaming. This operation is safe and can be
run multiple times.

Scanning database... done!

✅ Successfully consolidated 247 message fragments
   Your chat history should now display correctly!

🎉 Migration complete!
```

### When to Run

Run this **once** after upgrading to a version with the consolidation fix if you:
- Have existing chat sessions created before the fix
- Notice broken line breaks in chat history
- Want to clean up your database

### Technical Details

- Uses `SessionManager::consolidate_fragmented_messages()`
- Processes all sessions in creation order
- Only merges simple text messages (no tool requests/responses)
- Updates first message, deletes subsequent fragments
- Returns count of consolidated fragments

### After Running

- Reload any open sessions in the UI to see the fixed messages
- Chat history should display cleanly without fragmentation
- No further action needed - new messages will consolidate automatically
