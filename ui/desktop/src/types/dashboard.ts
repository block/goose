export interface WidgetPosition {
  x: number;
  y: number;
}

export interface WidgetSize {
  width: number;
  height: number;
}

export enum WidgetType {
  TOTAL_SESSIONS = 'total_sessions',
  TOTAL_TOKENS = 'total_tokens',
  RECENT_CHATS = 'recent_chats',
  ACTIVE_DIRECTORIES = 'active_directories',
  GREETING = 'greeting',
}

export interface WidgetData {
  id: string;
  type: WidgetType;
  position: WidgetPosition;
  size: WidgetSize;
  data?: {
    totalSessions?: number;
    totalTokens?: number;
    recentSessions?: Array<{
      id: string;
      metadata?: { description?: string };
      modified: string;
    }>;
    greeting?: string;
    subtitle?: string;
  };
  title?: string;
}

export interface DashboardState {
  widgets: WidgetData[];
  canvasSize: {
    width: number;
    height: number;
  };
}
