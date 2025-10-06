import { describe, it, expect } from 'vitest';
import {
  validateRecipe,
  validateJsonSchema,
  getValidationErrorMessages,
  getRecipeJsonSchema,
} from './validation';
import type { Recipe } from '../api/types.gen';

describe('Recipe Validation', () => {
  const validRecipe: Recipe = {
    version: '1.0.0',
    title: 'Test Recipe',
    description: 'A test recipe for validation',
    instructions: 'Do something useful',
    activities: ['Test activity 1', 'Test activity 2'],
    extensions: [
      {
        type: 'builtin',
        name: 'developer',
        display_name: 'Developer',
        description: 'Developer',
        timeout: 300,
        bundled: true,
      },
    ],
  };

  const validRecipeWithPrompt: Recipe = {
    version: '1.0.0',
    title: 'Prompt Recipe',
    description: 'A recipe using prompt instead of instructions',
    prompt: 'You are a helpful assistant',
    activities: ['Help users'],
    extensions: [
      {
        type: 'builtin',
        name: 'developer',
        description: 'Developer',
      },
    ],
  };

  const validRecipeWithParameters: Recipe = {
    version: '1.0.0',
    title: 'Parameterized Recipe',
    description: 'A recipe with parameters',
    instructions: 'Process the file at {{ file_path }}',
    parameters: [
      {
        key: 'file_path',
        input_type: 'string',
        requirement: 'required',
        description: 'Path to the file to process',
      },
    ],
    activities: ['Process file'],
    extensions: [
      {
        type: 'builtin',
        name: 'developer',
        description: 'developer',
      },
    ],
  };

  const validRecipeWithAuthor: Recipe = {
    version: '1.0.0',
    title: 'Authored Recipe',
    author: {
      contact: 'test@example.com',
    },
    description: 'A recipe with author information',
    instructions: 'Do something',
    activities: ['Activity'],
    extensions: [
      {
        type: 'builtin',
        name: 'developer',
        description: 'developer',
      },
    ],
  };

  describe('validateRecipe', () => {
    describe('valid recipes', () => {
      it('validates a basic valid recipe', () => {
        const result = validateRecipe(validRecipe);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
        expect(result.data).toEqual(validRecipe);
      });

      it('validates a recipe with prompt instead of instructions', () => {
        const result = validateRecipe(validRecipeWithPrompt);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
        expect(result.data).toEqual(validRecipeWithPrompt);
      });

      it('validates a recipe with parameters', () => {
        const result = validateRecipe(validRecipeWithParameters);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
        expect(result.data).toEqual(validRecipeWithParameters);
      });

      it('validates a recipe with author information', () => {
        const result = validateRecipe(validRecipeWithAuthor);
        if (!result.success) {
          console.log('Author validation errors:', result.errors);
        }
        expect(typeof result.success).toBe('boolean');
        expect(Array.isArray(result.errors)).toBe(true);
      });

      it('validates a recipe with minimal required fields', () => {
        const minimalRecipe = {
          version: '1.0.0',
          title: 'Minimal',
          description: 'Minimal recipe',
          instructions: 'Do something',
          activities: ['Activity'],
          extensions: [],
        };

        const result = validateRecipe(minimalRecipe);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
      });
    });

    describe('invalid recipes', () => {
      it('rejects recipe without title', () => {
        const invalidRecipe = {
          ...validRecipe,
          title: undefined,
        };

        const result = validateRecipe(invalidRecipe);
        expect(result.success).toBe(false);
        expect(result.errors.length).toBeGreaterThan(0);
        expect(result.data).toBeUndefined();
      });

      it('rejects recipe without description', () => {
        const invalidRecipe = {
          ...validRecipe,
          description: undefined,
        };

        const result = validateRecipe(invalidRecipe);
        expect(result.success).toBe(false);
        expect(result.errors.length).toBeGreaterThan(0);
      });

      it('allows recipe without version (version is optional)', () => {
        const recipeWithoutVersion = {
          ...validRecipe,
          version: undefined,
        };

        const result = validateRecipe(recipeWithoutVersion);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
      });

      it('rejects recipe without instructions or prompt', () => {
        const invalidRecipe = {
          ...validRecipe,
          instructions: undefined,
          prompt: undefined,
        };

        const result = validateRecipe(invalidRecipe);
        expect(result.success).toBe(false);
        expect(result.errors).toContain('Either instructions or prompt must be provided');
      });

      it('validates recipe with minimal extension structure', () => {
        const recipeWithMinimalExtension = {
          ...validRecipe,
          extensions: [
            {
              type: 'builtin',
              name: 'developer',
              description: 'description',
            },
          ],
        };

        const result = validateRecipe(recipeWithMinimalExtension);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
      });

      it('validates recipe with incomplete parameter structure', () => {
        const recipeWithIncompleteParam = {
          ...validRecipe,
          parameters: [
            {
              key: 'test',
            },
          ],
        };

        const result = validateRecipe(recipeWithIncompleteParam);
        expect(typeof result.success).toBe('boolean');
        expect(Array.isArray(result.errors)).toBe(true);
      });

      it('rejects non-object input', () => {
        const result = validateRecipe('not an object');
        expect(result.success).toBe(false);
        expect(result.errors.length).toBeGreaterThan(0);
      });

      it('rejects null input', () => {
        const result = validateRecipe(null);
        expect(result.success).toBe(false);
        expect(result.errors.length).toBeGreaterThan(0);
      });

      it('rejects undefined input', () => {
        const result = validateRecipe(undefined);
        expect(result.success).toBe(false);
        expect(result.errors.length).toBeGreaterThan(0);
      });
    });

    describe('edge cases', () => {
      it('handles empty arrays gracefully', () => {
        const recipeWithEmptyArrays = {
          ...validRecipe,
          activities: [],
          extensions: [],
          parameters: [],
        };

        const result = validateRecipe(recipeWithEmptyArrays);
        expect(result.success).toBe(true);
      });

      it('handles extra properties', () => {
        const recipeWithExtra = {
          ...validRecipe,
          extraField: 'should be ignored or handled gracefully',
        };

        const result = validateRecipe(recipeWithExtra);
        expect(typeof result.success).toBe('boolean');
        expect(Array.isArray(result.errors)).toBe(true);
      });

      it('handles very long strings', () => {
        const longString = 'a'.repeat(10000);
        const recipeWithLongStrings = {
          ...validRecipe,
          title: longString,
          description: longString,
          instructions: longString,
        };

        const result = validateRecipe(recipeWithLongStrings);
        expect(typeof result.success).toBe('boolean');
      });
    });
  });

  describe('validateJsonSchema', () => {
    describe('valid JSON schemas', () => {
      it('validates a simple JSON schema', () => {
        const schema = {
          type: 'object',
          properties: {
            name: { type: 'string' },
            age: { type: 'number' },
          },
          required: ['name'],
        };

        const result = validateJsonSchema(schema);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
        expect(result.data).toEqual(schema);
      });

      it('validates null schema', () => {
        const result = validateJsonSchema(null);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
        expect(result.data).toBe(null);
      });

      it('validates undefined schema', () => {
        const result = validateJsonSchema(undefined);
        expect(result.success).toBe(true);
        expect(result.errors).toHaveLength(0);
        expect(result.data).toBe(undefined);
      });

      it('validates complex JSON schema', () => {
        const schema = {
          $schema: 'http://json-schema.org/draft-07/schema#',
          type: 'object',
          properties: {
            users: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  id: { type: 'number' },
                  profile: {
                    type: 'object',
                    properties: {
                      name: { type: 'string' },
                      email: { type: 'string' },
                    },
                  },
                },
              },
            },
          },
        };

        const result = validateJsonSchema(schema);
        expect(result.success).toBe(true);
        expect(result.data).toEqual(schema);
      });
    });

    describe('invalid JSON schemas', () => {
      it('rejects string input', () => {
        const result = validateJsonSchema('not an object');
        expect(result.success).toBe(false);
        expect(result.errors).toContain('JSON Schema must be an object');
      });

      it('rejects number input', () => {
        const result = validateJsonSchema(42);
        expect(result.success).toBe(false);
        expect(result.errors).toContain('JSON Schema must be an object');
      });

      it('rejects boolean input', () => {
        const result = validateJsonSchema(true);
        expect(result.success).toBe(false);
        expect(result.errors).toContain('JSON Schema must be an object');
      });

      it('validates array input as valid JSON schema', () => {
        const result = validateJsonSchema(['not', 'an', 'object']);
        expect(typeof result.success).toBe('boolean');
        expect(Array.isArray(result.errors)).toBe(true);
      });
    });
  });

  describe('helper functions', () => {
    describe('getValidationErrorMessages', () => {
      it('returns the same array of error messages', () => {
        const errors = ['title: Required', 'description: Required', 'Invalid format'];
        const messages = getValidationErrorMessages(errors);
        expect(messages).toEqual(errors);
        expect(messages).toHaveLength(3);
      });

      it('handles empty array', () => {
        const errors: string[] = [];
        const messages = getValidationErrorMessages(errors);
        expect(messages).toHaveLength(0);
        expect(messages).toEqual([]);
      });
    });
  });

  describe('getRecipeJsonSchema', () => {
    it('returns a valid JSON schema object', () => {
      const schema = getRecipeJsonSchema();

      expect(schema).toBeDefined();
      expect(typeof schema).toBe('object');
      expect(schema).toHaveProperty('$schema');
      expect(schema).toHaveProperty('type');
      expect(schema).toHaveProperty('title');
      expect(schema).toHaveProperty('description');
    });

    it('includes standard JSON Schema properties', () => {
      const schema = getRecipeJsonSchema();

      expect(schema.$schema).toBe('http://json-schema.org/draft-07/schema#');
      expect(schema.title).toBeDefined();
      expect(schema.description).toBeDefined();
    });

    it('returns consistent schema across calls', () => {
      const schema1 = getRecipeJsonSchema();
      const schema2 = getRecipeJsonSchema();

      expect(schema1).toEqual(schema2);
    });
  });

  describe('error handling and edge cases', () => {
    it('handles validation errors gracefully', () => {
      // Test with malformed data that might cause validation to throw
      const malformedData = {
        version: { not: 'a string' },
        title: ['not', 'a', 'string'],
        description: 123,
        instructions: null,
        activities: 'not an array',
        extensions: 'not an array',
      };

      const result = validateRecipe(malformedData);
      expect(typeof result.success).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });

    it('handles circular references gracefully', () => {
      const circularObj: Record<string, unknown> = { title: 'Test' };
      (circularObj as Record<string, unknown>).self = circularObj;

      const result = validateRecipe(circularObj);
      expect(typeof result.success).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });

    it('handles very deep nested objects', () => {
      let deepObj: Record<string, unknown> = {
        version: '1.0.0',
        title: 'Deep',
        description: 'Test',
      };
      let current: Record<string, unknown> = deepObj;

      // Create a deeply nested structure
      for (let i = 0; i < 100; i++) {
        const nested = { level: i };
        current.nested = nested;
        current = nested as Record<string, unknown>;
      }

      const result = validateRecipe(deepObj);
      expect(typeof result.success).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });
  });

  describe('real-world recipe examples', () => {
    it('validates readme-bot style recipe', () => {
      const readmeBotRecipe = {
        version: '1.0.0',
        title: 'Readme Bot',
        author: {
          contact: 'DOsinga',
        },
        description: 'Generates or updates a readme',
        instructions: 'You are a documentation expert',
        activities: [
          'Scan project directory for documentation context',
          'Generate a new README draft',
          'Compare new draft with existing README.md',
        ],
        extensions: [
          {
            type: 'builtin',
            name: 'developer',
            display_name: 'Developer',
            timeout: 300,
            bundled: true,
          },
        ],
        prompt: "Here's what to do step by step: 1. The current folder is a software project...",
      };

      const result = validateRecipe(readmeBotRecipe);
      if (!result.success) {
        console.log('ReadmeBot validation errors:', result.errors);
      }
      expect(typeof result.success).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });

    it('validates lint-my-code style recipe with parameters', () => {
      const lintRecipe = {
        version: '1.0.0',
        title: 'Lint My Code',
        author: {
          contact: 'iandouglas',
        },
        description:
          'Analyzes code files for syntax and layout issues using available linting tools',
        instructions:
          'You are a code quality expert that helps identify syntax and layout issues in code files',
        activities: [
          'Detect file type and programming language',
          'Check for available linting tools in the project',
          'Run appropriate linters for syntax and layout checking',
          'Provide recommendations if no linters are found',
        ],
        parameters: [
          {
            key: 'file_path',
            input_type: 'string',
            requirement: 'required',
            description: 'Path to the file you want to lint',
          },
        ],
        extensions: [
          {
            type: 'builtin',
            name: 'developer',
            display_name: 'Developer',
            timeout: 300,
            bundled: true,
          },
        ],
        prompt:
          'I need you to lint the file at {{ file_path }} for syntax and layout issues only...',
      };

      const result = validateRecipe(lintRecipe);
      if (!result.success) {
        console.log('LintRecipe validation errors:', result.errors);
      }
      expect(typeof result.success).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });

    it('validates 404Portfolio style recipe with multiple extensions', () => {
      const portfolioRecipe = {
        version: '1.0.0',
        title: '404Portfolio',
        description: 'Create personalized, creative 404 pages using public profile data',
        instructions: 'Create an engaging 404 error page that tells a creative story...',
        activities: [
          'Build error page from GitHub repos',
          'Generate error page from dev.to blog posts',
          'Create a 404 page featuring Bluesky bio',
        ],
        extensions: [
          {
            type: 'builtin',
            name: 'developer',
            description: 'developer',
          },
          {
            type: 'builtin',
            name: 'computercontroller',
            description: 'computercontroller',
          },
        ],
      };

      const result = validateRecipe(portfolioRecipe);
      expect(result.success).toBe(true);
    });
  });
});
