import { AppEvents } from '../../constants/events';
/**
 * MCP Apps Renderer
 *
 * Uses the official @mcp-ui/client AppRenderer component for rendering MCP Apps.
 *
 * @see https://mcpui.dev/guide/mcp-apps#host-side-rendering-client-sdk
 */

import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import {
  AppRenderer,
  type AppRendererHandle,
  type AppRendererProps,
  type RequestHandlerExtra,
} from '@mcp-ui/client';
import type { McpUiHostContext, McpUiResourceCsp } from '@modelcontextprotocol/ext-apps/app-bridge';
import type {
  CallToolRequest,
  CallToolResult,
  ReadResourceRequest,
  ReadResourceResult,
  LoggingMessageNotification,
} from '@modelcontextprotocol/sdk/types.js';
import { cn } from '../../utils';
import { DEFAULT_IFRAME_HEIGHT, fetchMcpAppProxyUrl } from './utils';
import { readResource, callTool } from '../../api';
import { errorMessage } from '../../utils/conversionUtils';
import { useTheme } from '../../contexts/ThemeContext';
import type { CspMetadata, CallToolResponse } from '../../api/types.gen';

interface McpAppRendererProps {
  resourceUri: string;
  extensionName: string;
  sessionId?: string | null;
  toolInput?: { arguments: Record<string, unknown> };
  toolInputPartial?: { arguments: Record<string, unknown> };
  toolResult?: CallToolResponse;
  toolCancelled?: { reason?: string };
  append?: (text: string) => void;
  fullscreen?: boolean;
  cachedHtml?: string;
}

// Convert our CspMetadata (with nullable fields) to McpUiResourceCsp (without null)
function toMcpUiResourceCsp(csp: CspMetadata | null): McpUiResourceCsp | undefined {
  if (!csp) return undefined;
  return {
    connectDomains: csp.connectDomains ?? undefined,
    resourceDomains: csp.resourceDomains ?? undefined,
  };
}

export default function McpAppRenderer({
  resourceUri,
  extensionName,
  sessionId,
  toolInput,
  toolInputPartial,
  toolResult,
  toolCancelled,
  append,
  fullscreen = false,
  cachedHtml,
}: McpAppRendererProps) {
  const { resolvedTheme } = useTheme();
  const appRef = useRef<AppRendererHandle>(null);

  const [html, setHtml] = useState<string | null>(cachedHtml || null);
  const [csp, setCsp] = useState<CspMetadata | null>(null);
  const [prefersBorder, setPrefersBorder] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sandboxUrl, setSandboxUrl] = useState<URL | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);

  // Fetch sandbox proxy URL
  useEffect(() => {
    fetchMcpAppProxyUrl(csp).then((url) => {
      if (url) {
        setSandboxUrl(new URL(url));
      }
    });
  }, [csp]);

  // Fetch resource HTML if not cached
  useEffect(() => {
    if (!sessionId || cachedHtml) {
      return;
    }

    const fetchResource = async () => {
      try {
        const response = await readResource({
          body: {
            session_id: sessionId,
            uri: resourceUri,
            extension_name: extensionName,
          },
        });

        if (response.data) {
          const content = response.data;
          const meta = content._meta as
            | { ui?: { csp?: CspMetadata; prefersBorder?: boolean } }
            | undefined;

          if (content.text !== cachedHtml) {
            setHtml(content.text);
            setCsp(meta?.ui?.csp || null);
            setPrefersBorder(meta?.ui?.prefersBorder ?? true);
          }
        }
      } catch (err) {
        if (!cachedHtml) {
          setError(errorMessage(err, 'Failed to load resource'));
        } else {
          console.warn('Failed to fetch fresh resource, using cached version:', err);
        }
      }
    };

    fetchResource();
  }, [resourceUri, extensionName, sessionId, cachedHtml]);

  // Host context for the guest UI
  const hostContext: McpUiHostContext = useMemo(
    () => ({
      theme: resolvedTheme,
      displayMode: fullscreen ? 'fullscreen' : 'inline',
      availableDisplayModes: fullscreen ? ['fullscreen'] : ['inline'],
      viewport: {
        width: window.innerWidth,
        height: window.innerHeight,
        maxWidth: window.innerWidth,
        maxHeight: window.innerHeight,
      },
      locale: navigator.language,
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    }),
    [resolvedTheme, fullscreen]
  );

  // Handler for tools/call requests from the guest UI
  const handleCallTool = useCallback(
    async (
      params: CallToolRequest['params'],
      _extra: RequestHandlerExtra
    ): Promise<CallToolResult> => {
      if (!sessionId) {
        throw new Error('Session not initialized for tool call');
      }

      const fullToolName = `${extensionName}__${params.name}`;
      const response = await callTool({
        body: {
          session_id: sessionId,
          name: fullToolName,
          arguments: params.arguments || {},
        },
      });

      // Map our Content type to the SDK's expected format
      const content = (response.data?.content || []).map((item) => {
        if ('text' in item && typeof item.text === 'string') {
          return { type: 'text' as const, text: item.text };
        }
        if ('data' in item && 'mimeType' in item) {
          return {
            type: 'image' as const,
            data: item.data as string,
            mimeType: item.mimeType as string,
          };
        }
        // Fallback for other content types
        return { type: 'text' as const, text: JSON.stringify(item) };
      });

      return {
        content,
        isError: response.data?.is_error || false,
      };
    },
    [sessionId, extensionName]
  );

  // Handler for resources/read requests from the guest UI
  const handleReadResource = useCallback(
    async (
      params: ReadResourceRequest['params'],
      _extra: RequestHandlerExtra
    ): Promise<ReadResourceResult> => {
      if (!sessionId) {
        throw new Error('Session not initialized for resource read');
      }

      const response = await readResource({
        body: {
          session_id: sessionId,
          uri: params.uri,
          extension_name: extensionName,
        },
      });

      if (!response.data) {
        return { contents: [] };
      }

      // Map our response to the SDK's expected format
      const resourceContent = {
        uri: response.data.uri,
        text: response.data.text,
        mimeType: response.data.mimeType ?? undefined,
        _meta: response.data._meta as Record<string, unknown> | undefined,
      };

      return {
        contents: [resourceContent],
      };
    },
    [sessionId, extensionName]
  );

  // Handler for open-link requests
  const handleOpenLink: NonNullable<AppRendererProps['onOpenLink']> = useCallback(
    async ({ url }) => {
      await window.electron.openExternal(url);
      return {};
    },
    []
  );

  // Handler for message requests (prompts)
  const handleMessage: NonNullable<AppRendererProps['onMessage']> = useCallback(
    async ({ content }) => {
      if (!append) {
        throw new Error('Message handler not available in this context');
      }

      if (!Array.isArray(content)) {
        throw new Error('Invalid message format: content must be an array');
      }

      const textContent = content.find(
        (block): block is { type: 'text'; text: string } => block.type === 'text'
      );
      if (!textContent) {
        throw new Error('Invalid message format: content must contain a text block');
      }

      append(textContent.text);
      window.dispatchEvent(new CustomEvent(AppEvents.SCROLL_CHAT_TO_BOTTOM));
      return {};
    },
    [append]
  );

  // Handler for logging messages
  const handleLoggingMessage = useCallback((params: LoggingMessageNotification['params']) => {
    console.log(`[MCP App] ${params.level || 'info'}:`, params.data);
  }, []);

  // Handler for size changes
  const handleSizeChanged: NonNullable<AppRendererProps['onSizeChanged']> = useCallback(
    (params) => {
      if (params.height) {
        setIframeHeight(Math.max(DEFAULT_IFRAME_HEIGHT, params.height));
      }
    },
    []
  );

  // Handler for errors
  const handleError = useCallback((err: Error) => {
    console.error('[MCP App Error]:', err);
    setError(err.message);
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    const currentRef = appRef.current;
    return () => {
      currentRef?.teardownResource();
    };
  }, []);

  if (error) {
    return (
      <div className="p-4 border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20">
        <div className="text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      </div>
    );
  }

  if (!sandboxUrl || !html) {
    if (fullscreen) {
      return (
        <div
          style={{
            width: '100%',
            height: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          Loading...
        </div>
      );
    }

    return (
      <div className="flex items-center justify-center p-4" style={{ minHeight: '200px' }}>
        Loading MCP app...
      </div>
    );
  }

  // Extract tool name from resourceUri (e.g., "ui://server/tool" -> "tool")
  const toolName = resourceUri.split('/').pop() || 'unknown';

  if (fullscreen) {
    return (
      <AppRenderer
        ref={appRef}
        sandbox={{ url: sandboxUrl, csp: toMcpUiResourceCsp(csp) }}
        toolName={toolName}
        toolResourceUri={resourceUri}
        html={html}
        toolInput={toolInput?.arguments}
        toolInputPartial={toolInputPartial?.arguments}
        toolResult={toolResult as CallToolResult | undefined}
        toolCancelled={!!toolCancelled}
        hostContext={hostContext}
        onOpenLink={handleOpenLink}
        onMessage={handleMessage}
        onLoggingMessage={handleLoggingMessage}
        onSizeChanged={handleSizeChanged}
        onError={handleError}
        onCallTool={handleCallTool}
        onReadResource={handleReadResource}
      />
    );
  }

  return (
    <div
      className={cn(
        'bg-bgApp overflow-hidden',
        prefersBorder ? 'border border-borderSubtle rounded-lg' : 'my-6'
      )}
      style={{ height: `${iframeHeight}px` }}
    >
      <AppRenderer
        ref={appRef}
        sandbox={{ url: sandboxUrl, csp: toMcpUiResourceCsp(csp) }}
        toolName={toolName}
        toolResourceUri={resourceUri}
        html={html}
        toolInput={toolInput?.arguments}
        toolInputPartial={toolInputPartial?.arguments}
        toolResult={toolResult as CallToolResult | undefined}
        toolCancelled={!!toolCancelled}
        hostContext={hostContext}
        onOpenLink={handleOpenLink}
        onMessage={handleMessage}
        onLoggingMessage={handleLoggingMessage}
        onSizeChanged={handleSizeChanged}
        onError={handleError}
        onCallTool={handleCallTool}
        onReadResource={handleReadResource}
      />
    </div>
  );
}
