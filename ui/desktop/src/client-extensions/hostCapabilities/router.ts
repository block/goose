import type { HostCapability } from '../types';
import { HOST_CAPABILITY_REGISTRY } from './registry';
import {
  isHostCapabilityInvokeMessage,
  postHostCapabilityError,
  type HostCapabilityContext,
  type HostCapabilityHostMessage,
} from './types';

export function extensionHasHostCapability(
  capabilities: HostCapability[] | undefined,
  capability: HostCapability
): boolean {
  return capabilities?.includes(capability) ?? false;
}

export function notifyHostCapabilities(
  grantedCapabilities: HostCapability[] | undefined,
  postToExtension: (message: HostCapabilityHostMessage) => void
): void {
  postToExtension({
    type: 'grc/host/capabilities',
    capabilities: grantedCapabilities ?? [],
  });
}

export async function handleHostCapabilityInvoke(
  extensionId: string,
  grantedCapabilities: HostCapability[] | undefined,
  message: unknown,
  postToExtension: (payload: unknown) => void
): Promise<boolean> {
  if (!isHostCapabilityInvokeMessage(message)) {
    return false;
  }

  const { capability, method, payload } = message;

  const context: HostCapabilityContext = {
    extensionId,
    postToExtension,
  };

  if (!extensionHasHostCapability(grantedCapabilities, capability)) {
    postHostCapabilityError(
      context,
      capability,
      method,
      `Add-on "${extensionId}" is not granted host capability "${capability}"`
    );
    return true;
  }

  const definition = HOST_CAPABILITY_REGISTRY[capability];
  if (!definition) {
    postHostCapabilityError(
      context,
      capability,
      method,
      `Unknown host capability "${capability}"`
    );
    return true;
  }

  await definition.handleInvoke(context, method, payload);
  return true;
}
