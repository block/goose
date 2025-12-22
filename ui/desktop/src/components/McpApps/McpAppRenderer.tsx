/**
 * MCP Apps Renderer
 *
 * Temporary Goose implementation while waiting for official SDK components.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 */

import { useState, useCallback } from 'react';
import { useSandboxBridge } from './useSandboxBridge';
import { McpAppResource, ToolInput, ToolInputPartial, ToolResult, ToolCancelled } from './types';
import { cn } from '../../utils';
import { DEFAULT_IFRAME_HEIGHT } from './utils';

interface McpAppRendererProps {
  resource: McpAppResource;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  append?: (text: string) => void;
}

export default function McpAppRenderer({
  resource,
  toolInput,
  toolInputPartial,
  toolResult,
  toolCancelled,
  append,
}: McpAppRendererProps) {
  const prefersBorder = resource._meta?.ui?.prefersBorder ?? true;
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);

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

        case 'notifications/message':
        case 'tools/call':
        case 'resources/list':
        case 'resources/templates/list':
        case 'resources/read':
        case 'prompts/list':
        case 'ping':
          console.warn(`[MCP App] TODO: ${method} not yet implemented`);
          throw new Error(`Method not implemented: ${method}`);

        default:
          throw new Error(`Unknown method: ${method}`);
      }
    },
    [append]
  );

  const handleSizeChanged = useCallback((height: number, _width?: number) => {
    const newHeight = Math.max(DEFAULT_IFRAME_HEIGHT, height);
    setIframeHeight(newHeight);
  }, []);

  const { iframeRef, proxyUrl } = useSandboxBridge({
    resourceHtml: resource.text || '',
    resourceCsp: resource._meta?.ui?.csp || null,
    resourceUri: resource.uri,
    toolInput,
    toolInputPartial,
    toolResult,
    toolCancelled,
    onMcpRequest: handleMcpRequest,
    onSizeChanged: handleSizeChanged,
  });

  if (!resource) {
    return null;
  }

  return (
    <div
      className={cn(
        'mt-3 bg-bgApp',
        prefersBorder && 'border border-borderSubtle rounded-lg overflow-hidden'
      )}
    >
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
