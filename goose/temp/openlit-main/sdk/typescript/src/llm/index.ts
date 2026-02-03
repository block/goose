import { GuardResult } from '../guard/types';

export async function llmResponse(provider: string, prompt: string, model?: string, baseUrl?: string, apiKey?: string): Promise<string> {
  if (provider.toLowerCase() === 'openai') {
    const { llmResponseOpenAI } = await import('./openai');
    return llmResponseOpenAI({ prompt, model, baseUrl, apiKey });
  } else if (provider.toLowerCase() === 'anthropic') {
    const { llmResponseAnthropic } = await import('./anthropic');
    return llmResponseAnthropic({ prompt, model, apiKey });
  } else {
    throw new Error(`Unsupported provider: ${provider}`);
  }
}

export function parseLlmResponse(response: string | object): GuardResult {
  try {
    let data: Record<string, unknown>;
    if (typeof response === 'string') {
      data = JSON.parse(response);
    } else if (typeof response === 'object') {
      data = response as Record<string, unknown>;
    } else {
      throw new Error('Response must be a JSON string or an object.');
    }
    let verdict: 'none' | 'yes' | 'no' = 'none';
    if (typeof data.verdict === 'string') {
      if (data.verdict === 'yes' || data.verdict === 'no') {
        verdict = data.verdict;
      } else {
        // eslint-disable-next-line no-console
        console.warn(
          `Unexpected verdict value in LLM response: "${data.verdict}". Coercing to 'none'.`
        );
      }
    }
    return {
      score: typeof data.score === 'number' ? data.score : 0,
      verdict,
      guard: typeof data.guard === 'string' ? (data.guard as string) : 'none',
      classification: typeof data.classification === 'string' ? (data.classification as string) : 'none',
      explanation: typeof data.explanation === 'string' ? (data.explanation as string) : 'none',
    };
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error('Error parsing LLM response:', e);
    return {
      score: 0,
      verdict: 'none',
      guard: 'none',
      classification: 'none',
      explanation: 'none'
    };
  }
}
