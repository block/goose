import { zRecipeDto } from '@aaif/goose-sdk';
import { zodToJsonSchema } from 'zod-to-json-schema';

type JsonSchema = Record<string, unknown>;

const recipeDescription =
  'A Recipe represents a reusable agent configuration with instructions, optional prompt, parameters, supported extensions, settings, and subrecipes.';

let recipeJsonSchema: JsonSchema | null = null;

export function getRecipeJsonSchema(): JsonSchema {
  if (!recipeJsonSchema) {
    recipeJsonSchema = {
      ...(zodToJsonSchema(zRecipeDto, { $refStrategy: 'none' }) as JsonSchema),
      title: 'Recipe',
      description: recipeDescription,
    };
  }

  return recipeJsonSchema;
}
