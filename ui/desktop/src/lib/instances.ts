/**
 * A2A Instance management — thin wrapper around the auto-generated SDK.
 *
 * Re-exports generated types and provides:
 * 1. Convenience wrappers that unwrap SDK responses (throw on error)
 * 2. SSE/utility helpers that cannot be expressed in OpenAPI
 */

import { client } from '../api/client.gen';
import {
  listInstances as sdkListInstances,
  spawnInstance as sdkSpawnInstance,
  getInstance as sdkGetInstance,
  cancelInstance as sdkCancelInstance,
  getInstanceResult as sdkGetInstanceResult,
  listPersonas as sdkListPersonas,
} from '../api/sdk.gen';

// Re-export generated types so existing component imports keep working
export type {
  InstanceResponse,
  InstanceResultResponse,
  SpawnInstanceRequest,
  PersonaSummary,
} from '../api/types.gen';

import type {
  InstanceResponse,
  InstanceResultResponse,
  PersonaSummary,
  SpawnInstanceRequest,
} from '../api/types.gen';

// --- Types that are NOT in the OpenAPI spec ---

/** Status values returned by the backend */
export type InstanceStatus = 'running' | 'completed' | 'failed' | 'cancelled';

/** SSE event from the instance event stream */
export interface InstanceEvent {
  type: string;
  data: string;
  timestamp: number;
}

// --- Convenience wrappers (unwrap SDK { data, error } pattern) ---

export async function listInstances(): Promise<InstanceResponse[]> {
  const { data, error } = await sdkListInstances();
  if (error) throw new Error(String(error));
  return (data as InstanceResponse[]) ?? [];
}

export async function spawnInstance(req: SpawnInstanceRequest): Promise<InstanceResponse> {
  const { data, error } = await sdkSpawnInstance({
    body: req,
  });
  if (error) throw new Error(String(error));
  return data as InstanceResponse;
}

export async function getInstance(instanceId: string): Promise<InstanceResponse> {
  const { data, error } = await sdkGetInstance({
    path: { instance_id: instanceId },
  });
  if (error) throw new Error(String(error));
  return data as InstanceResponse;
}

export async function cancelInstance(instanceId: string): Promise<void> {
  const { error } = await sdkCancelInstance({
    path: { instance_id: instanceId },
  });
  if (error) throw new Error(String(error));
}

export async function getInstanceResult(instanceId: string): Promise<InstanceResultResponse> {
  const { data, error } = await sdkGetInstanceResult({
    path: { instance_id: instanceId },
  });
  if (error) throw new Error(String(error));
  return data as InstanceResultResponse;
}

export async function listPersonas(): Promise<PersonaSummary[]> {
  const { data, error } = await sdkListPersonas();
  if (error) throw new Error(String(error));
  return (data as PersonaSummary[]) ?? [];
}

// --- SSE helper (EventSource URLs can't be auto-generated) ---

/**
 * Build the full URL for an instance's SSE event stream.
 * Used by useInstanceEvents to create an EventSource connection.
 */
export function createInstanceEventSourceUrl(instanceId: string): string {
  const config = client.getConfig();
  const baseUrl = (config.baseUrl || '').replace(/\/$/, '');
  return `${baseUrl}/a2a/instances/${instanceId}/events`;
}
