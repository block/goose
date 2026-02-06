import { AppEvents } from '../../constants/events';
import { useState, useCallback, useEffect, useMemo } from 'react';
import { AppRenderer, type McpUiHostContext } from '@mcp-ui/client';
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
import type {
  McpUiSizeChangedNotification,
  McpUiResourceCsp,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import { ToolInput, ToolInputPartial, ToolCancelled, SandboxPermissions } from './types';
import type { CspMetadata, CallToolResponse } from '../../api/types.gen';
import { cn } from '../../utils';
import { readResource, callTool } from '../../api';
import { errorMessage } from '../../utils/conversionUtils';
import { isProtocolSafe, getProtocol } from '../../utils/urlSecurity';
import { useTheme } from '../../contexts/ThemeContext';

const DEFAULT_IFRAME_HEIGHT = 200;
const AVAILABLE_DISPLAY_MODES = ['inline' as const, 'fullscreen' as const];

async function fetchMcpAppProxyUrl(csp: CspMetadata | null): Promise<string | null> {
  try {
    const baseUrl = await window.electron.getGoosedHostPort();
    const secretKey = await window.electron.getSecretKey();

    if (!baseUrl || !secretKey) {
      console.error('[McpAppRenderer] Failed to get goosed host/port or secret key');
      return null;
    }

    const params = new URLSearchParams();
    params.set('secret', secretKey);

    if (csp?.connectDomains?.length) {
      params.set('connect_domains', csp.connectDomains.join(','));
    }
    if (csp?.resourceDomains?.length) {
      params.set('resource_domains', csp.resourceDomains.join(','));
    }

    return `${baseUrl}/mcp-app-proxy?${params.toString()}`;
  } catch (error) {
    console.error('[McpAppRenderer] Error fetching MCP App Proxy URL:', error);
    return null;
  }
}

interface McpAppRendererProps {
  resourceUri: string;
  extensionName: string;
  sessionId?: string | null;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: CallToolResponse;
  toolCancelled?: ToolCancelled;
  append?: (text: string) => void;
  fullscreen?: boolean;
  cachedHtml?: string;
}

interface ResourceData {
  html: string | null;
  csp: CspMetadata | null;
  permissions: SandboxPermissions | null;
  prefersBorder: boolean;
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
  const [resource, setResource] = useState<ResourceData>({
    html: cachedHtml || null,
    csp: null,
    permissions: null,
    prefersBorder: true,
  });
  const [error, setError] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);
  const [iframeWidth, setIframeWidth] = useState<number | null>(null);
  const [sandboxUrl, setSandboxUrl] = useState<URL | null>(null);
  const [sandboxCsp, setSandboxCsp] = useState<CspMetadata | null>(null);
  const [sandboxUrlFetched, setSandboxUrlFetched] = useState(false);

  // Fetch sandbox URL once after HTML loads to prevent iframe recreation
  useEffect(() => {
    if (!resource.html || sandboxUrlFetched) {
      return;
    }
    setSandboxUrlFetched(true);
    // Capture the CSP at fetch time to keep sandboxConfig stable
    const cspAtFetchTime = resource.csp;
    setSandboxCsp(cspAtFetchTime);
    fetchMcpAppProxyUrl(cspAtFetchTime).then((url) => {
      if (url) {
        setSandboxUrl(new URL(url));
      } else {
        console.error('[McpAppRenderer] Failed to get sandbox URL');
      }
    });
  }, [resource.html, resource.csp, sandboxUrlFetched]);

  useEffect(() => {
    if (!sessionId) {
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
            | {
                ui?: {
                  csp?: CspMetadata;
                  permissions?: { sandbox?: string };
                  prefersBorder?: boolean;
                };
              }
            | undefined;

          if (content.text !== cachedHtml) {
            setResource({
              html: content.text,
              csp: meta?.ui?.csp || null,
              permissions: meta?.ui?.permissions?.sandbox || null,
              prefersBorder: meta?.ui?.prefersBorder ?? true,
            });
          }
        }
      } catch (err) {
        console.error('[McpAppRenderer] Error fetching resource:', err);
        if (!cachedHtml) {
          setError(errorMessage(err, 'Failed to load resource'));
        } else {
          console.warn('Failed to fetch fresh resource, using cached version:', err);
        }
      }
    };

    fetchResource();
  }, [resourceUri, extensionName, sessionId, cachedHtml]);

  const handleOpenLink = useCallback(async ({ url }: { url: string }) => {
    if (isProtocolSafe(url)) {
      await window.electron.openExternal(url);
      return { status: 'success' as const };
    }

    const protocol = getProtocol(url);
    if (!protocol) {
      return { status: 'error' as const, message: 'Invalid URL' };
    }

    const result = await window.electron.showMessageBox({
      type: 'question',
      buttons: ['Cancel', 'Open'],
      defaultId: 0,
      title: 'Open External Link',
      message: `Open ${protocol} link?`,
      detail: `This will open: ${url}`,
    });

    if (result.response !== 1) {
      return { status: 'error' as const, message: 'User cancelled' };
    }

    await window.electron.openExternal(url);
    return { status: 'success' as const };
  }, []);

  const handleMessage = useCallback(
    async ({ content }: { content: Array<{ type: string; text?: string }> }) => {
      if (!append) {
        throw new Error('Message handler not available in this context');
      }
      if (!Array.isArray(content)) {
        throw new Error('Invalid message format: content must be an array of ContentBlock');
      }
      const textContent = content.find((block) => block.type === 'text');
      if (!textContent || !textContent.text) {
        throw new Error('Invalid message format: content must contain a text block');
      }
      append(textContent.text);
      window.dispatchEvent(new CustomEvent(AppEvents.SCROLL_CHAT_TO_BOTTOM));
      return {};
    },
    [append]
  );

  const handleCallTool = useCallback(
    async ({
      name,
      arguments: args,
    }: {
      name: string;
      arguments?: Record<string, unknown>;
    }): Promise<CallToolResult> => {
      if (!sessionId) {
        throw new Error('Session not initialized for MCP request');
      }

      const fullToolName = `${extensionName}__${name}`;
      const response = await callTool({
        body: {
          session_id: sessionId,
          name: fullToolName,
          arguments: args || {},
        },
      });

      const content = response.data?.content || [];
      return {
        content: content.map((item) => {
          if ('text' in item && item.text !== undefined) {
            return { type: 'text' as const, text: item.text };
          }
          if ('data' in item && item.data !== undefined) {
            return {
              type: 'image' as const,
              data: item.data,
              mimeType: item.mimeType || 'image/png',
            };
          }
          return { type: 'text' as const, text: JSON.stringify(item) };
        }),
        isError: response.data?.is_error || false,
      };
    },
    [sessionId, extensionName]
  );

  const handleReadResource = useCallback(
    async ({ uri }: { uri: string }) => {
      if (!sessionId) {
        throw new Error('Session not initialized for MCP request');
      }
      const response = await readResource({
        body: {
          session_id: sessionId,
          uri,
          extension_name: extensionName,
        },
      });
      const data = response.data;
      if (!data) {
        return { contents: [] };
      }
      return {
        contents: [{ uri: data.uri || uri, text: data.text, mimeType: data.mimeType || undefined }],
      };
    },
    [sessionId, extensionName]
  );

  const handleLoggingMessage = useCallback(
    ({ level, logger, data }: { level?: string; logger?: string; data?: unknown }) => {
      console.log(
        `[MCP App Notification]${logger ? ` [${logger}]` : ''} ${level || 'info'}:`,
        data
      );
    },
    []
  );

  const handleSizeChanged = useCallback(
    ({ height, width }: McpUiSizeChangedNotification['params']) => {
      if (height !== undefined) {
        setIframeHeight(Math.max(DEFAULT_IFRAME_HEIGHT, height));
      }
      setIframeWidth(width ?? null);
    },
    []
  );

  const handleError = useCallback((err: Error) => {
    console.error('[MCP App Error]:', err);
    setError(errorMessage(err));
  }, []);

  // Use sandboxCsp (captured at fetch time) to keep sandboxConfig stable
  const mcpUiCsp = useMemo((): McpUiResourceCsp | undefined => {
    if (!sandboxCsp) return undefined;
    return {
      connectDomains: sandboxCsp.connectDomains ?? undefined,
      resourceDomains: sandboxCsp.resourceDomains ?? undefined,
    };
  }, [sandboxCsp]);

  const sandboxConfig = useMemo(() => {
    if (!sandboxUrl) return null;
    return {
      url: sandboxUrl,
      permissions: resource.permissions || 'allow-scripts allow-same-origin allow-forms',
      csp: mcpUiCsp,
    };
  }, [sandboxUrl, resource.permissions, mcpUiCsp]);

  const hostContext = useMemo(
    (): McpUiHostContext => ({
      theme: resolvedTheme,
      displayMode: fullscreen ? 'fullscreen' : 'inline',
      availableDisplayModes: AVAILABLE_DISPLAY_MODES,
      // todo: add all the other properties... (aharvard)
      // toolInfo: {}
      // styles: {}
      // containerDimensions: {}
      // locale: ""
      // timeZone: ""
      // userAgent: ""
      // platform: ""
      // deviceCapabilities: {}
      // safeAreaInsets: {}
    }),
    [resolvedTheme, fullscreen]
  );

  const appToolResult = useMemo((): CallToolResult | undefined => {
    if (!toolResult) return undefined;
    const content = toolResult.content || [];
    return {
      content: content.map((item) => {
        if ('text' in item && item.text !== undefined) {
          return { type: 'text' as const, text: item.text };
        }
        if ('data' in item && item.data !== undefined) {
          return {
            type: 'image' as const,
            data: item.data,
            mimeType: item.mimeType || 'image/png',
          };
        }
        return { type: 'text' as const, text: JSON.stringify(item) };
      }),
      isError: toolResult.is_error || false,
    };
  }, [toolResult]);

  const isToolCancelled = toolCancelled?.reason !== undefined;

  if (error) {
    return (
      <div className="p-4 border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20">
        <div className="text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      </div>
    );
  }

  if (!sandboxConfig || !resource.html) {
    return (
      <div
        className={cn('flex items-center justify-center p-4', fullscreen ? 'w-full h-full' : '')}
        style={{ minHeight: fullscreen ? '100%' : '200px' }}
      >
        Loading MCP app...
      </div>
    );
  }

  const containerStyle = fullscreen
    ? { width: '100%', height: '100%' }
    : {
        width: iframeWidth ? `${iframeWidth}px` : '100%',
        maxWidth: '100%',
        height: `${iframeHeight}px`,
      };

  return (
    <div
      className={cn(
        'bg-background-default overflow-hidden',
        !fullscreen && resource.prefersBorder ? 'border border-border-default rounded-lg' : '',
        fullscreen ? 'w-full h-full' : 'my-6'
      )}
      style={containerStyle}
    >
      <AppRenderer
        sandbox={sandboxConfig}
        toolName={resourceUri}
        html={resource.html}
        toolInput={toolInput?.arguments}
        toolInputPartial={toolInputPartial ? { arguments: toolInputPartial.arguments } : undefined}
        toolResult={appToolResult}
        toolCancelled={isToolCancelled}
        hostContext={hostContext}
        onOpenLink={handleOpenLink}
        onMessage={handleMessage}
        onCallTool={handleCallTool}
        onReadResource={handleReadResource}
        onLoggingMessage={handleLoggingMessage}
        onSizeChanged={handleSizeChanged}
        onError={handleError}
      />
    </div>
  );
}
