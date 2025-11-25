import { ChevronRight } from 'lucide-react';
import React, { useCallback } from 'react';
import { ToolIconWithStatus, ToolCallStatus } from './ToolCallStatusIndicator';
import { getToolCallIcon } from '../utils/toolIconMapping';
import { ToolRequestMessageContent, ToolResponseMessageContent, Content } from '../types/message';
import { cn, snakeToTitleCase } from '../utils';
import { useTabContext } from '../contexts/TabContext';
import { NotificationEvent } from '../hooks/useMessageStream';
import MarkdownContent from './MarkdownContent';
import { ToolCallArguments } from './ToolCallArguments';
import { TooltipWrapper } from './settings/providers/subcomponents/buttons/TooltipWrapper';
import { LoadingStatus } from './ui/Dot';

interface CompactToolCallProps {
  tabId: string;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
  isStreamingMessage?: boolean;
  isCancelledMessage?: boolean;
}

export default function CompactToolCall({
  tabId,
  toolRequest,
  toolResponse,
  notifications,
  isStreamingMessage = false,
  isCancelledMessage = false,
}: CompactToolCallProps) {
  const { showDocumentEditor } = useTabContext();
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;

  if (!toolCall) {
    return null;
  }

  // Determine tool call status
  const getToolCallStatus = (notifications?: NotificationEvent[]): ToolCallStatus => {
    if (!notifications || notifications.length === 0) {
      return toolResponse ? 'complete' : 'loading';
    }

    const lastNotification = notifications[notifications.length - 1];
    const level = lastNotification.message.params?.level;

    if (level === 'error') return 'error';
    if (level === 'warning') return 'warning';
    
    return toolResponse ? 'complete' : 'loading';
  };

  const toolCallStatus = getToolCallStatus(notifications);

  // Get extension tooltip if applicable
  const getExtensionTooltip = () => {
    const extensionName = (toolCall.arguments as any)?.extension;
    if (extensionName && typeof extensionName === 'string') {
      return `MCP Extension: ${extensionName}`;
    }
    return null;
  };

  const extensionTooltip = getExtensionTooltip();

  // Get tool label
  const getToolLabel = () => {
    if (toolCall.name === 'mcp_list_resources') {
      return 'List Extensions';
    }
    if (toolCall.name === 'mcp_call_tool') {
      const toolName = (toolCall.arguments as any)?.name;
      return toolName ? snakeToTitleCase(toolName) : snakeToTitleCase(toolCall.name);
    }
    return snakeToTitleCase(toolCall.name);
  };

  const toolLabel = getToolLabel();

  // Handle click to open sidecar with full output
  const handleClick = useCallback(() => {
    // Build the full content to show in the sidecar
    let content = '';

    // Add tool details (name and arguments)
    content += `# ${toolLabel}\n\n`;
    content += `**Tool Name:** \`${toolCall.name}\`\n\n`;
    
    if (Object.keys(toolCall.arguments).length > 0) {
      content += `## Arguments\n\n`;
      content += '```json\n';
      content += JSON.stringify(toolCall.arguments, null, 2);
      content += '\n```\n\n';
    }

    // Add logs if any
    const logs: string[] = [];
    if (notifications) {
      for (const notification of notifications) {
        const params = notification.message.params;
        
        // Special case for developer system shell logs
        if (
          params &&
          params.data &&
          typeof params.data === 'object' &&
          'output' in params.data &&
          'stream' in params.data
        ) {
          logs.push(`[${params.data.stream}] ${params.data.output}`);
        } else if (params && typeof params.data === 'string') {
          logs.push(params.data);
        } else {
          logs.push(JSON.stringify(params));
        }
      }
    }

    if (logs.length > 0) {
      content += `## Logs\n\n`;
      content += '```\n';
      content += logs.join('\n');
      content += '\n```\n\n';
    }

    // Add tool output/results
    if (toolResponse?.toolResult?.status === 'success' && toolResponse.toolResult.value) {
      content += `## Output\n\n`;
      
      for (const result of toolResponse.toolResult.value) {
        if (result.type === 'text' && result.text) {
          content += result.text + '\n\n';
        } else if (result.type === 'image') {
          content += `![Tool result image](data:${result.mimeType};base64,${result.data})\n\n`;
        } else if (result.type === 'resource') {
          content += '```json\n';
          content += JSON.stringify(result, null, 2);
          content += '\n```\n\n';
        }
      }
    } else if (toolResponse?.toolResult?.status === 'error') {
      content += `## Error\n\n`;
      content += '```\n';
      content += JSON.stringify(toolResponse.toolResult.value, null, 2);
      content += '\n```\n\n';
    }

    // Open in document editor sidecar
    showDocumentEditor(
      tabId,
      undefined, // no file path
      content,
      `tool-${toolRequest.id}` // unique instance ID
    );
  }, [tabId, toolLabel, toolCall, toolResponse, notifications, toolRequest.id, showDocumentEditor]);

  const toolLabelContent = (
    <span
      className={cn(
        'flex items-center gap-2 cursor-pointer',
        extensionTooltip && 'hover:opacity-80'
      )}
      onClick={handleClick}
    >
      <ToolIconWithStatus ToolIcon={getToolCallIcon(toolCall.name)} status={toolCallStatus} />
      <span className="font-medium">{toolLabel}</span>
    </span>
  );

  return (
    <div
      className={cn(
        'inline-flex items-center gap-2 px-3 py-1.5 rounded-lg',
        'border border-zinc-200 dark:border-zinc-800',
        'bg-white/50 dark:bg-zinc-900/50',
        'hover:bg-white/80 dark:hover:bg-zinc-900/80',
        'transition-all cursor-pointer group'
      )}
      onClick={handleClick}
    >
      {extensionTooltip ? (
        <TooltipWrapper tooltipContent={extensionTooltip} side="top" align="start">
          {toolLabelContent}
        </TooltipWrapper>
      ) : (
        toolLabelContent
      )}
      <ChevronRight
        className={cn(
          'w-4 h-4 text-zinc-400 dark:text-zinc-500',
          'group-hover:text-zinc-600 dark:group-hover:text-zinc-300',
          'transition-colors'
        )}
      />
    </div>
  );
}

