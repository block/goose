import { llmProviders } from '../llm/providers';

describe('llmProviders', () => {
  it('should have openai and anthropic as keys', () => {
    expect(Object.keys(llmProviders)).toEqual(
      expect.arrayContaining(['openai', 'anthropic'])
    );
  });

  it('should return a function for each provider', () => {
    Object.values(llmProviders).forEach(fn => {
      expect(typeof fn).toBe('function');
    });
  });
});
