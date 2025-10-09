#!/bin/bash
# Test script to see what requests goose configure makes

echo "Starting proxy with error-interval 10 (so we can see successful requests)..."
uv run python proxy.py --error-interval 10 &
PROXY_PID=$!

sleep 2

echo ""
echo "Making test requests..."
echo "Request 1:"
curl -s http://localhost:8888/v1/models -H "Authorization: Bearer $OPENAI_API_KEY" | python3 -c "import sys, json; d=json.load(sys.stdin); print(f'Success: {len(d.get(\"data\", []))} models')"

echo "Request 2:"
curl -s http://localhost:8888/v1/models -H "Authorization: Bearer $OPENAI_API_KEY" | python3 -c "import sys, json; d=json.load(sys.stdin); print(f'Success: {len(d.get(\"data\", []))} models')"

echo "Request 3:"
curl -s http://localhost:8888/v1/models -H "Authorization: Bearer $OPENAI_API_KEY" | python3 -c "import sys, json; d=json.load(sys.stdin); print(f'Success: {len(d.get(\"data\", []))} models')"

echo ""
echo "Stopping proxy..."
kill $PROXY_PID
wait $PROXY_PID 2>/dev/null
