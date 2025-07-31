/**
 * MCPUIResourceRenderer Component
 *
 * This component renders MCP (Model Context Protocol) UI resources using the @mcp-ui/client package.
 * It handles interactive UI components that can be sent from MCP servers and rendered in the client.
 *
 * Features:
 * - Renders interactive HTML content in sandboxed iframes
 * - Supports external URL embedding
 * - Handles Remote DOM resources
 * - Processes UI actions from embedded content
 * - Provides fallback display for non-interactive resources
 *
 * Usage:
 * ```tsx
 * <MCPUIResourceRenderer
 *   content={resourceContent}
 *   onUIAction={async (result) => {
 *     console.log('UI Action:', result);
 *     return { status: 'handled' };
 *   }}
 * />
 * ```
 *
 * The component automatically detects resource types:
 * - Resources with 'ui://' URIs are rendered as interactive components
 * - Other resources are displayed as fallback content
 * - Non-resource content types are ignored
 */

import { UIResourceRenderer, UIActionResult } from '@mcp-ui/client';
import { Content } from '../types/message';
import { useState } from 'react';

// Extend UIActionResult to include size-change type
type ExtendedUIActionResult =
  | UIActionResult
  | {
      type: 'size-change';
      payload: {
        height: number;
      };
    };

interface MCPUIResourceRendererProps {
  content: Content;
}

export default function MCPUIResourceRenderer({ content }: MCPUIResourceRendererProps) {
  const [iframeHeight, setIframeHeight] = useState(200);

  // Check if this is a resource content with ui:// URI
  if (content.type === 'resource' && content.resource.mimeType === undefined) {
    console.error('Missing mimeType', content);
    return;
  }

  const handleUIAction = async (result: ExtendedUIActionResult) => {
    console.log('MCP UI Action:', result);

    if (result.type === 'size-change') {
      console.log('MCP UI Size Change:', result.payload);

      setIframeHeight(result.payload.height);
    }

    // Handle UI actions here
    return { status: 'handled' };
  };

  if (content.type === 'resource' && content.resource.uri?.startsWith('ui://')) {
    console.log('MCP UI Resource:', content);
    return (
      <>
        <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-background-muted">
          <div className="overflow-hidden rounded-sm">
            <UIResourceRenderer
              resource={content.resource}
              onUIAction={handleUIAction}
              htmlProps={{
                style: { minHeight: iframeHeight + 'px' },
              }}
            />
          </div>
        </div>
      </>
    );
  }

  // For non-resource content types, return null
  return null;
}
