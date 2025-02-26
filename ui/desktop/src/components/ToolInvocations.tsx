import React from 'react';
import { Card } from './ui/card';
import Box from './ui/Box';
import { ToolCallArguments } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { snakeToTitleCase } from '../utils';
import { LoadingPlaceholder } from './LoadingPlaceholder';
import { ChevronUp } from 'lucide-react';
import { Content } from '../types/message';

interface ToolInvocation {
  toolCallId: string;
  toolName: string;
  args: any;
  state: 'running' | 'result';
  result?: Content[];
}

interface ToolInvocationsProps {
  toolInvocations: ToolInvocation[];
}

export default function ToolInvocations({ toolInvocations }: ToolInvocationsProps) {
  return (
    <>
      {toolInvocations.map((toolInvocation) => (
        <ToolInvocation key={toolInvocation.toolCallId} toolInvocation={toolInvocation} />
      ))}
    </>
  );
}

function ToolInvocation({ toolInvocation }: { toolInvocation: ToolInvocation }) {
  return (
    <div className="w-full">
      <Card className="">
        <ToolCall call={toolInvocation} />
        {toolInvocation.state === 'result' ? (
          <ToolResult result={toolInvocation} />
        ) : (
          <LoadingPlaceholder />
        )}
      </Card>
    </div>
  );
}

interface ToolCallProps {
  call: {
    state: 'running' | 'result';
    toolCallId: string;
    toolName: string;
    args: Record<string, any>;
  };
}

function ToolCall({ call }: ToolCallProps) {
  return (
    <div>
      <div className="flex items-center mb-4">
        <Box size={16} />
        <span className="ml-[8px] text-textStandard">
          {snakeToTitleCase(call.toolName.substring(call.toolName.lastIndexOf('__') + 2))}
        </span>
      </div>

      {call.args && <ToolCallArguments args={call.args} />}

      <div className="self-stretch h-px my-[10px] -mx-4 bg-borderSubtle dark:bg-gray-700" />
    </div>
  );
}

interface ToolResultProps {
  result: {
    result?: Content[];
    state?: string;
    toolCallId?: string;
    toolName?: string;
    args?: any;
  };
}

function ToolResult({ result }: ToolResultProps) {
  // State to track expanded items
  const [expandedItems, setExpandedItems] = React.useState<number[]>([]);

  // If no result info, don't show anything
  if (!result || !result.result) return null;

  // Normalize to an array
  const results = Array.isArray(result.result) ? result.result : [result.result];

  // Find results where either audience is not set, or it's set to a list that contains user
  const filteredResults = results.filter(
    (item) => !item.audience || item.audience?.includes('user')
  );

  if (filteredResults.length === 0) return null;

  const toggleExpand = (index: number) => {
    setExpandedItems((prev) =>
      prev.includes(index) ? prev.filter((i) => i !== index) : [...prev, index]
    );
  };

  const shouldShowExpanded = (item: Content, index: number) => {
    // (priority is defined and > 0.5) OR already in the expandedItems
    return (
      (item.priority !== undefined && item.priority >= 0.5) ||
      expandedItems.includes(index)
    );
  };

  return (
    <div className="">
      {filteredResults.map((item, index) => {
        const isExpanded = shouldShowExpanded(item, index);
        // minimize if priority is not set or < 0.5
        const shouldMinimize =
          item.priority === undefined || item.priority < 0.5;
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
