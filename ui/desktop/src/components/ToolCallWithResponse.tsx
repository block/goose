import { ToolIconWithStatus, ToolCallStatus } from './ToolCallStatusIndicator';
import { getToolCallIcon } from '../utils/toolIconMapping';
import React, { useEffect, useRef, useState, useMemo } from 'react';
import { Button } from './ui/button';
import { ToolCallArguments, ToolCallArgumentValue } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import {
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  NotificationEvent,
} from '../types/message';
import { cn, snakeToTitleCase } from '../utils';
import { LoadingStatus } from './ui/Dot';
import { ChevronRight, FlaskConical } from 'lucide-react';
import { TooltipWrapper } from './settings/providers/subcomponents/buttons/TooltipWrapper';
import MCPUIResourceRenderer from './MCPUIResourceRenderer';
import { isUIResource } from '@mcp-ui/client';
import { CallToolResponse, Content, EmbeddedResource } from '../api';
import McpAppRenderer from './McpApps/McpAppRenderer';

interface ToolGraphNode {
  tool: string;
  description: string;
  depends_on: number[];
}

type UiMeta = {
  ui?: {
    resourceUri?: string;
  };
};

type ToolResultWithMeta = {
  status?: string;
  value?: CallToolResponse & {
    _meta?: UiMeta;
  };
};

type ToolRequestWithMeta = ToolRequestMessageContent & {
  _meta?: UiMeta;
  toolCall: {
    status: 'success';
    value: {
      name: string;
      arguments?: Record<string, unknown>;
    };
  };
};

interface ToolCallWithResponseProps {
  sessionId?: string;
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
  isStreamingMessage?: boolean;
  append?: (value: string) => void;
}

function getToolResultContent(toolResult: Record<string, unknown>): Content[] {
  if (toolResult.status !== 'success') {
    return [];
  }
  const value = toolResult.value as CallToolResponse;
  return value.content.filter((item) => {
    const annotations = (item as { annotations?: { audience?: string[] } }).annotations;
    return !annotations?.audience || annotations.audience.includes('user');
  });
}

function isEmbeddedResource(content: Content): content is EmbeddedResource {
  return 'resource' in content && typeof (content as Record<string, unknown>).resource === 'object';
}

interface McpAppWrapperProps {
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
  sessionId: string;
  append?: (value: string) => void;
}

function McpAppWrapper({
  toolRequest,
  toolResponse,
  sessionId,
  append,
}: McpAppWrapperProps): React.ReactNode {
  const requestWithMeta = toolRequest as ToolRequestWithMeta;
  let resourceUri = requestWithMeta._meta?.ui?.resourceUri;

  if (!resourceUri && toolResponse) {
    const resultWithMeta = toolResponse.toolResult as ToolResultWithMeta;
    if (resultWithMeta?.status === 'success' && resultWithMeta.value) {
      resourceUri = resultWithMeta.value._meta?.ui?.resourceUri;
    }
  }

  const extensionName =
    requestWithMeta.toolCall.status === 'success'
      ? requestWithMeta.toolCall.value.name.split('__')[0]
      : '';

  const toolArguments =
    requestWithMeta.toolCall.status === 'success'
      ? requestWithMeta.toolCall.value.arguments
      : undefined;

  // Memoize toolInput to prevent unnecessary re-renders
  const toolInput = useMemo(() => ({ arguments: toolArguments || {} }), [toolArguments]);

  // Memoize toolResult to prevent unnecessary re-renders
  const toolResult = useMemo(() => {
    if (!toolResponse) return undefined;
    const resultWithMeta = toolResponse.toolResult as ToolResultWithMeta;
    if (resultWithMeta?.status === 'success' && resultWithMeta.value) {
      return resultWithMeta.value;
    }
    return undefined;
  }, [toolResponse]);

  if (!resourceUri) return null;
  if (requestWithMeta.toolCall.status !== 'success') return null;

  return (
    <div className="mt-3">
      <McpAppRenderer
        resourceUri={resourceUri}
        toolInput={toolInput}
        toolResult={toolResult}
        extensionName={extensionName}
        sessionId={sessionId}
        append={append}
      />
      <div className="mt-3 p-4 py-3 border border-borderSubtle rounded-lg bg-background-muted flex items-center">
        <FlaskConical className="mr-2" size={20} />
        <div className="text-sm font-sans">
          MCP Apps are experimental and may change at any time.
        </div>
      </div>
    </div>
  );
}

export default function ToolCallWithResponse({
  sessionId,
  isCancelledMessage,
  toolRequest,
  toolResponse,
  notifications,
  isStreamingMessage,
  append,
}: ToolCallWithResponseProps) {
  // Handle both the wrapped ToolResult format and the unwrapped format
  // The server serializes ToolResult<T> as { status: "success", value: T } or { status: "error", error: string }
  const toolCallData = toolRequest.toolCall as Record<string, unknown>;
  const toolCall =
    toolCallData?.status === 'success'
      ? (toolCallData.value as { name: string; arguments: Record<string, unknown> })
      : (toolCallData as { name: string; arguments: Record<string, unknown> });

  if (!toolCall || !toolCall.name) {
    return null;
  }

  const requestWithMeta = toolRequest as ToolRequestWithMeta;
  const resultWithMeta = toolResponse?.toolResult as ToolResultWithMeta;
  const hasMcpAppResourceURI = Boolean(
    requestWithMeta._meta?.ui?.resourceUri || resultWithMeta?.value?._meta?.ui?.resourceUri
  );

  return (
    <>
      <div
        className={cn(
          'w-full text-sm font-sans rounded-lg overflow-hidden border-borderSubtle border bg-background-muted'
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
      {!hasMcpAppResourceURI &&
        toolResponse?.toolResult &&
        getToolResultContent(toolResponse.toolResult).map((content, index) => {
          const resourceContent = isEmbeddedResource(content)
            ? { ...content, type: 'resource' as const }
            : null;
          if (resourceContent && isUIResource(resourceContent)) {
            return (
              <div key={index} className="mt-3">
                <MCPUIResourceRenderer content={resourceContent} appendPromptToChat={append} />
                <div className="mt-3 p-4 py-3 border border-borderSubtle rounded-lg bg-background-muted flex items-center">
                  <FlaskConical className="mr-2" size={20} />
                  <div className="text-sm font-sans">
                    MCP UI is experimental and may change at any time.
                  </div>
                </div>
              </div>
            );
          } else {
            return null;
          }
        })}

      {hasMcpAppResourceURI && sessionId && (
        <McpAppWrapper
          toolRequest={toolRequest}
          toolResponse={toolResponse}
          sessionId={sessionId}
          append={append}
        />
      )}
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
        <span className="flex items-center font-sans text-sm truncate flex-1 min-w-0">{label}</span>
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
  const message = logMessage.message as { method: string; params: unknown };
  const params = message.params as Record<string, unknown>;

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

const notificationToProgress = (notification: NotificationEvent): Progress => {
  const message = notification.message as { method: string; params: unknown };
  return message.params as Progress;
};

// Helper function to extract toolcall name
const getToolName = (toolCallName: string): string => {
  const lastIndex = toolCallName.lastIndexOf('__');
  if (lastIndex === -1) return toolCallName;

  return toolCallName.substring(lastIndex + 2);
};

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

  useEffect(() => {
    const handleStorageChange = () => {
      setResponseStyle(localStorage.getItem('response_style'));
    };

    window.addEventListener('storage', handleStorageChange);

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

  const isToolDetails = toolCall?.arguments && Object.entries(toolCall.arguments).length > 0;

  // Check if streaming has finished but no tool response was received
  // This is a workaround for cases where the backend doesn't send tool responses
  const isStreamingComplete = !isStreamingMessage;
  const shouldShowAsComplete = isStreamingComplete && !toolResponse;

  const loadingStatus: LoadingStatus = !toolResponse
    ? shouldShowAsComplete
      ? 'success'
      : 'loading'
    : (toolResponse.toolResult as Record<string, unknown>).status === 'error'
      ? 'error'
      : 'success';

  // Tool call timing tracking
  const [startTime, setStartTime] = useState<number | null>(null);

  // Track when tool call starts (when there's no response yet)
  useEffect(() => {
    if (!toolResponse && startTime === null) {
      setStartTime(Date.now());
    }
  }, [toolResponse, startTime]);

  const toolResults =
    loadingStatus === 'success' && toolResponse?.toolResult
      ? getToolResultContent(toolResponse.toolResult)
      : [];

  const logs = notifications
    ?.filter((notification) => {
      const message = notification.message as { method?: string };
      return message.method === 'notifications/message';
    })
    .map(logToString);

  const progress = notifications
    ?.filter((notification) => {
      const message = notification.message as { method?: string };
      return message.method === 'notifications/progress';
    })
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

  // Function to create a descriptive representation of what the tool is doing
  const getToolDescription = (): string | null => {
    const args = toolCall.arguments as Record<string, ToolCallArgumentValue>;
    const toolName = getToolName(toolCall.name);

    const getStringValue = (value: ToolCallArgumentValue): string => {
      return typeof value === 'string' ? value : JSON.stringify(value);
    };

    // Generate descriptive text based on tool type
    switch (toolName) {
      case 'text_editor':
        if (args.command === 'write' && args.path) {
          return `writing ${getStringValue(args.path)}`;
        }
        if (args.command === 'view' && args.path) {
          return `reading ${getStringValue(args.path)}`;
        }
        if (args.command === 'str_replace' && args.path) {
          return `editing ${getStringValue(args.path)}`;
        }
        if (args.command && args.path) {
          return `${getStringValue(args.command)} ${getStringValue(args.path)}`;
        }
        break;

      case 'shell':
        if (args.command) {
          return `running ${getStringValue(args.command)}`;
        }
        break;

      case 'search':
        if (args.name) {
          return `searching for "${getStringValue(args.name)}"`;
        }
        if (args.mimeType) {
          return `searching for ${getStringValue(args.mimeType)} files`;
        }
        break;

      case 'read': {
        if (args.uri) {
          const uri = getStringValue(args.uri);
          const fileId = uri.replace('gdrive:///', '');
          return `reading file ${fileId}`;
        }
        if (args.url) {
          return `reading ${getStringValue(args.url)}`;
        }
        break;
      }

      case 'create_file':
        if (args.name) {
          return `creating ${getStringValue(args.name)}`;
        }
        break;

      case 'update_file':
        if (args.fileId) {
          return `updating file ${getStringValue(args.fileId)}`;
        }
        break;

      case 'sheets_tool': {
        if (args.operation && args.spreadsheetId) {
          const operation = getStringValue(args.operation);
          const sheetId = getStringValue(args.spreadsheetId);
          return `${operation} in sheet ${sheetId}`;
        }
        break;
      }

      case 'docs_tool': {
        if (args.operation && args.documentId) {
          const operation = getStringValue(args.operation);
          const docId = getStringValue(args.documentId);
          return `${operation} in document ${docId}`;
        }
        break;
      }

      case 'web_scrape':
        if (args.url) {
          return `scraping ${getStringValue(args.url)}`;
        }
        break;

      case 'remember_memory':
        if (args.category && args.data) {
          return `storing ${getStringValue(args.category)}: ${getStringValue(args.data)}`;
        }
        break;

      case 'retrieve_memories':
        if (args.category) {
          return `retrieving ${getStringValue(args.category)} memories`;
        }
        break;

      case 'screen_capture':
        if (args.window_title) {
          return `capturing window "${getStringValue(args.window_title)}"`;
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

      case 'execute_code': {
        const toolGraph = args.tool_graph as unknown as ToolGraphNode[] | undefined;
        if (toolGraph && Array.isArray(toolGraph) && toolGraph.length > 0) {
          if (toolGraph.length === 1) {
            return `${toolGraph[0].description}`;
          }
          if (toolGraph.length === 2) {
            return `${toolGraph[0].tool}, ${toolGraph[1].tool}`;
          }
          return `${toolGraph.length} tools used`;
        }
        return 'executing code';
      }

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
          return `${toolDisplayName} ${key}: ${stringValue}`;
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
    return snakeToTitleCase(getToolName(toolCall.name));
  };
  // Map LoadingStatus to ToolCallStatus
  const getToolCallStatus = (loadingStatus: LoadingStatus): ToolCallStatus => {
    switch (loadingStatus) {
      case 'success':
        return 'success';
      case 'error':
        return 'error';
      case 'loading':
        return 'loading';
      default:
        return 'pending';
    }
  };

  const toolCallStatus = getToolCallStatus(loadingStatus);

  const toolLabel = (
    <span
      className={cn(
        'flex items-center gap-2 min-w-0',
        extensionTooltip && 'cursor-pointer hover:opacity-80'
      )}
    >
      <ToolIconWithStatus ToolIcon={getToolCallIcon(toolCall.name)} status={toolCallStatus} />
      <span className="truncate flex-1 min-w-0">{getToolLabelContent()}</span>
    </span>
  );
  return (
    <ToolCallExpandable
      isStartExpanded={isRenderingProgress}
      isForceExpand={false}
      label={
        extensionTooltip ? (
          <TooltipWrapper tooltipContent={extensionTooltip} side="top" align="start">
            {toolLabel}
          </TooltipWrapper>
        ) : (
          toolLabel
        )
      }
    >
      {(() => {
        const toolName = toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2);
        const toolGraph = toolCall.arguments?.tool_graph as unknown as ToolGraphNode[] | undefined;
        const code = toolCall.arguments?.code as unknown as string | undefined;
        const hasToolGraph =
          toolName === 'execute_code' &&
          toolGraph &&
          Array.isArray(toolGraph) &&
          toolGraph.length > 0;

        if (hasToolGraph) {
          return (
            <div className="border-t border-borderSubtle">
              <ToolGraphView toolGraph={toolGraph} code={code} />
            </div>
          );
        }

        if (isToolDetails) {
          return (
            <div className="border-t border-borderSubtle">
              <ToolDetailsView toolCall={toolCall} isStartExpanded={isExpandToolDetails} />
            </div>
          );
        }

        return null;
      })()}

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
          {toolResults.map((result, index) => (
            <div key={index} className={cn('border-t border-borderSubtle')}>
              <ToolResultView result={result} isStartExpanded={false} />
            </div>
          ))}
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
      label={<span className="pl-4 font-sans text-sm">Tool Details</span>}
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

interface ToolGraphViewProps {
  toolGraph: ToolGraphNode[];
  code?: string;
}

function ToolGraphView({ toolGraph, code }: ToolGraphViewProps) {
  const renderGraph = () => {
    if (toolGraph.length === 0) return null;

    const lines: string[] = [];

    toolGraph.forEach((node, index) => {
      const deps =
        node.depends_on.length > 0 ? ` (uses ${node.depends_on.map((d) => d + 1).join(', ')})` : '';
      lines.push(`${index + 1}. ${node.tool}: ${node.description}${deps}`);
    });

    return lines.join('\n');
  };

  return (
    <div className="px-4 py-2">
      <pre className="font-mono text-xs text-textSubtle whitespace-pre-wrap">{renderGraph()}</pre>
      {code && (
        <div className="border-t border-borderSubtle -mx-4 mt-2">
          <ToolCallExpandable
            label={<span className="pl-4 font-sans text-sm">Code</span>}
            isStartExpanded={false}
          >
            <pre className="font-mono text-xs text-textSubtle whitespace-pre-wrap overflow-x-auto px-4 py-2">
              {code}
            </pre>
          </ToolCallExpandable>
        </div>
      )}
    </div>
  );
}

interface ToolResultViewProps {
  result: Content;
  isStartExpanded: boolean;
}

function ToolResultView({ result, isStartExpanded }: ToolResultViewProps) {
  const hasText = (c: Content): c is Content & { text: string } =>
    'text' in c && typeof (c as Record<string, unknown>).text === 'string';

  const hasImage = (c: Content): c is Content & { data: string; mimeType: string } => {
    if (!('data' in c && 'mimeType' in c)) return false;
    const mimeType = (c as Record<string, unknown>).mimeType;
    return typeof mimeType === 'string' && mimeType.startsWith('image');
  };

  const hasResource = (c: Content): c is Content & { resource: unknown } => 'resource' in c;

  return (
    <ToolCallExpandable
      label={<span className="pl-4 py-1 font-sans text-sm">Output</span>}
      isStartExpanded={isStartExpanded}
    >
      <div className="pl-4 pr-4 py-4">
        {hasText(result) && (
          <MarkdownContent
            content={result.text}
            className="whitespace-pre-wrap max-w-full overflow-x-auto"
          />
        )}
        {hasImage(result) && (
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
        {hasResource(result) && (
          <pre className="font-sans text-sm">{JSON.stringify(result, null, 2)}</pre>
        )}
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
  }, [logs.length]);
  // normally we do not want to put .length on an array in react deps:
  //
  // if the objects inside the array change but length doesn't change you want updates
  //
  // in this case, this is array of strings which once added do not change so this cuts
  // down on the possibility of unwanted runs

  return (
    <ToolCallExpandable
      label={
        <span className="pl-4 py-1 font-sans text-sm flex items-center">
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
          <span key={i} className="font-sans text-sm text-textSubtle">
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
      {message && <div className="font-sans text-sm text-textSubtle">{message}</div>}

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
