// First define the default handlers separately
import OnAdd from './AddProviderParameters';
import OnDelete from './DeleteProviderParameters';

const DEFAULT_HANDLERS = {
  onAdd: (providerId: string, config: any) => {
    OnAdd();
  },
  onDelete: (providerId: string) => {
    OnDelete();
  },
  onShowSettings: (providerId: string) => {
    /* default settings behavior */
  },
};

// Then use them in the registry
export const CALLBACK_REGISTRY = {
  default: DEFAULT_HANDLERS,

  anthropic: {
    onAdd: (providerId: string, config: any) => {
      /* Anthropic-specific add */
    },
    // Fall back to default handlers
    onDelete: DEFAULT_HANDLERS.onDelete,
    onShowSettings: DEFAULT_HANDLERS.onShowSettings,
  },

  ollama: {
    onAdd: (providerId: string, config: any) => {
      /* Ollama-specific add */
    },
    onDelete: (providerId: string) => {
      /* Ollama-specific delete */
    },
    onRefresh: (providerId: string) => {
      /* Ollama-specific refresh */
    },
  },
} as const;

// Type for the handlers
export type ActionHandler = typeof DEFAULT_HANDLERS;
export type ProviderId = keyof typeof CALLBACK_REGISTRY;
