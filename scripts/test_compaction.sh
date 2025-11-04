#!/bin/bash

# Compaction smoke test script
# Tests both manual (/compact command) and auto compaction (threshold-based)

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

if [ -z "$SKIP_BUILD" ]; then
  echo "Building goose..."
  cargo build --release --bin goose
  echo ""
else
  echo "Skipping build (SKIP_BUILD is set)..."
  echo ""
fi

SCRIPT_DIR=$(pwd)
GOOSE_BIN="$SCRIPT_DIR/target/release/goose"

# Validation function to check compaction structure in session JSON
validate_compaction() {
  local session_id=$1
  local test_name=$2

  echo "Validating compaction structure for session: $session_id"

  # Export the session to JSON
  local session_json=$($GOOSE_BIN session export --format json "$session_id" 2>&1)

  if [ $? -ne 0 ]; then
    echo "✗ FAILED: Could not export session JSON"
    echo "   Error: $session_json"
    return 1
  fi

  if ! command -v jq &> /dev/null; then
    echo "⚠ WARNING: jq not available, cannot validate compaction structure"
    return 0
  fi

  # Check basic structure
  echo "$session_json" | jq -e '.conversation' > /dev/null 2>&1
  if [ $? -ne 0 ]; then
    echo "✗ FAILED: Session JSON missing 'conversation' field"
    return 1
  fi

  local message_count=$(echo "$session_json" | jq '.conversation | length' 2>/dev/null)
  echo "   Session has $message_count messages"

  # Look for a summary message (assistant role with userVisible=false, agentVisible=true)
  local has_summary=$(echo "$session_json" | jq '[.conversation[] | select(.role == "assistant" and .metadata.userVisible == false and .metadata.agentVisible == true)] | length > 0' 2>/dev/null)

  if [ "$has_summary" != "true" ]; then
    echo "✗ FAILED: No summary message found (expected assistant message with userVisible=false, agentVisible=true)"
    return 1
  fi
  echo "✓ Found summary message with correct visibility flags"

  # Check for original messages with userVisible=true, agentVisible=false
  local has_hidden_originals=$(echo "$session_json" | jq '[.conversation[] | select(.metadata.userVisible == true and .metadata.agentVisible == false)] | length > 0' 2>/dev/null)

  if [ "$has_hidden_originals" != "true" ]; then
    echo "⚠ WARNING: No original messages found with userVisible=true, agentVisible=false"
    echo "   This might be OK if all messages were compacted"
  else
    echo "✓ Found original messages hidden from agent (userVisible=true, agentVisible=false)"
  fi

  # For auto-compaction, check for the preserved user message (userVisible=true, agentVisible=true)
  local has_preserved_user=$(echo "$session_json" | jq '[.conversation[] | select(.role == "user" and .metadata.userVisible == true and .metadata.agentVisible == true)] | length > 0' 2>/dev/null)

  if [ "$has_preserved_user" == "true" ]; then
    echo "✓ Found preserved user message (userVisible=true, agentVisible=true)"
  fi

  echo "✓ SUCCESS: Compaction structure is valid for $test_name"
  return 0
}

echo "=================================================="
echo "COMPACTION SMOKE TESTS"
echo "=================================================="
echo ""

# Check if jq is available
if ! command -v jq &> /dev/null; then
  echo "⚠ WARNING: jq is not installed. Compaction structure validation will be limited."
  echo "   Install jq to enable full validation: brew install jq (macOS) or apt-get install jq (Linux)"
  echo ""
fi

RESULTS=()

# ==================================================
# TEST 1: Manual Compaction
# ==================================================
echo "---------------------------------------------------"
echo "TEST 1: Manual Compaction via /compact command"
echo "---------------------------------------------------"

TESTDIR=$(mktemp -d)
echo "hello world" > "$TESTDIR/hello.txt"
echo "Test directory: $TESTDIR"
echo ""

OUTPUT=$(mktemp)

echo "Running session with prompts followed by /compact..."
# Use a script to send commands with delays so each is processed separately
(cd "$TESTDIR" && (
  sleep 1
  echo "list files"
  sleep 3
  echo "read hello.txt"
  sleep 3
  echo "/compact"
  sleep 3
  echo "exit"
) | "$GOOSE_BIN" session 2>&1) | tee "$OUTPUT"

echo ""
echo "Checking for compaction evidence..."

# Check if compaction occurred
if grep -qi "compacting\|compacted" "$OUTPUT"; then
  echo "✓ SUCCESS: Manual compaction was triggered"

  # Get the most recent session and validate structure
  if command -v jq &> /dev/null; then
    RECENT_SESSION=$("$GOOSE_BIN" session list --format json 2>/dev/null | jq -r '.[0].id' 2>/dev/null)
    if [ -n "$RECENT_SESSION" ] && [ "$RECENT_SESSION" != "null" ]; then
      if validate_compaction "$RECENT_SESSION" "manual compaction"; then
        RESULTS+=("✓ Manual Compaction")
      else
        RESULTS+=("✗ Manual Compaction (structure validation failed)")
      fi
    else
      echo "⚠ Could not retrieve recent session ID"
      RESULTS+=("✓ Manual Compaction (no structure validation)")
    fi
  else
    RESULTS+=("✓ Manual Compaction (jq not available)")
  fi
else
  echo "✗ FAILED: Manual compaction was not triggered"
  RESULTS+=("✗ Manual Compaction")
fi

rm -f "$OUTPUT"
rm -rf "$TESTDIR"

echo ""
echo ""

# ==================================================
# TEST 2: Auto Compaction
# ==================================================
echo "---------------------------------------------------"
echo "TEST 2: Auto Compaction via threshold (0.01)"
echo "---------------------------------------------------"

TESTDIR=$(mktemp -d)
echo "test content" > "$TESTDIR/test.txt"
echo "Test directory: $TESTDIR"
echo ""

# Set auto-compact threshold very low (1%) to trigger it quickly
export GOOSE_AUTO_COMPACT_THRESHOLD=0.01

OUTPUT=$(mktemp)

# Send two simple prompts - auto-compaction triggers on the second one
echo "Sending two prompts to trigger auto-compaction..."
(cd "$TESTDIR" && (
  sleep 1
  echo "hello"
  sleep 3
  echo "hi again"
  sleep 3
  echo "exit"
) | "$GOOSE_BIN" session 2>&1) | tee "$OUTPUT"

echo ""
echo "Checking for auto-compaction evidence..."

# Check if auto-compaction occurred
if grep -qi "auto.*compact\|exceeded.*auto.*compact.*threshold" "$OUTPUT"; then
  echo "✓ SUCCESS: Auto compaction was triggered"

  # Validate the compaction structure
  if command -v jq &> /dev/null; then
    RECENT_SESSION=$("$GOOSE_BIN" session list --format json 2>/dev/null | jq -r '.[0].id' 2>/dev/null)
    if [ -n "$RECENT_SESSION" ] && [ "$RECENT_SESSION" != "null" ]; then
      if validate_compaction "$RECENT_SESSION" "auto compaction"; then
        RESULTS+=("✓ Auto Compaction")
      else
        RESULTS+=("✗ Auto Compaction (structure validation failed)")
      fi
    else
      echo "⚠ Could not retrieve session ID"
      RESULTS+=("✓ Auto Compaction (no structure validation)")
    fi
  else
    RESULTS+=("✓ Auto Compaction (jq not available)")
  fi
else
  echo "✗ FAILED: Auto compaction was not triggered"
  echo "   Expected to see auto-compact messages with threshold of 0.01"
  RESULTS+=("✗ Auto Compaction")
fi

# Unset the env variable
unset GOOSE_AUTO_COMPACT_THRESHOLD

rm -f "$OUTPUT"
rm -rf "$TESTDIR"

echo ""
echo ""

# ==================================================
# Summary
# ==================================================
echo "=================================================="
echo "TEST SUMMARY"
echo "=================================================="
for result in "${RESULTS[@]}"; do
  echo "$result"
done

# Count actual failures
FAILURE_COUNT=$(echo "${RESULTS[@]}" | grep -o "✗" | wc -l | tr -d ' ')

if [ "$FAILURE_COUNT" -gt 0 ]; then
  echo ""
  echo "❌ $FAILURE_COUNT test(s) failed!"
  exit 1
else
  echo ""
  echo "✅ All tests passed!"
fi
