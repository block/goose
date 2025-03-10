import { getApiUrl, getSecretKey } from '../config';
import { loadAndAddStoredExtensions } from '../extensions';
import { GOOSE_PROVIDER, GOOSE_MODEL } from '../env_vars';
import { Model } from '../components/settings/models/ModelContext';
import { gooseModels } from '../components/settings/models/GooseModels';

export function getStoredProvider(config: any): string | null {
  return config.GOOSE_PROVIDER || localStorage.getItem(GOOSE_PROVIDER);
}

export function getStoredModel(): string | null {
  const storedModel = localStorage.getItem('GOOSE_MODEL'); // Adjust key name if necessary

  if (storedModel) {
    try {
      const modelInfo: Model = JSON.parse(storedModel);
      return modelInfo.name || null; // Return name if it exists, otherwise null
    } catch (error) {
      console.error('Error parsing GOOSE_MODEL from local storage:', error);
      return null; // Return null if parsing fails
    }
  }

  return null; // Return null if storedModel is not found
}

export interface Provider {
  id: string; // Lowercase key (e.g., "openai")
  name: string; // Provider name (e.g., "OpenAI")
  description: string; // Description of the provider
  models: string[]; // List of supported models
  requiredKeys: string[]; // List of required keys
}

const addAgent = async (provider: string, model: string) => {
  const response = await fetch(getApiUrl('/agent'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({ provider: provider, model: model }),
  });

  if (!response.ok) {
    throw new Error(`Failed to add agent: ${response.statusText}`);
  }

  return response;
};

// Desktop-specific system prompt extension
const desktopPrompt = `You are being accessed through the Goose Desktop application.

The user is interacting with you through a graphical user interface with the following features:
- A chat interface where messages are displayed in a conversation format
- Support for markdown formatting in your responses
- Support for code blocks with syntax highlighting
- Tool use messages are included in the chat but outputs may need to be expanded

The user can add extensions for you through the "Settings" page, which is available in the menu
on the top right of the window. There is a section on that page for extensions, and it links to
the registry.

Some extensions are builtin, such as Developer and Memory, while
3rd party extensions can be browsed at https://block.github.io/goose/v1/extensions/.
`;

export const initializeSystem = async (provider: string, model: string) => {
  try {
    console.log('initializing agent with provider', provider, 'model', model);
    await addAgent(provider.toLowerCase().replace(/ /g, '_'), model);

    // Sync the model state with React
    const syncedModel = syncModelWithAgent(provider, model);
    console.log('Model synced with React state:', syncedModel);

    // Extend the system prompt with desktop-specific information
    const response = await fetch(getApiUrl('/agent/prompt'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({ extension: desktopPrompt }),
    });

    if (!response.ok) {
      console.warn(`Failed to extend system prompt: ${response.statusText}`);
    } else {
      console.log('Extended system prompt with desktop-specific information');
    }

    loadAndAddStoredExtensions().catch((error) => {
      console.error('Failed to load and add stored extension configs:', error);
    });
  } catch (error) {
    console.error('Failed to initialize agent:', error);
    throw error;
  }
};

// This function ensures the agent initialization values and React model state stay in sync
const syncModelWithAgent = (provider: string, modelName: string): Model | null => {
  console.log('Syncing model state with agent:', { provider, modelName });

  // First, try to find a matching model in our predefined list
  let matchingModel = gooseModels.find(
    (m) => m.name === modelName && m.provider.toLowerCase() === provider.toLowerCase()
  );

  // If no match by exact name and provider, try just by provider
  if (!matchingModel) {
    matchingModel = gooseModels.find((m) => m.provider.toLowerCase() === provider.toLowerCase());

    if (matchingModel) {
      console.log('Found model by provider only:', matchingModel);
    }
  }

  // If still no match, create a custom model
  if (!matchingModel) {
    console.log('No matching model found, creating custom model');
    matchingModel = {
      id: Date.now(),
      name: modelName,
      provider: provider,
      alias: `${provider} - ${modelName}`,
    };
  }

  // Update localStorage with the model
  if (matchingModel) {
    localStorage.setItem(GOOSE_PROVIDER, matchingModel.provider.toLowerCase());
    localStorage.setItem(GOOSE_MODEL, JSON.stringify(matchingModel));

    return matchingModel;
  }

  return null;
};
