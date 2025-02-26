import React from 'react';
import ProviderDetails from './interfaces/ProviderDetails';
import DefaultProviderActions from './subcomponents/actions/DefaultProviderActions';
import OllamaActions from './subcomponents/actions/OllamaActions';

export interface ProviderRegistry {
  name: string;
  details: ProviderDetails;
}

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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onShowSettings } = callbacks || {};
        return [
          {
            id: 'default-provider-actions',
            renderButton: () => (
              <DefaultProviderActions
                name={provider.name}
                isConfigured={provider.isConfigured}
                onAdd={onAdd}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onShowSettings } = callbacks || {};
        return [
          {
            id: 'default-provider-actions',
            renderButton: () => (
              <DefaultProviderActions
                name={provider.name}
                isConfigured={provider.isConfigured}
                onAdd={onAdd}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onShowSettings } = callbacks || {};
        return [
          {
            id: 'default-provider-actions',
            renderButton: () => (
              <DefaultProviderActions
                name={provider.name}
                isConfigured={provider.isConfigured}
                onAdd={onAdd}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onShowSettings } = callbacks || {};
        return [
          {
            id: 'default-provider-actions',
            renderButton: () => (
              <DefaultProviderActions
                name={provider.name}
                isConfigured={provider.isConfigured}
                onAdd={onAdd}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onShowSettings } = callbacks || {};
        return [
          {
            id: 'default-provider-actions',
            renderButton: () => (
              <DefaultProviderActions
                name={provider.name}
                isConfigured={provider.isConfigured}
                onAdd={onAdd}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onShowSettings } = callbacks || {};
        return [
          {
            id: 'default-provider-actions',
            renderButton: () => (
              <DefaultProviderActions
                name={provider.name}
                isConfigured={provider.isConfigured}
                onAdd={onAdd}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
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
      getActions: (provider, callbacks) => {
        const { onAdd, onDelete, onRefresh, onShowSettings } = callbacks || {};
        return [
          {
            id: 'ollama-actions',
            renderButton: () => (
              <OllamaActions
                isConfigured={provider.isConfigured}
                ollamaMetadata={provider.metadata}
                onAdd={onAdd}
                onRefresh={onRefresh}
                onDelete={onDelete}
                onShowSettings={onShowSettings}
              />
            ),
          },
        ];
      },
    },
  },
];

// const ACTION_IMPLEMENTATIONS = {
//   'default': (provider, callbacks) => [{
//     id: 'default-provider-actions',
//     renderButton: () => <DefaultProviderActions {...} />
//   }],
//
//   'ollama': (provider, callbacks) => [{
//     id: 'ollama-actions',
//     renderButton: () => <OllamaActions {...} />
//   }]
// };
