#!/bin/bash
# Test script for the new command interface

echo "Testing provider error proxy command parsing..."
echo ""

# Test various command formats
test_commands=(
    "n"
    "c"
    "c4"
    "c 4"
    "c0.3"
    "c 0.3"
    "c30%"
    "c 30%"
    "c*"
    "c *"
    "r"
    "r 2"
    "u"
    "u 0.5"
    "q"
)

for cmd in "${test_commands[@]}"; do
    echo "Testing: '$cmd'"
    echo "$cmd" | timeout 1 uv run proxy.py 2>&1 | grep -A 5 "Current mode:" | head -6
    echo "---"
done

echo ""
echo "All tests completed!"
