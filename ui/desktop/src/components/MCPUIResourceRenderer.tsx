import { UIResourceRenderer, UIActionResult } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';
import { useCallback } from 'react';
import { toast } from 'react-toastify';

// TODOS
// support ui-lifecycle-iframe-ready
// support size-change

interface MCPUIResourceRendererProps {
  content: ResourceContent;
  // Optional callbacks for when we actually implement these actions
  onToolCall?: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  onPrompt?: (prompt: string) => Promise<void>;
  onNavigate?: (url: string) => void;
  onIntent?: (intent: string, params: Record<string, unknown>) => Promise<void>;
}

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

// Specific result types for each action
type ToolCallResult = {
  toolName: string;
  executionTime: number;
  output: unknown;
};

type NotificationResult = {
  notificationId?: string;
  displayedAt: string;
  message: string;
};

export default function MCPUIResourceRenderer({
  content,
  onToolCall,
  onPrompt,
  onNavigate,
  onIntent,
}: MCPUIResourceRendererProps) {
  // Separate handlers for each action type for better type safety
  const handleToolAction = useCallback(
    async (
      toolName: string,
      params: Record<string, unknown>
    ): Promise<UIActionHandlerResult<ToolCallResult>> => {
      if (!onToolCall) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.UNSUPPORTED_ACTION,
            message: 'Tool calls are not yet implemented',
            details: { toolName, params },
          },
        };
      }

      const startTime = Date.now();
      try {
        const output = await onToolCall(toolName, params);
        return {
          status: 'success',
          data: {
            toolName,
            executionTime: Date.now() - startTime,
            output,
          },
          message: `Tool "${toolName}" executed successfully`,
        };
      } catch (error) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.TOOL_EXECUTION_FAILED,
            message: `Failed to execute tool "${toolName}"`,
            details: error instanceof Error ? error.message : error,
          },
        };
      }
    },
    [onToolCall]
  );

  const handlePromptAction = useCallback(
    async (prompt: string): Promise<UIActionHandlerResult<void>> => {
      if (!onPrompt) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.UNSUPPORTED_ACTION,
            message: 'Prompt handling is not yet implemented',
            details: { prompt },
          },
        };
      }

      try {
        await onPrompt(prompt);
        return {
          status: 'success',
          message: 'Prompt sent successfully',
        };
      } catch (error) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.PROMPT_FAILED,
            message: 'Failed to send prompt',
            details: error instanceof Error ? error.message : error,
          },
        };
      }
    },
    [onPrompt]
  );

  const handleLinkAction = useCallback(
    async (url: string): Promise<UIActionHandlerResult<void>> => {
      if (!onNavigate) {
        // For links, we provide a default implementation using Electron shell
        try {
          // Validate URL before opening
          const urlObj = new URL(url);

          // Only allow http/https protocols for security
          if (!['http:', 'https:'].includes(urlObj.protocol)) {
            return {
              status: 'error',
              error: {
                code: UIActionErrorCode.NAVIGATION_FAILED,
                message: `Blocked potentially unsafe URL protocol: ${urlObj.protocol}`,
                details: { url, protocol: urlObj.protocol },
              },
            };
          }

          // Use the exposed electron API for secure external URL opening
          // This calls the main process via IPC, which then uses shell.openExternal()
          await window.electron.openExternal(url);

          return {
            status: 'success',
            message: `Opened ${url} in default browser`,
          };
        } catch (error) {
          // Handle different types of errors
          if (error instanceof TypeError && error.message.includes('Invalid URL')) {
            return {
              status: 'error',
              error: {
                code: UIActionErrorCode.INVALID_PARAMS,
                message: `Invalid URL format: ${url}`,
                details: { url, error: error.message },
              },
            };
          }

          if (error instanceof Error && error.message.includes('Failed to open')) {
            return {
              status: 'error',
              error: {
                code: UIActionErrorCode.NAVIGATION_FAILED,
                message: `Failed to open URL in default browser`,
                details: { url, error: error.message },
              },
            };
          }

          return {
            status: 'error',
            error: {
              code: UIActionErrorCode.NAVIGATION_FAILED,
              message: `Unexpected error opening URL: ${url}`,
              details: error instanceof Error ? error.message : error,
            },
          };
        }
      }

      try {
        onNavigate(url);
        return {
          status: 'success',
          message: `Navigated to ${url}`,
        };
      } catch (error) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.NAVIGATION_FAILED,
            message: `Failed to navigate to ${url}`,
            details: error instanceof Error ? error.message : error,
          },
        };
      }
    },
    [onNavigate]
  );

  const handleNotifyAction = useCallback(
    (message: string): UIActionHandlerResult<NotificationResult> => {
      try {
        const notificationId = `notify-${Date.now()}`;
        toast.info(message);

        return {
          status: 'success',
          data: {
            notificationId,
            displayedAt: new Date().toISOString(),
            message,
          },
        };
      } catch (error) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.UNKNOWN_ACTION,
            message: 'Failed to display notification',
            details: error instanceof Error ? error.message : error,
          },
        };
      }
    },
    []
  );

  const handleIntentAction = useCallback(
    async (
      intent: string,
      params: Record<string, unknown>
    ): Promise<UIActionHandlerResult<void>> => {
      if (!onIntent) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.UNSUPPORTED_ACTION,
            message: 'Intent handling is not yet implemented',
            details: { intent, params },
          },
        };
      }

      try {
        await onIntent(intent, params);
        return {
          status: 'success',
          message: `Intent "${intent}" processed successfully`,
        };
      } catch (error) {
        return {
          status: 'error',
          error: {
            code: UIActionErrorCode.INTENT_FAILED,
            message: `Failed to process intent "${intent}"`,
            details: error instanceof Error ? error.message : error,
          },
        };
      }
    },
    [onIntent]
  );

  // Main handler with exhaustive type checking
  const handleUIAction = useCallback(
    async (actionEvent: UIActionResult): Promise<UIActionHandlerResult> => {
      console.log('[MCP-UI] Action received:', actionEvent);

      let result: UIActionHandlerResult;

      try {
        switch (actionEvent.type) {
          case 'tool':
            result = await handleToolAction(
              actionEvent.payload.toolName,
              actionEvent.payload.params
            );
            break;

          case 'prompt':
            result = await handlePromptAction(actionEvent.payload.prompt);
            break;

          case 'link':
            result = await handleLinkAction(actionEvent.payload.url);
            break;

          case 'notify':
            result = handleNotifyAction(actionEvent.payload.message);
            break;

          case 'intent':
            result = await handleIntentAction(
              actionEvent.payload.intent,
              actionEvent.payload.params
            );
            break;

          default: {
            // TypeScript exhaustiveness check
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

      // Log result with appropriate level
      if (result.status === 'error') {
        console.error('[MCP-UI] Action failed:', result);
      } else if (result.status === 'pending') {
        console.info('[MCP-UI] Action pending:', result);
      } else {
        console.log('[MCP-UI] Action succeeded:', result);
      }

      return result;
    },
    [handleToolAction, handlePromptAction, handleLinkAction, handleNotifyAction, handleIntentAction]
  );

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
