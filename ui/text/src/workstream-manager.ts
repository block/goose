import { v4 as uuidv4 } from 'uuid';
import { SdkAcpClient, type RequestPermissionRequest, type RequestPermissionResponse, type SessionNotification } from './acp-client.js';
import { GitWorktreeManager } from './worktree.js';
import { 
  Workstream, 
  WorkstreamStatus, 
  Notification, 
  WorkstreamMessage,
  ToolCallInfo 
} from './types.js';
import type { TextContent, ToolCall, ToolCallUpdate } from '@agentclientprotocol/sdk';

export interface WorkstreamManagerConfig {
  serverUrl: string;
  repoPath: string;
  useWorktrees: boolean;
  transportType?: 'http' | 'websocket';
}

type WorkstreamEventHandler = (workstreamId: string, event: WorkstreamEvent) => void;

export type WorkstreamEvent = 
  | { type: 'status_change'; status: WorkstreamStatus; activity?: string }
  | { type: 'message'; message: WorkstreamMessage }
  | { type: 'tool_call'; tool: ToolCallInfo }
  | { type: 'tool_update'; toolId: string; status: string }
  | { type: 'notification'; notification: Notification }
  | { type: 'permission_request'; requestId: string; data: RequestPermissionRequest }
  | { type: 'error'; error: string };

interface PendingPermissionRequest {
  requestId: string;
  params: RequestPermissionRequest;
  workstreamId: string;
  resolve: (response: RequestPermissionResponse) => void;
}

export class WorkstreamManager {
  private config: WorkstreamManagerConfig;
  private workstreams: Map<string, Workstream> = new Map();
  private clients: Map<string, SdkAcpClient> = new Map();
  private eventHandlers: WorkstreamEventHandler[] = [];
  private worktreeManager: GitWorktreeManager | null = null;
  private activeTools: Map<string, Map<string, ToolCallInfo>> = new Map(); // workstreamId -> toolId -> info
  private pendingPermissions: Map<string, PendingPermissionRequest> = new Map();

  constructor(config: WorkstreamManagerConfig) {
    this.config = config;
    
    if (config.useWorktrees) {
      this.worktreeManager = new GitWorktreeManager(config.repoPath);
    }
  }

  onEvent(handler: WorkstreamEventHandler): () => void {
    this.eventHandlers.push(handler);
    return () => {
      const idx = this.eventHandlers.indexOf(handler);
      if (idx > -1) this.eventHandlers.splice(idx, 1);
    };
  }

  private emit(workstreamId: string, event: WorkstreamEvent): void {
    for (const handler of this.eventHandlers) {
      handler(workstreamId, event);
    }
  }

  private updateWorkstream(id: string, updates: Partial<Workstream>): void {
    const ws = this.workstreams.get(id);
    if (ws) {
      Object.assign(ws, updates, { lastActivity: new Date() });
    }
  }

  async createWorkstream(name: string, task: string): Promise<Workstream> {
    const id = uuidv4();
    const sanitizedName = name.toLowerCase().replace(/[^a-z0-9-]/g, '-').slice(0, 50);
    
    const workstream: Workstream = {
      id,
      name: sanitizedName,
      task,
      status: 'starting',
      worktreePath: null,
      branchName: null,
      acpSessionId: null,
      createdAt: new Date(),
      lastActivity: new Date(),
      currentActivity: 'Initializing...',
      notifications: [],
      messageHistory: []
    };

    this.workstreams.set(id, workstream);
    this.activeTools.set(id, new Map());

    // Create worktree if enabled and in a git repo
    if (this.worktreeManager && this.worktreeManager.isGitRepo()) {
      try {
        this.updateWorkstream(id, { currentActivity: 'Creating git worktree...' });
        const worktreeInfo = await this.worktreeManager.createWorktree(sanitizedName);
        this.updateWorkstream(id, {
          worktreePath: worktreeInfo.path,
          branchName: worktreeInfo.branch
        });
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Unknown error';
        this.addNotification(id, 'error', 'Worktree Creation Failed', errorMsg);
        // Continue without worktree - work in main repo
      }
    }

    // Connect to ACP server
    try {
      this.updateWorkstream(id, { currentActivity: 'Connecting to server...' });
      await this.connectWorkstream(id);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Connection failed';
      this.updateWorkstream(id, { 
        status: 'error', 
        currentActivity: `Error: ${errorMsg}` 
      });
      this.emit(id, { type: 'error', error: errorMsg });
      throw err;
    }

    return workstream;
  }

  private async connectWorkstream(workstreamId: string): Promise<void> {
    const workstream = this.workstreams.get(workstreamId);
    if (!workstream) throw new Error('Workstream not found');

    // Create SDK client with handlers
    const client = new SdkAcpClient(
      { serverUrl: this.config.serverUrl },
      {
        onSessionUpdate: (notification) => this.handleSessionUpdate(workstreamId, notification),
        onPermissionRequest: (request) => this.handlePermissionRequest(workstreamId, request)
      }
    );
    this.clients.set(workstreamId, client);

    // Connect and initialize
    const sessionId = await client.connect();

    this.updateWorkstream(workstreamId, {
      acpSessionId: sessionId,
      status: 'running',
      currentActivity: 'Ready'
    });

    this.emit(workstreamId, { 
      type: 'status_change', 
      status: 'running',
      activity: 'Connected and ready'
    });
  }

  private handleSessionUpdate(workstreamId: string, notification: SessionNotification): void {
    const workstream = this.workstreams.get(workstreamId);
    if (!workstream) return;

    const update = notification.update;
    const updateType = update.sessionUpdate;

    switch (updateType) {
      case 'agent_message_chunk': {
        if (update.content?.type === 'text') {
          const text = (update.content as TextContent).text || '';
          if (text) {
            // Append to current message or create new one
            const lastMsg = workstream.messageHistory[workstream.messageHistory.length - 1];
            if (lastMsg && lastMsg.role === 'assistant') {
              lastMsg.content += text;
            } else {
              const newMsg: WorkstreamMessage = {
                role: 'assistant',
                content: text,
                timestamp: new Date()
              };
              workstream.messageHistory.push(newMsg);
              this.emit(workstreamId, { type: 'message', message: newMsg });
            }
            this.updateWorkstream(workstreamId, { currentActivity: text.slice(0, 100) });
          }
        }
        break;
      }

      case 'agent_thought_chunk': {
        if (update.content?.type === 'text') {
          const thought = (update.content as TextContent).text || '';
          if (thought) {
            this.updateWorkstream(workstreamId, { 
              currentActivity: `💭 ${thought.slice(0, 100)}` 
            });
          }
        }
        break;
      }

      case 'tool_call': {
        const toolCall = update as ToolCall & { sessionUpdate: 'tool_call' };
        if (toolCall.toolCallId) {
          const toolInfo: ToolCallInfo = {
            id: toolCall.toolCallId,
            title: toolCall.title || 'Tool',
            status: (toolCall.status as 'pending' | 'completed' | 'failed') || 'pending'
          };
          this.activeTools.get(workstreamId)?.set(toolCall.toolCallId, toolInfo);
          this.updateWorkstream(workstreamId, { 
            currentActivity: `🔧 ${toolInfo.title}` 
          });
          this.emit(workstreamId, { type: 'tool_call', tool: toolInfo });
        }
        break;
      }

      case 'tool_call_update': {
        const toolUpdate = update as ToolCallUpdate & { sessionUpdate: 'tool_call_update' };
        if (toolUpdate.toolCallId) {
          const tools = this.activeTools.get(workstreamId);
          if (tools) {
            const tool = tools.get(toolUpdate.toolCallId);
            if (tool && toolUpdate.status) {
              tool.status = toolUpdate.status as 'pending' | 'completed' | 'failed';
              if (toolUpdate.status === 'completed' || toolUpdate.status === 'failed') {
                tools.delete(toolUpdate.toolCallId);
              }
            }
          }
          this.emit(workstreamId, { 
            type: 'tool_update', 
            toolId: toolUpdate.toolCallId, 
            status: toolUpdate.status || 'unknown' 
          });
        }
        break;
      }
    }
  }

  private handlePermissionRequest(workstreamId: string, request: RequestPermissionRequest): Promise<RequestPermissionResponse> {
    // SDK uses toolCall with direct properties (title, toolCallId, etc.)
    const toolTitle = request.toolCall?.title || 'Tool execution';
    
    // Notify the UI about the permission request
    this.updateWorkstream(workstreamId, { 
      status: 'waiting',
      currentActivity: `Permission needed: ${toolTitle}`
    });
    
    this.addNotification(
      workstreamId, 
      'action_required', 
      'Permission Required',
      `Tool: ${toolTitle}`
    );

    // Generate a unique request ID
    const requestId = uuidv4();
    
    // Return a promise that will be resolved when the user responds
    return new Promise((resolve) => {
      const pendingRequest: PendingPermissionRequest = {
        requestId,
        params: request,
        workstreamId,
        resolve
      };
      this.pendingPermissions.set(workstreamId, pendingRequest);
      
      this.emit(workstreamId, { 
        type: 'permission_request', 
        requestId,
        data: request 
      });
    });
  }

  async sendPrompt(workstreamId: string, prompt: string): Promise<void> {
    const client = this.clients.get(workstreamId);
    const workstream = this.workstreams.get(workstreamId);
    
    if (!client || !workstream?.acpSessionId) {
      throw new Error('Workstream not connected');
    }

    workstream.messageHistory.push({
      role: 'user',
      content: prompt,
      timestamp: new Date()
    });

    this.updateWorkstream(workstreamId, { 
      status: 'running',
      currentActivity: 'Processing...'
    });

    try {
      // Use the SDK client's prompt method
      await client.prompt(prompt);

      // Check if work is complete (simple heuristic - could be improved)
      const ws = this.workstreams.get(workstreamId);
      if (ws && ws.status === 'running') {
        this.updateWorkstream(workstreamId, { 
          currentActivity: 'Idle - awaiting next instruction'
        });
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Unknown error';
      this.updateWorkstream(workstreamId, { 
        status: 'error',
        currentActivity: `Error: ${errorMsg}`
      });
      this.emit(workstreamId, { type: 'error', error: errorMsg });
    }
  }

  async startTask(workstreamId: string): Promise<void> {
    const workstream = this.workstreams.get(workstreamId);
    if (!workstream) throw new Error('Workstream not found');

    // Build context-aware prompt
    let prompt = workstream.task;
    
    if (workstream.worktreePath) {
      prompt = `You are working in a git worktree at: ${workstream.worktreePath}
Branch: ${workstream.branchName}

Your task: ${workstream.task}

Please work on this task. When you're done or need input, let me know.`;
    }

    await this.sendPrompt(workstreamId, prompt);
  }

  async respondToPermission(
    workstreamId: string, 
    optionId: string
  ): Promise<void> {
    const pending = this.pendingPermissions.get(workstreamId);
    if (!pending) {
      throw new Error('No pending permission request for this workstream');
    }

    // Resolve the promise with the permission response using SDK types
    pending.resolve({
      outcome: { 
        outcome: 'selected',
        optionId
      }
    } as RequestPermissionResponse);

    // Clean up
    this.pendingPermissions.delete(workstreamId);

    this.updateWorkstream(workstreamId, { 
      status: 'running',
      currentActivity: 'Continuing...'
    });
  }

  // Get pending permission request for a workstream
  getPendingPermission(workstreamId: string): PendingPermissionRequest | undefined {
    return this.pendingPermissions.get(workstreamId);
  }

  pauseWorkstream(workstreamId: string): void {
    this.updateWorkstream(workstreamId, { 
      status: 'paused',
      currentActivity: 'Paused by user'
    });
    this.emit(workstreamId, { type: 'status_change', status: 'paused' });
  }

  async resumeWorkstream(workstreamId: string): Promise<void> {
    this.updateWorkstream(workstreamId, { 
      status: 'running',
      currentActivity: 'Resumed'
    });
    this.emit(workstreamId, { type: 'status_change', status: 'running' });
  }

  async stopWorkstream(workstreamId: string, cleanup: boolean = false): Promise<void> {
    const client = this.clients.get(workstreamId);
    const workstream = this.workstreams.get(workstreamId);

    if (client) {
      client.disconnect();
      this.clients.delete(workstreamId);
    }

    if (cleanup && workstream?.worktreePath && this.worktreeManager) {
      try {
        await this.worktreeManager.removeWorktree(workstream.name);
      } catch {
        // Ignore cleanup errors
      }
    }

    this.workstreams.delete(workstreamId);
    this.activeTools.delete(workstreamId);
  }

  getWorkstream(id: string): Workstream | undefined {
    return this.workstreams.get(id);
  }

  getAllWorkstreams(): Workstream[] {
    return Array.from(this.workstreams.values());
  }

  getActiveTools(workstreamId: string): ToolCallInfo[] {
    const tools = this.activeTools.get(workstreamId);
    return tools ? Array.from(tools.values()) : [];
  }

  private addNotification(
    workstreamId: string, 
    type: Notification['type'], 
    title: string, 
    message: string
  ): void {
    const notification: Notification = {
      id: uuidv4(),
      type,
      title,
      message,
      timestamp: new Date(),
      read: false,
      workstreamId
    };

    const workstream = this.workstreams.get(workstreamId);
    if (workstream) {
      workstream.notifications.push(notification);
    }

    this.emit(workstreamId, { type: 'notification', notification });
  }

  markNotificationRead(notificationId: string): void {
    for (const ws of this.workstreams.values()) {
      const notif = ws.notifications.find(n => n.id === notificationId);
      if (notif) {
        notif.read = true;
        break;
      }
    }
  }

  getUnreadNotifications(): Notification[] {
    const notifications: Notification[] = [];
    for (const ws of this.workstreams.values()) {
      notifications.push(...ws.notifications.filter(n => !n.read));
    }
    return notifications.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
  }

  // Git-related helpers
  getWorkstreamDiff(workstreamId: string): string {
    const workstream = this.workstreams.get(workstreamId);
    if (!workstream?.worktreePath || !this.worktreeManager) return '';
    return this.worktreeManager.getDiff(workstream.worktreePath);
  }

  getWorkstreamStatus(workstreamId: string): string {
    const workstream = this.workstreams.get(workstreamId);
    if (!workstream?.worktreePath || !this.worktreeManager) return '';
    return this.worktreeManager.getStatus(workstream.worktreePath);
  }

  commitWorkstreamChanges(workstreamId: string, message: string): boolean {
    const workstream = this.workstreams.get(workstreamId);
    if (!workstream?.worktreePath || !this.worktreeManager) return false;
    return this.worktreeManager.commitChanges(workstream.worktreePath, message);
  }
}
