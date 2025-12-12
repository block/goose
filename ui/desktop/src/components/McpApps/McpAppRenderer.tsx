/**
 * MCP Apps Renderer
 *
 * Temporary Goose implementation while waiting for official SDK components.
 *
 * @see SEP-1865 https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx
 */

import { useSandboxBridge } from './useSandboxBridge';
import { McpAppResource } from './types';
import { cn } from '../../utils';

export type { McpAppResource } from './types';

interface McpAppRendererProps {
  resource: McpAppResource;
  appendMessage?: (value: string) => void;
}

export default function McpAppRenderer({ resource, appendMessage }: McpAppRendererProps) {
  const prefersBorder = resource._meta?.ui?.prefersBorder ?? true;

  const { iframeRef, proxyUrl, iframeHeight } = useSandboxBridge({
    resourceHtml: resource.text || '',
    resourceCsp: resource._meta?.ui?.csp || null,
    resourceUri: resource.uri,
    appendMessage,
  });

  if (!resource) {
    return null;
  }

  const debug = true;
  if (debug) {
    console.log('🐛 McpAppRenderer Debug ===================');
    console.log({ resource, prefersBorder, proxyUrl, iframeHeight });
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
