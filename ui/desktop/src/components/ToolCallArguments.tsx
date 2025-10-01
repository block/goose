import { useState } from 'react';
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
          <div className="font-sans text-sm mb-2">
            <div className="flex flex-row">
              <span className="text-textSubtle min-w-[140px]">{key}</span>
              <span className="text-textPlaceholder">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className={`font-sans text-sm mb-2 ${isExpanded ? '' : 'truncate min-w-0'}`}>
          <div className={`flex flex-row items-stretch ${isExpanded ? '' : 'truncate min-w-0'}`}>
            <button
              onClick={() => toggleKey(key)}
              className="flex text-left text-textSubtle min-w-[140px]"
            >
              <span>{key}</span>
            </button>
            <div className={`w-full flex items-stretch ${isExpanded ? '' : 'truncate min-w-0'}`}>
              {isExpanded ? (
                <div>
                  <MarkdownContent
                    content={value}
                    className="font-sans text-sm text-textPlaceholder"
                  />
                </div>
              ) : (
                <button
                  onClick={() => toggleKey(key)}
                  className={`text-left text-textPlaceholder ${isExpanded ? '' : 'truncate min-w-0'}`}
                >
                  {value}
                </button>
              )}
              <button
                onClick={() => toggleKey(key)}
                className="flex flex-row items-stretch grow text-textPlaceholder pr-2"
              >
                <div className="min-w-2 grow" />
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
        <div className="flex flex-row font-sans text-sm">
          <span className="text-textSubtle min-w-[140px]">{key}</span>
          <pre className="whitespace-pre-wrap text-textPlaceholder overflow-x-auto max-w-full">
            {content}
          </pre>
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
