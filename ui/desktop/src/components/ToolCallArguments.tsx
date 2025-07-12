import { useState } from 'react';
import MarkdownContent from './MarkdownContent';
import Expand from './ui/Expand';
import { Button } from './ui/button';

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
          <div className="text-sm mb-3 bg-background-subtle bg-opacity-40 rounded-md p-2">
            <div className="flex flex-row">
              <span className="text-textSubtle font-medium min-w-[140px]">{key}</span>
              <span className="text-textSubtle">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className="text-sm mb-3 bg-background-subtle bg-opacity-40 rounded-md p-2">
          <div className="flex flex-row items-stretch">
            <button
              onClick={() => toggleKey(key)}
              className="flex text-left text-textSubtle font-medium min-w-[140px]"
            >
              <span>{key}</span>
            </button>
            <div className="w-full flex items-stretch">
              {isExpanded ? (
                <div className="w-full">
                  <MarkdownContent content={value} className="text-sm text-textSubtle" />
                </div>
              ) : (
                <button onClick={() => toggleKey(key)} className="text-left text-textSubtle mr-2">
                  {value.slice(0, 60)}...
                </button>
              )}
              <Button
                onClick={() => toggleKey(key)}
                variant="ghost"
                size="sm"
                className="flex flex-row items-stretch grow text-textPlaceholder p-1 h-auto ml-2"
              >
                <div className="min-w-2 grow" />
                <Expand size={5} isExpanded={isExpanded} />
              </Button>
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
      <div className="mb-3 bg-background-subtle bg-opacity-40 rounded-md p-2">
        <div className="flex flex-row">
          <span className="text-textSubtle font-medium min-w-[140px]">{key}</span>
          <pre className="whitespace-pre-wrap text-textSubtle overflow-x-auto max-w-full">
            {content}
          </pre>
        </div>
      </div>
    );
  };

  return (
    <div className="my-2 space-y-1">
      {Object.entries(args).map(([key, value]) => (
        <div key={key}>{renderValue(key, value)}</div>
      ))}
    </div>
  );
}
