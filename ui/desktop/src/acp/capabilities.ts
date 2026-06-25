import type { InitializeResponse } from '@agentclientprotocol/sdk';
import { getAcpInitializeResponse } from './acpConnection';

export interface AcpFeatureCapabilities {
  localInference: boolean;
}

export async function getAcpFeatureCapabilities(): Promise<AcpFeatureCapabilities> {
  const initializeResponse = await getAcpInitializeResponse();

  return {
    localInference: hasLocalInferenceCapability(initializeResponse),
  };
}

export function hasLocalInferenceCapability(
  initializeResponse: Pick<InitializeResponse, 'agentCapabilities'>
): boolean {
  const meta = initializeResponse.agentCapabilities._meta;
  if (!isRecord(meta)) {
    return false;
  }

  const goose = meta.goose;
  if (!isRecord(goose)) {
    return false;
  }

  return 'localInference' in goose;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
