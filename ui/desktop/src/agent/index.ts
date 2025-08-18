import { updateAgentProvider } from '../api';

interface initializeAgentProps {
  model: string;
  provider: string;
}

export async function initializeAgent({ model, provider }: initializeAgentProps) {
  const response = await updateAgentProvider({
    body: {
      provider: provider.toLowerCase().replace(/ /g, '_'),
      model: model,
    },
  });

  if (response.error) {
    throw new Error(`Failed to initialize agent: ${response.error}`);
  }

  return response;
}
