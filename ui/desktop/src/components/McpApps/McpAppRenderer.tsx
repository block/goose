/**
 * MCP Apps Renderer
 *
 * Temporary Goose implementation while waiting for official SDK components.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 */

import { useState } from 'react';
import { useSandboxBridge } from './useSandboxBridge';
import { McpAppResource, ToolInput, ToolInputPartial, ToolResult, ToolCancelled } from './types';
import { cn } from '../../utils';
import {
  handleMessage,
  handleOpenLink,
  handleNotificationMessage,
  handleResourcesList,
  handleResourceTemplatesList,
  handleResourcesRead,
  handlePromptsList,
  handlePing,
  handleSizeChanged,
  handleToolsCall,
} from './utils';

const DEFAULT_IFRAME_HEIGHT = 200;

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

  // Note: when @mcp-ui/client SDK provides AppRenderer we will be able to supply these as props to the renderer component
  const { iframeRef, proxyUrl } = useSandboxBridge({
    resourceHtml: resource.text || '',
    resourceCsp: resource._meta?.ui?.csp || null,
    resourceUri: resource.uri,
    iframeHeight,
    toolInput,
    toolInputPartial,
    toolResult,
    toolCancelled,
    onMessage: handleMessage(append),
    onOpenLink: handleOpenLink,
    onNotificationMessage: handleNotificationMessage,
    onResourcesList: handleResourcesList,
    onResourceTemplatesList: handleResourceTemplatesList,
    onResourcesRead: handleResourcesRead,
    onPromptsList: handlePromptsList,
    onPing: handlePing,
    onSizeChanged: handleSizeChanged(setIframeHeight),
    onToolsCall: handleToolsCall,
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
