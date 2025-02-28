import React from 'react';
import ProviderDetails from './interfaces/ProviderDetails';
import DefaultCardButtons from './subcomponents/buttons/DefaultCardButtons';
import ButtonCallbacks from '@/src/components/settings_v2/providers/interfaces/ButtonCallbacks';
import ProviderState from '@/src/components/settings_v2/providers/interfaces/ProviderState';

// Helper function to generate default actions for most providers
const getDefaultButtons = (
  provider: ProviderState,
  callbacks: ButtonCallbacks,
  isOnboardingPage
) => {
  return [
    {
      id: 'default-buttons',
      renderButton: () => (
        <DefaultCardButtons
          provider={provider}
          callbacks={callbacks}
          isOnboardingPage={isOnboardingPage}
        />
      ),
    },
  ];
};

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
 * 2. Two-Level Configuration:
 *    a) Provider Card UI - What buttons appear on each provider card
 *       - Controlled by the provider's getActions() function
 *       - Most providers use default buttons (configure/launch)
 *       - Can be customized for special providers
 *
 *    b) Modal Content - What form appears in the configuration modal
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
 * - Define parameters array with all required fields
 * - Use the default getActions function
 * - No need to specify a CustomForm
 *
 * For a provider needing custom configuration:
 * - Define parameters array (even if just for documentation)
 * - Create a custom form component and assign to CustomForm property
 * - Use the default or custom getActions function
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
      ],
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
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
      ],
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
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
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
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
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
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
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
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
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
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
        },
      ],
      getActions: (provider, callbacks, isOnboardingPage) =>
        getDefaultButtons(provider, callbacks, isOnboardingPage),
    },
  },
];
