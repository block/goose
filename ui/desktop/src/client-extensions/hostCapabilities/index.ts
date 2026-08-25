import type { HostCapability } from '../types';
import {
  extensionHasHostCapability,
  handleHostCapabilityInvoke,
  notifyHostCapabilities,
} from './router';
import {
  COMMON_HOST_POWERS,
  HOST_CAPABILITY_REGISTRY,
  KNOWN_HOST_CAPABILITIES,
} from './registry';

export {
  extensionHasHostCapability,
  handleHostCapabilityInvoke,
  notifyHostCapabilities,
  HOST_CAPABILITY_REGISTRY,
  KNOWN_HOST_CAPABILITIES,
  COMMON_HOST_POWERS,
};
export type { CommonHostPowerId } from './registry';
export type {
  HostCapabilityContext,
  HostCapabilityDefinition,
  HostCapabilityHostMessage,
  HostCapabilityInvokeMessage,
} from './types';
export { isHostCapabilityInvokeMessage } from './types';

export async function handleHostCapabilityMessage(
  extensionId: string,
  grantedCapabilities: HostCapability[] | undefined,
  message: unknown,
  postToExtension: (payload: unknown) => void
): Promise<boolean> {
  if (typeof message !== 'object' || message === null || !('type' in message)) {
    return false;
  }

  const type = (message as { type: unknown }).type;
  if (type !== 'grc/host/invoke') {
    return false;
  }

  return handleHostCapabilityInvoke(
    extensionId,
    grantedCapabilities,
    message,
    postToExtension
  );
}
