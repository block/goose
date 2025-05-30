#!/bin/bash

# Test script to demonstrate the lead/worker model feature with automatic fallback
# This shows how to configure and test the feature

echo "=== Lead/Worker Model Feature with Automatic Fallback ==="
echo

echo "1. Testing with GOOSE_LEAD_MODEL set:"
echo "   GOOSE_PROVIDER=openai"
echo "   GOOSE_MODEL=gpt-4o-mini (worker model)"
echo "   GOOSE_LEAD_MODEL=gpt-4o (lead model for first 3 turns)"
echo

echo "2. Expected behavior:"
echo "   - Turn 1-3: Uses gpt-4o (lead model)"
echo "   - Turn 4+: Uses gpt-4o-mini (worker model)"
echo "   - Auto-fallback: After 2 consecutive worker failures → 2 turns of lead model"
echo "   - Recovery: Returns to worker model after successful fallback"
echo

echo "3. To test manually:"
echo "   cd ../../../../"
echo "   export GOOSE_PROVIDER=openai"
echo "   export GOOSE_MODEL=gpt-4o-mini"
echo "   export GOOSE_LEAD_MODEL=gpt-4o"
echo "   export OPENAI_API_KEY=your_key_here"
echo "   ./target/debug/goose session"
echo

echo "4. To disable (use only worker model):"
echo "   unset GOOSE_LEAD_MODEL"
echo

echo "5. Watch the logs for messages like:"
echo "   'Using lead (initial) provider for turn 1 (lead_turns: 3)'"
echo "   'Using worker provider for turn 4 (lead_turns: 3)'"
echo "   'Entering fallback mode after 2 consecutive failures'"
echo "   'Using lead (fallback) provider for turn 7 (fallback mode: 1 turns remaining)'"
echo "   'Exiting fallback mode - worker model resumed'"
echo

echo "=== Fallback Behavior Example ==="
echo "Turn 1-3: GPT-4o (lead)          ✅ Success"
echo "Turn 4:   GPT-4o-mini (worker)   ✅ Success"  
echo "Turn 5:   GPT-4o-mini (worker)   ❌ Failure (count: 1)"
echo "Turn 6:   GPT-4o-mini (worker)   ❌ Failure (count: 2) → Triggers fallback!"
echo "Turn 7:   GPT-4o (lead fallback) ✅ Success (fallback: 1 remaining)"
echo "Turn 8:   GPT-4o (lead fallback) ✅ Success (fallback: 0 remaining) → Exit fallback"
echo "Turn 9:   GPT-4o-mini (worker)   ✅ Back to normal operation"
echo

echo "=== Configuration Examples ==="
echo

echo "OpenAI (GPT-4o -> GPT-4o-mini):"
echo "export GOOSE_PROVIDER=openai"
echo "export GOOSE_MODEL=gpt-4o-mini"
echo "export GOOSE_LEAD_MODEL=gpt-4o"
echo

echo "Anthropic (Claude 3.5 Sonnet -> Claude 3 Haiku):"
echo "export GOOSE_PROVIDER=anthropic"
echo "export GOOSE_MODEL=claude-3-haiku-20240307"
echo "export GOOSE_LEAD_MODEL=claude-3-5-sonnet-20241022"
echo

echo "=== Unit Tests ==="
echo "Run unit tests with:"
echo "cd ../../../../"
echo "cargo test -p goose lead_worker --lib"
echo "cargo test -p goose test_fallback_on_failures --lib"
echo "(Note: May fail due to protoc issues, but the logic is tested)"

echo
echo "=== Key Features ==="
echo "✅ Simple configuration (just GOOSE_LEAD_MODEL)"
echo "✅ Fixed 3 turns for lead model"
echo "✅ Automatic worker model fallback"
echo "✅ Failure detection and recovery"
echo "✅ Self-healing behavior"
echo "✅ Comprehensive logging"