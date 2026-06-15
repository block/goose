export const SECURITY_EXTENSION_IDS = [
  'browser-assist-mcp',
  'threat-intel-mcp',
  'aiseesec-mcp',
  'local-security-gateway-mcp',
] as const;

export type SecurityExtensionId = (typeof SECURITY_EXTENSION_IDS)[number];
export type SecurityExtensionStatus =
  | 'local_preview'
  | 'disabled_stub'
  | 'blocked_external_dependency';

export interface SecurityExtensionDefinition {
  id: SecurityExtensionId;
  displayName: string;
  status: SecurityExtensionStatus;
}

export const SECURITY_EXTENSIONS: readonly SecurityExtensionDefinition[] = [
  {
    id: 'browser-assist-mcp',
    displayName: 'Browser Assist',
    status: 'local_preview',
  },
  {
    id: 'threat-intel-mcp',
    displayName: 'Threat Intel',
    status: 'local_preview',
  },
  {
    id: 'aiseesec-mcp',
    displayName: 'AiseeSec',
    status: 'blocked_external_dependency',
  },
  {
    id: 'local-security-gateway-mcp',
    displayName: 'Security Gateway',
    status: 'disabled_stub',
  },
] as const;

const SECURITY_EXTENSION_MAP = new Map(
  SECURITY_EXTENSIONS.map((extension) => [extension.id, extension])
);

export function getSecurityExtensionById(
  extensionId: SecurityExtensionId
): SecurityExtensionDefinition {
  const extension = SECURITY_EXTENSION_MAP.get(extensionId);
  if (!extension) {
    throw new Error(`Unknown security extension: ${extensionId}`);
  }
  return extension;
}
