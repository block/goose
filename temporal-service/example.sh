#!/bin/bash

# Example usage script for the Temporal service
set -e

echo "Temporal Service Example Usage"
echo "=============================="
echo ""

# Check if service is running
if ! curl -s http://localhost:8080/health > /dev/null; then
    echo "Starting Temporal service..."
    echo "Please run in another terminal: ./temporal-service"
    echo "Then run this script again."
    exit 1
fi

echo "✓ Temporal service is running"
echo ""

# Create example recipe
RECIPE_FILE="/tmp/example-recipe.yaml"
cat > $RECIPE_FILE << EOF
version: "1.0.0"
title: "Daily Report Generator"
description: "Generates a daily report"
prompt: |
  Generate a daily report with the following information:
  - Current date and time
  - System status
  - Recent activity summary
  
  Please format the output as a structured report.
EOF

echo "Created example recipe: $RECIPE_FILE"
echo ""

# Function to make API calls
make_api_call() {
    local action="$1"
    local job_id="$2"
    local cron="$3"
    local recipe_path="$4"
    
    local payload="{\"action\": \"$action\""
    
    if [ -n "$job_id" ]; then
        payload="$payload, \"job_id\": \"$job_id\""
    fi
    
    if [ -n "$cron" ]; then
        payload="$payload, \"cron\": \"$cron\""
    fi
    
    if [ -n "$recipe_path" ]; then
        payload="$payload, \"recipe_path\": \"$recipe_path\""
    fi
    
    payload="$payload}"
    
    echo "API Call: $payload"
    curl -s -X POST http://localhost:8080/jobs \
        -H "Content-Type: application/json" \
        -d "$payload" | jq .
    echo ""
}

# Example 1: Create a daily job
echo "1. Creating a daily job (runs at 9 AM every day)..."
make_api_call "create" "daily-report" "0 9 * * *" "$RECIPE_FILE"

# Example 2: Create an hourly job
echo "2. Creating an hourly job..."
make_api_call "create" "hourly-check" "0 * * * *" "$RECIPE_FILE"

# Example 3: List all jobs
echo "3. Listing all scheduled jobs..."
make_api_call "list"

# Example 4: Pause a job
echo "4. Pausing the hourly job..."
make_api_call "pause" "hourly-check"

# Example 5: List jobs again to see paused status
echo "5. Listing jobs to see paused status..."
make_api_call "list"

# Example 6: Unpause the job
echo "6. Unpausing the hourly job..."
make_api_call "unpause" "hourly-check"

# Example 7: Run a job immediately
echo "7. Running daily-report job immediately..."
echo "Note: This will fail without goose-scheduler-executor binary"
make_api_call "run_now" "daily-report"

# Example 8: Delete jobs
echo "8. Cleaning up - deleting jobs..."
make_api_call "delete" "daily-report"
make_api_call "delete" "hourly-check"

# Example 9: Final list (should be empty)
echo "9. Final job list (should be empty)..."
make_api_call "list"

# Clean up
rm -f $RECIPE_FILE

echo "Example completed!"
echo ""
echo "Common cron expressions:"
echo "  '0 9 * * *'     - Daily at 9 AM"
echo "  '0 */6 * * *'   - Every 6 hours"
echo "  '*/15 * * * *'  - Every 15 minutes"
echo "  '0 0 * * 0'     - Weekly on Sunday at midnight"
echo "  '0 0 1 * *'     - Monthly on the 1st at midnight"