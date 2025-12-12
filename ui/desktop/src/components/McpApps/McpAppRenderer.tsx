/**
 * MCP Apps Renderer (SEP-1865)
 *
 * This component renders MCP Apps - interactive UI resources from MCP servers.
 * It implements the SEP-1865 specification for host-side rendering using the
 * official @modelcontextprotocol/ext-apps AppBridge.
 *
 * This is a temporary local implementation based on the mcp-ui PR #147.
 * Once that PR merges and @mcp-ui/client is updated, this can be replaced
 * with the official AppRenderer component.
 *
 * @see https://github.com/MCP-UI-Org/mcp-ui/pull/147
 */

import { useRef } from 'react';
import { MockReadResourceItem } from './types';
import { cn } from '../../utils';

export default function McpAppRenderer({ resource }: { resource: MockReadResourceItem }) {
  const iframeContainer = useRef<HTMLDivElement>(null);
  const prefersBorder = resource._meta?.ui?.prefersBorder ?? true;

  if (!resource) {
    return null;
  }

  // TODOs
  // 1. Create outer sandboxed iframe with a source of our MCP apps proxyHTML.
  // 2. Create inner iframe and inject HTML and CSP from the resources metadata.
  // 3. Add post message helper and set up iframe relay.
  // 4. Handle resizing
  // 5. Handle sending in tool input and tool results.
  // 6. Handle messages that should go to the apps MCP server.
  // 7. Handle messages that should be sent to the host.

  const debug = true;
  if (debug) {
    console.log('🐛 McpAppRenderer Debug ===================');
    console.log({ resource, prefersBorder });
  }
  return (
    <div
      className={cn(
        'mt-3 bg-bgApp',
        prefersBorder && 'border border-borderSubtle rounded-lg overflow-hidden'
      )}
    >
      <div
        ref={iframeContainer}
        style={{
          width: '100%',
          minHeight: '200px',
        }}
      />
    </div>
  );
}
