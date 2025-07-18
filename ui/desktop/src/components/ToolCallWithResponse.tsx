import React, { useEffect, useRef, useState } from 'react';
import { Button } from './ui/button';
import { ToolCallArguments, ToolCallArgumentValue } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import {
  Content,
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  ResourceContent,
  getResourceText,
} from '../types/message';
import { cn, snakeToTitleCase } from '../utils';
import Dot, { LoadingStatus } from './ui/Dot';
import { NotificationEvent } from '../hooks/useMessageStream';
import { ChevronRight, LoaderCircle, Monitor, ExternalLink } from 'lucide-react';
import { isUIResource, extractUIResource, Resource } from './UIResourceRenderer';
import { useSidecar } from './SidecarLayout';
import { TooltipWrapper } from './settings/providers/subcomponents/buttons/TooltipWrapper';

// Extend the Window interface to include our custom property
declare global {
  interface Window {
    pendingDiffContent?: string;
  }
}

// Helper functions for diff content detection are defined at the end of the file

interface ToolCallWithResponseProps {
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
  isStreamingMessage?: boolean;
}

export default function ToolCallWithResponse({
  isCancelledMessage,
  toolRequest,
  toolResponse,
  notifications,
  isStreamingMessage = false,
}: ToolCallWithResponseProps) {
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;
  if (!toolCall) {
    return null;
  }

  return (
    <div
      className={cn(
        'w-full text-sm rounded-lg overflow-hidden border-borderSubtle border bg-background-muted'
      )}
    >
      <ToolCallView
        {...{ isCancelledMessage, toolCall, toolResponse, notifications, isStreamingMessage }}
      />
    </div>
  );
}

interface ToolCallExpandableProps {
  label: string | React.ReactNode;
  isStartExpanded?: boolean;
  isForceExpand?: boolean;
  children: React.ReactNode;
  className?: string;
}

function ToolCallExpandable({
  label,
  isStartExpanded = false,
  isForceExpand,
  children,
  className = '',
}: ToolCallExpandableProps) {
  const [isExpandedState, setIsExpanded] = React.useState<boolean | null>(null);
  const isExpanded = isExpandedState === null ? isStartExpanded : isExpandedState;
  const toggleExpand = () => setIsExpanded(!isExpanded);
  React.useEffect(() => {
    if (isForceExpand) setIsExpanded(true);
  }, [isForceExpand]);

  return (
    <div className={className}>
      <Button
        onClick={toggleExpand}
        className="group w-full flex justify-between items-center pr-2 transition-colors rounded-none"
        variant="ghost"
      >
        <span className="flex items-center font-mono">{label}</span>
        <ChevronRight
          className={cn(
            'group-hover:opacity-100 transition-transform opacity-70',
            isExpanded && 'rotate-90'
          )}
        />
      </Button>
      {isExpanded && <div>{children}</div>}
    </div>
  );
}

interface ToolCallViewProps {
  isCancelledMessage: boolean;
  toolCall: {
    name: string;
    arguments: Record<string, unknown>;
  };
  toolResponse?: ToolResponseMessageContent;
  notifications?: NotificationEvent[];
  isStreamingMessage?: boolean;
}

interface Progress {
  progress: number;
  progressToken: string;
  total?: number;
  message?: string;
}

const logToString = (logMessage: NotificationEvent) => {
  const params = logMessage.message.params;

  // Special case for the developer system shell logs
  if (
    params &&
    params.data &&
    typeof params.data === 'object' &&
    'output' in params.data &&
    'stream' in params.data
  ) {
    return `[${params.data.stream}] ${params.data.output}`;
  }

  return typeof params.data === 'string' ? params.data : JSON.stringify(params.data);
};

const notificationToProgress = (notification: NotificationEvent): Progress =>
  notification.message.params as unknown as Progress;

// Helper function to extract extension name for tooltip
const getExtensionTooltip = (toolCallName: string): string | null => {
  const lastIndex = toolCallName.lastIndexOf('__');
  if (lastIndex === -1) return null;

  const extensionName = toolCallName.substring(0, lastIndex);
  if (!extensionName) return null;

  return `${extensionName} extension`;
};

function ToolCallView({
  isCancelledMessage,
  toolCall,
  toolResponse,
  notifications,
  isStreamingMessage = false,
}: ToolCallViewProps) {
  const [responseStyle, setResponseStyle] = useState(() => localStorage.getItem('response_style'));

  // Listen for localStorage changes to update the response style
  useEffect(() => {
    const handleStorageChange = () => {
      setResponseStyle(localStorage.getItem('response_style'));
    };

    // Listen for storage events (changes from other tabs/windows)
    window.addEventListener('storage', handleStorageChange);

    // Listen for custom events (changes from same tab)
    window.addEventListener('responseStyleChanged', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      window.removeEventListener('responseStyleChanged', handleStorageChange);
    };
  }, []);

  const isExpandToolDetails = (() => {
    switch (responseStyle) {
      case 'concise':
        return false;
      case 'detailed':
      default:
        return true;
    }
  })();

  //extract resource content if present
  const result = toolResponse?.toolResult.value || [];
  const resourceContents = result.filter((item) => item.type === 'resource') as ResourceContent[];
  const checkpoint = resourceContents.find((item) => item.resource.uri === 'goose://checkpoint');
  const diffContent = JSON.parse(checkpoint ? getResourceText(checkpoint.resource) || '{}' : '{}').diff;
  console.log(resourceContents);
  console.log(checkpoint);
  console.log(diffContent);

  const isToolDetails = Object.entries(toolCall?.arguments).length > 0;

  // Check if streaming has finished but no tool response was received
  // This is a workaround for cases where the backend doesn't send tool responses
  const isStreamingComplete = !isStreamingMessage;
  const shouldShowAsComplete = isStreamingComplete && !toolResponse;

  const loadingStatus: LoadingStatus = !toolResponse
    ? shouldShowAsComplete
      ? 'success'
      : 'loading'
    : toolResponse.toolResult.status;

  // Tool call timing tracking
  const [startTime, setStartTime] = useState<number | null>(null);

  // Track when tool call starts (when there's no response yet)
  useEffect(() => {
    if (!toolResponse && startTime === null) {
      setStartTime(Date.now());
    }
  }, [toolResponse, startTime]);

  const toolResults: { result: Content; isExpandToolResults: boolean }[] =
    loadingStatus === 'success' && Array.isArray(toolResponse?.toolResult.value)
      ? (() => {
          console.log('🔍 DEBUGGING: Tool response processing');
          console.log('loadingStatus:', loadingStatus);
          console.log('toolResponse?.toolResult.value:', toolResponse?.toolResult.value);

          const rawResults = toolResponse!.toolResult.value;
          console.log('Raw tool results:', rawResults);

          const hasUIResources = rawResults.some((item) => {
            const isUI = item.type === 'resource' && isUIResource(item);
            console.log(`Item type: ${item.type}, isUIResource: ${isUI}`, item);
            return isUI;
          });

          if (hasUIResources) {
            console.log('✅ Tool result contains UI resources');
          } else {
            console.log('❌ No UI resources found in tool results');
          }

          const filteredResults = rawResults.filter((item) => {
            const audience = item.annotations?.audience as string[] | undefined;
            const shouldInclude = !audience || audience.includes('user');
            console.log(`Audience filter: ${audience} -> include: ${shouldInclude}`, item);
            return shouldInclude;
          });

          console.log('Filtered results:', filteredResults);

          const mappedResults = filteredResults.map((item) => {
            // Use user preference for detailed/concise, but still respect high priority items
            const priority = (item.annotations?.priority as number | undefined) ?? -1;
            const isHighPriority = priority >= 0.5;
            const shouldExpandBasedOnStyle = responseStyle === 'detailed' || responseStyle === null;

            return {
              result: item,
              isExpandToolResults: isHighPriority || shouldExpandBasedOnStyle,
            };
          });

          console.log('Final mapped results:', mappedResults);
          return mappedResults;
        })()
      : [];

  const logs = notifications
    ?.filter((notification) => notification.message.method === 'notifications/message')
    .map(logToString);

  const progress = notifications
    ?.filter((notification) => notification.message.method === 'notifications/progress')
    .map(notificationToProgress)
    .reduce((map, item) => {
      const key = item.progressToken;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(item);
      return map;
    }, new Map<string, Progress[]>());

  const progressEntries = [...(progress?.values() || [])].map(
    (entries) => entries.sort((a, b) => b.progress - a.progress)[0]
  );

  const isRenderingProgress =
    loadingStatus === 'loading' && (progressEntries.length > 0 || (logs || []).length > 0);

  // Determine if the main tool call should be expanded
  const isShouldExpand = (() => {
    // Always expand if there are high priority results that need to be shown
    const hasHighPriorityResults = toolResults.some((v) => v.isExpandToolResults);

    // Also expand based on user preference for detailed mode
    const shouldExpandBasedOnStyle = responseStyle === 'detailed' || responseStyle === null;

    return hasHighPriorityResults || shouldExpandBasedOnStyle;
  })();

  // Function to create a descriptive representation of what the tool is doing
  const getToolDescription = (): string | null => {
    const args = toolCall.arguments as Record<string, ToolCallArgumentValue>;
    const toolName = toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2);

    // Helper function to get string value safely
    const getStringValue = (value: ToolCallArgumentValue): string => {
      return typeof value === 'string' ? value : JSON.stringify(value);
    };

    // Helper function to truncate long values
    const truncate = (str: string, maxLength: number = 50): string => {
      return str.length > maxLength ? str.substring(0, maxLength) + '...' : str;
    };

    // Generate descriptive text based on tool type
    switch (toolName) {
      case 'text_editor':
        if (args.command === 'write' && args.path) {
          return `writing ${truncate(getStringValue(args.path))}`;
        }
        if (args.command === 'view' && args.path) {
          return `reading ${truncate(getStringValue(args.path))}`;
        }
        if (args.command === 'str_replace' && args.path) {
          return `editing ${truncate(getStringValue(args.path))}`;
        }
        if (args.command && args.path) {
          return `${getStringValue(args.command)} ${truncate(getStringValue(args.path))}`;
        }
        break;

      case 'shell':
        if (args.command) {
          return `running ${truncate(getStringValue(args.command))}`;
        }
        break;

      case 'search':
        if (args.name) {
          return `searching for "${truncate(getStringValue(args.name))}"`;
        }
        if (args.mimeType) {
          return `searching for ${getStringValue(args.mimeType)} files`;
        }
        break;

      case 'read': {
        if (args.uri) {
          const uri = getStringValue(args.uri);
          const fileId = uri.replace('gdrive:///', '');
          return `reading file ${truncate(fileId)}`;
        }
        if (args.url) {
          return `reading ${truncate(getStringValue(args.url))}`;
        }
        break;
      }

      case 'create_file':
        if (args.name) {
          return `creating ${truncate(getStringValue(args.name))}`;
        }
        break;

      case 'update_file':
        if (args.fileId) {
          return `updating file ${truncate(getStringValue(args.fileId))}`;
        }
        break;

      case 'sheets_tool': {
        if (args.operation && args.spreadsheetId) {
          const operation = getStringValue(args.operation);
          const sheetId = truncate(getStringValue(args.spreadsheetId));
          return `${operation} in sheet ${sheetId}`;
        }
        break;
      }

      case 'docs_tool': {
        if (args.operation && args.documentId) {
          const operation = getStringValue(args.operation);
          const docId = truncate(getStringValue(args.documentId));
          return `${operation} in document ${docId}`;
        }
        break;
      }

      case 'web_scrape':
        if (args.url) {
          return `scraping ${truncate(getStringValue(args.url))}`;
        }
        break;

      case 'remember_memory':
        if (args.category && args.data) {
          return `storing ${getStringValue(args.category)}: ${truncate(getStringValue(args.data))}`;
        }
        break;

      case 'retrieve_memories':
        if (args.category) {
          return `retrieving ${getStringValue(args.category)} memories`;
        }
        break;

      case 'screen_capture':
        if (args.window_title) {
          return `capturing window "${truncate(getStringValue(args.window_title))}"`;
        }
        return `capturing screen`;

      case 'automation_script':
        if (args.language) {
          return `running ${getStringValue(args.language)} script`;
        }
        break;

      case 'final_output':
        return 'final output';

      case 'computer_control':
        return `poking around...`;

      default: {
        // Generic fallback for unknown tools: ToolName + CompactArguments
        // This ensures any MCP tool works without explicit handling
        const toolDisplayName = snakeToTitleCase(toolName);
        const entries = Object.entries(args);

        if (entries.length === 0) {
          return `${toolDisplayName}`;
        }

        // For a single parameter, show key and truncated value
        if (entries.length === 1) {
          const [key, value] = entries[0];
          const stringValue = getStringValue(value);
          const truncatedValue = truncate(stringValue, 30);
          return `${toolDisplayName} ${key}: ${truncatedValue}`;
        }

        // For multiple parameters, show tool name and keys
        const keys = entries.map(([key]) => key).join(', ');
        return `${toolDisplayName} ${keys}`;
      }
    }

    return null;
  };

  // Get extension tooltip for the current tool
  const extensionTooltip = getExtensionTooltip(toolCall.name);

  // Extract tool label content to avoid duplication
  const getToolLabelContent = () => {
    const description = getToolDescription();
    if (description) {
      return description;
    }
    // Fallback tool name formatting
    return snakeToTitleCase(toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2));
  };

  const toolLabel = (
    <span className={cn('ml-2', extensionTooltip && 'cursor-pointer hover:opacity-80')}>
      {getToolLabelContent()}
    </span>
  );

  return (
    <ToolCallExpandable
      isStartExpanded={isRenderingProgress}
      isForceExpand={isShouldExpand}
      label={
        <div className="flex items-center justify-between w-full pr-2">
          <div className="flex items-center">
            <div className="w-2 flex items-center justify-center">
              {loadingStatus === 'loading' ? (
                <LoaderCircle className="animate-spin text-text-muted" size={3} />
              ) : (
                <Dot size={2} loadingStatus={loadingStatus} />
              )}
            </div>
            {extensionTooltip ? (
              <TooltipWrapper tooltipContent={extensionTooltip} side="top" align="start">
                {toolLabel}
              </TooltipWrapper>
            ) : (
              toolLabel
            )}
          </div>
        </div>
      }
    >
      {/* Tool Details */}
      {isToolDetails && (
        <div className="border-t border-borderSubtle">
          <ToolDetailsView toolCall={toolCall} isStartExpanded={isExpandToolDetails} />
        </div>
      )}

      {logs && logs.length > 0 && (
        <div className="border-t border-borderSubtle">
          <ToolLogsView
            logs={logs}
            working={loadingStatus === 'loading'}
            isStartExpanded={
              loadingStatus === 'loading' || responseStyle === 'detailed' || responseStyle === null
            }
          />
        </div>
      )}

      {toolResults.length === 0 &&
        progressEntries.length > 0 &&
        progressEntries.map((entry, index) => (
          <div className="p-3 border-t border-borderSubtle" key={index}>
            <ProgressBar progress={entry.progress} total={entry.total} message={entry.message} />
          </div>
        ))}

      {/* Tool Output */}
      {!isCancelledMessage && (
        <>
          {toolResults.map(({ result, isExpandToolResults }, index) => {
            return (
              <div key={index} className={cn('border-t border-borderSubtle')}>
                <ToolResultView
                  result={result}
                  isStartExpanded={isExpandToolResults}
                  toolCall={toolCall}
                />
              </div>
            );
          })}
        </>
      )}
    </ToolCallExpandable>
  );
}

interface ToolDetailsViewProps {
  toolCall: {
    name: string;
    arguments: Record<string, unknown>;
  };
  isStartExpanded: boolean;
}

function ToolDetailsView({ toolCall, isStartExpanded }: ToolDetailsViewProps) {
  return (
    <ToolCallExpandable
      label={<span className="pl-4 font-medium">Tool Details</span>}
      isStartExpanded={isStartExpanded}
    >
      <div className="pr-4 pl-8">
        {toolCall.arguments && (
          <ToolCallArguments args={toolCall.arguments as Record<string, ToolCallArgumentValue>} />
        )}
      </div>
    </ToolCallExpandable>
  );
}

interface ToolResultViewProps {
  result: Content;
  isStartExpanded: boolean;
  toolCall: { name?: string } | null; // Tool call object for generating fallback UI
}

function ToolResultView({ result, isStartExpanded, toolCall }: ToolResultViewProps) {
  const sidecar = useSidecar();
  const hasShownInSidecar = useRef(false);

  // Helper function to show UI resource in sidecar only once
  const showInSidecarOnce = (resource: Resource, title: string) => {
    if (sidecar && !hasShownInSidecar.current) {
      sidecar.showUIResource(resource, title);
      hasShownInSidecar.current = true;
      return true;
    }
    return false;
  };

  // Handle UI resources by showing them in the sidecar
  if (result.type === 'resource' && isUIResource(result)) {
    console.log('✅ Processing UI resource in ToolResultView - moving to sidecar');
    const uiResource = extractUIResource(result);
    if (uiResource) {
      const title = uiResource.name || 'Interactive Component';
      if (showInSidecarOnce(uiResource, title)) {
        // Return a placeholder in the chat that indicates the UI has been moved to sidecar
        return (
          <ToolCallExpandable
            label={<span className="pl-4 py-1 font-medium">Interactive Output</span>}
            isStartExpanded={isStartExpanded}
          >
            <div className="pl-4 pr-4 py-4">
              <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg">
                <div className="flex items-center space-x-3">
                  <Monitor className="text-blue-600" size={20} />
                  <div>
                    <p className="font-medium text-blue-900">Interactive UI Component</p>
                    <p className="text-sm text-blue-600">
                      {uiResource.name || 'Opened in side panel'}
                    </p>
                  </div>
                </div>
                <ExternalLink className="text-blue-500" size={16} />
              </div>
            </div>
          </ToolCallExpandable>
        );
      }
    }
  }

  return (
    <ToolCallExpandable
      label={<span className="pl-4 py-1 font-medium">Output</span>}
      isStartExpanded={isStartExpanded}
    >
      <div className="pl-4 pr-4 py-4">
        {result.type === 'text' &&
          (() => {
            const textContent = result as { type: 'text'; text: string };
            if (!textContent.text) return null;

            // Only trigger fallback for STRONG evidence that UI content was flattened to text

            // Technical patterns that indicate a UI resource was converted to text
            const hasTechnicalUIIndicator =
              textContent.text.includes('[Interactive UI Component:') ||
              textContent.text.includes('ui://') ||
              textContent.text.includes('application/vnd.mcp-ui.') ||
              textContent.text.includes('Remote DOM') ||
              textContent.text.includes('mimeType') ||
              // Pattern: JSON-like structure with UI fields
              (textContent.text.includes('"uri"') && textContent.text.includes('"mimeType"')) ||
              // Pattern: Resource object that got stringified
              (textContent.text.includes('{"name"') && textContent.text.includes('text/html')) ||
              // Pattern: MCP resource object structure
              (textContent.text.includes('{"resource"') && textContent.text.includes('ui://'));

            // Semantic patterns that indicate product or recommendation content
            const hasSemanticUIIndicator =
              // Product-related patterns
              ((textContent.text.includes('product') ||
                textContent.text.includes('catalog') ||
                textContent.text.includes('recommendation') ||
                textContent.text.includes('item')) &&
                // AND technical indicators
                (textContent.text.includes('USD') ||
                  textContent.text.includes('$') ||
                  textContent.text.includes('price') ||
                  textContent.text.includes('category') ||
                  textContent.text.includes('description'))) ||
              // List/array patterns with multiple items
              (textContent.text.includes('[') &&
                textContent.text.includes(']') &&
                textContent.text.split('\n').length > 5);

            // Obvious UI content patterns
            const hasObviousUIContent =
              textContent.text.includes('document.createElement') ||
              textContent.text.includes('<div') ||
              textContent.text.includes('<html') ||
              textContent.text.includes('innerHTML') ||
              textContent.text.includes('onclick') ||
              textContent.text.includes('JavaScript');

            // Combined UI indicator
            const hasUIIndicator =
              hasTechnicalUIIndicator || hasSemanticUIIndicator || hasObviousUIContent;

            const toolName = toolCall?.name || 'unknown';
            const isProductTool =
              toolName.toLowerCase().includes('product') ||
              toolName.toLowerCase().includes('catalog') ||
              toolName.toLowerCase().includes('list');

            if (hasUIIndicator) {
              console.log('🎯 Detected potential UI content in text response:', {
                technical: hasTechnicalUIIndicator,
                semantic: hasSemanticUIIndicator,
                obvious: hasObviousUIContent,
                toolName,
                textLength: textContent.text.length,
              });

              // Try to parse the text content as JSON first to see if it's a structured UI resource
              try {
                const parsed = JSON.parse(textContent.text);
                if (parsed && typeof parsed === 'object') {
                  // Check if it's already a properly structured UI resource
                  if (parsed.uri && parsed.uri.startsWith('ui://') && parsed.mimeType) {
                    console.log('✅ Found structured UI resource in JSON - moving to sidecar!');

                    const title = parsed.name || 'Interactive Content';
                    if (showInSidecarOnce(parsed, title)) {
                      return (
                        <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg mt-2">
                          <div className="flex items-center space-x-3">
                            <Monitor className="text-blue-600" size={20} />
                            <div>
                              <p className="font-medium text-blue-900">Interactive UI Component</p>
                              <p className="text-sm text-blue-600">
                                {parsed.name || 'Opened in side panel'}
                              </p>
                            </div>
                          </div>
                          <ExternalLink className="text-blue-500" size={16} />
                        </div>
                      );
                    }
                  }

                  // Check if it contains UI resource information that needs to be restructured
                  if (
                    parsed.text &&
                    (parsed.text.includes('ui://') ||
                      parsed.text.includes('document.createElement'))
                  ) {
                    const uiResource = {
                      uri: `ui://mcp-server/extracted-content/${Date.now()}`,
                      mimeType: parsed.text.includes('document.createElement')
                        ? 'application/vnd.mcp-ui.remote-dom+javascript'
                        : 'text/html',
                      name: parsed.name || 'Interactive Content',
                      text: parsed.text,
                    };

                    console.log('✅ Extracted UI resource from parsed JSON - moving to sidecar!');

                    const title = uiResource.name || 'Interactive Content';
                    if (showInSidecarOnce(uiResource, title)) {
                      return (
                        <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg mt-2">
                          <div className="flex items-center space-x-3">
                            <Monitor className="text-blue-600" size={20} />
                            <div>
                              <p className="font-medium text-blue-900">Interactive UI Component</p>
                              <p className="text-sm text-blue-600">
                                {uiResource.name || 'Opened in side panel'}
                              </p>
                            </div>
                          </div>
                          <ExternalLink className="text-blue-500" size={16} />
                        </div>
                      );
                    }
                  }
                }
              } catch (e) {
                // Not valid JSON, continue with other detection methods
                console.log('Content is not valid JSON, trying other detection methods');
              }

              // Try to extract UI resource information from the text using regex patterns
              const uriMatch = textContent.text.match(/ui:\/\/[^\s\]"'\n]+/);
              if (uriMatch) {
                const mockResource = {
                  uri: uriMatch[0],
                  mimeType: 'text/html',
                  name: 'MCP UI Resource',
                  text: textContent.text,
                };

                console.log('✅ Rendering UI from extracted URI - moving to sidecar!');

                const title = 'MCP UI Resource';
                if (showInSidecarOnce(mockResource, title)) {
                  return (
                    <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg mt-2">
                      <div className="flex items-center space-x-3">
                        <Monitor className="text-blue-600" size={20} />
                        <div>
                          <p className="font-medium text-blue-900">Interactive UI Component</p>
                          <p className="text-sm text-blue-600">
                            {mockResource.name || 'Opened in side panel'}
                          </p>
                        </div>
                      </div>
                      <ExternalLink className="text-blue-500" size={16} />
                    </div>
                  );
                }
              }

              // Check if it's JavaScript content that should be executed
              const hasJavaScript =
                textContent.text.includes('document.createElement') ||
                textContent.text.includes('const ') ||
                textContent.text.includes('appendChild') ||
                textContent.text.includes('innerHTML');

              if (hasJavaScript) {
                console.log('✅ Detected JavaScript UI content - creating executable resource');
                const jsResource = {
                  uri: `ui://mcp-server/remote-dom/${Date.now()}`,
                  mimeType: 'application/vnd.mcp-ui.remote-dom+javascript',
                  name: 'Interactive JavaScript Component',
                  text: textContent.text,
                };

                console.log('✅ Rendering JavaScript UI resource - moving to sidecar!');

                const title = 'Interactive JavaScript Component';
                if (showInSidecarOnce(jsResource, title)) {
                  return (
                    <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg mt-2">
                      <div className="flex items-center space-x-3">
                        <Monitor className="text-blue-600" size={20} />
                        <div>
                          <p className="font-medium text-blue-900">
                            Interactive JavaScript Component
                          </p>
                          <p className="text-sm text-blue-600">Opened in side panel</p>
                        </div>
                      </div>
                      <ExternalLink className="text-blue-500" size={16} />
                    </div>
                  );
                }
              }

              // Check if it contains HTML content
              if (textContent.text.includes('<') && textContent.text.includes('>')) {
                console.log('✅ Detected HTML content - creating HTML resource');
                const htmlResource = {
                  uri: `ui://mcp-server/html-content/${Date.now()}`,
                  mimeType: 'text/html',
                  name: 'HTML Content',
                  text: textContent.text,
                };

                const title = 'HTML Content';
                if (showInSidecarOnce(htmlResource, title)) {
                  return (
                    <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg mt-2">
                      <div className="flex items-center space-x-3">
                        <Monitor className="text-blue-600" size={20} />
                        <div>
                          <p className="font-medium text-blue-900">HTML Content</p>
                          <p className="text-sm text-blue-600">Opened in side panel</p>
                        </div>
                      </div>
                      <ExternalLink className="text-blue-500" size={16} />
                    </div>
                  );
                }
              }

              // Fallback: if we detected UI indicators but can't parse the content,
              // create a simple HTML wrapper for the content
              console.log('🔄 Creating fallback HTML wrapper for detected UI content');
              const fallbackResource = {
                uri: `ui://mcp-server/fallback-content/${Date.now()}`,
                mimeType: 'text/html',
                name: isProductTool ? 'Product Information' : 'Interactive Content',
                text: `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>${isProductTool ? 'Product Information' : 'Content Display'}</title>
  <style>
    body { 
      font-family: system-ui, -apple-system, sans-serif; 
      padding: 20px; 
      line-height: 1.6;
      max-width: 800px;
      margin: 0 auto;
    }
    .content { 
      background: #f8f9fa; 
      padding: 20px; 
      border-radius: 8px; 
      border: 1px solid #e9ecef;
      white-space: pre-wrap;
      overflow-wrap: break-word;
    }
  </style>
</head>
<body>
  <h2>${isProductTool ? '🛍️ Product Information' : '📄 Content'}</h2>
  <div class="content">${textContent.text.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</div>
</body>
</html>`,
              };

              const title = isProductTool ? 'Product Information' : 'Interactive Content';
              if (showInSidecarOnce(fallbackResource, title)) {
                return (
                  <div className="flex items-center justify-between p-4 bg-blue-50 border border-blue-200 rounded-lg mt-2">
                    <div className="flex items-center space-x-3">
                      <Monitor className="text-blue-600" size={20} />
                      <div>
                        <p className="font-medium text-blue-900">
                          {isProductTool ? 'Product Information' : 'Interactive Content'}
                        </p>
                        <p className="text-sm text-blue-600">Opened in side panel</p>
                      </div>
                    </div>
                    <ExternalLink className="text-blue-500" size={16} />
                  </div>
                );
              }
            }

            // Normal text rendering
            return (
              <MarkdownContent
                content={textContent.text}
                className="whitespace-pre-wrap max-w-full overflow-x-auto"
              />
            );
          })()}
        {result.type === 'image' && (
          <img
            src={`data:${result.mimeType};base64,${result.data}`}
            alt="Tool result"
            className="max-w-full h-auto rounded-md my-2"
            onError={(e) => {
              console.error('Failed to load image');
              e.currentTarget.style.display = 'none';
            }}
          />
        )}
        {result.type === 'resource' && !isUIResource(result) && (
          <div className="bg-gray-50 p-3 rounded border">
            <p className="text-sm text-gray-600 mb-2">
              <strong>Resource:</strong> {result.resource.uri}
            </p>
            {getResourceText(result.resource) && (
              <pre className="text-xs bg-white p-2 rounded border max-h-40 overflow-auto">
                {getResourceText(result.resource)}
              </pre>
            )}
          </div>
        )}
      </div>
    </ToolCallExpandable>
  );
}

function ToolLogsView({
  logs,
  working,
  isStartExpanded,
}: {
  logs: string[];
  working: boolean;
  isStartExpanded?: boolean;
}) {
  const boxRef = useRef<HTMLDivElement>(null);

  // Whenever logs update, jump to the newest entry
  useEffect(() => {
    if (boxRef.current) {
      boxRef.current.scrollTop = boxRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <ToolCallExpandable
      label={
        <span className="pl-4 py-1 font-medium flex items-center">
          <span>Logs</span>
          {working && (
            <div className="mx-2 inline-block">
              <span
                className="inline-block animate-spin rounded-full border-2 border-t-transparent border-current"
                style={{ width: 8, height: 8 }}
                role="status"
                aria-label="Loading spinner"
              />
            </div>
          )}
        </span>
      }
      isStartExpanded={isStartExpanded}
    >
      <div
        ref={boxRef}
        className={`flex flex-col items-start space-y-2 overflow-y-auto p-4 ${working ? 'max-h-[4rem]' : 'max-h-[20rem]'}`}
      >
        {logs.map((log, i) => (
          <span key={i} className="font-mono text-sm text-textSubtle">
            {log}
          </span>
        ))}
      </div>
    </ToolCallExpandable>
  );
}

const ProgressBar = ({ progress, total, message }: Omit<Progress, 'progressToken'>) => {
  const isDeterminate = typeof total === 'number';
  const percent = isDeterminate ? Math.min((progress / total!) * 100, 100) : 0;

  return (
    <div className="w-full space-y-2">
      {message && <div className="text-sm text-textSubtle">{message}</div>}

      <div className="w-full bg-background-subtle rounded-full h-4 overflow-hidden relative">
        {isDeterminate ? (
          <div
            className="bg-primary h-full transition-all duration-300"
            style={{ width: `${percent}%` }}
          />
        ) : (
          <div className="absolute inset-0 animate-indeterminate bg-primary" />
        )}
      </div>
    </div>
  );
};

// Export utility functions for diff content detection and extraction
export function hasDiffContent(toolResponse?: ToolResponseMessageContent): boolean {
  if (!toolResponse || toolResponse.toolResult.status !== 'success') {
    return false;
  }

  const results = Array.isArray(toolResponse.toolResult.value) ? toolResponse.toolResult.value : [];

  return results.some((result) => {
    if (result.type === 'text') {
      const textContent = result as { type: 'text'; text: string };
      return (
        textContent.text.includes('diff --git') ||
        textContent.text.includes('+++') ||
        textContent.text.includes('---') ||
        textContent.text.includes('@@ ')
      );
    }
    return false;
  });
}

export function extractDiffContent(toolResponse?: ToolResponseMessageContent): string | null {
  if (!hasDiffContent(toolResponse)) {
    return null;
  }

  const results = Array.isArray(toolResponse!.toolResult.value)
    ? toolResponse!.toolResult.value
    : [];

  for (const result of results) {
    if (result.type === 'text') {
      const textContent = result as { type: 'text'; text: string };
      if (
        textContent.text.includes('diff --git') ||
        textContent.text.includes('+++') ||
        textContent.text.includes('---') ||
        textContent.text.includes('@@ ')
      ) {
        return textContent.text;
      }
    }
  }

  return null;
}
