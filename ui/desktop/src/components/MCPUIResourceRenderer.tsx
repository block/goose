import { UIResourceRenderer, UIActionResult } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';
import { useCallback } from 'react';
import { toast } from 'react-toastify';

interface MCPUIResourceRendererProps {
  content: ResourceContent;
}

export default function MCPUIResourceRenderer({ content }: MCPUIResourceRendererProps) {
  const handleUnsupportedMessage = (type: string) => {
    console.warn(`MCP-UI "${type}" message type not supported`);
    toast.info(`MCP-UI "${type}" message posted, refer to console for more info`);
  };

  const handleUIAction = useCallback(async (result: UIActionResult) => {
    switch (result.type) {
      case 'tool':
        handleUnsupportedMessage('tool');
        break;
      case 'intent':
        handleUnsupportedMessage('intent');
        break;
      case 'prompt':
        handleUnsupportedMessage('prompt');
        break;
      case 'link':
        handleUnsupportedMessage('link');
        break;
      case 'notify':
        handleUnsupportedMessage('notify');
        break;
      default:
        console.log(`MCP-UI message received:`, result);
        break;
    }

    // SUPER IMPORTANT: MCP-UIs depend on receiving a response to their message
    const response = {
      type: 'ui-message-response',
      payload: result,
    };

    console.info(
      `Goose posted the following response message back to the MCP-UI request:`,
      response
    );

    return response;
  }, []);

  return (
    <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-background-muted">
      <div className="overflow-hidden rounded-sm">
        <UIResourceRenderer
          resource={content.resource}
          onUIAction={handleUIAction}
          htmlProps={{
            autoResizeIframe: {
              height: true,
              width: false, // set to false to allow for responsive design
            },
            sandboxPermissions: 'allow-forms',
          }}
        />
      </div>
    </div>
  );
}
