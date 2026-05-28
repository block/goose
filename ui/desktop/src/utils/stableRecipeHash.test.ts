import { describe, expect, it } from 'vitest';
import { calculateStableRecipeHash } from './stableRecipeHash';

describe('calculateStableRecipeHash', () => {
  it('is stable when object keys are serialized in a different order', () => {
    const fromSave = {
      version: '1.0.0',
      title: '本地验收工作流',
      description: '用于验证保存后信任',
      instructions: '请按要求回答。',
      parameters: [{ key: 'question', input_type: 'string', requirement: 'required' }],
    };
    const fromList = {
      parameters: [{ requirement: 'required', input_type: 'string', key: 'question' }],
      instructions: '请按要求回答。',
      description: '用于验证保存后信任',
      title: '本地验收工作流',
      version: '1.0.0',
    };

    expect(calculateStableRecipeHash(fromSave)).toBe(calculateStableRecipeHash(fromList));
  });
});
