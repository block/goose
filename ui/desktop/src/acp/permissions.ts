import type { PermissionLevel } from '../api';
import { getAcpClient } from './acpConnection';

export interface ToolPermissionEntry {
  tool_name: string;
  permission: PermissionLevel;
}

export async function setToolPermissions(toolPermissions: ToolPermissionEntry[]): Promise<void> {
  const client = await getAcpClient();
  await client.goose.toolsPermissionsSet_unstable({ tool_permissions: toolPermissions });
}
