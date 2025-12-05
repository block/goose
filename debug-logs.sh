#!/bin/bash

# Debug logs extraction script for session isolation issue
# Usage: ./debug-logs.sh [log_file_or_paste_logs_here]

echo "=== SESSION ISOLATION DEBUG LOG ANALYZER ==="
echo ""

# If a file is provided, use it; otherwise read from stdin
if [ "$1" ]; then
    LOG_SOURCE="$1"
    echo "Reading from file: $LOG_SOURCE"
else
    echo "Paste your logs below and press Ctrl+D when done:"
    LOG_SOURCE="/tmp/debug_logs_$(date +%s).txt"
    cat > "$LOG_SOURCE"
fi

echo ""
echo "=== FILTERING RELEVANT LOGS ==="
echo ""

# Extract useChatStream logs (fullSessionId)
echo "📡 CHAT STREAM SESSION IDs:"
grep -E "(useChatStream|fullSessionId)" "$LOG_SOURCE" | head -20

echo ""
echo "🏷️  TAB CONTEXT - NEW TAB CREATION:"
# Extract TabContext handleNewTab logs
grep -E "(handleNewTab|Current tab states|New session ID|Unique sessions)" "$LOG_SOURCE" | head -20

echo ""
echo "🔍 SESSION ID PATTERNS:"
# Extract just the session IDs to see duplicates
grep -oE "sessionId: [a-f0-9-]+" "$LOG_SOURCE" | sort | uniq -c | sort -nr

echo ""
echo "📊 TAB ID PATTERNS:"
# Extract tab IDs to see how many tabs are involved
grep -oE "tabId: [a-f0-9-]+" "$LOG_SOURCE" | sort | uniq -c | sort -nr

echo ""
echo "⚡ MESSAGE SEND EVENTS:"
# Look for message sending patterns
grep -E "(Sending message|Message sent|handleSendMessage)" "$LOG_SOURCE" | head -10

echo ""
echo "🎯 POTENTIAL ISSUES:"
echo "Look for:"
echo "  - Same sessionId appearing with different tabIds"
echo "  - Multiple tabs sharing the same session"
echo "  - New tab creation not generating unique sessionId"

# Clean up temp file if we created one
if [ "$1" = "" ]; then
    rm -f "$LOG_SOURCE"
fi
