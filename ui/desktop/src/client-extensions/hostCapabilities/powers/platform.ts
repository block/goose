import { postHostCapabilityError, postHostCapabilityResult, type HostCapabilityDefinition } from '../types';

export const platformPower: HostCapabilityDefinition = {
  id: 'platform',
  description: 'OS and runtime info available to privileged plugins.',
  methods: ['getInfo'],
  async handleInvoke(context, method) {
    if (method !== 'getInfo') {
      postHostCapabilityError(context, 'platform', method, `Unsupported method "${method}"`);
      return;
    }

    postHostCapabilityResult(context, 'platform', method, {
      platform: window.electron?.platform ?? 'unknown',
      arch: window.electron?.arch ?? 'unknown',
    });
  },
};
