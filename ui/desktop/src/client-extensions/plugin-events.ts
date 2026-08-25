// ACP session events forwarded renderer → main → host.js plugins via ctx.session.on()
export type PluginSessionEvent =
  | {
      type: 'agent_message_chunk';
      sessionId: string;
      content: unknown;
    }
  | {
      type: 'tool_call';
      sessionId: string;
      toolCallId: string;
      title?: string | null;
      status?: string | null;
    }
  | {
      type: 'status_message';
      sessionId: string;
      message: string;
      level: 'notice' | 'progress';
    };

export const PLUGIN_SESSION_EVENT_CHANNEL = 'plugin-host:session-event';
