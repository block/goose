/**
 * MCP Apps Renderer
 *
 * Temporary Goose implementation while waiting for official SDK components.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 */

import { useState, useCallback, useEffect } from 'react';
import { useSandboxBridge } from './useSandboxBridge';
import { ToolInput, ToolInputPartial, ToolResult, ToolCancelled, CspMetadata } from './types';
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
  const [error, setError] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);
  // TODO: Get CSP from backend when supported
  const resourceCsp: CspMetadata | null = null;

  // Fetch the HTML resource from the MCP server
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
          setResourceHtml(response.data.html);
          // TODO: Extract CSP from resource metadata when backend supports it
          // For now, CSP will be null and the proxy will use default restrictions
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load resource');
      }
    };

    fetchResource();
  }, [resourceUri, extensionName, sessionId]);

  // Handle MCP requests from the guest app
  const handleMcpRequest = useCallback(
    async (method: string, params: unknown, id?: string | number): Promise<unknown> => {
      console.log(`[MCP App] Request: ${method}`, { params, id });

      switch (method) {
        case 'ui/open-link':
          if (params && typeof params === 'object' && 'url' in params) {
            const { url } = params as { url: string };
            window.electron.openExternal(url).catch(console.error);
            return { status: 'success', message: 'Link opened successfully' };
          }
          throw new Error('Invalid params for ui/open-link');

        case 'ui/message':
          if (params && typeof params === 'object' && 'content' in params) {
            const content = params.content as { type: string; text: string };
            if (!append) {
              throw new Error('Message handler not available in this context');
            }
            if (!content.text) {
              throw new Error('Missing message text');
            }
            append(content.text);
            window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
            return { status: 'success', message: 'Message appended successfully' };
          }
          throw new Error('Invalid params for ui/message');

        case 'tools/call':
          if (params && typeof params === 'object' && 'name' in params) {
            const { name, arguments: args } = params as {
              name: string;
              arguments?: Record<string, unknown>;
            };
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
            };
          }
          throw new Error('Invalid params for tools/call');

        case 'resources/read':
          if (params && typeof params === 'object' && 'uri' in params) {
            const { uri } = params as { uri: string };
            const response = await readResource({
              body: {
                session_id: sessionId,
                uri,
                extension_name: extensionName,
              },
            });
            return {
              contents: response.data ? [response.data] : [],
            };
          }
          throw new Error('Invalid params for resources/read');

        case 'notifications/message':
        case 'resources/list':
        case 'resources/templates/list':
        case 'prompts/list':
        case 'ping':
          console.warn(`[MCP App] TODO: ${method} not yet implemented`);
          throw new Error(`Method not implemented: ${method}`);

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
