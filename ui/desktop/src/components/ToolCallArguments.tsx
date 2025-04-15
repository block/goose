import { ChevronUp } from 'lucide-react';
import React, { useState } from 'react';
import MarkdownContent from './MarkdownContent';

type ToolCallArgumentValue =
  | string
  | number
  | boolean
  | null
  | ToolCallArgumentValue[]
  | { [key: string]: ToolCallArgumentValue };

interface ToolCallArgumentsProps {
  args: Record<string, ToolCallArgumentValue>;
}

export function ToolCallArguments({ args }: ToolCallArgumentsProps) {
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});

  const toggleKey = (key: string) => {
    setExpandedKeys((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const renderValue = (key: string, value: ToolCallArgumentValue) => {
    if (typeof value === 'string') {
      const needsExpansion = value.length > 60;
      const isExpanded = expandedKeys[key];

      if (!needsExpansion) {
        return (
          <div className="mb-2">
            <div className="flex flex-row">
              <span className="text-sm font-medium text-textSubtle min-w-[140px]">{key}</span>
              <span className="text-sm text-textStandard">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className="mb-2">
          <div className="flex flex-row">
            <span className="text-sm font-medium text-textSubtle min-w-[140px]">{key}</span>
            <div className="flex items-center">
              {isExpanded ? (
                <div className="mt-2">
                  <MarkdownContent content={value} />
                </div>
              ) : (
                <span className="text-sm text-textStandard mr-2">{value.slice(0, 60)}...</span>
              )}
              <button
                onClick={() => toggleKey(key)}
                className="text-sm hover:opacity-75 text-textStandard"
              >
                <ChevronUp
                  className={`h-5 w-5 transition-all origin-center ${!isExpanded ? 'rotate-180' : ''}`}
                />
              </button>
            </div>
          </div>
        </div>
      );
    }

    // Handle non-string values (arrays, objects, etc.)
    const content = Array.isArray(value)
      ? value.map((item, index) => `${index + 1}. ${JSON.stringify(item)}`).join('\n')
      : typeof value === 'object' && value !== null
        ? JSON.stringify(value, null, 2)
        : String(value);

    return (
      <div className="mb-2">
        <div className="flex flex-row">
          <span className="font-medium mr- text-textStandard min-w-[140px]2">{key}:</span>
          <pre className="whitespace-pre-wrap text-textStandard">{content}</pre>
        </div>
      </div>
    );
  };

  return (
    <div className="my-2">
      {Object.entries(args).map(([key, value]) => (
        <div key={key}>{renderValue(key, value)}</div>
      ))}
    </div>
  );
}
