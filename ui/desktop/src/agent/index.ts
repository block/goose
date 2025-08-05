import { getApiUrl } from '../config';

interface initializeAgentProps {
  model: string;
  provider: string;
}

export async function initializeAgent({ model, provider }: initializeAgentProps) {
  const response = await fetch(getApiUrl('/agent/update_provider'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': await window.electron.getSecretKey(),
    },
    body: JSON.stringify({
      provider: provider.toLowerCase().replace(/ /g, '_'),
      model: model,
    }),
  });

  if (!response.ok) {
    const responseText = await response.text();
    throw new Error(
      `Failed to initialize agent: ${response.status} ${response.statusText} - ${responseText}`
    );
  }

  return response;
}
