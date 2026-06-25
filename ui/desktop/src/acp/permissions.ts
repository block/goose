import type { ToolPermissionEntry, ToolPermissionLevel } from '@aaif/goose-sdk';
import { getAcpClient } from './acpConnection';

export type { ToolPermissionEntry, ToolPermissionLevel };

export async function setToolPermissions(toolPermissions: ToolPermissionEntry[]): Promise<void> {
  const client = await getAcpClient();
  await client.goose.toolsPermissionsSet_unstable({ toolPermissions });
}
