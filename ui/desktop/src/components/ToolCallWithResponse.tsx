import React from 'react';
import { Card } from './ui/card';
import Box from './ui/Box';
import { ToolCallArguments } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { LoadingPlaceholder } from './LoadingPlaceholder';
import { ChevronUp } from 'lucide-react';
import { Content, ToolRequestMessageContent, ToolResponseMessageContent } from '../types/message';
import { snakeToTitleCase } from '../utils';

interface ToolCallWithResponseProps {
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
}

export default function ToolCallWithResponse({
  toolRequest,
  toolResponse,
}: ToolCallWithResponseProps) {
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;

  if (!toolCall) {
    return null;
  }

  return (
    <div className="w-full">
      <Card className="">
        <ToolCallView toolCall={toolCall} />
        {toolResponse ? (
          <ToolResultView
            result={
              toolResponse.toolResult.status === 'success'
                ? toolResponse.toolResult.value
                : undefined
            }
          />
        ) : (
          <LoadingPlaceholder />
        )}
      </Card>
    </div>
  );
}

interface ToolCallViewProps {
  toolCall: {
    name: string;
    arguments: Record<string, unknown>;
  };
}

function ToolCallView({ toolCall }: ToolCallViewProps) {
  return (
    <div>
      <div className="flex items-center mb-4">
        <Box size={16} />
        <span className="ml-[8px] text-textStandard">
          {snakeToTitleCase(toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2))}
        </span>
      </div>

      {toolCall.arguments && <ToolCallArguments args={toolCall.arguments} />}

      <div className="self-stretch h-px my-[10px] -mx-4 bg-borderSubtle dark:bg-gray-700" />
    </div>
  );
}

interface ToolResultViewProps {
  result?: Content[];
}

function ToolResultView({ result }: ToolResultViewProps) {
  // State to track expanded items
  const [expandedItems, setExpandedItems] = React.useState<number[]>([]);

  // If no result info, don't show anything
  if (!result) return null;

  // Find results where either audience is not set, or it's set to a list that includes user
  const filteredResults = result.filter((item) => {
    if (!item.annotations) {
      return false;
    }
    // Check audience (which may not be in the type)
    const audience = item.annotations?.audience;

    return !audience || audience.includes('user');
  });

  if (filteredResults.length === 0) return null;

  const toggleExpand = (index: number) => {
    setExpandedItems((prev) =>
      prev.includes(index) ? prev.filter((i) => i !== index) : [...prev, index]
    );
  };

  const shouldShowExpanded = (item: Content, index: number) => {
    return (
      (item.annotations.priority !== undefined && item.annotations.priority >= 0.5) ||
      expandedItems.includes(index)
    );
  };

  return (
    <div className="">
      {filteredResults.map((item, index) => {
        const isExpanded = shouldShowExpanded(item, index);
        const shouldMinimize =
          item.annotations.priority === undefined || item.annotations.priority < 0.5;
        return (
          <div key={index} className="relative">
            {shouldMinimize && (
              <button
                onClick={() => toggleExpand(index)}
                className="mb-1 flex items-center text-textStandard"
              >
                <span className="mr-2 text-sm">Output</span>
                <ChevronUp
                  className={`h-5 w-5 transition-all origin-center ${!isExpanded ? 'rotate-180' : ''}`}
                />
              </button>
            )}
            {(isExpanded || !shouldMinimize) && (
              <>
                {item.text && (
                  <MarkdownContent
                    content={item.text}
                    className="whitespace-pre-wrap p-2 max-w-full overflow-x-auto"
                  />
                )}
              </>
            )}
          </div>
        );
      })}
    </div>
  );
}
