#!/usr/bin/env bash

# Demo script showing how to use the error_proxy provider with a live goose instance
# This demonstrates runtime error injection without modifying the goose binary

set -e

CONTROL_FILE="/tmp/goose-error-control.json"
CONTROL_SCRIPT="./goose-error-control.py"

echo "=== Goose Error Proxy Provider Demo ==="
echo
echo "This demo shows how to inject errors into a running goose instance"
echo "using the error_proxy provider."
echo

# Check if control script exists
if [ ! -f "$CONTROL_SCRIPT" ]; then
    CONTROL_SCRIPT="../examples/goose-error-control.py"
fi

echo "Step 1: Start goose with the error_proxy provider"
echo "----------------------------------------"
echo "In a separate terminal, run:"
echo
echo "  export GOOSE_PROVIDER=error_proxy"
echo "  export ERROR_PROXY_TARGET_PROVIDER=openai  # or anthropic, etc."
echo "  export ERROR_PROXY_CONTROL_FILE=$CONTROL_FILE"
echo "  export OPENAI_API_KEY=your-key  # Set your actual provider credentials"
echo "  goose session start"
echo
echo "The proxy will wrap your real provider and inject errors based on the control file."
echo
read -p "Press Enter when goose is running..."

echo
echo "Step 2: Check initial status (errors disabled)"
echo "----------------------------------------"
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" status
echo

echo "Step 3: Enable rate limit errors every 3rd call"
echo "----------------------------------------"
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" enable rate_limit --pattern every_nth --nth 3
echo
echo "Now try sending 3-4 messages to goose. The 3rd one should fail with a rate limit error."
echo
read -p "Press Enter to continue..."

echo
echo "Step 4: Switch to random context length errors (30% chance)"
echo "----------------------------------------"
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" enable context_exceeded --pattern random --probability 0.3
echo
echo "Now messages have a 30% chance of failing with context length exceeded."
echo
read -p "Press Enter to continue..."

echo
echo "Step 5: Try a burst of server errors"
echo "----------------------------------------"
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" enable server_error --pattern burst --burst-count 3
echo
echo "The next 3 messages will fail with server errors, then return to normal."
echo
read -p "Press Enter to continue..."

echo
echo "Step 6: Use a preset configuration"
echo "----------------------------------------"
echo "Available presets:"
echo "  - flaky: Random server errors (20% chance)"
echo "  - overloaded: Rate limits every 5th call"
echo "  - broken: Continuous server errors"
echo "  - slow: Random timeouts (30% chance)"
echo
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" preset flaky
echo
read -p "Press Enter to continue..."

echo
echo "Step 7: Target specific models only"
echo "----------------------------------------"
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" enable auth_error \
    --pattern every_nth --nth 2 \
    --target-models "gpt-4" "gpt-4-turbo" \
    --message "Simulated auth error for GPT-4 models"
echo
echo "Now only GPT-4 models will get auth errors."
echo
read -p "Press Enter to continue..."

echo
echo "Step 8: Disable error injection"
echo "----------------------------------------"
$CONTROL_SCRIPT --control-file "$CONTROL_FILE" disable
echo
echo "All errors are now disabled. Goose will work normally."
echo

echo
echo "=== Advanced Usage ==="
echo
echo "1. Watch mode - Monitor control file changes in real-time:"
echo "   $CONTROL_SCRIPT watch"
echo
echo "2. Custom control file location:"
echo "   export ERROR_PROXY_CONTROL_FILE=/path/to/your/control.json"
echo
echo "3. Programmatic control from another script:"
cat << 'EOF'
   
   import json
   
   # Enable errors
   with open('/tmp/goose-error-control.json', 'w') as f:
       json.dump({
           "enabled": True,
           "error_type": "rate_limit",
           "pattern": "every_nth",
           "nth": 5,
           "retry_after_seconds": 60
       }, f)
   
   # Disable errors
   with open('/tmp/goose-error-control.json', 'w') as f:
       json.dump({"enabled": False}, f)
EOF
echo
echo "4. Test automation example:"
echo "   - Start goose with error_proxy"
echo "   - Run your test suite"
echo "   - Inject errors at specific points"
echo "   - Verify error handling behavior"
echo

echo "=== Demo Complete ==="
echo
echo "The error_proxy provider allows you to:"
echo "✓ Test error handling without modifying code"
echo "✓ Simulate various failure scenarios"
echo "✓ Control errors dynamically at runtime"
echo "✓ Target specific models or patterns"
echo "✓ Perfect for testing, debugging, and demos"
