import { postHostCapabilityError, type HostCapabilityDefinition } from '../types';

export const goosePower: HostCapabilityDefinition = {
  id: 'goose',
  description: 'Read and update goose config, providers, and active model selection.',
  methods: [
    'listProviders',
    'getActiveProvider',
    'setActiveProvider',
    'createCustomProvider',
    'updateCustomProvider',
  ],
  async handleInvoke(context, method) {
    postHostCapabilityError(
      context,
      'goose',
      method,
      `goose host power "${method}" is not wired yet`
    );
  },
};
