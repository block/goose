import React from 'react';
import { Card } from './ui/card';
import Box from './ui/Box';
import { ToolCallArguments } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { snakeToTitleCase } from '../utils';
import { LoadingPlaceholder } from './LoadingPlaceholder';
import { ChevronUp } from 'lucide-react';

export default function ToolInvocations({ toolInvocations }) {
  return (
    <>
      {toolInvocations.map((toolInvocation) => (
        <ToolInvocation key={toolInvocation.toolCallId} toolInvocation={toolInvocation} />
      ))}
    </>
  );
}

function ToolInvocation({ toolInvocation }) {
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
    state: 'call' | 'result';
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

interface Annotations {
  audience?: string[]; // Array of audience types
  priority?: number; // Priority value between 0 and 1
}

interface ResultItem {
  text?: string;
  type: 'text' | 'image';
  mimeType?: string;
  data?: string; // Base64 encoded image data
  annotations?: Annotations;
}

interface ToolResultProps {
  result: {
    message?: string;
    result?: ResultItem[];
    state?: string;
    toolCallId?: string;
    toolName?: string;
    args?: any;
    input_todo?: any;
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
    (item: ResultItem) =>
      !item.annotations?.audience || item.annotations?.audience?.includes('user')
  );

  if (filteredResults.length === 0) return null;

  const toggleExpand = (index: number) => {
    setExpandedItems((prev) =>
      prev.includes(index) ? prev.filter((i) => i !== index) : [...prev, index]
    );
  };

  const shouldShowExpanded = (item: ResultItem, index: number) => {
    // (priority is defined and > 0.5) OR already in the expandedItems
    return (
      (item.annotations?.priority !== undefined && item.annotations?.priority >= 0.5) ||
      expandedItems.includes(index)
    );
  };

  return (
    <div className="">
      {filteredResults.map((item: ResultItem, index: number) => {
        const isExpanded = shouldShowExpanded(item, index);
        // minimize if priority is not set or < 0.5
        const shouldMinimize =
          item.annotations?.priority === undefined || item.annotations?.priority < 0.5;
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
                {item.type === 'text' && item.text && (
                  <MarkdownContent
                    content={item.text}
                    className="whitespace-pre-wrap p-2 max-w-full overflow-x-auto"
                  />
                )}
                {item.type === 'image' && item.data && item.mimeType && (
                  <img
                    src={`data:${item.mimeType};base64,${item.data}`}
                    alt="Tool result"
                    className="max-w-full h-auto rounded-md"
                    onError={(e) => {
                      console.error('Failed to load image: Invalid MIME-type encoded image data');
                      e.currentTarget.style.display = 'none';
                    }}
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
