import type { HostCapabilityDefinition } from './types';
import { COMMON_HOST_POWERS } from './powers';

export const HOST_CAPABILITY_REGISTRY: Record<string, HostCapabilityDefinition> =
  Object.fromEntries(COMMON_HOST_POWERS.map((power) => [power.id, power]));

export const KNOWN_HOST_CAPABILITIES: string[] = Object.keys(HOST_CAPABILITY_REGISTRY);

export { COMMON_HOST_POWERS };
export type { CommonHostPowerId } from './powers';
