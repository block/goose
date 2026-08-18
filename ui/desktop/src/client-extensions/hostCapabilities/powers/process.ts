import { postHostCapabilityError, type HostCapabilityDefinition } from '../types';

export const processPower: HostCapabilityDefinition = {
  id: 'process',
  description:
    'Managed local processes (check/start/stop). Each plugin uses a processId allowlisted for that extension.',
  methods: ['check', 'start', 'stop', 'status'],
  async handleInvoke(context, method) {
    postHostCapabilityError(
      context,
      'process',
      method,
      `process host power "${method}" is not wired yet`
    );
  },
};
