import { getApiUrl, getSecretKey } from '../config';

interface initializeAgentProps {
  model: string;
  provider: string;
}

export async function initializeAgent({ model, provider }: initializeAgentProps) {
  const response = await fetch(getApiUrl('/agent/update_provider'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      provider: provider.toLowerCase().replace(/ /g, '_'),
      model,
    }),
  });
  if (!response.ok) {
    console.error('Failed to initialize agent with provider:', response.statusText);
    throw new Error(`Failed to initialize agent: ${response.statusText}`);
  }
}
