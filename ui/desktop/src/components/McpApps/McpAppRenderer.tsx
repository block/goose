import { AppRenderer } from '@mcp-ui/client';
import type {
  McpUiDisplayMode,
  McpUiHostContext,
  McpUiResourceCsp,
  McpUiResourcePermissions,
  McpUiSizeChangedNotification,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { callTool, readResource } from '../../api';
import { AppEvents } from '../../constants/events';
import { useTheme } from '../../contexts/ThemeContext';
import { cn } from '../../utils';
import { errorMessage } from '../../utils/conversionUtils';
import { getProtocol, isProtocolSafe } from '../../utils/urlSecurity';
import {
  GooseDisplayMode,
  SandboxPermissions,
  ToolCancelled,
  ToolInput,
  ToolInputPartial,
  ToolResult,
} from './types';

/** Minimum height for the MCP app iframe in pixels */
const DEFAULT_IFRAME_HEIGHT = 100;

/** Display modes the host supports within a chat session.
 * Currently only inline is supported. Fullscreen (in-window takeover) and pip
 * are not yet implemented. Standalone (separate Electron window) is handled
 * outside of this component and is not an McpUiDisplayMode. */
const AVAILABLE_DISPLAY_MODES: McpUiDisplayMode[] = ['inline'];

/**
 * Builds the URL for the MCP app sandbox proxy.
 * The proxy handles CSP (Content Security Policy) enforcement for network requests
 * made by the sandboxed iframe, allowing controlled access to external domains.
 */
async function fetchMcpAppProxyUrl(csp: McpUiResourceCsp | null): Promise<string | null> {
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
    if (csp?.frameDomains?.length) {
      params.set('frame_domains', csp.frameDomains.join(','));
    }
    if (csp?.baseUriDomains?.length) {
      params.set('base_uri_domains', csp.baseUriDomains.join(','));
    }

    return `${baseUrl}/mcp-app-proxy?${params.toString()}`;
  } catch (error) {
    console.error('[McpAppRenderer] Error fetching MCP App Proxy URL:', error);
    return null;
  }
}

interface McpAppRendererProps {
  /** MCP resource URI that identifies the app (e.g., "ui://my-extension/app") */
  resourceUri: string;
  /** Name of the MCP extension providing this app */
  extensionName: string;
  /** Active session ID for MCP communication */
  sessionId?: string | null;
  /** Complete tool arguments when tool execution starts */
  toolInput?: ToolInput;
  /** Partial/streaming tool input to send to the guest UI */
  toolInputPartial?: ToolInputPartial;
  /** Complete tool result to send to the guest UI */
  toolResult?: ToolResult;
  /** Set to true to notify the guest UI that the tool execution was cancelled */
  toolCancelled?: ToolCancelled;
  /** Callback to append text to the chat (for onMessage handler) */
  append?: (text: string) => void;
  /**
   * Display mode for the MCP app.
   * - `inline`: Embedded in chat flow (default)
   * - `fullscreen`: Takes over the current Goose window
   * - `pip`: Picture-in-picture floating window
   * - `standalone`: Rendered in a separate Electron window
   */
  displayMode?: GooseDisplayMode;
  /** Pre-cached HTML to show immediately while fetching fresh content */
  cachedHtml?: string;
}

/** Data fetched from the MCP resource endpoint */
interface ResourceData {
  html: string | null;
  csp: McpUiResourceCsp | null;
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
  displayMode = 'inline',
  cachedHtml,
}: McpAppRendererProps) {
  // Helper: true when app should fill its container (fullscreen or standalone window)
  const isExpandedView = displayMode === 'fullscreen' || displayMode === 'standalone';

  const { resolvedTheme } = useTheme();
  const [resource, setResource] = useState<ResourceData>({
    html: cachedHtml || null,
    csp: null,
    permissions: null,
    prefersBorder: true,
  });
  const [error, setError] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);
  // null = fluid (100% width), number = explicit width from app
  const [iframeWidth, setIframeWidth] = useState<number | null>(null);
  const [sandboxUrl, setSandboxUrl] = useState<URL | null>(null);
  const [sandboxCsp, setSandboxCsp] = useState<McpUiResourceCsp | null>(null);
  const [sandboxUrlFetched, setSandboxUrlFetched] = useState(false);
  // Tracks whether the resource fetch has completed so the sandbox URL effect
  // waits for metadata (CSP) before creating the proxy.
  const [resourceFetched, setResourceFetched] = useState(false);

  // Initialize sandbox URL once after HTML and metadata are available.
  // We wait for resourceFetched so that when cachedHtml is provided, we don't
  // create the proxy before the resource fetch has had a chance to populate CSP.
  // We only fetch once (tracked by sandboxUrlFetched) to prevent iframe recreation
  // which would cause the app to lose state.
  useEffect(() => {
    if (!resource.html || sandboxUrlFetched) {
      return;
    }
    // When there's a sessionId, wait for the resource fetch to complete so we
    // have metadata (CSP). Without a sessionId there's no fetch to wait for.
    if (sessionId && !resourceFetched) {
      return;
    }
    const cspAtFetchTime = resource.csp;
    setSandboxCsp(cspAtFetchTime);
    fetchMcpAppProxyUrl(cspAtFetchTime).then((url) => {
      if (url) {
        setSandboxUrl(new URL(url));
      } else {
        setError('Failed to initialize sandbox proxy');
      }
      setSandboxUrlFetched(true);
    });
  }, [resource.html, resource.csp, sandboxUrlFetched, sessionId, resourceFetched]);

  // Fetch the MCP resource (HTML + metadata) from the extension.
  // If cachedHtml is provided, we show it immediately and only update if the
  // fetched content differs. Metadata (CSP, permissions, prefersBorder) is
  // always applied regardless of whether the HTML changed.
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
                  csp?: McpUiResourceCsp;
                  permissions?: McpUiResourcePermissions;
                  prefersBorder?: boolean;
                };
              }
            | undefined;

          setResource({
            html: content.text ?? cachedHtml ?? null,
            csp: meta?.ui?.csp || null,
            // Per the ext-apps spec, _meta.ui.permissions is McpUiResourcePermissions
            // (camera, microphone, etc.) used to build the iframe Permission Policy
            // `allow` attribute. The @mcp-ui/client SDK does not yet forward these
            // via sendSandboxResourceReady — tracked in:
            // https://github.com/MCP-UI-Org/mcp-ui/issues/180
            permissions: null,
            prefersBorder: meta?.ui?.prefersBorder ?? true,
          });
        }
      } catch (err) {
        console.error('[McpAppRenderer] Error fetching resource:', err);
        if (!cachedHtml) {
          setError(errorMessage(err, 'Failed to load resource'));
        } else {
          console.warn('Failed to fetch fresh resource, using cached version:', err);
        }
      } finally {
        setResourceFetched(true);
      }
    };

    fetchResource();
  }, [resourceUri, extensionName, sessionId, cachedHtml]);

  /** Handles `ui/open-link` - opens external URLs with user confirmation for non-standard protocols */
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

  /** Handles `ui/message` - appends text content from the MCP app to the chat */
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

  /** Handles `tools/call` - invokes an MCP tool and returns the result */
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

      // The server (rmcp) serializes Content with a `type` discriminator
      // (e.g. "text", "image", "audio", "resource", "resource_link") via
      // #[serde(tag = "type")]. Our generated TS types don't reflect this,
      // but the wire format already matches CallToolResult.content.
      return {
        content: (response.data?.content || []) as unknown as CallToolResult['content'],
        isError: response.data?.is_error || false,
        structuredContent: response.data?.structured_content as { [key: string]: unknown } | undefined,
      };
    },
    [sessionId, extensionName]
  );

  /** Handles `resources/read` - reads content from an MCP resource URI */
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

  /** Handles `notifications/message` - logs messages from the MCP app */
  const handleLoggingMessage = useCallback(
    ({ level, logger, data }: { level?: string; logger?: string; data?: unknown }) => {
      console.log(
        `[MCP App Notification]${logger ? ` [${logger}]` : ''} ${level || 'info'}:`,
        data
      );
    },
    []
  );

  /** Handles `ui/size-changed` - updates container dimensions when the MCP app resizes.
   * - Height: always respected (with minimum of 0)
   * - Width: if provided, container uses that width (capped at 100%); if not provided, container is fluid (100%) */
  const handleSizeChanged = useCallback(
    ({ height, width }: McpUiSizeChangedNotification['params']) => {
      if (height !== undefined && height > 0) {
        setIframeHeight(height);
      }
      // Only update width if explicitly provided. null = fluid (100% width)
      if (width !== undefined) {
        setIframeWidth(width > 0 ? width : null);
      }
    },
    []
  );

  /** Handles errors from the MCP app iframe */
  const handleError = useCallback((err: Error) => {
    console.error('[MCP App Error]:', err);
    setError(errorMessage(err));
  }, []);

  // TODO: Add onFallbackRequest handler when SDK supports it
  //  https://github.com/MCP-UI-Org/mcp-ui/pull/176
  // const handleFallbackRequest = useCallback(async (method: string, params: unknown) => {
  //   switch (method) {
  //     case 'sampling/createMessage':
  //       // TODO: Call goosed sampling endpoint
  //       break;
  //   }
  // }, []);

  // Forward CSP to the SDK. Uses sandboxCsp (captured at fetch time) to keep config stable.
  const mcpUiCsp = useMemo((): McpUiResourceCsp | undefined => {
    if (!sandboxCsp) return undefined;
    return {
      connectDomains: sandboxCsp.connectDomains ?? undefined,
      resourceDomains: sandboxCsp.resourceDomains ?? undefined,
      frameDomains: sandboxCsp.frameDomains ?? undefined,
      baseUriDomains: sandboxCsp.baseUriDomains ?? undefined,
    };
  }, [sandboxCsp]);

  // Configuration for the sandboxed iframe that runs the MCP app.
  // Includes the proxy URL, sandbox permissions, and CSP settings.
  const sandboxConfig = useMemo(() => {
    if (!sandboxUrl) return null;
    return {
      url: sandboxUrl,
      permissions: resource.permissions || 'allow-scripts allow-same-origin',
      csp: mcpUiCsp,
    };
  }, [sandboxUrl, resource.permissions, mcpUiCsp]);

  // Context passed to the MCP app describing the host environment.
  // Apps can use this to adapt their UI (e.g., theme, display mode).
  const hostContext = useMemo((): McpUiHostContext => {
    const context: McpUiHostContext = {
      // todo: toolInfo: {}
      theme: resolvedTheme,
      // todo:  styles: { variables: {}, styles: {}}
      displayMode: displayMode === 'standalone' ? 'fullscreen' : displayMode, // should this be a currentDisplayMode?
      availableDisplayModes: AVAILABLE_DISPLAY_MODES,
      // todo:  containerDimensions: {} (depends on displayMode)
      locale: navigator.language,
      timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      userAgent: navigator.userAgent,
      platform: 'desktop',
      deviceCapabilities: {
        touch: navigator.maxTouchPoints > 0,
        hover: window.matchMedia('(hover: hover)').matches,
      },
      safeAreaInsets: {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
      },
    };

    return context;
  }, [resolvedTheme, displayMode]);

  // The server serializes content with a `type` discriminator that matches
  // CallToolResult.content, so we pass through without re-mapping.
  const appToolResult = useMemo((): CallToolResult | undefined => {
    if (!toolResult) return undefined;
    return {
      content: toolResult.content as unknown as CallToolResult['content'],
      structuredContent: toolResult.structuredContent as { [key: string]: unknown } | undefined,
    };
  }, [toolResult]);

  const isToolCancelled = !!toolCancelled;
  const isLoading = !sandboxConfig || !resource.html;

  // Render content based on state: error, loading, or app
  const renderContent = () => {
    if (error) {
      return (
        <div className="p-4 text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      );
    }

    if (isLoading) {
      return <div className="flex items-center justify-center p-4">Loading MCP app...</div>;
    }

    return (
      <AppRenderer
        sandbox={sandboxConfig}
        toolName={resourceUri}
        html={resource.html ?? undefined}
        toolInput={toolInput?.arguments}
        toolInputPartial={toolInputPartial ? { arguments: toolInputPartial.arguments } : undefined}
        toolCancelled={isToolCancelled}
        hostContext={hostContext}
        toolResult={appToolResult}
        onOpenLink={handleOpenLink}
        onMessage={handleMessage}
        onCallTool={handleCallTool}
        onReadResource={handleReadResource}
        onLoggingMessage={handleLoggingMessage}
        onSizeChanged={handleSizeChanged}
        onError={handleError}
        // todo: add expected props from client SDK when available
        // onFallbackRequest={handleFallbackRequest}
        // hostInfo={hostInfo}
        // hostCapabilities={hostCapabilities}
      />
    );
  };

  // Compute container classes based on state.
  // When app declares explicit width (iframeWidth !== null), let SDK control iframe width.
  // When app is fluid (iframeWidth === null), force iframe to fill container with [&_iframe]:!w-full.
  const containerClasses = cn(
    'bg-background-default overflow-hidden',
    iframeWidth === null && '[&_iframe]:!w-full',
    error && 'border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20',
    !error && !isExpandedView && 'mt-6 mb-2',
    !error && !isExpandedView && resource.prefersBorder && 'border border-border-default rounded-lg'
  );

  // Compute container dimensions based on display mode.
  // - Expanded views: fill the container (100% width and height)
  // - Inline views with explicit width: use app-declared width (capped at 100% to prevent overflow)
  // - Inline views without width (fluid): use 100% width
  const containerStyle = isExpandedView
    ? { width: '100%', height: '100%' }
    : {
        width: iframeWidth !== null ? `${iframeWidth}px` : '100%',
        maxWidth: '100%',
        height: `${iframeHeight || DEFAULT_IFRAME_HEIGHT}px`,
      };

  return (
    <div className={containerClasses} style={containerStyle}>
      {renderContent()}
    </div>
  );
}
