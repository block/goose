import { AppEvents } from '../../constants/events';
/**
 * MCP Apps Renderer
 *
 * Uses the official @mcp-ui/client AppRenderer component for rendering MCP Apps.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 * @see @mcp-ui/client https://github.com/MCP-UI-Org/mcp-ui
 */

import { useState, useCallback, useEffect, useMemo } from 'react';
import { AppRenderer } from '@mcp-ui/client';
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
import type {
  McpUiSizeChangedNotification,
  McpUiResourceCsp,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import { ToolInput, ToolInputPartial, ToolResult, ToolCancelled, CspMetadata } from './types';
import { cn } from '../../utils';
import { DEFAULT_IFRAME_HEIGHT, fetchMcpAppProxyUrl } from './utils';
import { readResource, callTool } from '../../api';
import { errorMessage } from '../../utils/conversionUtils';
import { isProtocolSafe, getProtocol } from '../../utils/urlSecurity';
import { useTheme } from '../../contexts/ThemeContext';

interface McpAppRendererProps {
  resourceUri: string;
  extensionName: string;
  sessionId?: string | null;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  append?: (text: string) => void;
  fullscreen?: boolean;
  cachedHtml?: string;
}

interface ResourceData {
  html: string | null;
  csp: CspMetadata | null;
  permissions: string | null;
  prefersBorder: boolean;
}

export default function McpAppRenderer({
  resourceUri,
  extensionName,
  sessionId,
  toolInput,
  toolInputPartial,
  toolResult,
  toolCancelled,
  append,
  fullscreen = false,
  cachedHtml,
}: McpAppRendererProps) {
  const { resolvedTheme } = useTheme();
  const [resource, setResource] = useState<ResourceData>({
    html: cachedHtml || null,
    csp: null,
    permissions: null,
    prefersBorder: true,
  });
  const [error, setError] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);
  const [iframeWidth, setIframeWidth] = useState<number | null>(null);
  const [sandboxUrl, setSandboxUrl] = useState<URL | null>(null);

  // Fetch the sandbox proxy URL
  useEffect(() => {
    fetchMcpAppProxyUrl(resource.csp).then((url) => {
      if (url) {
        setSandboxUrl(new URL(url));
      }
    });
  }, [resource.csp]);

  // Fetch the resource HTML and metadata
  useEffect(() => {
    if (!sessionId) {
      return;
    }

    const fetchResourceData = async () => {
      try {
        const response = await readResource({
          body: {
            session_id: sessionId,
            uri: resourceUri,
            extension_name: extensionName,
          },
        });

        if (response.data) {
          const content = response.data;
          const meta = content._meta as
            | {
                ui?: {
                  csp?: CspMetadata;
                  permissions?: { sandbox?: string };
                  prefersBorder?: boolean;
                };
              }
            | undefined;

          if (content.text !== cachedHtml) {
            setResource({
              html: content.text,
              csp: meta?.ui?.csp || null,
              permissions: meta?.ui?.permissions?.sandbox || null,
              prefersBorder: meta?.ui?.prefersBorder ?? true,
            });
          }
        }
      } catch (err) {
        if (!cachedHtml) {
          setError(errorMessage(err, 'Failed to load resource'));
        } else {
          console.warn('Failed to fetch fresh resource, using cached version:', err);
        }
      }
    };

    fetchResourceData();
  }, [resourceUri, extensionName, sessionId, cachedHtml]);

  // Handler for open-link requests from the guest UI
  const handleOpenLink = useCallback(async ({ url }: { url: string }) => {
    // Safe protocols open directly, unknown protocols require confirmation
    // Dangerous protocols are blocked by main.ts in the open-external handler
    if (isProtocolSafe(url)) {
      await window.electron.openExternal(url);
      return { status: 'success' as const };
    }

    const protocol = getProtocol(url);
    if (!protocol) {
      return { status: 'error' as const, message: 'Invalid URL' };
    }

    const result = await window.electron.showMessageBox({
      type: 'question',
      buttons: ['Cancel', 'Open'],
      defaultId: 0,
      title: 'Open External Link',
      message: `Open ${protocol} link?`,
      detail: `This will open: ${url}`,
    });

    if (result.response !== 1) {
      return { status: 'error' as const, message: 'User cancelled' };
    }

    await window.electron.openExternal(url);
    return { status: 'success' as const };
  }, []);

  // Handler for message requests from the guest UI
  const handleMessage = useCallback(
    async ({ content }: { content: Array<{ type: string; text?: string }> }) => {
      if (!append) {
        throw new Error('Message handler not available in this context');
      }

      if (!Array.isArray(content)) {
        throw new Error('Invalid message format: content must be an array of ContentBlock');
      }

      // Extract first text block from content, ignoring other block types
      const textContent = content.find((block) => block.type === 'text');
      if (!textContent || !textContent.text) {
        throw new Error('Invalid message format: content must contain a text block');
      }

      append(textContent.text);
      window.dispatchEvent(new CustomEvent(AppEvents.SCROLL_CHAT_TO_BOTTOM));
      return {};
    },
    [append]
  );

  // Handler for tools/call requests from the guest UI
  const handleCallTool = useCallback(
    async ({
      name,
      arguments: args,
    }: {
      name: string;
      arguments?: Record<string, unknown>;
    }): Promise<CallToolResult> => {
      if (!sessionId) {
        throw new Error('Session not initialized for MCP request');
      }

      const fullToolName = `${extensionName}__${name}`;
      const response = await callTool({
        body: {
          session_id: sessionId,
          name: fullToolName,
          arguments: args || {},
        },
      });

      // Map from snake_case API response to camelCase SDK types
      const content = response.data?.content || [];
      return {
        content: content.map((item) => {
          if ('text' in item && item.text !== undefined) {
            return { type: 'text' as const, text: item.text };
          }
          if ('data' in item && item.data !== undefined) {
            return {
              type: 'image' as const,
              data: item.data,
              mimeType: item.mimeType || 'image/png',
            };
          }
          // Default to text type for unknown content
          return { type: 'text' as const, text: JSON.stringify(item) };
        }),
        isError: response.data?.is_error || false,
      };
    },
    [sessionId, extensionName]
  );

  // Handler for resources/read requests from the guest UI
  const handleReadResource = useCallback(
    async ({ uri }: { uri: string }) => {
      if (!sessionId) {
        throw new Error('Session not initialized for MCP request');
      }

      const response = await readResource({
        body: {
          session_id: sessionId,
          uri,
          extension_name: extensionName,
        },
      });

      // Map from API response to SDK types
      const data = response.data;
      if (!data) {
        return { contents: [] };
      }

      // Convert to the expected format with required uri field
      const resourceContent = {
        uri: data.uri || uri,
        text: data.text,
        mimeType: data.mimeType || undefined,
      };

      return {
        contents: [resourceContent],
      };
    },
    [sessionId, extensionName]
  );

  // Handler for logging messages from the guest UI
  const handleLoggingMessage = useCallback(
    ({ level, logger, data }: { level?: string; logger?: string; data?: unknown }) => {
      console.log(
        `[MCP App Notification]${logger ? ` [${logger}]` : ''} ${level || 'info'}:`,
        data
      );
    },
    []
  );

  // Handler for size change notifications from the guest UI
  const handleSizeChanged = useCallback(
    ({ height, width }: McpUiSizeChangedNotification['params']) => {
      if (height !== undefined) {
        const newHeight = Math.max(DEFAULT_IFRAME_HEIGHT, height);
        setIframeHeight(newHeight);
      }
      setIframeWidth(width ?? null);
    },
    []
  );

  // Handler for errors
  const handleError = useCallback((err: Error) => {
    console.error('[MCP App Error]:', err);
    setError(errorMessage(err));
  }, []);

  // Convert CspMetadata to McpUiResourceCsp (handle null -> undefined)
  const convertCspToMcpUi = useCallback((csp: CspMetadata | null): McpUiResourceCsp | undefined => {
    if (!csp) return undefined;
    return {
      connectDomains: csp.connectDomains ?? undefined,
      resourceDomains: csp.resourceDomains ?? undefined,
    };
  }, []);

  // Sandbox configuration
  const sandboxConfig = useMemo(() => {
    if (!sandboxUrl) return null;
    return {
      url: sandboxUrl,
      permissions: resource.permissions || 'allow-scripts allow-same-origin allow-forms',
      csp: convertCspToMcpUi(resource.csp),
    };
  }, [sandboxUrl, resource.permissions, resource.csp, convertCspToMcpUi]);

  // Host context for the guest UI
  const hostContext = useMemo(
    () => ({
      theme: resolvedTheme,
      displayMode: fullscreen ? ('fullscreen' as const) : ('inline' as const),
      availableDisplayModes: ['inline' as const, 'fullscreen' as const],
    }),
    [resolvedTheme, fullscreen]
  );

  // Convert toolResult to CallToolResult format expected by AppRenderer
  const appToolResult = useMemo((): CallToolResult | undefined => {
    if (!toolResult) return undefined;
    // Map from snake_case to camelCase
    const content = toolResult.content || [];
    return {
      content: content.map((item) => {
        if ('text' in item && item.text !== undefined) {
          return { type: 'text' as const, text: item.text };
        }
        if ('data' in item && item.data !== undefined) {
          return {
            type: 'image' as const,
            data: item.data,
            mimeType: item.mimeType || 'image/png',
          };
        }
        return { type: 'text' as const, text: JSON.stringify(item) };
      }),
      isError: toolResult.is_error || false,
    };
  }, [toolResult]);

  // Convert toolCancelled to boolean
  const isToolCancelled = toolCancelled?.reason !== undefined;

  if (error) {
    return (
      <div className="p-4 border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20">
        <div className="text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      </div>
    );
  }

  if (!sandboxConfig || !resource.html) {
    return (
      <div
        className={cn('flex items-center justify-center p-4', fullscreen ? 'w-full h-full' : '')}
        style={{ minHeight: fullscreen ? '100%' : '200px' }}
      >
        Loading MCP app...
      </div>
    );
  }

  const containerStyle = fullscreen
    ? { width: '100%', height: '100%' }
    : {
        width: iframeWidth ? `${iframeWidth}px` : '100%',
        maxWidth: '100%',
        height: `${iframeHeight}px`,
      };

  return (
    <div
      className={cn(
        'bg-bgApp overflow-hidden',
        !fullscreen && resource.prefersBorder ? 'border border-borderSubtle rounded-lg' : '',
        fullscreen ? 'w-full h-full' : 'my-6'
      )}
      style={containerStyle}
    >
      <AppRenderer
        sandbox={sandboxConfig}
        toolName={resourceUri}
        html={resource.html}
        toolInput={toolInput?.arguments}
        toolInputPartial={toolInputPartial ? { arguments: toolInputPartial.arguments } : undefined}
        toolResult={appToolResult}
        toolCancelled={isToolCancelled}
        hostContext={hostContext}
        onOpenLink={handleOpenLink}
        onMessage={handleMessage}
        onCallTool={handleCallTool}
        onReadResource={handleReadResource}
        onLoggingMessage={handleLoggingMessage}
        onSizeChanged={handleSizeChanged}
        onError={handleError}
      />
    </div>
  );
}
