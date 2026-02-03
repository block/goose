import { llmResponseOpenAI } from '../llm/openai';

describe('llm', () => {
  it('llmResponseOpenAI throws if no apiKey', async () => {
    await expect(llmResponseOpenAI({ prompt: 'p', model: 'm', apiKey: undefined, baseUrl: undefined })).rejects.toThrow();
  });
});
