import { describe, it, expect } from 'vitest';
import { getRecipeJsonSchema } from './validation';

describe('Recipe Validation', () => {
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

    it('documents only ACP-supported recipe extension variants', () => {
      const schemaJson = JSON.stringify(getRecipeJsonSchema());

      expect(schemaJson).toContain('builtin');
      expect(schemaJson).toContain('platform');
      expect(schemaJson).toContain('stdio');
      expect(schemaJson).toContain('streamable_http');
      expect(schemaJson).not.toContain('sse');
      expect(schemaJson).not.toContain('frontend');
      expect(schemaJson).not.toContain('inline_python');
      expect(schemaJson).not.toContain('available_tools');
    });
  });
});
