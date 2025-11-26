import { useState } from 'react';
import MarkdownContent from './MarkdownContent';
import Expand from './ui/Expand';
import { Circle } from 'lucide-react';

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

// Tree visualization for execution mode
function ExecutionModeTree({ mode, taskCount = 3 }: { mode: string; taskCount?: number }) {
  const isParallel = mode === 'parallel';
  
  if (isParallel) {
    // Parallel: vertical tree with branches - all labeled "1" since they run simultaneously
    return (
      <div className="flex items-start gap-2">
        {/* Root node */}
        <div className="flex flex-col items-center">
          <div className="w-2 h-2 rounded-full bg-borderSubtle" />
          <div className="w-0.5 h-4 bg-borderSubtle" />
        </div>
        
        {/* Parallel branches */}
        <div className="flex flex-col gap-1">
          {Array.from({ length: taskCount }).map((_, index) => (
            <div key={index} className="flex items-center gap-1">
              <div className="w-3 h-0.5 bg-borderSubtle" />
              <div className="w-4 h-4 rounded-sm border border-borderSubtle bg-background-default flex items-center justify-center">
                <span className="text-[8px] font-sans text-textSubtle">1</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }
  
  // Sequential: horizontal tree with connected nodes
  return (
    <div className="flex items-center gap-1">
      {Array.from({ length: taskCount }).map((_, index) => (
        <div key={index} className="flex items-center">
          <div className="w-4 h-4 rounded-sm border border-borderSubtle bg-background-default flex items-center justify-center">
            <span className="text-[8px] font-sans text-textSubtle">{index + 1}</span>
          </div>
          {index < taskCount - 1 && (
            <div className="w-2 h-0.5 bg-borderSubtle mx-0.5" />
          )}
        </div>
      ))}
    </div>
  );
}

// Timeline component for task parameters
function TaskParametersTimeline({ tasks, executionMode }: { tasks: any[]; executionMode?: string }) {
  return (
    <div className="space-y-3">
      {/* Execution mode indicator as a pill badge */}
      {executionMode && (
        <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-background-muted border border-borderSubtle mb-3">
          <ExecutionModeTree mode={executionMode} taskCount={tasks.length} />
          <span className="text-textSubtle font-sans text-xs font-medium">{executionMode}</span>
        </div>
      )}
      
      {tasks.map((task, index) => {
        // Extract task details
        const instructions = task.instructions || task.prompt || 'No instructions provided';
        const title = task.title || `Task ${index + 1}`;
        
        return (
          <div key={index} className="flex gap-3">
            {/* Timeline indicator */}
            <div className="flex flex-col items-center">
              <div className="w-6 h-6 rounded-full border-2 border-borderSubtle bg-background-default flex items-center justify-center flex-shrink-0">
                <span className="text-xs font-sans text-textSubtle">{index + 1}</span>
              </div>
              {index < tasks.length - 1 && (
                <div className="w-0.5 h-full bg-borderSubtle flex-grow mt-1" />
              )}
            </div>
            
            {/* Task content */}
            <div className="flex-1 pb-3">
              {title !== `Task ${index + 1}` && (
                <div className="font-sans text-xs font-medium text-textProminent mb-1">
                  {title}
                </div>
              )}
              <div className="font-sans text-xs text-textPlaceholder">
                {instructions}
              </div>
              {/* Show other task properties if they exist */}
              {task.description && (
                <div className="font-sans text-xs text-textSubtle mt-1 italic">
                  {task.description}
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function ToolCallArguments({ args }: ToolCallArgumentsProps) {
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});

  const toggleKey = (key: string) => {
    setExpandedKeys((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  // Extract execution_mode if it exists
  const executionMode = typeof args.execution_mode === 'string' ? args.execution_mode : undefined;

  const renderValue = (key: string, value: ToolCallArgumentValue) => {
    // Determine if this parameter should use smaller text
    const useSmallText = ['command', 'path', 'file_text', 'task_parameters', 'execution_mode', 'task_ids'].includes(key);
    const textSizeClass = useSmallText ? 'text-xs' : 'text-sm';

    // Special handling for task_parameters - render as timeline with execution mode
    if (key === 'task_parameters' && Array.isArray(value)) {
      return (
        <div className="mb-2">
          <TaskParametersTimeline tasks={value as any[]} executionMode={executionMode} />
        </div>
      );
    }

    // Hide execution_mode as standalone - it's now shown in the timeline
    if (key === 'execution_mode') {
      return null;
    }

    if (typeof value === 'string') {
      const needsExpansion = value.length > 60;
      const isExpanded = expandedKeys[key];

      if (!needsExpansion) {
        return (
          <div className={`font-sans ${textSizeClass} mb-2`}>
            <div className="flex flex-row">
              <span className="text-textSubtle min-w-[140px]">{key}</span>
              <span className="text-textPlaceholder">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className={`font-sans ${textSizeClass} mb-2`}>
          <div className="flex flex-row items-stretch">
            <button
              onClick={() => toggleKey(key)}
              className="flex text-left text-textSubtle min-w-[140px]"
            >
              <span>{key}</span>
            </button>
            <div className="w-full flex items-stretch">
              {isExpanded ? (
                <div>
                  <MarkdownContent
                    content={value}
                    className={`font-sans ${textSizeClass} text-textPlaceholder`}
                  />
                </div>
              ) : (
                <button onClick={() => toggleKey(key)} className="text-left text-textPlaceholder">
                  {value.slice(0, 60)}...
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
        <div className={`flex flex-row font-sans ${textSizeClass}`}>
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
