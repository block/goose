import crypto from 'crypto';

function normalizeForHash(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(normalizeForHash);
  }

  if (value && typeof value === 'object') {
    return Object.keys(value as Record<string, unknown>)
      .sort()
      .reduce<Record<string, unknown>>((acc, key) => {
        const item = (value as Record<string, unknown>)[key];
        if (item !== undefined) {
          acc[key] = normalizeForHash(item);
        }
        return acc;
      }, {});
  }

  return value;
}

export function calculateStableRecipeHash(recipe: unknown): string {
  const hash = crypto.createHash('sha256');
  hash.update(JSON.stringify(normalizeForHash(recipe)));
  return hash.digest('hex');
}
