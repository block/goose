#!/bin/bash

# Test script for Temporal service
set -e

echo "Testing Temporal service..."

# Check if service is running
if ! curl -s http://localhost:8080/health > /dev/null; then
    echo "Error: Temporal service is not running on port 8080"
    echo "Please start it with: ./temporal-service"
    exit 1
fi

echo "✓ Service is running"

# Test health endpoint
echo "Testing health endpoint..."
HEALTH_RESPONSE=$(curl -s http://localhost:8080/health)
if [[ $HEALTH_RESPONSE == *"healthy"* ]]; then
    echo "✓ Health check passed"
else
    echo "✗ Health check failed: $HEALTH_RESPONSE"
    exit 1
fi

# Test list schedules (should be empty initially)
echo "Testing list schedules..."
LIST_RESPONSE=$(curl -s -X POST http://localhost:8080/jobs \
    -H "Content-Type: application/json" \
    -d '{"action": "list"}')

if [[ $LIST_RESPONSE == *"\"success\":true"* ]]; then
    echo "✓ List schedules works"
else
    echo "✗ List schedules failed: $LIST_RESPONSE"
    exit 1
fi

# Create a test recipe file
TEST_RECIPE="/tmp/test-recipe.yaml"
cat > $TEST_RECIPE << EOF
version: "1.0.0"
title: "Test Recipe"
description: "A test recipe for the scheduler"
prompt: "This is a test prompt for scheduled execution."
EOF

echo "Created test recipe at $TEST_RECIPE"

# Test create schedule
echo "Testing create schedule..."
CREATE_RESPONSE=$(curl -s -X POST http://localhost:8080/jobs \
    -H "Content-Type: application/json" \
    -d "{\"action\": \"create\", \"job_id\": \"test-job\", \"cron\": \"0 */6 * * *\", \"recipe_path\": \"$TEST_RECIPE\"}")

if [[ $CREATE_RESPONSE == *"\"success\":true"* ]]; then
    echo "✓ Create schedule works"
else
    echo "✗ Create schedule failed: $CREATE_RESPONSE"
    exit 1
fi

# Test list schedules again (should have one job)
echo "Testing list schedules with job..."
LIST_RESPONSE=$(curl -s -X POST http://localhost:8080/jobs \
    -H "Content-Type: application/json" \
    -d '{"action": "list"}')

if [[ $LIST_RESPONSE == *"test-job"* ]]; then
    echo "✓ Job appears in list"
else
    echo "✗ Job not found in list: $LIST_RESPONSE"
    exit 1
fi

# Test pause schedule
echo "Testing pause schedule..."
PAUSE_RESPONSE=$(curl -s -X POST http://localhost:8080/jobs \
    -H "Content-Type: application/json" \
    -d '{"action": "pause", "job_id": "test-job"}')

if [[ $PAUSE_RESPONSE == *"\"success\":true"* ]]; then
    echo "✓ Pause schedule works"
else
    echo "✗ Pause schedule failed: $PAUSE_RESPONSE"
    exit 1
fi

# Test unpause schedule
echo "Testing unpause schedule..."
UNPAUSE_RESPONSE=$(curl -s -X POST http://localhost:8080/jobs \
    -H "Content-Type: application/json" \
    -d '{"action": "unpause", "job_id": "test-job"}')

if [[ $UNPAUSE_RESPONSE == *"\"success\":true"* ]]; then
    echo "✓ Unpause schedule works"
else
    echo "✗ Unpause schedule failed: $UNPAUSE_RESPONSE"
    exit 1
fi

# Test delete schedule
echo "Testing delete schedule..."
DELETE_RESPONSE=$(curl -s -X POST http://localhost:8080/jobs \
    -H "Content-Type: application/json" \
    -d '{"action": "delete", "job_id": "test-job"}')

if [[ $DELETE_RESPONSE == *"\"success\":true"* ]]; then
    echo "✓ Delete schedule works"
else
    echo "✗ Delete schedule failed: $DELETE_RESPONSE"
    exit 1
fi

# Clean up
rm -f $TEST_RECIPE

echo ""
echo "🎉 All tests passed!"
echo ""
echo "The Temporal service is working correctly."
echo "You can now integrate it with the Rust scheduler."