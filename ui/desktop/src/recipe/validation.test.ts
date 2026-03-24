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
  });
});
