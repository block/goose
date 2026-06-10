import type { ForkSessionRequest, ListSessionsRequest, SessionInfo } from '@agentclientprotocol/sdk';
import { getAcpClient } from './acpConnection';
import { DEFAULT_CHAT_TITLE } from '../contexts/ChatContext';

interface GooseSessionInfoMeta {
  messageCount?: number;
  createdAt?: string;
  archivedAt?: string;
  projectId?: string;
  providerId?: string;
  modelId?: string;
  userSetName?: boolean;
  hasRecipe?: boolean;
}

export interface SessionListItem {
  id: string;
  name: string;
  workingDir: string;
  updatedAt: string;
  messageCount: number;
  createdAt: string;
  archivedAt?: string;
  projectId?: string;
  providerId?: string;
  modelId?: string;
  userSetName?: boolean;
  hasRecipe?: boolean;
}

export interface SessionListPage {
  sessions: SessionListItem[];
  nextCursor: string | null;
}

function sessionInfoToListItem(s: SessionInfo): SessionListItem {
  const meta = (s._meta ?? {}) as GooseSessionInfoMeta;
  return {
    id: String(s.sessionId),
    name: s.title ?? DEFAULT_CHAT_TITLE,
    workingDir: s.cwd,
    updatedAt: s.updatedAt ?? '',
    messageCount: meta.messageCount ?? 0,
    createdAt: meta.createdAt ?? s.updatedAt ?? '',
    archivedAt: meta.archivedAt,
    projectId: meta.projectId,
    providerId: meta.providerId,
    modelId: meta.modelId,
    userSetName: meta.userSetName,
    hasRecipe: meta.hasRecipe,
  };
}

export async function acpListSessions(cursor?: string | null): Promise<SessionListPage> {
  const client = await getAcpClient();
  const request: ListSessionsRequest = cursor ? { cursor } : {};
  const response = await client.listSessions(request);
  return {
    sessions: response.sessions.map(sessionInfoToListItem),
    nextCursor: response.nextCursor ?? null,
  };
}

export async function acpListRecentSessions(maxSessions: number): Promise<SessionListItem[]> {
  if (maxSessions <= 0) {
    return [];
  }

  const client = await getAcpClient();
  const response = await client.listSessions({});
  return response.sessions.slice(0, maxSessions).map(sessionInfoToListItem);
}

export async function acpDeleteSession(sessionId: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionDelete({ sessionId });
}

export async function acpRenameSession(sessionId: string, title: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionRename_unstable({ sessionId, title });
}

export async function acpForkSession(sessionId: string, cwd: string): Promise<void> {
  const client = await getAcpClient();
  const request: ForkSessionRequest = { sessionId, cwd };
  await client.unstable_forkSession(request);
}

export async function acpExportSession(sessionId: string): Promise<string> {
  const client = await getAcpClient();
  const response = await client.goose.sessionExport_unstable({ sessionId });
  return response.data;
}

export async function acpImportSession(data: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionImport_unstable({ data });
}
