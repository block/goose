import { getApiUrl, getSecretKey } from '../config';

export async function initializeAgent(model: string, provider: string) {
  console.log('fetching...', provider, model);
  const response = await fetch(getApiUrl('/agent'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      provider: provider.toLowerCase().replace(/ /g, '_'),
      model: model,
    }),
  });
  return response;
}
