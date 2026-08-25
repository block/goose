export type HostCapability = string;

export interface HostCapabilityDefinition {
  id: HostCapability;
  description: string;
  methods: string[];
  handleInvoke: (
    context: HostCapabilityContext,
    method: string,
    payload: unknown
  ) => Promise<void>;
}

export interface HostCapabilityContext {
  extensionId: string;
  postToExtension: (message: HostCapabilityHostMessage) => void;
}

export type HostCapabilityHostMessage =
  | {
      type: 'grc/host/capabilities';
      capabilities: HostCapability[];
    }
  | {
      type: 'grc/host/result';
      capability: HostCapability;
      method: string;
      payload?: unknown;
    }
  | {
      type: 'grc/host/error';
      capability: HostCapability;
      method: string;
      error: string;
    };

export interface HostCapabilityInvokeMessage {
  type: 'grc/host/invoke';
  capability: HostCapability;
  method: string;
  payload?: unknown;
}

export function isHostCapabilityInvokeMessage(
  value: unknown
): value is HostCapabilityInvokeMessage {
  if (typeof value !== 'object' || value === null || !('type' in value)) {
    return false;
  }
  const record = value as HostCapabilityInvokeMessage;
  return (
    record.type === 'grc/host/invoke' &&
    typeof record.capability === 'string' &&
    typeof record.method === 'string'
  );
}

export function postHostCapabilityError(
  context: HostCapabilityContext,
  capability: HostCapability,
  method: string,
  error: string
): void {
  context.postToExtension({
    type: 'grc/host/error',
    capability,
    method,
    error,
  });
}

export function postHostCapabilityResult(
  context: HostCapabilityContext,
  capability: HostCapability,
  method: string,
  payload?: unknown
): void {
  context.postToExtension({
    type: 'grc/host/result',
    capability,
    method,
    payload,
  });
}
