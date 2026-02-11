/**
 * McpAppRenderer — Renders interactive MCP App UIs inside a sandboxed iframe.
 *
 * This component implements the host side of the MCP Apps protocol using the
 * @mcp-ui/client SDK's AppRenderer. It handles resource fetching, sandbox
 * proxy setup, CSP enforcement, and bidirectional communication with guest apps.
 *
 * Protocol references:
 * - MCP Apps Extension (ext-apps): https://github.com/modelcontextprotocol/ext-apps
 * - MCP-UI Client SDK: https://github.com/idosal/mcp-ui
 * - App Bridge types: @modelcontextprotocol/ext-apps/app-bridge
 *
 * Display modes:
 * - "inline" | "fullscreen" | "pip" — standard MCP display modes
 * - "standalone" — Goose-specific mode for dedicated Electron windows
 */

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
import FlyingBird from '../FlyingBird';
import {
  GooseDisplayMode,
  SandboxPermissions,
  ToolCancelled,
  ToolInput,
  ToolInputPartial,
  ToolResult,
} from './types';

const DEFAULT_IFRAME_HEIGHT = 200;

const AVAILABLE_DISPLAY_MODES: McpUiDisplayMode[] = ['inline'];

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
  resourceUri: string;
  extensionName: string;
  sessionId?: string | null;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  append?: (text: string) => void;
  displayMode?: GooseDisplayMode;
  cachedHtml?: string;
}

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
            // todo: pass meta?.ui?.permissions to SDK once it supports sendSandboxResourceReady
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

      // rmcp serializes Content with a `type` discriminator via #[serde(tag = "type")].
      // Our generated TS types don't reflect this, but the wire format matches CallToolResult.content.
      return {
        content: (response.data?.content || []) as unknown as CallToolResult['content'],
        isError: response.data?.is_error || false,
        structuredContent: response.data?.structured_content as { [key: string]: unknown } | undefined,
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

  /**
   * Height: non-positive values are ignored (keeps previous height).
   * Width: if provided, container uses that width (capped at 100%);
   * if omitted or non-positive, container is fluid (100%).
   */
  const handleSizeChanged = useCallback(
    ({ height, width }: McpUiSizeChangedNotification['params']) => {
      if (height !== undefined && height > 0) {
        setIframeHeight(height);
      }
      if (width !== undefined) {
        setIframeWidth(width > 0 ? width : null);
      }
    },
    []
  );

  const handleError = useCallback((err: Error) => {
    console.error('[MCP App Error]:', err);
    setError(errorMessage(err));
  }, []);

  const mcpUiCsp = useMemo((): McpUiResourceCsp | undefined => {
    if (!sandboxCsp) return undefined;
    return {
      connectDomains: sandboxCsp.connectDomains ?? undefined,
      resourceDomains: sandboxCsp.resourceDomains ?? undefined,
      frameDomains: sandboxCsp.frameDomains ?? undefined,
      baseUriDomains: sandboxCsp.baseUriDomains ?? undefined,
    };
  }, [sandboxCsp]);

  const sandboxConfig = useMemo(() => {
    if (!sandboxUrl) return null;
    return {
      url: sandboxUrl,
      permissions: resource.permissions || 'allow-scripts allow-same-origin',
      csp: mcpUiCsp,
    };
  }, [sandboxUrl, resource.permissions, mcpUiCsp]);

  const hostContext = useMemo((): McpUiHostContext => {
    const context: McpUiHostContext = {
      // todo: toolInfo: {}
      theme: resolvedTheme,
      // todo: styles: { variables: {}, styles: {} }
      // 'standalone' is a Goose-specific display mode (dedicated Electron window)
      // that maps to the spec's inline | fullscreen | pip modes.
      displayMode: displayMode as McpUiDisplayMode,
      availableDisplayModes: displayMode === 'standalone'
        ? [displayMode as McpUiDisplayMode]
        : AVAILABLE_DISPLAY_MODES,
      // todo: containerDimensions: {} (depends on displayMode)
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

  const appToolResult = useMemo((): CallToolResult | undefined => {
    if (!toolResult) return undefined;
    // rmcp serializes Content with a `type` discriminator via #[serde(tag = "type")].
    // Our generated TS types don't reflect this, but the wire format matches CallToolResult.content.
    return {
      content: toolResult.content as unknown as CallToolResult['content'],
      structuredContent: toolResult.structuredContent as { [key: string]: unknown } | undefined,
    };
  }, [toolResult]);

  const isToolCancelled = !!toolCancelled;
  const isLoading = !sandboxConfig || !resource.html;

  const renderContent = () => {
    if (error) {
      return (
        <div className="p-4 text-red-700 dark:text-red-300">Failed to load MCP app: {error}</div>
      );
    }

    if (isLoading) {
      return (
        <div className="relative flex h-full w-full items-center justify-center overflow-hidden rounded bg-black/[0.03] dark:bg-white/[0.03]">
          <div
            className="absolute inset-0 animate-shimmer"
            style={{
              animationDuration: '2s',
              background:
                'linear-gradient(90deg, transparent 0%, rgba(128,128,128,0.08) 40%, rgba(128,128,128,0.12) 50%, rgba(128,128,128,0.08) 60%, transparent 100%)',
            }}
          />
          <FlyingBird className="relative z-10 scale-200 opacity-30" cycleInterval={120} />
        </div>
      );
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
      />
    );
  };

  const containerClasses = cn(
    'bg-background-default overflow-hidden',
    iframeWidth === null && '[&_iframe]:!w-full',
    error && 'border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20',
    !error && !isExpandedView && 'mt-6 mb-2',
    !error && !isExpandedView && resource.prefersBorder && 'border border-border-default rounded-lg'
  );

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
