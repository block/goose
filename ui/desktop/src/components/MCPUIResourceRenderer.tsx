import { UIResourceRenderer, UIActionResult } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';
import { useCallback } from 'react';
import { toast } from 'react-toastify';

interface MCPUIResourceRendererProps {
  content: ResourceContent;
}

export default function MCPUIResourceRenderer({ content }: MCPUIResourceRendererProps) {
  const stubbledHandleAction = (action: UIActionResult) => {
    console.log(
      `MCP UI message received (but only handled with a toast notification for now):`,
      action
    );
    toast.info(`${action.type} message sent from MCP UI, refer to console for more info`, {
      data: action,
    });
    return { status: 'handled', message: `${action.type} action logged` };
  };

  const handleUIAction = useCallback(async (result: UIActionResult) => {
    switch (result.type) {
      case 'intent': {
        // TODO: Implement intent handling
        stubbledHandleAction(result);
        break;
      }

      case 'link': {
        // TODO: Implement link handling
        stubbledHandleAction(result);
        break;
      }

      case 'notify': {
        // TODO: Implement notify handling
        stubbledHandleAction(result);
        break;
      }

      case 'prompt': {
        // TODO: Implement prompt handling
        stubbledHandleAction(result);
        break;
      }

      case 'tool': {
        // TODO: Implement tool call handling
        stubbledHandleAction(result);
        break;
      }

      default: {
        console.warn('Unknown message sent from MCP-UI:', result);
        break;
      }
    }
  }, []);

  return (
    <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-background-muted">
      <div className="overflow-hidden rounded-sm">
        <UIResourceRenderer
          resource={content.resource}
          onUIAction={handleUIAction}
          htmlProps={{
            autoResizeIframe: true,
          }}
        />
      </div>
    </div>
  );
}
