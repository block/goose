import React, { useEffect, useRef } from 'react';
import { Card } from './ui/card';
import { ToolCallArguments, ToolCallArgumentValue } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { Content, ToolRequestMessageContent, ToolResponseMessageContent } from '../types/message';
import { snakeToTitleCase } from '../utils';
import Dot, { LoadingStatus } from './ui/Dot';
import Expand from './ui/Expand';
import { NotificationEvent } from '../hooks/useMessageStream';

interface ToolCallWithResponseProps {
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
}

export default function ToolCallWithResponse({
  isCancelledMessage,
  toolRequest,
  toolResponse,
  notifications,
}: ToolCallWithResponseProps) {
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;
  if (!toolCall) {
    return null;
  }

  return (
    <div className={'w-full text-textSubtle text-sm'}>
      <Card className="">
        <ToolCallView {...{ isCancelledMessage, toolCall, toolResponse, notifications }} />
      </Card>
    </div>
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
      <button onClick={toggleExpand} className="w-full flex justify-between items-center pr-2">
        <span className="flex items-center">{label}</span>
        <Expand size={5} isExpanded={isExpanded} />
      </button>
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

function ToolCallView({
  isCancelledMessage,
  toolCall,
  toolResponse,
  notifications,
}: ToolCallViewProps) {
  const responseStyle = localStorage.getItem('response_style');
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
  const loadingStatus: LoadingStatus = !toolResponse?.toolResult.status
    ? 'loading'
    : toolResponse?.toolResult.status;

  const toolResults: { result: Content; isExpandToolResults: boolean }[] =
    loadingStatus === 'success' && Array.isArray(toolResponse?.toolResult.value)
      ? toolResponse!.toolResult.value
          .filter((item) => {
            const audience = item.annotations?.audience as string[] | undefined;
            return !audience || audience.includes('user');
          })
          .map((item) => ({
            result: item,
            isExpandToolResults: ((item.annotations?.priority as number | undefined) ?? -1) >= 0.5,
          }))
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

  const isShouldExpand = isExpandToolDetails || toolResults.some((v) => v.isExpandToolResults);

  // Function to create a compact representation of arguments
  const getCompactArguments = () => {
    const args = toolCall.arguments as Record<string, ToolCallArgumentValue>;
    const entries = Object.entries(args);

    if (entries.length === 0) return null;

    // For a single parameter, show key and truncated value
    if (entries.length === 1) {
      const [key, value] = entries[0];
      const stringValue = typeof value === 'string' ? value : JSON.stringify(value);
      const truncatedValue =
        stringValue.length > 30 ? stringValue.substring(0, 30) + '...' : stringValue;

      return (
        <span className="ml-2 text-textSubtle truncate text-xs opacity-70">
          {key}: {truncatedValue}
        </span>
      );
    }

    // For multiple parameters, just show the keys
    return (
      <span className="ml-2 text-textSubtle truncate text-xs opacity-70">
        {entries.map(([key]) => key).join(', ')}
      </span>
    );
  };

  return (
    <ToolCallExpandable
      isStartExpanded={isShouldExpand || isRenderingProgress}
      isForceExpand={isShouldExpand}
      label={
        <>
          <Dot size={2} loadingStatus={loadingStatus} />
          <span className="ml-[10px]">
            {snakeToTitleCase(toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2))}
          </span>
          {/* Display compact arguments inline */}
          {isToolDetails && getCompactArguments()}
        </>
      }
    >
      {/* Tool Details */}
      {isToolDetails && (
        <div className="bg-bgStandard rounded-t mt-1">
          <ToolDetailsView toolCall={toolCall} isStartExpanded={isExpandToolDetails} />
        </div>
      )}

      {logs && logs.length > 0 && (
        <div className="bg-bgStandard mt-1">
          <ToolLogsView
            logs={logs}
            working={toolResults.length === 0}
            isStartExpanded={toolResults.length === 0}
          />
        </div>
      )}

      {toolResults.length === 0 &&
        progressEntries.length > 0 &&
        progressEntries.map((entry, index) => (
          <div className="p-2" key={index}>
            <ProgressBar progress={entry.progress} total={entry.total} message={entry.message} />
          </div>
        ))}

      {/* Tool Output */}
      {!isCancelledMessage && (
        <>
          {toolResults.map(({ result, isExpandToolResults }, index) => {
            const isLast = index === toolResults.length - 1;
            return (
              <div
                key={index}
                className={`bg-bgStandard mt-1 
                  ${isToolDetails || index > 0 ? '' : 'rounded-t'} 
                  ${isLast ? 'rounded-b' : ''}
                `}
              >
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
      label="Tool Details"
      className="pl-[19px] py-1"
      isStartExpanded={isStartExpanded}
    >
      {toolCall.arguments && (
        <ToolCallArguments args={toolCall.arguments as Record<string, ToolCallArgumentValue>} />
      )}
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
      label={<span className="pl-[19px] py-1">Output</span>}
      isStartExpanded={isStartExpanded}
    >
      <div className="bg-bgApp rounded-b pl-[19px] pr-2 py-4">
        {result.type === 'text' && result.text && (
          <MarkdownContent
            content={result.text}
            className="whitespace-pre-wrap p-2 max-w-full overflow-x-auto"
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
        <span className="pl-[19px] py-1">
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
        className={`flex flex-col items-start space-y-2 overflow-y-auto ${working ? 'max-h-[4rem]' : 'max-h-[20rem]'} bg-bgApp`}
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
      {message && <div className="text-sm text-gray-700">{message}</div>}

      <div className="w-full bg-gray-200 rounded-full h-4 overflow-hidden relative">
        {isDeterminate ? (
          <div
            className="bg-blue-500 h-full transition-all duration-300"
            style={{ width: `${percent}%` }}
          />
        ) : (
          <div className="absolute inset-0 animate-indeterminate bg-blue-500" />
        )}
      </div>
    </div>
  );
};
