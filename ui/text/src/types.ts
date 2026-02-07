// Workstream represents an independent goose agent working on a task
export interface Workstream {
  id: string;
  name: string;
  task: string;
  status: WorkstreamStatus;
  worktreePath: string | null;
  branchName: string | null;
  acpSessionId: string | null;
  createdAt: Date;
  lastActivity: Date;
  currentActivity: string;
  notifications: Notification[];
  messageHistory: WorkstreamMessage[];
}

export type WorkstreamStatus = 
  | 'starting'      // Setting up worktree and agent
  | 'running'       // Agent is actively working
  | 'waiting'       // Agent needs user input/decision
  | 'reviewing'     // Work complete, needs review
  | 'paused'        // User paused the workstream
  | 'completed'     // Task finished successfully
  | 'error';        // Something went wrong

export interface Notification {
  id: string;
  type: 'action_required' | 'review_ready' | 'error' | 'info';
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
  workstreamId: string;
}

export interface WorkstreamMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
}

export interface ToolCallInfo {
  id: string;
  title: string;
  status: 'pending' | 'completed' | 'failed';
  content?: string;
}

// View modes for the TUI
export type ViewMode = 'dashboard' | 'focus' | 'new-task' | 'help';

// App state
export interface OrchestratorState {
  viewMode: ViewMode;
  workstreams: Map<string, Workstream>;
  focusedWorkstreamId: string | null;
  notifications: Notification[];
  repoPath: string;
  serverUrl: string;
  isGitRepo: boolean;
}
