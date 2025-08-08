import React, { useEffect, useRef, useState } from 'react';
import { Button } from './ui/button';
import { ToolCallArguments, ToolCallArgumentValue } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { Content, ToolRequestMessageContent, ToolResponseMessageContent } from '../types/message';
import { cn, snakeToTitleCase } from '../utils';
import Dot, { LoadingStatus } from './ui/Dot';
import { NotificationEvent } from '../hooks/useMessageStream';
import { ChevronRight, FlaskConical, LoaderCircle } from 'lucide-react';
import { TooltipWrapper } from './settings/providers/subcomponents/buttons/TooltipWrapper';
import MCPUIResourceRenderer from './MCPUIResourceRenderer';

interface ToolCallWithResponseProps {
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
  isStreamingMessage?: boolean;
}

export default function ToolCallWithResponse({
  isCancelledMessage,
  toolRequest,
  toolResponse,
  notifications,
  isStreamingMessage = false,
}: ToolCallWithResponseProps) {
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;
  if (!toolCall) {
    return null;
  }

  return (
    <>
      <div
        className={cn(
          'w-full text-sm rounded-lg overflow-hidden border-borderSubtle border bg-background-muted'
        )}
      >
        <ToolCallView
          {...{
            isCancelledMessage,
            toolCall,
            toolResponse,
            notifications,
            isStreamingMessage,
          }}
        />
      </div>
      {/* MCP UI — Inline */}
      {toolResponse?.toolResult?.value &&
        toolResponse.toolResult.value.map((content, index) => {
          if (content.type === 'resource' && content.resource.uri?.startsWith('ui://')) {
            return (
              <div key={`${content.type}-${index}`} className="mt-3">
                <MCPUIResourceRenderer content={content} />
                <div className="mt-3 p-4 py-3 border border-borderSubtle rounded-lg bg-background-muted flex items-center">
                  <FlaskConical className="mr-2" size={20} />
                  <div className="text-sm font-medium mono">
                    MCP UI is experimental and may change at any time.
                  </div>
                </div>
              </div>
            );
          } else {
            return null;
          }
        })}
    </>
  );
}

interface ToolCallExpandableProps {
  label: string | React.ReactNode;
  isStartExpanded?: boolean;
  isForceExpand?: boolean;
  children: React.ReactNode;
  className?: string;
}

function ToolCallExpandable({
  label,
  isStartExpanded = false,
  isForceExpand,
  children,
  className = '',
}: ToolCallExpandableProps) {
  const [isExpandedState, setIsExpanded] = React.useState<boolean | null>(null);
  const isExpanded = isExpandedState === null ? isStartExpanded : isExpandedState;
  const toggleExpand = () => setIsExpanded(!isExpanded);
  React.useEffect(() => {
    if (isForceExpand) setIsExpanded(true);
  }, [isForceExpand]);

  return (
    <div className={className}>
      <Button
        onClick={toggleExpand}
        className="group w-full flex justify-between items-center pr-2 transition-colors rounded-none"
        variant="ghost"
      >
        <span className="flex items-center font-mono">{label}</span>
        <ChevronRight
          className={cn(
            'group-hover:opacity-100 transition-transform opacity-70',
            isExpanded && 'rotate-90'
          )}
        />
      </Button>
      {isExpanded && <div>{children}</div>}
    </div>
  );
}

interface ToolCallViewProps {
  isCancelledMessage: boolean;
  toolCall: {
    name: string;
    arguments: Record<string, unknown>;
  };
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
  isStreamingMessage?: boolean;
}

interface Progress {
  progress: number;
  progressToken: string;
  total?: number;
  message?: string;
}

const logToString = (logMessage: NotificationEvent) => {
  const params = logMessage.message.params;

  // Special case for the developer system shell logs
  if (
    params &&
    params.data &&
    typeof params.data === 'object' &&
    'output' in params.data &&
    'stream' in params.data
  ) {
    return `[${params.data.stream}] ${params.data.output}`;
  }

  return typeof params.data === 'string' ? params.data : JSON.stringify(params.data);
};

const notificationToProgress = (notification: NotificationEvent): Progress =>
  notification.message.params as unknown as Progress;

// Helper function to extract extension name for tooltip
const getExtensionTooltip = (toolCallName: string): string | null => {
  const lastIndex = toolCallName.lastIndexOf('__');
  if (lastIndex === -1) return null;

  const extensionName = toolCallName.substring(0, lastIndex);
  if (!extensionName) return null;

  return `${extensionName} extension`;
};

function ToolCallView({
  isCancelledMessage,
  toolCall,
  toolResponse,
  notifications,
  isStreamingMessage = false,
}: ToolCallViewProps) {
  const [responseStyle, setResponseStyle] = useState(() => localStorage.getItem('response_style'));

  // Listen for localStorage changes to update the response style
  useEffect(() => {
    const handleStorageChange = () => {
      setResponseStyle(localStorage.getItem('response_style'));
    };

    // Listen for storage events (changes from other tabs/windows)
    window.addEventListener('storage', handleStorageChange);

    // Listen for custom events (changes from same tab)
    window.addEventListener('responseStyleChanged', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      window.removeEventListener('responseStyleChanged', handleStorageChange);
    };
  }, []);

  const isExpandToolDetails = (() => {
    switch (responseStyle) {
      case 'concise':
        return false;
      case 'detailed':
      default:
        return true;
    }
  })();

  const isToolDetails = Object.entries(toolCall?.arguments).length > 0;

  // Check if streaming has finished but no tool response was received
  // This is a workaround for cases where the backend doesn't send tool responses
  const isStreamingComplete = !isStreamingMessage;
  const shouldShowAsComplete = isStreamingComplete && !toolResponse;

  const loadingStatus: LoadingStatus = !toolResponse
    ? shouldShowAsComplete
      ? 'success'
      : 'loading'
    : toolResponse.toolResult.status;

  // Tool call timing tracking
  const [startTime, setStartTime] = useState<number | null>(null);

  // Track when tool call starts (when there's no response yet)
  useEffect(() => {
    if (!toolResponse && startTime === null) {
      setStartTime(Date.now());
    }
  }, [toolResponse, startTime]);

  const toolResults: { result: Content; isExpandToolResults: boolean }[] =
    loadingStatus === 'success' && Array.isArray(toolResponse?.toolResult.value)
      ? toolResponse!.toolResult.value
          .filter((item) => {
            const audience = item.annotations?.audience as string[] | undefined;
            return !audience || audience.includes('user');
          })
          .map((item) => {
            // Use user preference for detailed/concise, but still respect high priority items
            const priority = (item.annotations?.priority as number | undefined) ?? -1;
            const isHighPriority = priority >= 0.5;
            const shouldExpandBasedOnStyle = responseStyle === 'detailed' || responseStyle === null;

            return {
              result: item,
              isExpandToolResults: isHighPriority || shouldExpandBasedOnStyle,
            };
          })
      : [];

  const logs = notifications
    ?.filter((notification) => notification.message.method === 'notifications/message')
    .map(logToString);

  const progress = notifications
    ?.filter((notification) => notification.message.method === 'notifications/progress')
    .map(notificationToProgress)
    .reduce((map, item) => {
      const key = item.progressToken;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(item);
      return map;
    }, new Map<string, Progress[]>());

  const progressEntries = [...(progress?.values() || [])].map(
    (entries) => entries.sort((a, b) => b.progress - a.progress)[0]
  );

  const isRenderingProgress =
    loadingStatus === 'loading' && (progressEntries.length > 0 || (logs || []).length > 0);

  // Determine if the main tool call should be expanded
  const isShouldExpand = (() => {
    // Always expand if there are high priority results that need to be shown
    const hasHighPriorityResults = toolResults.some((v) => v.isExpandToolResults);

    // Also expand based on user preference for detailed mode
    const shouldExpandBasedOnStyle = responseStyle === 'detailed' || responseStyle === null;

    return hasHighPriorityResults || shouldExpandBasedOnStyle;
  })();

  // Function to create a descriptive representation of what the tool is doing
  const getToolDescription = (): string | null => {
    const args = toolCall.arguments as Record<string, ToolCallArgumentValue>;
    const toolName = toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2);

    // Helper function to get string value safely
    const getStringValue = (value: ToolCallArgumentValue): string => {
      return typeof value === 'string' ? value : JSON.stringify(value);
    };

    // Helper function to truncate long values
    const truncate = (str: string, maxLength: number = 50): string => {
      return str.length > maxLength ? str.substring(0, maxLength) + '...' : str;
    };

    // Generate descriptive text based on tool type
    switch (toolName) {
      case 'text_editor':
        if (args.command === 'write' && args.path) {
          return `writing ${truncate(getStringValue(args.path))}`;
        }
        if (args.command === 'view' && args.path) {
          return `reading ${truncate(getStringValue(args.path))}`;
        }
        if (args.command === 'str_replace' && args.path) {
          return `editing ${truncate(getStringValue(args.path))}`;
        }
        if (args.command && args.path) {
          return `${getStringValue(args.command)} ${truncate(getStringValue(args.path))}`;
        }
        break;

      case 'shell':
        if (args.command) {
          return `running ${truncate(getStringValue(args.command))}`;
        }
        break;

      case 'search':
        if (args.name) {
          return `searching for "${truncate(getStringValue(args.name))}"`;
        }
        if (args.mimeType) {
          return `searching for ${getStringValue(args.mimeType)} files`;
        }
        break;

      case 'read': {
        if (args.uri) {
          const uri = getStringValue(args.uri);
          const fileId = uri.replace('gdrive:///', '');
          return `reading file ${truncate(fileId)}`;
        }
        if (args.url) {
          return `reading ${truncate(getStringValue(args.url))}`;
        }
        break;
      }

      case 'create_file':
        if (args.name) {
          return `creating ${truncate(getStringValue(args.name))}`;
        }
        break;

      case 'update_file':
        if (args.fileId) {
          return `updating file ${truncate(getStringValue(args.fileId))}`;
        }
        break;

      case 'sheets_tool': {
        if (args.operation && args.spreadsheetId) {
          const operation = getStringValue(args.operation);
          const sheetId = truncate(getStringValue(args.spreadsheetId));
          return `${operation} in sheet ${sheetId}`;
        }
        break;
      }

      case 'docs_tool': {
        if (args.operation && args.documentId) {
          const operation = getStringValue(args.operation);
          const docId = truncate(getStringValue(args.documentId));
          return `${operation} in document ${docId}`;
        }
        break;
      }

      case 'web_scrape':
        if (args.url) {
          return `scraping ${truncate(getStringValue(args.url))}`;
        }
        break;

      case 'remember_memory':
        if (args.category && args.data) {
          return `storing ${getStringValue(args.category)}: ${truncate(getStringValue(args.data))}`;
        }
        break;

      case 'retrieve_memories':
        if (args.category) {
          return `retrieving ${getStringValue(args.category)} memories`;
        }
        break;

      case 'screen_capture':
        if (args.window_title) {
          return `capturing window "${truncate(getStringValue(args.window_title))}"`;
        }
        return `capturing screen`;

      case 'automation_script':
        if (args.language) {
          return `running ${getStringValue(args.language)} script`;
        }
        break;

      case 'final_output':
        return 'final output';

      case 'computer_control':
        return `poking around...`;

      default: {
        // Generic fallback for unknown tools: ToolName + CompactArguments
        // This ensures any MCP tool works without explicit handling
        const toolDisplayName = snakeToTitleCase(toolName);
        const entries = Object.entries(args);

        if (entries.length === 0) {
          return `${toolDisplayName}`;
        }

        // For a single parameter, show key and truncated value
        if (entries.length === 1) {
          const [key, value] = entries[0];
          const stringValue = getStringValue(value);
          const truncatedValue = truncate(stringValue, 30);
          return `${toolDisplayName} ${key}: ${truncatedValue}`;
        }

        // For multiple parameters, show tool name and keys
        const keys = entries.map(([key]) => key).join(', ');
        return `${toolDisplayName} ${keys}`;
      }
    }

    return null;
  };

  // Get extension tooltip for the current tool
  const extensionTooltip = getExtensionTooltip(toolCall.name);

  // Extract tool label content to avoid duplication
  const getToolLabelContent = () => {
    const description = getToolDescription();
    if (description) {
      return description;
    }
    // Fallback tool name formatting
    return snakeToTitleCase(toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2));
  };

  const toolLabel = (
    <span className={cn('ml-2', extensionTooltip && 'cursor-pointer hover:opacity-80')}>
      {getToolLabelContent()}
    </span>
  );

  return (
    <ToolCallExpandable
      isStartExpanded={isRenderingProgress}
      isForceExpand={isShouldExpand}
      label={
        <>
          <div className="w-2 flex items-center justify-center">
            {loadingStatus === 'loading' ? (
              <LoaderCircle className="animate-spin text-text-muted" size={3} />
            ) : (
              <Dot size={2} loadingStatus={loadingStatus} />
            )}
          </div>
          {extensionTooltip ? (
            <TooltipWrapper tooltipContent={extensionTooltip} side="top" align="start">
              {toolLabel}
            </TooltipWrapper>
          ) : (
            toolLabel
          )}
        </>
      }
    >
      {/* Tool Details */}
      {isToolDetails && (
        <div className="border-t border-borderSubtle">
          <ToolDetailsView toolCall={toolCall} isStartExpanded={isExpandToolDetails} />
        </div>
      )}

      {logs && logs.length > 0 && (
        <div className="border-t border-borderSubtle">
          <ToolLogsView
            logs={logs}
            working={loadingStatus === 'loading'}
            isStartExpanded={
              loadingStatus === 'loading' || responseStyle === 'detailed' || responseStyle === null
            }
          />
        </div>
      )}

      {toolResults.length === 0 &&
        progressEntries.length > 0 &&
        progressEntries.map((entry, index) => (
          <div className="p-3 border-t border-borderSubtle" key={index}>
            <ProgressBar progress={entry.progress} total={entry.total} message={entry.message} />
          </div>
        ))}

      {/* Tool Output */}
      {!isCancelledMessage && (
        <>
          {toolResults.map(({ result, isExpandToolResults }, index) => {
            return (
              <div key={index} className={cn('border-t border-borderSubtle')}>
                <ToolResultView result={result} isStartExpanded={isExpandToolResults} />
              </div>
            );
          })}
        </>
      )}
    </ToolCallExpandable>
  );
}

interface ToolDetailsViewProps {
  toolCall: {
    name: string;
    arguments: Record<string, unknown>;
  };
  isStartExpanded: boolean;
}

function ToolDetailsView({ toolCall, isStartExpanded }: ToolDetailsViewProps) {
  return (
    <ToolCallExpandable
      label={<span className="pl-4 font-medium">Tool Details</span>}
      isStartExpanded={isStartExpanded}
    >
      <div className="pr-4 pl-8">
        {toolCall.arguments && (
          <ToolCallArguments args={toolCall.arguments as Record<string, ToolCallArgumentValue>} />
        )}
      </div>
    </ToolCallExpandable>
  );
}

interface ToolResultViewProps {
  result: Content;
  isStartExpanded: boolean;
}

function ToolResultView({ result, isStartExpanded }: ToolResultViewProps) {
  return (
    <ToolCallExpandable
      label={<span className="pl-4 py-1 font-medium">Output</span>}
      isStartExpanded={isStartExpanded}
    >
      <div className="pl-4 pr-4 py-4">
        {result.type === 'text' && result.text && (
          <MarkdownContent
            content={result.text}
            className="whitespace-pre-wrap max-w-full overflow-x-auto"
          />
        )}
        {result.type === 'image' && (
          <img
            src={`data:${result.mimeType};base64,${result.data}`}
            alt="Tool result"
            className="max-w-full h-auto rounded-md my-2"
            onError={(e) => {
              console.error('Failed to load image');
              e.currentTarget.style.display = 'none';
            }}
          />
        )}
        {result.type === 'resource' && <pre>{JSON.stringify(result, null, 2)}</pre>}
      </div>
    </ToolCallExpandable>
  );
}

function ToolLogsView({
  logs,
  working,
  isStartExpanded,
}: {
  logs: string[];
  working: boolean;
  isStartExpanded?: boolean;
}) {
  const boxRef = useRef<HTMLDivElement>(null);

  // Whenever logs update, jump to the newest entry
  useEffect(() => {
    if (boxRef.current) {
      boxRef.current.scrollTop = boxRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <ToolCallExpandable
      label={
        <span className="pl-4 py-1 font-medium flex items-center">
          <span>Logs</span>
          {working && (
            <div className="mx-2 inline-block">
              <span
                className="inline-block animate-spin rounded-full border-2 border-t-transparent border-current"
                style={{ width: 8, height: 8 }}
                role="status"
                aria-label="Loading spinner"
              />
            </div>
          )}
        </span>
      }
      isStartExpanded={isStartExpanded}
    >
      <div
        ref={boxRef}
        className={`flex flex-col items-start space-y-2 overflow-y-auto p-4 ${working ? 'max-h-[4rem]' : 'max-h-[20rem]'}`}
      >
        {logs.map((log, i) => (
          <span key={i} className="font-mono text-sm text-textSubtle">
            {log}
          </span>
        ))}
      </div>
    </ToolCallExpandable>
  );
}

const ProgressBar = ({ progress, total, message }: Omit<Progress, 'progressToken'>) => {
  const isDeterminate = typeof total === 'number';
  const percent = isDeterminate ? Math.min((progress / total!) * 100, 100) : 0;

  return (
    <div className="w-full space-y-2">
      {message && <div className="text-sm text-textSubtle">{message}</div>}

      <div className="w-full bg-background-subtle rounded-full h-4 overflow-hidden relative">
        {isDeterminate ? (
          <div
            className="bg-primary h-full transition-all duration-300"
            style={{ width: `${percent}%` }}
          />
        ) : (
          <div className="absolute inset-0 animate-indeterminate bg-primary" />
        )}
      </div>
    </div>
  );
};
