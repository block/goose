#!/bin/bash
# Extract and resolve Recipe schema from OpenAPI spec at a specific git version
# Usage: ./extract-schema.sh <version>
# Example: ./extract-schema.sh v1.15.0

set -e

VERSION=${1:-"main"}
GOOSE_REPO=${GOOSE_REPO:-"$HOME/Development/goose"}

if [ ! -d "$GOOSE_REPO" ]; then
    echo "Error: GOOSE_REPO directory not found: $GOOSE_REPO" >&2
    exit 1
fi

cd "$GOOSE_REPO"

# Verify version exists (for non-main versions)
if [ "$VERSION" != "main" ]; then
    if ! git rev-parse "$VERSION" >/dev/null 2>&1; then
        echo "Error: Version $VERSION not found in git history" >&2
        exit 1
    fi
fi

# Extract OpenAPI spec from git
if [ "$VERSION" = "main" ]; then
    if [ ! -f ui/desktop/openapi.json ]; then
        echo "Error: ui/desktop/openapi.json not found in working directory" >&2
        exit 1
    fi
    OPENAPI_JSON=$(cat ui/desktop/openapi.json)
else
    OPENAPI_JSON=$(git show "$VERSION:ui/desktop/openapi.json" 2>/dev/null || {
        echo "Error: Could not find ui/desktop/openapi.json at version $VERSION" >&2
        exit 1
    })
fi

# Use Node.js to extract and resolve Recipe schema
echo "$OPENAPI_JSON" | node -e "
const openApiSpec = JSON.parse(require('fs').readFileSync(0, 'utf-8'));

/**
 * Resolves \$ref references in OpenAPI schemas by expanding them with the actual schema definitions
 * Ported from ui/desktop/src/recipe/validation.ts
 */
function resolveRefs(schema, openApiSpec) {
  if (!schema || typeof schema !== 'object') {
    return schema;
  }

  // Handle \$ref
  if (typeof schema.\$ref === 'string') {
    const refPath = schema.\$ref.replace('#/', '').split('/');
    let resolved = openApiSpec;

    for (const segment of refPath) {
      if (resolved && typeof resolved === 'object' && segment in resolved) {
        resolved = resolved[segment];
      } else {
        console.warn(\`Could not resolve \$ref: \${schema.\$ref}\`);
        return schema; // Return original if can't resolve
      }
    }

    if (resolved && typeof resolved === 'object') {
      // Recursively resolve refs in the resolved schema
      return resolveRefs(resolved, openApiSpec);
    }

    return schema;
  }

  // Handle allOf (merge schemas)
  if (Array.isArray(schema.allOf)) {
    const merged = {};
    for (const subSchema of schema.allOf) {
      if (typeof subSchema === 'object' && subSchema !== null) {
        const resolved = resolveRefs(subSchema, openApiSpec);
        Object.assign(merged, resolved);
      }
    }
    // Keep other properties from the original schema
    const { allOf, ...rest } = schema;
    return { ...merged, ...rest };
  }

  // Handle oneOf/anyOf (keep as union)
  if (Array.isArray(schema.oneOf)) {
    return {
      ...schema,
      oneOf: schema.oneOf.map((subSchema) =>
        typeof subSchema === 'object' && subSchema !== null
          ? resolveRefs(subSchema, openApiSpec)
          : subSchema
      ),
    };
  }

  if (Array.isArray(schema.anyOf)) {
    return {
      ...schema,
      anyOf: schema.anyOf.map((subSchema) =>
        typeof subSchema === 'object' && subSchema !== null
          ? resolveRefs(subSchema, openApiSpec)
          : subSchema
      ),
    };
  }

  // Handle object properties
  if (schema.type === 'object' && schema.properties && typeof schema.properties === 'object') {
    const resolvedProperties = {};
    for (const [key, value] of Object.entries(schema.properties)) {
      if (typeof value === 'object' && value !== null) {
        resolvedProperties[key] = resolveRefs(value, openApiSpec);
      } else {
        resolvedProperties[key] = value;
      }
    }
    return {
      ...schema,
      properties: resolvedProperties,
    };
  }

  // Handle array items
  if (schema.type === 'array' && schema.items && typeof schema.items === 'object') {
    return {
      ...schema,
      items: resolveRefs(schema.items, openApiSpec),
    };
  }

  // Return schema as-is if no refs to resolve
  return schema;
}

// Extract Recipe schema
const recipeSchema = openApiSpec.components?.schemas?.Recipe;

if (!recipeSchema) {
  console.error('Error: Recipe schema not found in OpenAPI specification');
  process.exit(1);
}

// Resolve all \$refs in the schema
const resolvedSchema = resolveRefs(recipeSchema, openApiSpec);

// Convert OpenAPI schema to JSON Schema format
const jsonSchema = {
  '\$schema': 'http://json-schema.org/draft-07/schema#',
  ...resolvedSchema,
  title: resolvedSchema.title || 'Recipe',
  description: resolvedSchema.description || 'A Recipe represents a personalized, user-generated agent configuration that defines specific behaviors and capabilities within the Goose system.',
};

// Output the resolved schema
console.log(JSON.stringify(jsonSchema, null, 2));
"
