import type { ToolPermissionEntry, ToolPermissionLevel } from '@aaif/goose-sdk';
import { getAcpClient } from './acpConnection';

export type { ToolPermissionEntry, ToolPermissionLevel };

export interface AcpToolInfo {
  name: string;
  description: string;
  parameters: string[];
  permission?: string | null;
  input_schema?: unknown;
}

export async function listTools(sessionId: string, extensionName?: string): Promise<AcpToolInfo[]> {
  const client = await getAcpClient();
  const response = await client.goose.toolsList_unstable({
    sessionId,
    extensionName: extensionName ?? null,
  });
  return (response.tools as AcpToolInfo[]) ?? [];
}

export async function setToolPermissions(toolPermissions: ToolPermissionEntry[]): Promise<void> {
  const client = await getAcpClient();
  await client.goose.toolsPermissionsSet_unstable({ toolPermissions });
}
