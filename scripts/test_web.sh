#!/bin/bash
# Test script for Goose Web Interface

echo "Testing Goose Web Interface..."
echo "================================"

# Start the web server in the background
echo "Starting web server on port 8080..."
./target/debug/goose web --port 8080 &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Test the health endpoint
echo -e "\nTesting health endpoint:"
curl -s http://localhost:8080/api/health | jq .

# Open browser (optional)
# open http://localhost:8080

echo -e "\nWeb server is running at http://localhost:8080"
echo "Press Ctrl+C to stop the server"

# Wait for user to stop
wait $SERVER_PID