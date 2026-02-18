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

import { AppRenderer, type RequestHandlerExtra } from '@mcp-ui/client';
import type {
  McpUiDisplayMode,
  McpUiHostContext,
  McpUiResourceCsp,
  McpUiResourcePermissions,
  McpUiSizeChangedNotification,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import type { CallToolResult, JSONRPCRequest } from '@modelcontextprotocol/sdk/types.js';
import { GripHorizontal, Maximize2, Minimize2, PictureInPicture2, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
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
  McpAppToolCancelled,
  McpAppToolInput,
  McpAppToolInputPartial,
  McpAppToolResult,
  DimensionLayout,
  OnDisplayModeChange,
} from './types';

const DEFAULT_IFRAME_HEIGHT = 200;

const AVAILABLE_DISPLAY_MODES: McpUiDisplayMode[] = ['inline', 'fullscreen', 'pip'];

const DISPLAY_MODE_LAYOUTS: Record<GooseDisplayMode, DimensionLayout> = {
  inline: { width: 'fixed', height: 'unbounded' },
  fullscreen: { width: 'fixed', height: 'fixed' },
  standalone: { width: 'fixed', height: 'fixed' },
  pip: { width: 'fixed', height: 'fixed' },
  // sidecar: { width: 'fixed', height: 'flexible' }, // example on how to use flexible layout
};

function getContainerDimensions(
  displayMode: GooseDisplayMode,
  measuredWidth: number,
  measuredHeight: number
): McpUiHostContext['containerDimensions'] {
  const layout = DISPLAY_MODE_LAYOUTS[displayMode] ?? DISPLAY_MODE_LAYOUTS.inline;

  // Only require a measurement for axes that are fixed or flexible (unbounded axes are omitted).
  if (
    (layout.width !== 'unbounded' && measuredWidth <= 0) ||
    (layout.height !== 'unbounded' && measuredHeight <= 0)
  )
    return undefined;

  const widthDimension = (() => {
    switch (layout.width) {
      case 'fixed':
        return { width: measuredWidth };
      case 'flexible':
        return { maxWidth: measuredWidth };
      case 'unbounded':
        return {};
    }
  })();

  const heightDimension = (() => {
    switch (layout.height) {
      case 'fixed':
        return { height: measuredHeight };
      case 'flexible':
        return { maxHeight: measuredHeight };
      case 'unbounded':
        return {};
    }
  })();

  return { ...widthDimension, ...heightDimension };
}

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

const PIP_WIDTH = 400;
const PIP_HEIGHT = 300;

interface McpAppRendererProps {
  resourceUri: string;
  extensionName: string;
  sessionId?: string | null;
  toolInput?: McpAppToolInput;
  toolInputPartial?: McpAppToolInputPartial;
  toolResult?: McpAppToolResult;
  toolCancelled?: McpAppToolCancelled;
  append?: (text: string) => void;
  displayMode?: GooseDisplayMode;
  cachedHtml?: string;
  onDisplayModeChange?: OnDisplayModeChange;
}

interface ResourceMeta {
  csp: McpUiResourceCsp | null;
  permissions: SandboxPermissions | null;
  prefersBorder: boolean;
}

const DEFAULT_META: ResourceMeta = { csp: null, permissions: null, prefersBorder: true };

// Lifecycle: idle → loading_resource → loading_sandbox → ready
// Any state can transition to error. The sandbox URL is fetched only once
// to prevent iframe recreation (which would cause the app to lose state).
type AppState =
  | { status: 'idle' }
  | { status: 'loading_resource'; html: string | null; meta: ResourceMeta }
  | { status: 'loading_sandbox'; html: string; meta: ResourceMeta }
  | {
      status: 'ready';
      html: string;
      meta: ResourceMeta;
      sandboxUrl: URL;
      sandboxCsp: McpUiResourceCsp | null;
    }
  | { status: 'error'; message: string; html: string | null; meta: ResourceMeta };

type AppAction =
  | { type: 'FETCH_RESOURCE' }
  | { type: 'RESOURCE_LOADED'; html: string | null; meta: ResourceMeta }
  | { type: 'RESOURCE_FAILED'; message: string }
  | { type: 'SANDBOX_READY'; sandboxUrl: string; sandboxCsp: McpUiResourceCsp | null }
  | { type: 'SANDBOX_FAILED'; message: string }
  | { type: 'ERROR'; message: string };

function getMeta(state: AppState): ResourceMeta {
  return state.status === 'idle' ? DEFAULT_META : state.meta;
}

function getHtml(state: AppState): string | null {
  return state.status === 'idle' ? null : state.html;
}

function appReducer(state: AppState, action: AppAction): AppState {
  const meta = getMeta(state);
  const html = getHtml(state);

  switch (action.type) {
    case 'FETCH_RESOURCE':
      return { status: 'loading_resource', html, meta };

    case 'RESOURCE_LOADED':
      if (!action.html) {
        return { status: 'loading_resource', html: null, meta: action.meta };
      }
      if (state.status === 'ready') {
        return { ...state, html: action.html, meta: action.meta };
      }
      return { status: 'loading_sandbox', html: action.html, meta: action.meta };

    case 'RESOURCE_FAILED':
      if (html) {
        if (state.status === 'ready') return state;
        return { status: 'loading_sandbox', html, meta };
      }
      return { status: 'error', message: action.message, html: null, meta };

    case 'SANDBOX_READY':
      if (!html) return state;
      return {
        status: 'ready',
        html,
        meta,
        sandboxUrl: new URL(action.sandboxUrl),
        sandboxCsp: action.sandboxCsp,
      };

    case 'SANDBOX_FAILED':
      return { status: 'error', message: action.message, html, meta };

    case 'ERROR':
      return { status: 'error', message: action.message, html, meta };
  }
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
  onDisplayModeChange,
}: McpAppRendererProps) {
  // Internal display mode — starts matching the prop, but can be changed by host-side controls.
  // Standalone mode is externally controlled (dedicated Electron window) and cannot be toggled.
  const [activeDisplayMode, setActiveDisplayMode] = useState<GooseDisplayMode>(displayMode);

  // Sync internal state when the prop changes (e.g. parent forces a mode).
  useEffect(() => {
    setActiveDisplayMode(displayMode);
  }, [displayMode]);

  const isStandalone = displayMode === 'standalone';

  // Display modes the app declared support for during ui/initialize.
  // null = not yet known (show all controls), empty = app didn't declare any.
  const [appDeclaredModes, setAppDeclaredModes] = useState<string[] | null>(null);

  // Remember the inline iframe height when leaving inline mode so we can
  // size the placeholder and landing target correctly on return.
  const inlineHeightRef = useRef(DEFAULT_IFRAME_HEIGHT);

  const changeDisplayMode = useCallback(
    (mode: GooseDisplayMode) => {
      const el = containerRef.current;
      const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

      // Save inline height before leaving inline (read from DOM for accuracy)
      if (activeDisplayMode === 'inline' && el) {
        inlineHeightRef.current = el.getBoundingClientRect().height || DEFAULT_IFRAME_HEIGHT;
      }

      // FLIP: capture First rect before state change
      const first = el?.getBoundingClientRect();

      setActiveDisplayMode(mode);
      onDisplayModeChange?.(mode);

      // FLIP: after React re-renders, compute Invert and Play
      if (el && first && !prefersReducedMotion) {
        requestAnimationFrame(() => {
          const last = el.getBoundingClientRect();

          const dx = first.left - last.left;
          const dy = first.top - last.top;
          const sw = first.width / last.width;
          const sh = first.height / last.height;

          if (dx === 0 && dy === 0 && sw === 1 && sh === 1) return;

          el.classList.add('is-flipping');
          el.animate(
            [
              {
                transformOrigin: 'top left',
                transform: `translate(${dx}px, ${dy}px) scale(${sw}, ${sh})`,
                borderRadius: '12px',
              },
              {
                transformOrigin: 'top left',
                transform: 'none',
                borderRadius: mode === 'pip' ? '12px' : mode === 'inline' ? '8px' : '0px',
              },
            ],
            {
              duration: 300,
              easing: 'cubic-bezier(0.4, 0, 0.2, 1)',
              fill: 'none',
            }
          ).onfinish = () => {
            el.classList.remove('is-flipping');
          };
        });
      }
    },
    [onDisplayModeChange, activeDisplayMode]
  );

  // PiP drag state
  const [pipPosition, setPipPosition] = useState({ x: 0, y: 0 });
  const pipDragRef = useRef<{ startX: number; startY: number; originX: number; originY: number } | null>(null);

  const handlePipPointerDown = useCallback((e: React.PointerEvent) => {
    e.preventDefault();
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    pipDragRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      originX: pipPosition.x,
      originY: pipPosition.y,
    };
  }, [pipPosition]);

  const handlePipPointerMove = useCallback((e: React.PointerEvent) => {
    if (!pipDragRef.current) return;
    const dx = e.clientX - pipDragRef.current.startX;
    const dy = e.clientY - pipDragRef.current.startY;
    setPipPosition({
      x: pipDragRef.current.originX + dx,
      y: pipDragRef.current.originY + dy,
    });
  }, []);

  const handlePipPointerUp = useCallback(() => {
    pipDragRef.current = null;
  }, []);

  // Intercept app postMessages for:
  // 1. ui/initialize — extract appCapabilities.availableDisplayModes
  // 2. ui/request-display-mode — change display mode on behalf of the app
  // We filter by event.source to only handle messages from iframes within our container.
  useEffect(() => {
    if (isStandalone) return;

    const handleMessage = (e: MessageEvent) => {
      const data = e.data;
      if (!data || typeof data !== 'object') return;

      // Only respond to messages from iframes owned by this instance.
      const container = containerRef.current;
      if (!container || !e.source) return;
      const iframes = container.querySelectorAll('iframe');
      const fromOurIframe = Array.from(iframes).some(
        (iframe) => iframe.contentWindow === e.source
      );
      if (!fromOurIframe) return;

      // Extract app's declared display modes from ui/initialize
      if (data.method === 'ui/initialize' && data.params) {
        const caps = data.params.appCapabilities || data.params.capabilities;
        if (caps?.availableDisplayModes && Array.isArray(caps.availableDisplayModes)) {
          setAppDeclaredModes(caps.availableDisplayModes);
        }
      }

      // Handle app-initiated display mode requests
      if (data.method === 'ui/request-display-mode' && data.params?.mode) {
        const requested = data.params.mode as McpUiDisplayMode;
        if (AVAILABLE_DISPLAY_MODES.includes(requested)) {
          changeDisplayMode(requested);
        }
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [isStandalone, changeDisplayMode]);

  // Escape key exits fullscreen
  useEffect(() => {
    if (activeDisplayMode !== 'fullscreen') return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') changeDisplayMode('inline');
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeDisplayMode, changeDisplayMode]);

  // Reset PiP position when entering PiP mode
  useEffect(() => {
    if (activeDisplayMode === 'pip') {
      setPipPosition({ x: 0, y: 0 });
    }
  }, [activeDisplayMode]);

  const { resolvedTheme } = useTheme();

  const initialState: AppState = cachedHtml
    ? { status: 'loading_sandbox', html: cachedHtml, meta: DEFAULT_META }
    : { status: 'idle' };

  const [state, dispatch] = useReducer(appReducer, initialState);
  const [iframeHeight, setIframeHeight] = useState(DEFAULT_IFRAME_HEIGHT);

  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState<number>(0);
  const [containerHeight, setContainerHeight] = useState<number>(0);

  // Fetch the resource from the extension to get HTML and metadata (CSP, permissions, etc.).
  // If cachedHtml is provided we show it immediately; the fetch updates metadata and
  // replaces HTML only if the server returns different content.
  useEffect(() => {
    if (!sessionId) return;

    const fetchResourceData = async () => {
      dispatch({ type: 'FETCH_RESOURCE' });
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
          const rawMeta = content._meta as
            | {
                ui?: {
                  csp?: McpUiResourceCsp;
                  permissions?: McpUiResourcePermissions;
                  prefersBorder?: boolean;
                };
              }
            | undefined;

          dispatch({
            type: 'RESOURCE_LOADED',
            html: content.text ?? cachedHtml ?? null,
            meta: {
              csp: rawMeta?.ui?.csp || null,
              // todo: pass permissions to SDK once it supports sendSandboxResourceReady
              // https://github.com/MCP-UI-Org/mcp-ui/issues/180
              permissions: null,
              prefersBorder: rawMeta?.ui?.prefersBorder ?? true,
            },
          });
        }
      } catch (err) {
        console.error('[McpAppRenderer] Error fetching resource:', err);
        if (cachedHtml) {
          console.warn('Failed to fetch fresh resource, using cached version:', err);
        }
        dispatch({
          type: 'RESOURCE_FAILED',
          message: errorMessage(err, 'Failed to load resource'),
        });
      }
    };

    fetchResourceData();
  }, [resourceUri, extensionName, sessionId, cachedHtml]);

  // Create the sandbox proxy URL once we have HTML and metadata.
  // Fetched only once — recreating the proxy would destroy iframe state.
  const pendingCsp = state.status === 'loading_sandbox' ? state.meta.csp : null;
  useEffect(() => {
    if (state.status !== 'loading_sandbox') return;

    fetchMcpAppProxyUrl(pendingCsp).then((url) => {
      if (url) {
        dispatch({ type: 'SANDBOX_READY', sandboxUrl: url, sandboxCsp: pendingCsp });
      } else {
        dispatch({ type: 'SANDBOX_FAILED', message: 'Failed to initialize sandbox proxy' });
      }
    });
  }, [state.status, pendingCsp]);

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
        structuredContent: response.data?.structured_content as
          | { [key: string]: unknown }
          | undefined,
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

  const handleSizeChanged = useCallback(({ height }: McpUiSizeChangedNotification['params']) => {
    if (height !== undefined && height > 0) {
      setIframeHeight(height);
    }
  }, []);

  // Track the container's pixel dimensions so we can report them to apps via containerDimensions.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        setContainerWidth((prev) => (prev !== Math.round(width) ? Math.round(width) : prev));
        setContainerHeight((prev) => (prev !== Math.round(height) ? Math.round(height) : prev));
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const handleFallbackRequest = useCallback(
    async (request: JSONRPCRequest, _extra: RequestHandlerExtra) => {
      // todo: handle `sampling/createMessage` per https://github.com/block/goose/pull/7039
      if (request.method === 'sampling/createMessage') {
        return { status: 'success' as const };
      }
      return {
        status: 'error' as const,
        message: `Unhandled JSON-RPC method: ${request.method ?? '<unknown>'}`,
      };
    },
    []
  );

  const handleError = useCallback((err: Error) => {
    console.error('[MCP App Error]:', err);
    dispatch({ type: 'ERROR', message: errorMessage(err) });
  }, []);

  const meta = getMeta(state);
  const html = getHtml(state);

  const readyCsp = state.status === 'ready' ? state.sandboxCsp : null;
  const mcpUiCsp = useMemo((): McpUiResourceCsp | undefined => {
    if (!readyCsp) return undefined;
    return {
      connectDomains: readyCsp.connectDomains ?? undefined,
      resourceDomains: readyCsp.resourceDomains ?? undefined,
      frameDomains: readyCsp.frameDomains ?? undefined,
      baseUriDomains: readyCsp.baseUriDomains ?? undefined,
    };
  }, [readyCsp]);

  const readySandboxUrl = state.status === 'ready' ? state.sandboxUrl : null;
  const sandboxConfig = useMemo(() => {
    if (!readySandboxUrl) return null;
    return {
      url: readySandboxUrl,
      permissions: meta.permissions || 'allow-scripts allow-same-origin',
      csp: mcpUiCsp,
    };
  }, [readySandboxUrl, meta.permissions, mcpUiCsp]);

  const hostContext = useMemo((): McpUiHostContext => {
    const context: McpUiHostContext = {
      // todo: toolInfo: {}
      theme: resolvedTheme,
      // todo: styles: { variables: {}, styles: {} }
      displayMode: activeDisplayMode as McpUiDisplayMode,
      availableDisplayModes: isStandalone
        ? [activeDisplayMode as McpUiDisplayMode]
        : AVAILABLE_DISPLAY_MODES,
      containerDimensions: getContainerDimensions(
        activeDisplayMode,
        containerWidth,
        containerHeight
      ),
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
  }, [resolvedTheme, activeDisplayMode, isStandalone, containerWidth, containerHeight]);

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
  const isError = state.status === 'error';
  const isReady = state.status === 'ready';

  const renderContent = () => {
    if (isError) {
      return (
        <div className="p-4 text-red-700 dark:text-red-300">
          Failed to load MCP app: {state.message}
        </div>
      );
    }

    if (!isReady) {
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

    if (!sandboxConfig) return null;

    return (
      <AppRenderer
        sandbox={sandboxConfig}
        toolName={resourceUri}
        html={html ?? undefined}
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
        onFallbackRequest={handleFallbackRequest}
        onError={handleError}
      />
    );
  };

  // Host-side display mode control buttons.
  // Only show controls for modes the app declared support for (via appCapabilities.availableDisplayModes).
  // If the app hasn't initialized yet (appDeclaredModes === null), don't show controls.
  // Never show controls in standalone mode or error state.
  const appSupportsFullscreen =
    appDeclaredModes !== null && appDeclaredModes.includes('fullscreen');
  const appSupportsPip = appDeclaredModes !== null && appDeclaredModes.includes('pip');
  const showControls = !isStandalone && !isError && (appSupportsFullscreen || appSupportsPip);

  const renderDisplayModeControls = () => {
    if (!showControls) return null;

    if (activeDisplayMode === 'fullscreen') {
      return (
        <div className="no-drag absolute top-3 right-3 z-[60] flex gap-1">
          {appSupportsPip && (
            <button
              onClick={() => changeDisplayMode('pip')}
              className="cursor-pointer rounded-md bg-black/50 p-1.5 text-white backdrop-blur-sm transition-opacity hover:bg-black/70"
              title="Picture-in-Picture"
            >
              <PictureInPicture2 size={16} />
            </button>
          )}
          <button
            onClick={() => changeDisplayMode('inline')}
            className="cursor-pointer rounded-md bg-black/50 p-1.5 text-white backdrop-blur-sm transition-opacity hover:bg-black/70"
            title="Exit fullscreen (Esc)"
          >
            <Minimize2 size={16} />
          </button>
        </div>
      );
    }

    if (activeDisplayMode === 'pip') {
      return (
        <div className="sticky top-2 right-2 z-10 float-right mr-2 mt-2 flex gap-1">
          {appSupportsFullscreen && (
            <button
              onClick={() => changeDisplayMode('fullscreen')}
              className="cursor-pointer rounded-md bg-black/50 p-1 text-white backdrop-blur-sm transition-opacity hover:bg-black/70"
              title="Fullscreen"
            >
              <Maximize2 size={14} />
            </button>
          )}
          <button
            onClick={() => changeDisplayMode('inline')}
            className="cursor-pointer rounded-md bg-black/50 p-1 text-white backdrop-blur-sm transition-opacity hover:bg-black/70"
            title="Close"
          >
            <X size={14} />
          </button>
        </div>
      );
    }

    // Inline mode — show controls on hover
    return (
      <div className="absolute top-2 right-2 z-10 flex gap-1 opacity-0 transition-opacity group-hover/mcp-app:opacity-100">
        {appSupportsFullscreen && (
          <button
            onClick={() => changeDisplayMode('fullscreen')}
            className="cursor-pointer rounded-md bg-black/40 p-1.5 text-white backdrop-blur-sm transition-opacity hover:bg-black/60"
            title="Fullscreen"
          >
            <Maximize2 size={14} />
          </button>
        )}
        {appSupportsPip && (
          <button
            onClick={() => changeDisplayMode('pip')}
            className="cursor-pointer rounded-md bg-black/40 p-1.5 text-white backdrop-blur-sm transition-opacity hover:bg-black/60"
            title="Picture-in-Picture"
          >
            <PictureInPicture2 size={14} />
          </button>
        )}
      </div>
    );
  };

  const isFullscreen = activeDisplayMode === 'fullscreen';
  const isPip = activeDisplayMode === 'pip';
  const isInline = !isFullscreen && !isPip;

  // Single stable container — CSS switches between inline/fullscreen/pip positioning.
  // The AppRenderer and its iframe are never unmounted, preserving app state across mode changes.
  const containerClasses = cn(
    'mcp-app-container bg-background-default [&_iframe]:!w-full',
    isFullscreen && 'fixed inset-0 z-[1000] overflow-hidden [&_iframe]:!h-full',
    isPip &&
      'fixed z-[900] overflow-y-auto overflow-x-hidden rounded-xl border border-border-default shadow-2xl',
    isInline && 'group/mcp-app relative overflow-hidden',
    isInline && !isError && 'mt-6 mb-2',
    isInline && !isError && meta.prefersBorder && 'border border-border-default rounded-lg',
    isError && 'border border-red-500 rounded-lg bg-red-50 dark:bg-red-900/20'
  );

  const containerStyle: React.CSSProperties = {
    ...(isFullscreen
      ? {}
      : isPip
        ? {
            width: `${PIP_WIDTH}px`,
            height: `${PIP_HEIGHT}px`,
            right: `${16 - pipPosition.x}px`,
            bottom: `${16 - pipPosition.y}px`,
          }
        : {
            width: '100%',
            height: `${iframeHeight || DEFAULT_IFRAME_HEIGHT}px`,
          }),
  };

  return (
    <>
      {/* Placeholder in chat flow when app is detached (fullscreen or pip) */}
      {isFullscreen && (
        <div
          className="invisible mt-6 mb-2"
          style={{ width: '100%', height: `${inlineHeightRef.current}px` }}
        />
      )}
      {isPip && (
        <div
          className="mt-6 mb-2 flex items-center justify-center rounded-lg border border-dashed border-border-default bg-black/[0.02] dark:bg-white/[0.02]"
          style={{ width: '100%', height: `${inlineHeightRef.current}px` }}
        >
          <button
            onClick={() => changeDisplayMode('inline')}
            className="cursor-pointer flex items-center gap-2 rounded-md px-3 py-1.5 text-xs text-text-subtler transition-colors hover:bg-black/5 hover:text-text-default dark:hover:bg-white/5"
          >
            <PictureInPicture2 size={14} />
            <span>Playing in Picture-in-Picture</span>
          </button>
        </div>
      )}

      {/* Stable app container — never unmounted, only repositioned via CSS */}
      <div ref={containerRef} className={cn(containerClasses, isPip && 'group/pip')} style={containerStyle}>
        {isPip && (
          <div className="pointer-events-none sticky top-0 z-20 flex h-0 items-start justify-between opacity-0 transition-opacity group-hover/pip:pointer-events-auto group-hover/pip:opacity-100">
            <div
              className="pointer-events-auto m-1 cursor-grab rounded-md bg-black/50 p-1 text-white backdrop-blur-sm hover:bg-black/70 active:cursor-grabbing"
              onPointerDown={handlePipPointerDown}
              onPointerMove={handlePipPointerMove}
              onPointerUp={handlePipPointerUp}
            >
              <GripHorizontal size={14} />
            </div>
            <div className="m-1 flex gap-1">
              {renderDisplayModeControls()}
            </div>
          </div>
        )}
        <div className={cn('relative w-full', !isPip && 'h-full')}>
          {!isPip && renderDisplayModeControls()}
          {renderContent()}
        </div>
      </div>
    </>
  );
}
