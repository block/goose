import { getApiUrl } from '../config';

/**
 * Utility to ask the LLM any question to clarify without wider context.
 */
export async function askAi(promptTemplates: string[]) {
  const responses = await Promise.all(
    promptTemplates.map(async (template) => {
      const response = await fetch(getApiUrl('/ask'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ prompt: template }),
      });

      if (!response.ok) {
        throw new Error('Failed to get response');
      }

      const data = await response.json();

      return data.response;
    })
  );

  return responses;
}