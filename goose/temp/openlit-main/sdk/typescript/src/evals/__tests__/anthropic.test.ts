import { llmResponseAnthropic } from '../llm/anthropic';

describe('llm', () => {
  it('llmResponseAnthropic throws if no apiKey', async () => {
    await expect(llmResponseAnthropic({ prompt: 'p', model: 'm', apiKey: undefined })).rejects.toThrow();
  });
});
