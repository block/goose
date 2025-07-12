import { UIResourceRenderer as McpUIResourceRenderer } from '@mcp-ui/client';
import { Content } from '../types/message';

// Define Resource type based on MCP SDK structure
interface Resource {
  name: string;
  uri: string;
  description?: string;
  mimeType?: string;
  text?: string;
  data?: string;
  [key: string]: unknown;
}

// Define UI Action types
interface UIAction {
  type: 'tool' | 'intent' | 'prompt' | 'notification' | 'link';
  payload: {
    toolName?: string;
    params?: Record<string, unknown>;
    intent?: string;
    prompt?: string;
    message?: string;
    url?: string;
  };
}

interface UIResourceRendererProps {
  resource: Resource;
  onUIAction?: (action: UIAction) => Promise<Record<string, unknown>>;
  className?: string;
}

export function UIResourceRenderer({
  resource,
  onUIAction,
  className = '',
}: UIResourceRendererProps) {
  const handleUIAction = async (action: UIAction): Promise<Record<string, unknown>> => {
    console.log('UI Action received:', action);

    // Handle different action types
    if (action.type === 'tool') {
      // Tool call from UI - we need to invoke this tool
      console.log('Tool call from UI:', action.payload.toolName, action.payload.params);

      // TODO: Implement tool calling mechanism
      // For now, just log the action
      if (onUIAction) {
        return await onUIAction(action);
      }

      return { status: 'handled' };
    }

    if (action.type === 'intent') {
      console.log('Intent from UI:', action.payload.intent, action.payload.params);
      if (onUIAction) {
        return await onUIAction(action);
      }
      return { status: 'handled' };
    }

    if (action.type === 'prompt') {
      console.log('Prompt from UI:', action.payload.prompt);
      if (onUIAction) {
        return await onUIAction(action);
      }
      return { status: 'handled' };
    }

    if (action.type === 'notification') {
      console.log('Notification from UI:', action.payload.message);
      // Could show a toast notification here
      return { status: 'handled' };
    }

    if (action.type === 'link') {
      console.log('Link from UI:', action.payload.url);
      // Could open the link in external browser
      if (action.payload.url) {
        window.electron.openInChrome(action.payload.url);
      }
      return { status: 'handled' };
    }

    return { status: 'unhandled' };
  };

  return (
    <div
      className={`mcp-ui-resource-renderer ${className}`}
      style={{
        border: '1px solid #e2e8f0',
        borderRadius: '8px',
        minHeight: '200px',
        backgroundColor: '#ffffff',
      }}
    >
      <McpUIResourceRenderer
        resource={resource}
        onUIAction={handleUIAction}
        htmlProps={{
          iframeProps: {
            title: 'MCP UI Resource',
          },
        }}
      />
    </div>
  );
}

// Helper function to check if content contains a UI resource
export function isUIResource(content: Content): boolean {
  // Check if the content is a text content that contains a UI resource
  if (content.type === 'text' && content.text) {
    try {
      // Try to parse the text as JSON to see if it's a resource object
      const parsed = JSON.parse(content.text);
      return (
        parsed &&
        typeof parsed === 'object' &&
        parsed.uri &&
        parsed.uri.startsWith('ui://') &&
        parsed.mimeType &&
        (parsed.mimeType === 'text/html' ||
          parsed.mimeType === 'text/uri-list' ||
          parsed.mimeType.startsWith('application/vnd.mcp-ui.remote-dom'))
      );
    } catch {
      // Not JSON, check if it looks like a resource response
      return content.text.includes('ui://') && content.text.includes('mimeType');
    }
  }

  return false;
}

// Helper function to extract UI resource from content
export function extractUIResource(content: Content): Resource | null {
  if (content.type === 'text' && content.text) {
    try {
      const parsed = JSON.parse(content.text);
      if (parsed && typeof parsed === 'object' && parsed.uri && parsed.mimeType) {
        return parsed as Resource;
      }
    } catch {
      // Not valid JSON
    }
  }

  return null;
}
