import { UIResourceRenderer, UIActionResult } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';
import { useState, useCallback } from 'react';

// Extend UIActionResult to include size-change type
type ExtendedUIActionResult =
  | UIActionResult
  | {
      type: 'size-change';
      payload: {
        height: string;
      };
    };

interface MCPUIResourceRendererProps {
  content: ResourceContent;
}

export default function MCPUIResourceRenderer({ content }: MCPUIResourceRendererProps) {
  console.log('MCPUIResourceRenderer', content);
  const [iframeHeight, setIframeHeight] = useState('200px');

  const handleUIAction = useCallback(async (result: ExtendedUIActionResult) => {
    console.log('Handle action from MCP UI Action:', result);

    // Handle UI actions here
    switch (result.type) {
      case 'intent':
        // TODO: Implement intent handling
        break;

      case 'link':
        // TODO: Implement link handling
        break;

      case 'notify':
        // TODO: Implement notification handling
        break;

      case 'prompt':
        // TODO: Implement prompt handling
        break;

      case 'tool':
        // TODO: Implement tool handling
        break;

      // Currently, `size-change` is non-standard
      case 'size-change': {
        // We expect the height to be a string with a unit
        console.log('Setting iframe height to:', result.payload.height);
        setIframeHeight(result.payload.height);
        break;
      }
    }

    return { status: 'handled' };
  }, []);

  return (
    <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-background-muted">
      <div className="overflow-hidden rounded-sm">
        <UIResourceRenderer
          resource={content.resource}
          onUIAction={handleUIAction}
          htmlProps={{
            style: { minHeight: iframeHeight },
          }}
        />
      </div>
    </div>
  );
}
