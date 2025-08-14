import { UIResourceRenderer, UIActionResult } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';
import { useCallback } from 'react';
import { toast } from 'react-toastify';
import { getApiUrl } from '../config';

interface MCPUIResourceRendererProps {
  content: ResourceContent;
}

export default function MCPUIResourceRenderer({ content }: MCPUIResourceRendererProps) {
  // Safe access helpers to avoid `any`
  const getProp = useCallback((obj: unknown, key: string): unknown => {
    return obj && typeof obj === 'object' && key in (obj as Record<string, unknown>)
      ? (obj as Record<string, unknown>)[key]
      : undefined;
  }, []);
  const getString = useCallback(
    (obj: unknown, key: string): string | undefined => {
      const v = getProp(obj, key);
      return typeof v === 'string' ? v : undefined;
    },
    [getProp]
  );
  const getRecord = useCallback(
    (obj: unknown, key: string): Record<string, unknown> | undefined => {
      const v = getProp(obj, key);
      return v && typeof v === 'object' && !Array.isArray(v)
        ? (v as Record<string, unknown>)
        : undefined;
    },
    [getProp]
  );

  const handleAction = (action: UIActionResult, showToast: boolean = true) => {
    console.log(
      `MCP UI message received (but only handled with a toast notification for now):`,
      action
    );
    if (showToast) {
      toast.info(`${action.type} message sent from MCP UI, refer to console for more info`, {
        data: action,
      });
    }
    return { status: 'handled', message: `${action.type} action logged` };
  };

  const handleUIAction = useCallback(
    async (result: UIActionResult) => {
      switch (result.type) {
        case 'intent': {
          // TODO: Implement intent handling
          handleAction(result);
          break;
        }

        case 'link': {
          // TODO: Implement link handling
          handleAction(result);
          break;
        }

        case 'notify': {
          // TODO: Implement notify handling
          handleAction(result);
          break;
        }

        case 'prompt': {
          // TODO: Implement prompt handling
          handleAction(result);
          break;
        }

        case 'tool': {
          // Execute the tool directly via the backend
          try {
            // Extract tool information from the result
            // The MCP UI sends payload with toolName and params
            const toolData = getProp(result, 'payload') ?? getProp(result, 'data') ?? result;

            const toolName =
              getString(toolData, 'toolName') ||
              getString(toolData, 'tool') ||
              getString(toolData, 'name');
            // MCP UI sends 'params' not 'arguments'
            const toolArguments =
              getRecord(toolData, 'params') ||
              getRecord(toolData, 'parameters') ||
              getRecord(toolData, 'arguments') ||
              {};

            if (!toolName) {
              throw new Error('Tool name not found in action result');
            }

            const requestBody = {
              tool_name: toolName,
              arguments: toolArguments,
              session_id: null, // You may want to pass the current session ID if available
            };

            const response = await fetch(getApiUrl('/agent/execute_tool'), {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
                'X-Secret-Key': await window.electron.getSecretKey(),
              },
              body: JSON.stringify(requestBody),
            });

            if (!response.ok) {
              const errorText = await response.text();
              throw new Error(`Tool execution failed: ${errorText}`);
            }

            const data = await response.json();

            if (data.success) {
              // Emit event so chat can display tool request/response
              try {
                window.dispatchEvent(
                  new CustomEvent('external-tool-executed', {
                    detail: { toolName, toolArguments, result: data.result },
                  })
                );
                // Request chat to scroll to bottom to reveal new messages
                window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
              } catch (e) {
                // ignore event dispatch errors
              }
              handleAction(result, false);
            } else {
              throw new Error(data.error || 'Tool execution failed');
            }
          } catch (error) {
            // toast.error(
            //   `Failed to execute tool: ${error instanceof Error ? error.message : String(error)}`
            // );
            console.error(error);
          }
          break;
        }

        default: {
          console.warn('unsupported message sent from MCP-UI:', result);
          break;
        }
      }
    },
    [getProp, getRecord, getString]
  );

  return (
    <div className="p-1 border border-borderSubtle rounded-lg bg-background-muted">
      <div className="overflow-hidden rounded-sm">
        <UIResourceRenderer
          resource={content.resource}
          onUIAction={handleUIAction}
          htmlProps={{
            style: { height: '400px', minHeight: 0 },
            autoResizeIframe: {
              height: true,
              width: false, // set to false to allow for responsive design
            },
          }}
        />
      </div>
    </div>
  );
}
