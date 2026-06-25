import { zRecipeDto } from '@aaif/goose-sdk';
import { zodToJsonSchema } from 'zod-to-json-schema';

type JsonSchema = Record<string, unknown>;

const recipeDescription =
  'A Workflow represents a personalized, user-generated agent configuration that defines specific behaviors and capabilities within ApeMind Agent.';

let recipeJsonSchema: JsonSchema | null = null;

export function getRecipeJsonSchema(): JsonSchema {
  if (!recipeJsonSchema) {
    recipeJsonSchema = {
      ...(zodToJsonSchema(zRecipeDto, { $refStrategy: 'none' }) as JsonSchema),
      title: 'Workflow',
      description: recipeDescription,
    };
  }

  return recipeJsonSchema;
}
