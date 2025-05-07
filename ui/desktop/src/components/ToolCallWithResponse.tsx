import React from 'react';
import { Card } from './ui/card';
import { ToolCallArguments } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { Content, ToolRequestMessageContent, ToolResponseMessageContent } from '../types/message';
import { snakeToTitleCase } from '../utils';
import Dot, { LoadingStatus } from './ui/Dot';
import Expand from './ui/Expand';

interface ToolCallWithResponseProps {
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
}

export default function ToolCallWithResponse({
  isCancelledMessage,
  toolRequest,
  toolResponse,
}: ToolCallWithResponseProps) {
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;
  if (!toolCall) {
    return null;
  }

  return (
    <div className={'w-full text-textSubtle text-sm'}>
      <Card className="">
        <ToolCallView {...{ isCancelledMessage, toolCall, toolResponse }} />
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
  const [isExpanded, setIsExpanded] = React.useState(isStartExpanded);
  const toggleExpand = () => setIsExpanded((prev) => !prev);
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
}

function ToolCallView({ isCancelledMessage, toolCall, toolResponse }: ToolCallViewProps) {
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
      ? toolResponse.toolResult.value
          .filter((item) => {
            const audience = item.annotations?.audience as string[] | undefined;
            return !audience || audience.includes('user');
          })
          .map((item) => ({
            result: item,
            isExpandToolResults: ((item.annotations?.priority as number | undefined) ?? -1) >= 0.5,
          }))
      : [];

  const isShouldExpand = isExpandToolDetails || toolResults.some((v) => v.isExpandToolResults);

  return (
    <ToolCallExpandable
      isStartExpanded={isShouldExpand}
      isForceExpand={isShouldExpand}
      label={
        <>
          <Dot size={2} loadingStatus={loadingStatus} />
          <span className="ml-[10px]">
            {snakeToTitleCase(toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2))}
          </span>
        </>
      }
    >
      {/* Tool Details */}
      {isToolDetails && (
        <div className="bg-bgStandard rounded-t mt-1">
          <ToolDetailsView toolCall={toolCall} isStartExpanded={isExpandToolDetails} />
        </div>
      )}

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
      {toolCall.arguments && <ToolCallArguments args={toolCall.arguments} />}
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
