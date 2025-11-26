import { useState } from 'react';
import MarkdownContent from './MarkdownContent';
import Expand from './ui/Expand';
import { Circle } from 'lucide-react';
import { useTaskExecution, TaskStatus } from '../contexts/TaskExecutionContext';

export type ToolCallArgumentValue =
  | string
  | number
  | boolean
  | null
  | ToolCallArgumentValue[]
  | { [key: string]: ToolCallArgumentValue };

interface ToolCallArgumentsProps {
  args: Record<string, ToolCallArgumentValue>;
  toolCallId?: string; // ID of the tool call to track execution status
  toolName?: string; // Name of the tool (e.g., 'create_task', 'execute_task')
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

// Timeline component for task IDs (execute_task)
function TaskIdsTimeline({ taskIds, executionMode }: { taskIds: string[]; executionMode?: string }) {
  const isParallel = executionMode === 'parallel';
  
  return (
    <div className="space-y-3">
      {/* Execution mode indicator as a simple pill badge */}
      {executionMode && (
        <div className="inline-flex items-center px-3 py-1.5 rounded-full bg-background-muted border border-borderSubtle mb-3">
          <span className="text-textSubtle font-sans text-xs font-medium">{executionMode}</span>
        </div>
      )}
      
      {taskIds.map((taskId, index) => {
        return (
          <div key={index} className="flex gap-3">
            {/* Timeline indicator */}
            <div className="flex flex-col items-center">
              <div className="w-6 h-6 rounded-full border-2 border-borderSubtle bg-background-default flex items-center justify-center flex-shrink-0 relative">
                <span className="text-xs font-sans text-textSubtle">{index + 1}</span>
                {/* Parallel execution indicator - small numbered box overlaid on circle */}
                {isParallel && (
                  <div className="absolute -bottom-1 -right-1 w-3 h-3 rounded-sm border border-borderSubtle bg-background-muted flex items-center justify-center">
                    <span className="text-[8px] font-sans text-textSubtle">1</span>
                  </div>
                )}
              </div>
              {index < taskIds.length - 1 && (
                <div className="w-0.5 h-full bg-borderSubtle flex-grow mt-1" />
              )}
            </div>
            
            {/* Task ID content */}
            <div className="flex-1 pb-3">
              <div className="font-sans text-xs text-textPlaceholder font-mono">
                {taskId}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

// Task status type
type TaskStatus = 'pending' | 'running' | 'completed' | 'error';

// Timeline component for task parameters
function TaskParametersTimeline({ 
  tasks, 
  executionMode,
  taskStatuses 
}: { 
  tasks: any[]; 
  executionMode?: string;
  taskStatuses?: Map<number, TaskStatus>;
}) {
  const isParallel = executionMode === 'parallel';
  
  return (
    <div className="space-y-3">
      {/* Execution mode indicator as a simple pill badge */}
      {executionMode && (
        <div className="inline-flex items-center px-3 py-1.5 rounded-full bg-background-muted border border-borderSubtle mb-3">
          <span className="text-textSubtle font-sans text-xs font-medium">{executionMode}</span>
        </div>
      )}
      
      {tasks.map((task, index) => {
        // Extract task details
        const instructions = task.instructions || task.prompt || 'No instructions provided';
        const title = task.title || `Task ${index + 1}`;
        const status = taskStatuses?.get(index) || 'pending';
        
        // Determine circle styling based on status
        const getCircleStyle = () => {
          switch (status) {
            case 'running':
              return 'border-blue-500 bg-blue-50';
            case 'completed':
              return 'border-green-500 bg-green-50';
            case 'error':
              return 'border-red-500 bg-red-50';
            default:
              return 'border-borderSubtle bg-background-default';
          }
        };
        
        return (
          <div key={index} className="flex gap-3">
            {/* Timeline indicator */}
            <div className="flex flex-col items-center">
              <div className={`w-6 h-6 rounded-full border-2 flex items-center justify-center flex-shrink-0 relative ${getCircleStyle()}`}>
                <span className="text-xs font-sans text-textSubtle">{index + 1}</span>
                {/* Parallel execution indicator - small numbered box overlaid on circle */}
                {isParallel && (
                  <div className="absolute -bottom-1 -right-1 w-3 h-3 rounded-sm border border-borderSubtle bg-background-muted flex items-center justify-center">
                    <span className="text-[8px] font-sans text-textSubtle">1</span>
                  </div>
                )}
                {/* Status indicator - small spinner or checkmark */}
                {status === 'running' && (
                  <div className="absolute inset-0 flex items-center justify-center">
                    <div className="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
                  </div>
                )}
                {status === 'completed' && (
                  <div className="absolute inset-0 flex items-center justify-center">
                    <svg className="w-4 h-4 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                    </svg>
                  </div>
                )}
                {status === 'error' && (
                  <div className="absolute inset-0 flex items-center justify-center">
                    <svg className="w-4 h-4 text-red-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </div>
                )}
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
              <div className={`font-sans text-xs ${status === 'completed' ? 'text-textSubtle line-through' : 'text-textPlaceholder'}`}>
                {instructions}
              </div>
              {/* Show other task properties if they exist */}
              {task.description && (
                <div className="font-sans text-xs text-textSubtle mt-1 italic">
                  {task.description}
                </div>
              )}
              {/* Status label */}
              {status !== 'pending' && (
                <div className="mt-1">
                  <span className={`font-sans text-[10px] font-medium ${
                    status === 'running' ? 'text-blue-500' :
                    status === 'completed' ? 'text-green-500' :
                    'text-red-500'
                  }`}>
                    {status === 'running' ? 'Running...' : 
                     status === 'completed' ? 'Completed' : 
                     'Error'}
                  </span>
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function ToolCallArguments({ args, toolCallId, toolName }: ToolCallArgumentsProps) {
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});
  const { getTaskStatuses, registerCreateTask } = useTaskExecution();

  const toggleKey = (key: string) => {
    setExpandedKeys((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  // Extract execution_mode if it exists
  const executionMode = typeof args.execution_mode === 'string' ? args.execution_mode : undefined;
  
  // Get task statuses if this is a create_task
  const taskStatuses = toolCallId && toolName === 'create_task' ? getTaskStatuses(toolCallId) : undefined;

  const renderValue = (key: string, value: ToolCallArgumentValue) => {
    // Determine if this parameter should use smaller text
    const useSmallText = ['command', 'path', 'file_text', 'task_parameters', 'execution_mode', 'task_ids'].includes(key);
    const textSizeClass = useSmallText ? 'text-xs' : 'text-sm';

    // Special handling for task_parameters - render as timeline with execution mode and statuses
    if (key === 'task_parameters' && Array.isArray(value)) {
      // Register this create_task if we have an ID
      if (toolCallId && toolName === 'create_task' && !taskStatuses) {
        // Generate task IDs based on the task count (format: "task-0", "task-1", etc.)
        const taskIds = Array.from({ length: value.length }, (_, i) => `task-${i}`);
        registerCreateTask(toolCallId, taskIds);
      }
      
      return (
        <div className="mb-2">
          <TaskParametersTimeline 
            tasks={value as any[]} 
            executionMode={executionMode}
            taskStatuses={taskStatuses}
          />
        </div>
      );
    }

    // Special handling for task_ids - render as timeline with execution mode
    if (key === 'task_ids' && Array.isArray(value)) {
      const taskIds = value.map(id => typeof id === 'string' ? id : String(id));
      return (
        <div className="mb-2">
          <TaskIdsTimeline taskIds={taskIds} executionMode={executionMode} />
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
