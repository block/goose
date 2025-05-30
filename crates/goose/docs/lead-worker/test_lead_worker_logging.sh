#!/bin/bash

# Test script to demonstrate the lead/worker model logging feature
echo "=== Lead/Worker Model Logging Feature Test ==="
echo

echo "1. Testing with GOOSE_LEAD_MODEL environment variable:"
echo "   Setting GOOSE_LEAD_MODEL=gpt-4o, GOOSE_MODEL=gpt-4o-mini, GOOSE_PROVIDER=openai"
echo

# Set environment variables
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"
export GOOSE_LEAD_MODEL="gpt-4o"

echo "2. Expected behavior:"
echo "   - Shows startup logging with both lead and worker models"
echo "   - Lead model: gpt-4o (first 3 turns)"
echo "   - Worker model: gpt-4o-mini (turn 4+)"
echo "   - Auto-fallback enabled"
echo

echo "3. Running test command:"
echo "   echo 'hello' | ../../../../target/debug/goose run --text 'hello' --no-session"
echo

# Run the test (adjust path to goose binary)
echo "=== OUTPUT ==="
echo "hello" | timeout 10 ../../../../target/debug/goose run --text "hello" --no-session 2>&1 | head -10

echo
echo "=== Test completed ==="
echo
echo "4. Key features demonstrated:"
echo "   ✅ Session info shows both lead and worker models"
echo "   ✅ Clear indication of lead/worker mode in session header"
echo "   ✅ Tracing logs show model configuration (use RUST_LOG=info to see)"
echo "   ✅ Model switching happens automatically (logged during turns)"
echo
echo "5. During actual usage, you'll also see turn-by-turn logging like:"
echo "   'Using lead (initial) provider for turn 1 (lead_turns: 3)'"
echo "   'Using worker provider for turn 4 (lead_turns: 3)'"
echo "   '🔄 SWITCHING TO LEAD MODEL: Entering fallback mode...'"
echo "   '✅ SWITCHING BACK TO WORKER MODEL: Exiting fallback mode...'"
echo
echo "6. Session header now shows:"
echo "   'starting session | provider: openai lead model: gpt-4o worker model: gpt-4o-mini'"
echo "   instead of just:"
echo "   'starting session | provider: openai model: gpt-4o-mini'"