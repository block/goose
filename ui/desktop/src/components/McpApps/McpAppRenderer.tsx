/**
 * MCP Apps Renderer
 *
 * Temporary Goose implementation while waiting for official SDK components.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 */

import { useState, useCallback, useEffect } from 'react';
import { useSandboxBridge } from './useSandboxBridge';
import {
  ToolInput,
  ToolInputPartial,
  ToolResult,
  ToolCancelled,
  CspMetadata,
  McpMethodParams,
  McpMethodResponse,
} from './types';
import { cn } from '../../utils';
import { DEFAULT_IFRAME_HEIGHT } from './utils';
import { readResource, callTool } from '../../api';

interface McpAppRendererProps {
  resourceUri: string;
  extensionName: string;
  sessionId: string;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  append?: (text: string) => void;
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
}: McpAppRendererProps) {
  const [resourceHtml, setResourceHtml] = useState<string | null>(null);
  const [resourceCsp, setResourceCsp] = useState<CspMetadata | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);

  useEffect(() => {
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

          setResourceHtml(content.text);

          const meta = content._meta as { ui?: { csp?: CspMetadata } } | undefined;
          setResourceCsp(meta?.ui?.csp || null);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load resource');
      }
    };

    fetchResource();
  }, [resourceUri, extensionName, sessionId]);

  const handleMcpRequest = useCallback(
    async (
      method: string,
      params: Record<string, unknown> = {},
      id?: string | number
    ): Promise<unknown> => {
      switch (method) {
        case 'ui/open-link': {
          const { url } = params as McpMethodParams['ui/open-link'];
          await window.electron.openExternal(url);
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
          append(content.text);
          window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
          return {
            status: 'success',
            message: 'Message appended successfully',
          } satisfies McpMethodResponse['ui/message'];
        }

        case 'tools/call': {
          const { name, arguments: args } = params as McpMethodParams['tools/call'];
          const fullToolName = `${extensionName}__${name}`;
          const response = await callTool({
            body: {
              session_id: sessionId,
              name: fullToolName,
              arguments: args || {},
            },
          });
          return {
            content: response.data?.content || [],
            isError: response.data?.is_error || false,
            structuredContent: (response.data as any)?.structured_content,
          } satisfies McpMethodResponse['tools/call'];
        }

        case 'resources/read': {
          const { uri } = params as McpMethodParams['resources/read'];
          const response = await readResource({
            body: {
              session_id: sessionId,
              uri,
              extension_name: extensionName,
            },
          });
          return {
            contents: response.data ? [response.data] : [],
          } satisfies McpMethodResponse['resources/read'];
        }

        case 'notifications/message': {
          const { level, logger, data } = params as McpMethodParams['notifications/message'];
          console.log(
            `[MCP App Notification]${logger ? ` [${logger}]` : ''} ${level || 'info'}:`,
            data
          );
          return {} satisfies McpMethodResponse['notifications/message'];
        }

        case 'ping':
          return {} satisfies McpMethodResponse['ping'];

        default:
          throw new Error(`Unknown method: ${method}`);
      }
    },
    [append, sessionId, extensionName]
  );

  const handleSizeChanged = useCallback((height: number, _width?: number) => {
    const newHeight = Math.max(DEFAULT_IFRAME_HEIGHT, height);
    setIframeHeight(newHeight);
  }, []);

  const { iframeRef, proxyUrl } = useSandboxBridge({
    resourceHtml: resourceHtml || '',
    resourceCsp,
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
      <div className="mt-3 p-4 border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20">
        <div className="text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      </div>
    );
  }

  if (!resourceHtml) {
    return (
      <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-bgApp">
        <div className="flex items-center justify-center" style={{ minHeight: '200px' }}>
          Loading MCP app...
        </div>
      </div>
    );
  }

  return (
    <div className={cn('mt-3 bg-bgApp', 'border border-borderSubtle rounded-lg overflow-hidden')}>
      {proxyUrl ? (
        <iframe
          ref={iframeRef}
          src={proxyUrl}
          style={{
            width: '100%',
            height: `${iframeHeight}px`,
            border: 'none',
            overflow: 'hidden',
          }}
          sandbox="allow-scripts allow-same-origin"
        />
      ) : (
        <div
          style={{
            width: '100%',
            minHeight: '200px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          Loading...
        </div>
      )}
    </div>
  );
}
