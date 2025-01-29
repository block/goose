import React, { useState } from 'react';
import MarkdownContent from './MarkdownContent';

interface ToolCallArgumentsProps {
  args: Record<string, any>;
}

export function ToolCallArguments({ args }: ToolCallArgumentsProps) {
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});

  const toggleKey = (key: string) => {
    setExpandedKeys((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const renderValue = (key: string, value: any) => {
    if (typeof value === 'string') {
      const needsExpansion = value.length > 60;
      const isExpanded = expandedKeys[key];

      if (!needsExpansion) {
        return (
          <div className="p-1">
            <div className="flex">
              <span className="text-textStandard mr-2">{key}:</span>
              <span className="text-textStandard">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className="p-1">
          <div className="flex items-baseline">
            <span className="text-textStandard mr-2">{key}:</span>
            <div className="flex-1">
              <button
                onClick={() => toggleKey(key)}
                className="hover:opacity-75 text-gray-600 dark:text-white"
              >
                {isExpanded ? '▼ ' : '▶ '}
              </button>
              {!isExpanded && <span className="ml-2 text-gray-600">{value.slice(0, 60)}...</span>}
            </div>
          </div>
          {isExpanded && (
            <div className="mt-2 ml-4">
              <MarkdownContent content={value} />
            </div>
          )}
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
      <div className="p-1">
        <div className="flex">
          <span className="font-medium mr-2">{key}:</span>
          <pre className="whitespace-pre-wrap">{content}</pre>
        </div>
      </div>
    );
  };

  return (
    <div className="mt-2">
      {Object.entries(args).map(([key, value]) => (
        <div key={key}>{renderValue(key, value)}</div>
      ))}
    </div>
  );
}
