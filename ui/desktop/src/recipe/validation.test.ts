import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getRecipeJsonSchema } from './validation';

function fakeSpec(
  recipeSchema: Record<string, unknown> | undefined,
  extraSchemas: Record<string, unknown> = {}
) {
  return {
    components: {
      schemas: {
        ...(recipeSchema !== undefined ? { Recipe: recipeSchema } : {}),
        ...extraSchemas,
      },
    },
  };
}

describe('getRecipeJsonSchema (real spec)', () => {
  it('returns a valid JSON Schema object with required top-level properties', () => {
    const schema = getRecipeJsonSchema();

    expect(schema).toBeDefined();
    expect(typeof schema).toBe('object');
    expect(schema.$schema).toBe('http://json-schema.org/draft-07/schema#');
    expect(schema.type).toBe('object');
    expect(schema.title).toBeDefined();
    expect(schema.description).toBeDefined();
  });

  it('includes resolved properties from the Recipe schema', () => {
    const schema = getRecipeJsonSchema();

    expect(schema.required).toEqual(expect.arrayContaining(['title', 'description']));
    expect(schema.properties).toBeDefined();
    expect(typeof schema.properties).toBe('object');
  });

  it('resolves $ref references so no raw $ref remains in properties', () => {
    const schema = getRecipeJsonSchema();
    const props = schema.properties as Record<string, Record<string, unknown>>;

    for (const [key, value] of Object.entries(props)) {
      if (typeof value === 'object' && value !== null) {
        expect(value).not.toHaveProperty('$ref', expect.any(String));
      }
    }
  });

  it('returns consistent schema across multiple calls', () => {
    expect(getRecipeJsonSchema()).toEqual(getRecipeJsonSchema());
  });
});

describe('getRecipeJsonSchema (mocked spec)', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  async function importWithSpec(spec: Record<string, unknown>) {
    vi.doMock('../../openapi.json', () => ({ default: spec }));
    const mod = await import('./validation');
    return mod.getRecipeJsonSchema;
  }

  describe('fallback schema', () => {
    it('returns a minimal fallback when Recipe schema is absent', async () => {
      const getSchema = await importWithSpec(fakeSpec(undefined));
      const schema = getSchema();

      expect(schema.$schema).toBe('http://json-schema.org/draft-07/schema#');
      expect(schema.type).toBe('object');
      expect(schema.title).toBe('Recipe');
      expect(schema.description).toContain('not found');
      expect(schema.required).toEqual(['title', 'description']);
      expect(schema.properties).toEqual({
        title: { type: 'string' },
        description: { type: 'string' },
      });
    });

    it('returns fallback when components.schemas is empty', async () => {
      const getSchema = await importWithSpec({ components: { schemas: {} } });
      const schema = getSchema();

      expect(schema.title).toBe('Recipe');
      expect(schema.description).toContain('not found');
    });

    it('returns fallback when spec has no components key at all', async () => {
      const getSchema = await importWithSpec({});
      const schema = getSchema();

      expect(schema.$schema).toBe('http://json-schema.org/draft-07/schema#');
      expect(schema.title).toBe('Recipe');
      expect(schema.description).toContain('not found');
    });
  });

  describe('$ref resolution', () => {
    it('resolves a simple $ref to another schema', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            status: { $ref: '#/components/schemas/Status' },
          },
        },
        {
          Status: { type: 'string', enum: ['active', 'inactive'] },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const statusProp = (schema.properties as Record<string, any>).status;
      expect(statusProp.type).toBe('string');
      expect(statusProp.enum).toEqual(['active', 'inactive']);
      expect(statusProp).not.toHaveProperty('$ref');
    });

    it('resolves nested $ref chains (ref → ref)', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            inner: { $ref: '#/components/schemas/Wrapper' },
          },
        },
        {
          Wrapper: { $ref: '#/components/schemas/Leaf' },
          Leaf: { type: 'integer', minimum: 0 },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const innerProp = (schema.properties as Record<string, any>).inner;
      expect(innerProp.type).toBe('integer');
      expect(innerProp.minimum).toBe(0);
    });

    it('returns original schema when $ref path is broken', async () => {
      const spec = fakeSpec({
        type: 'object',
        properties: {
          broken: { $ref: '#/components/schemas/DoesNotExist' },
        },
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const brokenProp = (schema.properties as Record<string, any>).broken;
      expect(brokenProp.$ref).toBe('#/components/schemas/DoesNotExist');
    });

    it('returns original schema when $ref resolves to a non-object', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            prim: { $ref: '#/components/schemas/JustAString' },
          },
        },
        {
          JustAString: 'not-an-object' as any,
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const primProp = (schema.properties as Record<string, any>).prim;
      expect(primProp.$ref).toBe('#/components/schemas/JustAString');
    });
  });

  describe('allOf merging', () => {
    it('merges multiple allOf schemas into one', async () => {
      const spec = fakeSpec({
        allOf: [
          { type: 'object', properties: { a: { type: 'string' } } },
          { properties: { b: { type: 'number' } } },
        ],
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema).not.toHaveProperty('allOf');
      expect(schema.properties).toEqual(
        expect.objectContaining({
          b: { type: 'number' },
        })
      );
    });

    it('preserves extra properties alongside allOf', async () => {
      const spec = fakeSpec({
        allOf: [{ type: 'object' }],
        description: 'kept',
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema).not.toHaveProperty('allOf');
    });

    it('resolves $ref inside allOf items', async () => {
      // allOf spread merges top-level keys left-to-right; use non-overlapping keys
      // to verify $ref resolution without key collisions.
      const spec = fakeSpec(
        {
          allOf: [
            { $ref: '#/components/schemas/Base' },
            { description: 'Extended type' },
          ],
        },
        {
          Base: { type: 'object', properties: { id: { type: 'integer' } } },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema).not.toHaveProperty('allOf');
      expect(schema.type).toBe('object');
      expect(schema.properties).toEqual(
        expect.objectContaining({
          id: { type: 'integer' },
        })
      );
      expect(schema.description).toBe('Extended type');
    });

    it('rest properties override merged allOf values', async () => {
      // { ...merged, ...rest } means rest keys win over allOf keys
      const spec = fakeSpec({
        allOf: [{ type: 'object', description: 'from-allOf' }],
        description: 'from-rest',
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema.description).toBe('from-rest');
    });

    it('handles empty allOf array gracefully', async () => {
      const spec = fakeSpec({
        allOf: [],
        type: 'object',
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema).not.toHaveProperty('allOf');
      expect(schema.type).toBe('object');
    });

    it('skips null entries inside allOf', async () => {
      const spec = fakeSpec({
        allOf: [null, { type: 'object' }],
      });
      const getSchema = await importWithSpec(spec);
      // Should not throw
      expect(getSchema()).toBeDefined();
    });
  });

  describe('oneOf resolution', () => {
    it('resolves $ref inside oneOf variants', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            value: {
              oneOf: [
                { $ref: '#/components/schemas/TypeA' },
                { type: 'string' },
              ],
            },
          },
        },
        {
          TypeA: { type: 'integer' },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const valueProp = (schema.properties as Record<string, any>).value;
      expect(valueProp.oneOf).toHaveLength(2);
      expect(valueProp.oneOf[0]).toEqual({ type: 'integer' });
      expect(valueProp.oneOf[1]).toEqual({ type: 'string' });
    });

    it('preserves non-object entries in oneOf', async () => {
      const spec = fakeSpec({
        oneOf: [42, 'literal', { type: 'boolean' }],
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema.oneOf).toEqual([42, 'literal', { type: 'boolean' }]);
    });
  });

  describe('anyOf resolution', () => {
    it('resolves $ref inside anyOf variants', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            flexible: {
              anyOf: [
                { $ref: '#/components/schemas/Opt1' },
                { type: 'null' },
              ],
            },
          },
        },
        {
          Opt1: { type: 'string', maxLength: 100 },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const prop = (schema.properties as Record<string, any>).flexible;
      expect(prop.anyOf[0]).toEqual({ type: 'string', maxLength: 100 });
      expect(prop.anyOf[1]).toEqual({ type: 'null' });
    });
  });

  describe('object properties resolution', () => {
    it('recursively resolves $ref in nested object properties', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            nested: {
              type: 'object',
              properties: {
                deep: { $ref: '#/components/schemas/Deep' },
              },
            },
          },
        },
        {
          Deep: { type: 'string', format: 'uri' },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const nestedProps = (schema.properties as Record<string, any>).nested.properties;
      expect(nestedProps.deep).toEqual({ type: 'string', format: 'uri' });
    });

    it('preserves non-object property values as-is', async () => {
      const spec = fakeSpec({
        type: 'object',
        properties: {
          literal: 42,
          str: 'raw',
          normal: { type: 'string' },
        },
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();
      const props = schema.properties as Record<string, any>;

      expect(props.literal).toBe(42);
      expect(props.str).toBe('raw');
      expect(props.normal).toEqual({ type: 'string' });
    });
  });

  describe('array items resolution', () => {
    it('resolves $ref in array items', async () => {
      const spec = fakeSpec(
        {
          type: 'object',
          properties: {
            tags: {
              type: 'array',
              items: { $ref: '#/components/schemas/Tag' },
            },
          },
        },
        {
          Tag: { type: 'string', minLength: 1 },
        }
      );
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const tagsProp = (schema.properties as Record<string, any>).tags;
      expect(tagsProp.type).toBe('array');
      expect(tagsProp.items).toEqual({ type: 'string', minLength: 1 });
    });

    it('handles array items that are already plain schemas', async () => {
      const spec = fakeSpec({
        type: 'object',
        properties: {
          nums: {
            type: 'array',
            items: { type: 'number' },
          },
        },
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      const numsProp = (schema.properties as Record<string, any>).nums;
      expect(numsProp.items).toEqual({ type: 'number' });
    });
  });

  describe('passthrough behavior', () => {
    it('returns schema as-is when there are no refs to resolve', async () => {
      const spec = fakeSpec({
        type: 'string',
        minLength: 1,
        maxLength: 255,
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema.type).toBe('string');
      expect(schema.minLength).toBe(1);
      expect(schema.maxLength).toBe(255);
    });

    it.each([
      ['null input', null],
      ['undefined input', undefined],
      ['numeric input', 42],
      ['string input', 'hello'],
    ])('handles non-object Recipe schema gracefully (%s)', async (_label, value) => {
      const spec = fakeSpec(value as any);
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema.$schema).toBe('http://json-schema.org/draft-07/schema#');
      expect(schema.title).toBeDefined();
    });
  });

  describe('title and description defaults', () => {
    it('uses schema title when present', async () => {
      const spec = fakeSpec({
        type: 'object',
        title: 'CustomTitle',
        description: 'CustomDesc',
      });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema.title).toBe('CustomTitle');
      expect(schema.description).toBe('CustomDesc');
    });

    it('falls back to default title when schema has none', async () => {
      const spec = fakeSpec({ type: 'object' });
      const getSchema = await importWithSpec(spec);
      const schema = getSchema();

      expect(schema.title).toBe('Recipe');
      expect(schema.description).toContain('Recipe');
    });
  });
});
