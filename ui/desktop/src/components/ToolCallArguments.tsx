import React, { useState } from 'react';
import MarkdownContent from './MarkdownContent';
import Expand from './ui/Expand';

export type ToolCallArgumentValue =
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
          <div className="text-sm mb-2">
            <div className="flex flex-row">
              <span className="text-textSubtle min-w-[140px]">{key}</span>
              <span className="text-textPlaceholder">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className="text-sm mb-2">
          <div className="flex flex-row">
            <span className="text-textSubtle min-w-[140px]">{key}</span>
            <div className="w-full flex justify-between items-start">
              {isExpanded ? (
                <div className="">
                  <MarkdownContent content={value} className="text-sm text-textPlaceholder" />
                </div>
              ) : (
                <span className="text-textPlaceholder mr-2">{value.slice(0, 60)}...</span>
              )}
              <button
                onClick={() => toggleKey(key)}
                className="hover:opacity-75 text-textPlaceholder pr-2"
              >
                <Expand size={5} isExpanded={isExpanded} />
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
          <span className="mr- text-textPlaceholder min-w-[140px]2">{key}:</span>
          <pre className="whitespace-pre-wrap text-textPlaceholder">{content}</pre>
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
