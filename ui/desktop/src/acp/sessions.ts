import type {
  ContentBlock,
  ListSessionsRequest,
  ListSessionsResponse,
  LoadSessionResponse,
  NewSessionResponse,
  PromptResponse,
  SessionConfigOption,
  SessionInfo,
} from '@agentclientprotocol/sdk';
import { getAcpClient } from './acpConnection';
import { DEFAULT_CHAT_TITLE } from '../contexts/ChatContext';
import type { ExtensionLoadResult, Message, Recipe } from '../api';
import type { Session } from '../api';

interface AcpLoadSessionMeta {
  extensionResults?: ExtensionLoadResult[] | null;
  recipe?: Recipe | null;
  userRecipeValues?: Record<string, string> | null;
  workingDir?: string;
  configOptions?: SessionConfigOption[] | null;
}

export async function acpLoadSession(
  sessionId: string,
  workingDir: string
): Promise<LoadSessionResponse> {
  const client = await getAcpClient();
  return client.loadSession({
    sessionId,
    cwd: workingDir,
    mcpServers: [],
  });
}

export async function acpNewSession(workingDir: string): Promise<NewSessionResponse> {
  const client = await getAcpClient();
  return client.newSession({
    cwd: workingDir,
    mcpServers: [],
    _meta: { client: 'goose' },
  });
}

export function acpNewSessionToSession(response: NewSessionResponse, workingDir: string): Session {
  const now = new Date().toISOString();
  return {
    id: response.sessionId,
    name: DEFAULT_CHAT_TITLE,
    working_dir: workingDir,
    created_at: now,
    updated_at: now,
    message_count: 0,
    extension_data: {},
    conversation: [],
  };
}

export async function acpPromptSession(
  sessionId: string,
  message: Message
): Promise<PromptResponse> {
  const client = await getAcpClient();
  return client.prompt({
    sessionId,
    prompt: messageToAcpPromptContent(message),
  });
}

export async function acpCancelPrompt(sessionId: string): Promise<void> {
  const client = await getAcpClient();
  await client.cancel({ sessionId });
}

export async function acpSetSessionConfigOption(
  sessionId: string,
  configId: string,
  value: string
): Promise<SessionConfigOption[]> {
  const client = await getAcpClient();
  const response = await client.setSessionConfigOption({ sessionId, configId, value });
  return response.configOptions;
}

export function messageToAcpPromptContent(message: Message): ContentBlock[] {
  const prompt: ContentBlock[] = [];

  for (const content of message.content) {
    switch (content.type) {
      case 'text':
        if (content.text.trim()) {
          prompt.push({
            type: 'text',
            text: content.text,
          });
        }
        break;
      case 'image':
        prompt.push({
          type: 'image',
          data: content.data,
          mimeType: content.mimeType,
        });
        break;
    }
  }

  return prompt;
}

export async function acpListSessions(): Promise<ListSessionsResponse> {
  const client = await getAcpClient();
  const sessions: SessionInfo[] = [];
  let cursor: string | null | undefined;
  let meta: ListSessionsResponse['_meta'] = undefined;

  do {
    const request: ListSessionsRequest = cursor ? { cursor } : {};
    const response = await client.listSessions(request);
    sessions.push(...response.sessions);
    meta = response._meta;
    cursor = response.nextCursor;
  } while (cursor);

  return {
    sessions,
    nextCursor: null,
    ...(meta === undefined ? {} : { _meta: meta }),
  };
}

export async function acpListRecentSessions(maxSessions: number): Promise<ListSessionsResponse> {
  if (maxSessions <= 0) {
    return { sessions: [], nextCursor: null };
  }

  const client = await getAcpClient();
  const response = await client.listSessions({});

  return {
    sessions: response.sessions.slice(0, maxSessions),
    nextCursor: null,
    ...(response._meta === undefined ? {} : { _meta: response._meta }),
  };
}

export async function acpRenameSession(sessionId: string, title: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionRename_unstable({ sessionId, title });
}

export async function acpUpdateWorkingDir(
  sessionId: string,
  workingDir: string
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionWorkingDirUpdate_unstable({ sessionId, workingDir });
}

export async function acpDeleteSession(sessionId: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionDelete({ sessionId });
}

export async function acpForkSession(sessionId: string, cwd: string): Promise<void> {
  const client = await getAcpClient();
  await client.unstable_forkSession({ sessionId, cwd, mcpServers: [] });
}

export async function acpExportSession(sessionId: string): Promise<string> {
  const client = await getAcpClient();
  const result = await client.goose.sessionExport_unstable({ sessionId });
  return result.data;
}

export async function acpImportSession(data: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.sessionImport_unstable({ data });
}

export function acpLoadSessionMeta(response: LoadSessionResponse): AcpLoadSessionMeta {
  const meta = (response._meta ?? {}) as Record<string, unknown>;
  return {
    extensionResults: meta.extensionResults as ExtensionLoadResult[] | null | undefined,
    recipe: meta.recipe as Recipe | null | undefined,
    userRecipeValues: meta.userRecipeValues as Record<string, string> | null | undefined,
    workingDir: typeof meta.workingDir === 'string' ? meta.workingDir : undefined,
    configOptions: response.configOptions,
  };
}

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

export function sessionInfoToListItem(s: SessionInfo): SessionListItem {
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
