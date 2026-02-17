// Manual API client for /a2a/instances endpoints
// These endpoints are not yet in the OpenAPI spec, so we use manual fetch
// following the same pattern as components/analytics/AgentCatalog.tsx
import { client } from './client.gen';

// ─── Types ──────────────────────────────────────────────────────────────────

export type InstanceStatus = 'running' | 'completed' | 'failed' | 'cancelled';

export interface SpawnInstanceRequest {
  persona: string;
  instructions?: string;
  provider?: string;
  model?: string;
  max_turns?: number;
}

export interface InstanceResponse {
  id: string;
  persona: string;
  status: InstanceStatus;
  turns: number;
  provider_name: string;
  model_name: string;
  elapsed_secs: number;
  last_activity_ms: number;
}

export interface InstanceResultResponse {
  id: string;
  persona: string;
  status: string;
  output?: string;
  error?: string;
  turns_taken: number;
  duration_secs: number;
}

export interface InstanceEvent {
  timestamp: number;
  type: string;
  data: string;
}

// ─── Internal helpers ───────────────────────────────────────────────────────

function getClientConfig(): { baseUrl: string; headers: Record<string, string> } {
  const baseUrl = client.getConfig().baseUrl || '';
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const rawHeaders = client.getConfig().headers;
  if (rawHeaders) {
    const h = rawHeaders as Record<string, string>;
    const secretKey =
      typeof h.get === 'function'
        ? (h as unknown as globalThis.Headers).get('X-Secret-Key')
        : h['X-Secret-Key'];
    if (secretKey) {
      headers['X-Secret-Key'] = secretKey;
    }
  }
  return { baseUrl, headers };
}

async function handleResponse<T>(resp: Response): Promise<T> {
  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`HTTP ${resp.status}: ${body || resp.statusText}`);
  }
  return resp.json();
}

// ─── API Functions ──────────────────────────────────────────────────────────

export async function listInstances(): Promise<InstanceResponse[]> {
  const { baseUrl, headers } = getClientConfig();
  const resp = await fetch(`${baseUrl}/a2a/instances`, { headers });
  return handleResponse<InstanceResponse[]>(resp);
}

export async function spawnInstance(body: SpawnInstanceRequest): Promise<InstanceResponse> {
  const { baseUrl, headers } = getClientConfig();
  const resp = await fetch(`${baseUrl}/a2a/instances`, {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
  });
  return handleResponse<InstanceResponse>(resp);
}

export async function getInstance(id: string): Promise<InstanceResponse> {
  const { baseUrl, headers } = getClientConfig();
  const resp = await fetch(`${baseUrl}/a2a/instances/${encodeURIComponent(id)}`, { headers });
  return handleResponse<InstanceResponse>(resp);
}

export async function cancelInstance(id: string): Promise<void> {
  const { baseUrl, headers } = getClientConfig();
  const resp = await fetch(`${baseUrl}/a2a/instances/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    headers,
  });
  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`HTTP ${resp.status}: ${body || resp.statusText}`);
  }
}

export async function getInstanceResult(id: string): Promise<InstanceResultResponse> {
  const { baseUrl, headers } = getClientConfig();
  const resp = await fetch(`${baseUrl}/a2a/instances/${encodeURIComponent(id)}/result`, {
    headers,
  });
  return handleResponse<InstanceResultResponse>(resp);
}

export async function listPersonas(): Promise<string[]> {
  const { baseUrl, headers } = getClientConfig();
  const resp = await fetch(`${baseUrl}/a2a/agents`, { headers });
  return handleResponse<string[]>(resp);
}

export function createInstanceEventSourceUrl(id: string): string {
  const { baseUrl } = getClientConfig();
  return `${baseUrl}/a2a/instances/${encodeURIComponent(id)}/events`;
}
