import ProviderDetails from './interfaces/ProviderDetails';

export interface ProviderRegistry {
  name: string;
  details: ProviderDetails;
}

/**
 * Provider Registry System
 * ========================
 *
 * This registry defines all available providers and how they behave in the UI.
 * It works with a dynamic modal system to create a flexible, extensible architecture
 * for managing provider configurations.
 *
 * How the System Works:
 * --------------------
 *
 * 1. Provider Definition:
 *    Each provider entry in the registry defines its core properties:
 *    - Basic info (id, name, description)
 *    - Parameters needed for configuration
 *    - Optional custom form component
 *    - Action buttons that appear on provider cards
 *
 * 2. Configuration submission:
 *
 *    Modal Content - What form appears in the configuration modal
 *       - A single modal component exists in the app
 *       - Content changes dynamically based on the provider being configured
 *       - If provider has CustomForm property, that component is rendered
 *       - Otherwise, DefaultProviderForm renders based on parameters array
 *
 * 3. Modal Flow:
 *    - User clicks Configure button on a provider card
 *    - Button handler calls openModal() with the provider object
 *    - Modal context stores the current provider and opens the modal
 *    - ProviderConfigModal checks for CustomForm on the current provider
 *    - Appropriate form is rendered with provider data passed as props
 *
 * Adding a New Provider:
 * ---------------------
 *
 * For a standard provider with simple configuration:
 * - Define parameters array with all required fields and any defaults that should be supplied
 * - No need to specify a CustomForm
 *
 * For a provider needing custom configuration:
 * - Define parameters array (if needed, otherwise leave as an empty list)
 * - Create a custom form component and assign to CustomForm property
 *
 * This architecture centralizes provider definitions while allowing
 * flexibility for special cases, keeping the codebase maintainable.
 */

export const PROVIDER_REGISTRY: ProviderRegistry[] = [
  {
    name: 'OpenAI',
    details: {
      id: 'openai',
      name: 'OpenAI',
      description: 'Access GPT-4, GPT-3.5 Turbo, and other OpenAI models',
      parameters: [
        {
          name: 'OPENAI_API_KEY',
          is_secret: true,
        },
        {
          name: 'OPENAI_HOST',
          is_secret: false,
          default: 'https://api.openai.com',
        },
        {
          name: 'OPENAI_BASE_PATH',
          is_secret: false,
          default: 'v1/chat/completions',
        },
      ],
    },
  },
  {
    name: 'Anthropic',
    details: {
      id: 'anthropic',
      name: 'Anthropic',
      description: 'Access Claude and other Anthropic models',
      parameters: [
        {
          name: 'ANTHROPIC_API_KEY',
          is_secret: true,
        },
        {
          name: 'ANTHROPIC_HOST',
          is_secret: false,
          default: 'https://api.anthropic.com',
        },
      ],
    },
  },
  {
    name: 'xAI',
    details: {
      id: 'xai',
      name: 'xAI',
      description: 'Access Grok models from xAI, including reasoning and multimodal capabilities',
      parameters: [
        {
          name: 'XAI_API_KEY',
          is_secret: true,
        },
        {
          name: 'XAI_HOST',
          is_secret: false,
          default: 'https://api.x.ai/v1',
        },
      ],
    },
  },
  {
    name: 'Google',
    details: {
      id: 'google',
      name: 'Google',
      description: 'Access Gemini and other Google AI models',
      parameters: [
        {
          name: 'GOOGLE_API_KEY',
          is_secret: true,
        },
      ],
    },
  },
  {
    name: 'Groq',
    details: {
      id: 'groq',
      name: 'Groq',
      description: 'Access Mixtral and other Groq-hosted models',
      parameters: [
        {
          name: 'GROQ_API_KEY',
          is_secret: true,
        },
      ],
    },
  },
  {
    name: 'Databricks',
    details: {
      id: 'databricks',
      name: 'Databricks',
      description: 'Access models hosted on your Databricks instance',
      parameters: [
        {
          name: 'DATABRICKS_HOST',
          is_secret: false,
        },
      ],
    },
  },
  {
    name: 'OpenRouter',
    details: {
      id: 'openrouter',
      name: 'OpenRouter',
      description: 'Access a variety of AI models through OpenRouter',
      parameters: [
        {
          name: 'OPENROUTER_API_KEY',
          is_secret: true,
        },
      ],
    },
  },
  {
    name: 'Ollama',
    details: {
      id: 'ollama',
      name: 'Ollama',
      description: 'Run and use open-source models locally',
      parameters: [
        {
          name: 'OLLAMA_HOST',
          is_secret: false,
          default: 'localhost',
        },
      ],
    },
  },
  {
    name: 'Azure OpenAI',
    details: {
      id: 'azure_openai',
      name: 'Azure OpenAI',
      description:
        'Access Azure OpenAI models using API key or Azure credentials. If no API key is provided, Azure credential chain will be used.',
      parameters: [
        {
          name: 'AZURE_OPENAI_ENDPOINT',
          is_secret: false,
        },
        {
          name: 'AZURE_OPENAI_DEPLOYMENT_NAME',
          is_secret: false,
        },
        {
          name: 'AZURE_OPENAI_API_VERSION',
          is_secret: false,
          default: '2024-10-21',
        },
        {
          name: 'AZURE_OPENAI_API_KEY',
          is_secret: true,
          default: '',
        },
      ],
    },
  },
  {
    name: 'GCP Vertex AI',
    details: {
      id: 'gcp_vertex_ai',
      name: 'GCP Vertex AI',
      description: 'GCP Vertex AI models',
      parameters: [
        {
          name: 'GCP_PROJECT_ID',
          is_secret: false,
        },
        {
          name: 'GCP_LOCATION',
          is_secret: false,
          default: 'us-central1',
        },
      ],
    },
  },
  {
    name: 'Snowflake',
    details: {
      id: 'snowflake',
      name: 'Snowflake',
      description: 'Access the latest models using Snowflake Cortex services.',
      parameters: [
        {
          name: 'SNOWFLAKE_HOST',
          is_secret: false,
        },
        {
          name: 'SNOWFLAKE_TOKEN',
          is_secret: true,
        },
      ],
    },
  },
];
