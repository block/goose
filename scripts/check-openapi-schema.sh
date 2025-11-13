#!/usr/bin/env bash
set -e

# Check if OpenAPI schema is up-to-date
# This script generates the OpenAPI schema and compares it with the committed version

echo "🔍 Checking OpenAPI schema is up-to-date..."

# Check if the generated schema differs from the committed version
echo "🔍 Comparing generated schema with committed version..."
if ! git diff --ignore-space-change --exit-code ui/desktop/openapi.json ui/desktop/src/api/; then
  echo ""
  echo "❌ OpenAPI schema is out of date!"
  echo ""
  echo "The generated OpenAPI schema differs from the committed version."
  echo "This usually means that API types were added or modified without updating the schema."
  echo ""
  echo "To fix this issue:"
  echo "1. Run 'just generate-openapi' locally"
  echo "2. Commit the changes to ui/desktop/openapi.json and ui/desktop/src/api/"
  echo "3. Push your changes"
  echo ""
  echo "Changes detected:"
  git diff ui/desktop/openapi.json ui/desktop/src/api/
  exit 1
fi

echo "✅ OpenAPI schema is up-to-date"
