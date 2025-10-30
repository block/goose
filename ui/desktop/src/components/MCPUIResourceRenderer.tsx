import {
  UIResourceRenderer,
  UIActionResultIntent,
  UIActionResultLink,
  UIActionResultNotification,
  UIActionResultPrompt,
  UIActionResultToolCall,
} from '@mcp-ui/client';
import { useState, useEffect } from 'react';
import { toast } from 'react-toastify';
import { EmbeddedResource } from '../api';

interface MCPUIResourceRendererProps {
  content: EmbeddedResource & { type: 'resource' };
  appendPromptToChat?: (value: string) => void;
}

type UISizeChange = {
  type: 'ui-size-change';
  payload: {
    height: number;
    width: number;
  };
};

// Reserved message types from iframe to host
type UILifecycleIframeReady = {
  type: 'ui-lifecycle-iframe-ready';
  payload?: Record<string, unknown>;
};

type UIRequestData = {
  type: 'ui-request-data';
  messageId: string;
  payload: {
    requestType: string;
    params: Record<string, unknown>;
  };
};

// We are creating a new type to support all reserved message types that may come from the iframe
// Not all reserved message types are currently exported by @mcp-ui/client
type ActionEventsFromIframe =
  | UIActionResultIntent
  | UIActionResultLink
  | UIActionResultNotification
  | UIActionResultPrompt
  | UIActionResultToolCall
  | UISizeChange
  | UILifecycleIframeReady
  | UIRequestData;

// More specific result types using discriminated unions
type UIActionHandlerSuccess<T = unknown> = {
  status: 'success';
  data?: T;
  message?: string;
};

type UIActionHandlerError = {
  status: 'error';
  error: {
    code: UIActionErrorCode;
    message: string;
    details?: unknown;
  };
};

type UIActionHandlerPending = {
  status: 'pending';
  message: string;
};

type UIActionHandlerResult<T = unknown> =
  | UIActionHandlerSuccess<T>
  | UIActionHandlerError
  | UIActionHandlerPending;

// Strongly typed error codes
enum UIActionErrorCode {
  UNSUPPORTED_ACTION = 'UNSUPPORTED_ACTION',
  UNKNOWN_ACTION = 'UNKNOWN_ACTION',
  TOOL_NOT_FOUND = 'TOOL_NOT_FOUND',
  TOOL_EXECUTION_FAILED = 'TOOL_EXECUTION_FAILED',
  NAVIGATION_FAILED = 'NAVIGATION_FAILED',
  PROMPT_FAILED = 'PROMPT_FAILED',
  INTENT_FAILED = 'INTENT_FAILED',
  INVALID_PARAMS = 'INVALID_PARAMS',
  NETWORK_ERROR = 'NETWORK_ERROR',
  TIMEOUT = 'TIMEOUT',
}

// toast component
const ToastComponent = ({
  messageType,
  message,
  isImplemented = true,
}: {
  messageType: string;
  message?: string;
  isImplemented?: boolean;
}) => {
  const title = `MCP-UI ${messageType} message`;

  return (
    <div className="flex flex-col gap-0 py-2 pr-4">
      <p className="font-bold">{title}</p>
      {isImplemented ? (
        <p>
          Message received for <span className="font-bold">{message}</span>.
        </p>
      ) : (
        <p>
          Message received for <span className="font-bold">{message}</span>.
          <br />
          {messageType.charAt(0).toUpperCase() + messageType.slice(1)} messages aren't supported
          yet, refer to console for more details.
        </p>
      )}
    </div>
  );
};

export default function MCPUIResourceRenderer({
  content,
  appendPromptToChat,
}: MCPUIResourceRendererProps) {
  const [currentThemeValue, setCurrentThemeValue] = useState<string>('light');

  useEffect(() => {
    const theme = localStorage.getItem('theme') || 'light';
    setCurrentThemeValue(theme);
    console.log('[MCP-UI] Current theme value:', theme);
  }, []);

  const handleUIAction = async (
    actionEvent: ActionEventsFromIframe
  ): Promise<UIActionHandlerResult> => {
    // result to pass back to the MCP-UI
    let result: UIActionHandlerResult;

    const handleToolCase = async (
      actionEvent: UIActionResultToolCall
    ): Promise<UIActionHandlerResult> => {
      const { toolName, params } = actionEvent.payload;
      toast.info(<ToastComponent messageType="tool" message={toolName} isImplemented={false} />, {
        theme: currentThemeValue,
      });
      return {
        status: 'error' as const,
        error: {
          code: UIActionErrorCode.UNSUPPORTED_ACTION,
          message: 'Tool calls are not yet implemented',
          details: { toolName, params },
        },
      };
    };

    const handlePromptCase = async (
      actionEvent: UIActionResultPrompt
    ): Promise<UIActionHandlerResult> => {
      const { prompt } = actionEvent.payload;

      if (appendPromptToChat) {
        try {
          appendPromptToChat(prompt);
          window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
          return {
            status: 'success' as const,
            message: 'Prompt sent to chat successfully',
          };
        } catch (error) {
          return {
            status: 'error' as const,
            error: {
              code: UIActionErrorCode.PROMPT_FAILED,
              message: 'Failed to send prompt to chat',
              details: error instanceof Error ? error.message : error,
            },
          };
        }
      }

      return {
        status: 'error' as const,
        error: {
          code: UIActionErrorCode.UNSUPPORTED_ACTION,
          message: 'Prompt handling is not implemented - append prop is required',
          details: { prompt },
        },
      };
    };

    const handleLinkCase = async (actionEvent: UIActionResultLink) => {
      const { url } = actionEvent.payload;

      try {
        const urlObj = new URL(url);
        if (!['http:', 'https:'].includes(urlObj.protocol)) {
          return {
            status: 'error' as const,
            error: {
              code: UIActionErrorCode.NAVIGATION_FAILED,
              message: `Blocked potentially unsafe URL protocol: ${urlObj.protocol}`,
              details: { url, protocol: urlObj.protocol },
            },
          };
        }

        await window.electron.openExternal(url);
        return {
          status: 'success' as const,
          message: `Opened ${url} in default browser`,
        };
      } catch (error) {
        if (error instanceof TypeError && error.message.includes('Invalid URL')) {
          return {
            status: 'error' as const,
            error: {
              code: UIActionErrorCode.INVALID_PARAMS,
              message: `Invalid URL format: ${url}`,
              details: { url, error: error.message },
            },
          };
        } else if (error instanceof Error && error.message.includes('Failed to open')) {
          return {
            status: 'error' as const,
            error: {
              code: UIActionErrorCode.NAVIGATION_FAILED,
              message: `Failed to open URL in default browser`,
              details: { url, error: error.message },
            },
          };
        } else {
          return {
            status: 'error' as const,
            error: {
              code: UIActionErrorCode.NAVIGATION_FAILED,
              message: `Unexpected error opening URL: ${url}`,
              details: error instanceof Error ? error.message : error,
            },
          };
        }
      }
    };

    const handleNotifyCase = async (
      actionEvent: UIActionResultNotification
    ): Promise<UIActionHandlerResult> => {
      const { message } = actionEvent.payload;

      toast.info(<ToastComponent messageType="notify" message={message} isImplemented={true} />, {
        theme: currentThemeValue,
      });
      return {
        status: 'success' as const,
        data: {
          displayedAt: new Date().toISOString(),
          message: 'Notification displayed',
          details: actionEvent.payload,
        },
      };
    };

    const handleIntentCase = async (
      actionEvent: UIActionResultIntent
    ): Promise<UIActionHandlerResult> => {
      toast.info(
        <ToastComponent
          messageType="intent"
          message={actionEvent.payload.intent}
          isImplemented={false}
        />,
        {
          theme: currentThemeValue,
        }
      );
      return {
        status: 'error' as const,
        error: {
          code: UIActionErrorCode.UNSUPPORTED_ACTION,
          message: 'Intent handling is not yet implemented',
          details: actionEvent.payload,
        },
      };
    };

    const handleSizeChangeCase = async (
      actionEvent: UISizeChange
    ): Promise<UIActionHandlerResult> => {
      return {
        status: 'success' as const,
        message: 'Size change handled',
        data: actionEvent.payload,
      };
    };

    const handleIframeReadyCase = async (
      actionEvent: UILifecycleIframeReady
    ): Promise<UIActionHandlerResult> => {
      console.log('[MCP-UI] Iframe ready to receive messages');
      return {
        status: 'success' as const,
        message: 'Iframe is ready to receive messages',
        data: actionEvent.payload,
      };
    };

    const handleRequestDataCase = async (
      actionEvent: UIRequestData
    ): Promise<UIActionHandlerResult> => {
      const { messageId, payload } = actionEvent;
      const { requestType, params } = payload;
      console.log('[MCP-UI] Data request received:', { messageId, requestType, params });
      return {
        status: 'success' as const,
        message: `Data request received: ${requestType}`,
        data: {
          messageId,
          requestType,
          params,
          response: { status: 'acknowledged' },
        },
      };
    };

    try {
      switch (actionEvent.type) {
        case 'tool':
          result = await handleToolCase(actionEvent);
          break;

        case 'prompt':
          result = await handlePromptCase(actionEvent);
          break;

        case 'link':
          result = await handleLinkCase(actionEvent);
          break;

        case 'notify':
          result = await handleNotifyCase(actionEvent);
          break;

        case 'intent':
          result = await handleIntentCase(actionEvent);
          break;

        case 'ui-size-change':
          result = await handleSizeChangeCase(actionEvent);
          break;

        case 'ui-lifecycle-iframe-ready':
          result = await handleIframeReadyCase(actionEvent);
          break;

        case 'ui-request-data':
          result = await handleRequestDataCase(actionEvent);
          break;

        default: {
          const _exhaustiveCheck: never = actionEvent;
          console.error('Unhandled action type:', _exhaustiveCheck);
          result = {
            status: 'error',
            error: {
              code: UIActionErrorCode.UNKNOWN_ACTION,
              message: `Unknown action type`,
              details: actionEvent,
            },
          };
        }
      }
    } catch (error) {
      console.error('[MCP-UI] Unexpected error:', error);
      result = {
        status: 'error',
        error: {
          code: UIActionErrorCode.UNKNOWN_ACTION,
          message: 'An unexpected error occurred',
          details: error instanceof Error ? error.stack : error,
        },
      };
    }

    if (result.status === 'error') {
      console.error('[MCP-UI] Action failed:', result);
    } else {
      console.log('[MCP-UI] Action succeeded:', result);
    }

    return result;
  };

  return (
    <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-background-muted">
      <div className="overflow-hidden rounded-sm">
        <UIResourceRenderer
          resource={content.resource}
          onUIAction={handleUIAction}
          supportedContentTypes={['rawHtml', 'externalUrl']} // Goose does not support remoteDOM content
          htmlProps={{
            autoResizeIframe: {
              height: true,
              width: false, // set to false to allow for responsive design
            },
            sandboxPermissions: 'allow-forms', // enabled for experimentation, is spread into underlying iframe defaults
            iframeRenderData: {
              // iframeRenderData allows us to pass data down to MCP-UIs
              // MPC-UIs might find stuff like host and theme for conditional rendering
              // usage of this is experimental, leaving in place for demos
              host: 'goose',
              theme: currentThemeValue,
            },
          }}
        />
      </div>
    </div>
  );
}
