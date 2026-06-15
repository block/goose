#!/bin/bash
# Script to clean up blank sessions (sessions with 0 messages and short duration)
# These are typically caused by duplicate session creation race conditions

set -e

DB_PATH="${HOME}/.local/share/goose/sessions/sessions.db"

if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database not found at $DB_PATH"
    exit 1
fi

echo "Analyzing blank sessions in goose database..."
echo ""

# Count blank sessions
BLANK_COUNT=$(sqlite3 "$DB_PATH" "
    SELECT COUNT(*) FROM (
        SELECT s.id 
        FROM sessions s 
        LEFT JOIN messages m ON s.id = m.session_id 
        WHERE s.session_type = 'user' 
        GROUP BY s.id 
        HAVING COUNT(m.id) = 0
    )
")

if [ -z "$BLANK_COUNT" ] || [ "$BLANK_COUNT" -eq 0 ]; then
    echo "No blank sessions found. Database is clean!"
    exit 0
fi

echo "Found $BLANK_COUNT blank sessions (sessions with 0 messages)"
echo ""

# Show details of blank sessions
echo "Details of blank sessions:"
echo "=========================="
sqlite3 -header -column "$DB_PATH" "
    SELECT 
        s.id,
        s.name,
        s.created_at,
        s.updated_at,
        CAST((julianday(s.updated_at) - julianday(s.created_at)) * 86400 AS INTEGER) as duration_seconds
    FROM sessions s 
    LEFT JOIN messages m ON s.id = m.session_id 
    WHERE s.session_type = 'user' 
    GROUP BY s.id 
    HAVING COUNT(m.id) = 0 
    ORDER BY s.created_at DESC
    LIMIT 20
"
echo ""

# Check for duplicate timestamps (indicator of race condition)
DUPLICATE_TIMESTAMPS=$(sqlite3 "$DB_PATH" "
    SELECT COUNT(*) 
    FROM (
        SELECT created_at, COUNT(*) as count 
        FROM sessions 
        WHERE session_type = 'user' 
        GROUP BY created_at 
        HAVING count > 1
    )
")

if [ "$DUPLICATE_TIMESTAMPS" -gt 0 ]; then
    echo "⚠️  Found $DUPLICATE_TIMESTAMPS timestamps with duplicate sessions (race condition indicator)"
    echo ""
fi

# Ask for confirmation
read -p "Do you want to delete these blank sessions? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Aborted. No sessions were deleted."
    exit 0
fi

# Create backup first
BACKUP_PATH="${DB_PATH}.backup.$(date +%Y%m%d_%H%M%S)"
echo ""
echo "Creating backup at: $BACKUP_PATH"
cp "$DB_PATH" "$BACKUP_PATH"

# Delete blank sessions
echo "Deleting blank sessions..."
DELETED=$(sqlite3 "$DB_PATH" "
    DELETE FROM sessions 
    WHERE id IN (
        SELECT s.id 
        FROM sessions s 
        LEFT JOIN messages m ON s.id = m.session_id 
        WHERE s.session_type = 'user' 
        GROUP BY s.id 
        HAVING COUNT(m.id) = 0
    );
    SELECT changes();
")

echo ""
echo "✓ Successfully deleted $DELETED blank sessions"
echo "✓ Backup saved to: $BACKUP_PATH"
echo ""
echo "To restore from backup if needed:"
echo "  cp \"$BACKUP_PATH\" \"$DB_PATH\""
