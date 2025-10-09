/**
 * OpenAPI-based validation utilities for Recipe objects.
 *
 * This module uses the generated OpenAPI specification directly for validation,
 * ensuring automatic synchronization with backend schema changes.
 * Zod schemas are generated dynamically from the OpenAPI spec.
 */

// Import the OpenAPI spec directly for schema extraction
import openApiSpec from '../../openapi.json';

// Extract the Recipe schema from OpenAPI components
function getRecipeSchema() {
  return openApiSpec.components?.schemas?.Recipe;
}

/**
 * Resolves $ref references in OpenAPI schemas by expanding them with the actual schema definitions
 */
function resolveRefs(
  schema: Record<string, unknown>,
  openApiSpec: Record<string, unknown>
): Record<string, unknown> {
  if (!schema || typeof schema !== 'object') {
    return schema;
  }

  // Handle $ref
  if (typeof schema.$ref === 'string') {
    const refPath = schema.$ref.replace('#/', '').split('/');
    let resolved: unknown = openApiSpec;

    for (const segment of refPath) {
      if (resolved && typeof resolved === 'object' && segment in resolved) {
        resolved = (resolved as Record<string, unknown>)[segment];
      } else {
        console.warn(`Could not resolve $ref: ${schema.$ref}`);
        return schema; // Return original if can't resolve
      }
    }

    if (resolved && typeof resolved === 'object') {
      // Recursively resolve refs in the resolved schema
      return resolveRefs(resolved as Record<string, unknown>, openApiSpec);
    }

    return schema;
  }

  // Handle allOf (merge schemas)
  if (Array.isArray(schema.allOf)) {
    const merged: Record<string, unknown> = {};
    for (const subSchema of schema.allOf) {
      if (typeof subSchema === 'object' && subSchema !== null) {
        const resolved = resolveRefs(subSchema as Record<string, unknown>, openApiSpec);
        Object.assign(merged, resolved);
      }
    }
    // Keep other properties from the original schema
    const { allOf: _allOf, ...rest } = schema;
    return { ...merged, ...rest };
  }

  // Handle oneOf/anyOf (keep as union)
  if (Array.isArray(schema.oneOf)) {
    return {
      ...schema,
      oneOf: schema.oneOf.map((subSchema) =>
        typeof subSchema === 'object' && subSchema !== null
          ? resolveRefs(subSchema as Record<string, unknown>, openApiSpec)
          : subSchema
      ),
    };
  }

  if (Array.isArray(schema.anyOf)) {
    return {
      ...schema,
      anyOf: schema.anyOf.map((subSchema) =>
        typeof subSchema === 'object' && subSchema !== null
          ? resolveRefs(subSchema as Record<string, unknown>, openApiSpec)
          : subSchema
      ),
    };
  }

  // Handle object properties
  if (schema.type === 'object' && schema.properties && typeof schema.properties === 'object') {
    const resolvedProperties: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(schema.properties)) {
      if (typeof value === 'object' && value !== null) {
        resolvedProperties[key] = resolveRefs(value as Record<string, unknown>, openApiSpec);
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
      items: resolveRefs(schema.items as Record<string, unknown>, openApiSpec),
    };
  }

  // Return schema as-is if no refs to resolve
  return schema;
}

/**
 * Returns a JSON schema representation derived directly from the OpenAPI specification.
 * This schema is used for documentation in form help text.
 *
 * This function extracts the Recipe schema from the OpenAPI spec and converts it
 * to a standard JSON Schema format, ensuring it stays in sync with backend changes.
 *
 * All $ref references are automatically resolved and expanded.
 */
export function getRecipeJsonSchema() {
  const recipeSchema = getRecipeSchema();

  if (!recipeSchema) {
    // Fallback minimal schema if OpenAPI schema is not available
    return {
      $schema: 'http://json-schema.org/draft-07/schema#',
      type: 'object',
      title: 'Recipe',
      description: 'Recipe schema not found in OpenAPI specification',
      required: ['title', 'description'],
      properties: {
        title: { type: 'string' },
        description: { type: 'string' },
      },
    };
  }

  // Resolve all $refs in the schema
  const resolvedSchema = resolveRefs(
    recipeSchema as Record<string, unknown>,
    openApiSpec as Record<string, unknown>
  );

  // Convert OpenAPI schema to JSON Schema format
  return {
    $schema: 'http://json-schema.org/draft-07/schema#',
    ...resolvedSchema,
    title: resolvedSchema.title || 'Recipe',
    description:
      resolvedSchema.description ||
      'A Recipe represents a personalized, user-generated agent configuration that defines specific behaviors and capabilities within the Goose system.',
  };
}
