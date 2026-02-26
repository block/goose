import { AppEvents } from '@/constants/events';
/**
 * MCP Apps Renderer
 *
 * Temporary Goose implementation while waiting for official SDK components.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 */

import { useCallback, useEffect, useState } from 'react';
import { callTool, readResource } from '@/api';
import { cn } from '@/utils';
import { errorMessage } from '@/utils/conversionUtils';
import { getProtocol, isProtocolSafe } from '@/utils/urlSecurity';
import type {
  CspMetadata,
  McpMethodParams,
  McpMethodResponse,
  PermissionsMetadata,
  ToolCancelled,
  ToolInput,
  ToolInputPartial,
  ToolResult,
} from './types';
import { useSandboxBridge } from './useSandboxBridge';
import { DEFAULT_IFRAME_HEIGHT } from './utils';

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
  permissions: PermissionsMetadata | null;
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
  const [resource, setResource] = useState<ResourceData>({
    html: cachedHtml || null,
    csp: null,
    permissions: null,
    prefersBorder: true,
  });
  const [error, setError] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);
  const [iframeWidth, setIframeWidth] = useState<number | null>(null);

  useEffect(() => {
    if (!sessionId) {
      return;
    }

    const fetchResource = async () => {
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
                  permissions?: PermissionsMetadata;
                  prefersBorder?: boolean;
                };
              }
            | undefined;

          if (content.text !== cachedHtml) {
            setResource({
              html: content.text,
              csp: meta?.ui?.csp || null,
              permissions: meta?.ui?.permissions || null,
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

    fetchResource();
  }, [resourceUri, extensionName, sessionId, cachedHtml]);

  const handleMcpRequest = useCallback(
    async (
      method: string,
      params: Record<string, unknown> = {},
      _id?: string | number
    ): Promise<unknown> => {
      // Methods that require a session
      const requiresSession = ['tools/call', 'resources/read'];
      if (requiresSession.includes(method) && !sessionId) {
        throw new Error('Session not initialized for MCP request');
      }

      switch (method) {
        case 'ui/open-link': {
          const { url } = params as McpMethodParams['ui/open-link'];

          // Safe protocols open directly, unknown protocols require confirmation
          // Dangerous protocols are blocked by main.ts in the open-external handler
          if (isProtocolSafe(url)) {
            await window.electron.openExternal(url);
          } else {
            const protocol = getProtocol(url);
            if (!protocol) {
              return {
                status: 'error',
                message: 'Invalid URL',
              } as McpMethodResponse['ui/open-link'];
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
              return {
                status: 'error',
                message: 'User cancelled',
              } as McpMethodResponse['ui/open-link'];
            }
            await window.electron.openExternal(url);
          }

          return {
            status: 'success',
            message: 'Link opened successfully',
          } satisfies McpMethodResponse['ui/open-link'];
        }

        case 'ui/message': {
          const { content } = params as McpMethodParams['ui/message'];
          if (!append) {
            throw new Error('Message handler not available in this context');
          }

          if (!Array.isArray(content)) {
            throw new Error('Invalid message format: content must be an array of ContentBlock');
          }

          // Extract first text block from content, ignoring other block types
          const textContent = content.find((block) => block.type === 'text');
          if (!textContent) {
            throw new Error('Invalid message format: content must contain a text block');
          }

          // MCP Apps can send other content block types, but we only append text blocks for now

          append(textContent.text);
          window.dispatchEvent(new CustomEvent(AppEvents.SCROLL_CHAT_TO_BOTTOM));
          return {} satisfies McpMethodResponse['ui/message'];
        }

        case 'tools/call': {
          const { name, arguments: args } = params as McpMethodParams['tools/call'];
          const fullToolName = `${extensionName}__${name}`;
          const sid = sessionId;
          if (!sid) {
            throw new Error('Session not initialized for MCP request');
          }
          const response = await callTool({
            body: {
              session_id: sid,
              name: fullToolName,
              arguments: args || {},
            },
          });
          return {
            content: response.data?.content || [],
            isError: response.data?.is_error || false,
            structuredContent: (response.data as Record<string, unknown>)?.structured_content as
              | Record<string, unknown>
              | undefined,
          } satisfies McpMethodResponse['tools/call'];
        }

        case 'resources/read': {
          const { uri } = params as McpMethodParams['resources/read'];
          const sid = sessionId;
          if (!sid) {
            throw new Error('Session not initialized for MCP request');
          }
          const response = await readResource({
            body: {
              session_id: sid,
              uri,
              extension_name: extensionName,
            },
          });
          return {
            contents: response.data ? [response.data] : [],
          } satisfies McpMethodResponse['resources/read'];
        }

        case 'notifications/message':
          return {} satisfies McpMethodResponse['notifications/message'];

        case 'ping':
          return {} satisfies McpMethodResponse['ping'];

        default:
          throw new Error(`Unknown method: ${method}`);
      }
    },
    [append, sessionId, extensionName]
  );

  const handleSizeChanged = useCallback((height: number, width?: number) => {
    const newHeight = Math.max(DEFAULT_IFRAME_HEIGHT, height);
    setIframeHeight(newHeight);
    setIframeWidth(width ?? null);
  }, []);

  const { iframeRef, proxyUrl } = useSandboxBridge({
    resourceHtml: resource.html || '',
    resourceCsp: resource.csp,
    resourcePermissions: resource.permissions,
    resourceUri,
    toolInput,
    toolInputPartial,
    toolResult,
    toolCancelled,
    onMcpRequest: handleMcpRequest,
    onSizeChanged: handleSizeChanged,
  });

  if (error) {
    return (
      <div className="p-4 border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20">
        <div className="text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      </div>
    );
  }

  if (fullscreen) {
    return proxyUrl ? (
      <iframe
        ref={iframeRef}
        src={proxyUrl}
        title="MCP app"
        style={{
          width: '100%',
          height: '100%',
          border: 'none',
        }}
        sandbox="allow-scripts allow-same-origin"
      />
    ) : (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        Loading...
      </div>
    );
  }

  return (
    <div
      className={cn(
        'bg-background-default overflow-hidden',
        resource.prefersBorder ? 'border border-border-default rounded-lg' : 'my-6'
      )}
    >
      {resource.html && proxyUrl ? (
        <iframe
          ref={iframeRef}
          src={proxyUrl}
          title="MCP app"
          style={{
            width: iframeWidth ? `${iframeWidth}px` : '100%',
            maxWidth: '100%',
            height: `${iframeHeight}px`,
            border: 'none',
            overflow: 'hidden',
          }}
          sandbox="allow-scripts allow-same-origin"
        />
      ) : (
        <div className="flex items-center justify-center p-4" style={{ minHeight: '200px' }}>
          Loading MCP app...
        </div>
      )}
    </div>
  );
}
