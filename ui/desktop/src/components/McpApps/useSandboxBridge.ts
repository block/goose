import { useRef, useEffect, useState, useCallback } from 'react';
import { JsonRpcMessage, JsonRpcRequest } from './types';
import {
  fetchMcpAppProxyUrl,
  createSandboxResourceReadyMessage,
  createInitializeResponse,
  createHostContextChangedNotification,
  getCurrentTheme,
  HostContext,
} from './utils';

interface SandboxBridgeOptions {
  resourceHtml: string;
  resourceCsp: Record<string, string[]> | null;
  resourceUri: string;
  appendMessage?: (value: string) => void;
}

interface SandboxBridgeResult {
  iframeRef: React.RefObject<HTMLIFrameElement | null>;
  proxyUrl: string | null;
  iframeHeight: number;
}

export function useSandboxBridge(options: SandboxBridgeOptions): SandboxBridgeResult {
  const { resourceHtml, resourceCsp, resourceUri, appendMessage } = options;

  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const pendingMessagesRef = useRef<JsonRpcMessage[]>([]);

  const [proxyUrl, setProxyUrl] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(200);
  const [isSandboxReady, setIsSandboxReady] = useState(false);
  const [isGuestInitialized, setIsGuestInitialized] = useState(false);

  // Fetch proxy URL on mount
  useEffect(() => {
    fetchMcpAppProxyUrl().then(setProxyUrl);
  }, []);

  // Reset state when resource changes
  useEffect(() => {
    setIsSandboxReady(false);
    setIsGuestInitialized(false);
    pendingMessagesRef.current = [];
  }, [resourceUri]);

  const sendToSandbox = useCallback((message: JsonRpcMessage) => {
    const iframe = iframeRef.current;
    if (iframe?.contentWindow) {
      iframe.contentWindow.postMessage(message, '*');
    }
  }, []);

  const flushPendingMessages = useCallback(() => {
    pendingMessagesRef.current.forEach((msg) => sendToSandbox(msg));
    pendingMessagesRef.current = [];
  }, [sendToSandbox]);

  const handleMessage = useCallback(
    (data: JsonRpcMessage) => {
      const method = 'method' in data ? data.method : undefined;

      switch (method) {
        case 'ui/notifications/sandbox-ready':
          console.log('🐛 McpAppRenderer: Sandbox ready');
          setIsSandboxReady(true);
          sendToSandbox(createSandboxResourceReadyMessage(resourceHtml, resourceCsp));
          return;

        case 'ui/initialize': {
          if (!('id' in data) || data.id === undefined) return;
          console.log('🐛 McpAppRenderer: Guest UI requesting initialization');
          const request = data as JsonRpcRequest;

          // Build host context with all fields from spec
          const iframe = iframeRef.current;
          const hostContext: HostContext = {
            // TODO: Populate toolInfo when we have tool call context
            toolInfo: undefined,
            theme: getCurrentTheme(),
            displayMode: 'inline',
            availableDisplayModes: ['inline'],
            viewport: iframe
              ? {
                  width: iframe.clientWidth,
                  height: iframe.clientHeight,
                  maxWidth: window.innerWidth,
                  maxHeight: window.innerHeight,
                }
              : undefined,
            locale: navigator.language,
            timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
            userAgent: navigator.userAgent,
            platform: 'desktop',
            deviceCapabilities: {
              touch: 'ontouchstart' in window || navigator.maxTouchPoints > 0,
              hover: window.matchMedia('(hover: hover)').matches,
            },
            safeAreaInsets: {
              top: 0,
              right: 0,
              bottom: 0,
              left: 0,
            },
          };

          sendToSandbox(createInitializeResponse(request.id, hostContext));
          return;
        }

        case 'ui/notifications/initialized':
          console.log('🐛 McpAppRenderer: Guest UI initialized');
          setIsGuestInitialized(true);
          flushPendingMessages();
          return;

        case 'ui/notifications/size-changed': {
          const params = 'params' in data ? data.params : undefined;
          if (params && typeof params === 'object' && 'height' in params) {
            const height = params.height;
            if (typeof height === 'number') {
              const newHeight = Math.max(200, height);
              setIframeHeight(newHeight);

              // Send updated viewport dimensions to guest
              const iframe = iframeRef.current;
              if (iframe) {
                sendToSandbox(
                  createHostContextChangedNotification({
                    viewport: {
                      width: iframe.clientWidth,
                      height: newHeight,
                      maxWidth: window.innerWidth,
                      maxHeight: window.innerHeight,
                    },
                  })
                );
              }
            }
          }
          return;
        }

        case 'ui/open-link': {
          const params = 'params' in data ? data.params : undefined;
          if (params && typeof params === 'object' && 'url' in params) {
            const url = params.url;
            if (typeof url === 'string') {
              window.electron.openExternal(url).catch(console.error);
            }
          }
          return;
        }

        case 'ui/message': {
          const params = 'params' in data ? data.params : undefined;
          if (params && typeof params === 'object' && 'content' in params) {
            const content = params.content as { type?: string; text?: string };
            if (content.type === 'text' && typeof content.text === 'string') {
              if (appendMessage) {
                appendMessage(content.text);
                window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
                // Send success response if this is a request (has id)
                if ('id' in data && data.id !== undefined) {
                  sendToSandbox({
                    jsonrpc: '2.0',
                    id: data.id,
                    result: {},
                  });
                }
              } else {
                // Send error response if this is a request
                if ('id' in data && data.id !== undefined) {
                  sendToSandbox({
                    jsonrpc: '2.0',
                    id: data.id,
                    error: {
                      code: -32603,
                      message: 'Message handling not available',
                    },
                  });
                }
              }
            }
          }
          return;
        }

        default:
          // Forward non-UI methods to MCP server
          if (method && !method.startsWith('ui/')) {
            // TODO: Forward to MCP Apps server
            console.log('🐛 McpAppRenderer: Forward to MCP server', data);
            return;
          }
          console.log('🐛 McpAppRenderer: Unhandled message', data);
      }
    },
    [resourceHtml, resourceCsp, sendToSandbox, flushPendingMessages, appendMessage]
  );

  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      const iframe = iframeRef.current;
      if (!iframe || event.source !== iframe.contentWindow) {
        return;
      }

      const data = event.data as JsonRpcMessage;
      if (!data || typeof data !== 'object') {
        return;
      }

      handleMessage(data);
    };

    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, [handleMessage]);

  // Watch for theme changes via localStorage
  useEffect(() => {
    if (!isGuestInitialized) return;

    let lastTheme = getCurrentTheme();

    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === 'theme' || e.key === 'use_system_theme') {
        const newTheme = getCurrentTheme();
        if (newTheme !== lastTheme) {
          lastTheme = newTheme;
          sendToSandbox(createHostContextChangedNotification({ theme: newTheme }));
        }
      }
    };

    // Also handle system theme changes when using system theme
    const handleSystemThemeChange = () => {
      if (localStorage.getItem('use_system_theme') === 'true') {
        const newTheme = getCurrentTheme();
        if (newTheme !== lastTheme) {
          lastTheme = newTheme;
          sendToSandbox(createHostContextChangedNotification({ theme: newTheme }));
        }
      }
    };

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    window.addEventListener('storage', handleStorageChange);
    mediaQuery.addEventListener('change', handleSystemThemeChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
      mediaQuery.removeEventListener('change', handleSystemThemeChange);
    };
  }, [isGuestInitialized, sendToSandbox]);

  // Watch for viewport size changes
  useEffect(() => {
    if (!isGuestInitialized) return;

    const iframe = iframeRef.current;
    if (!iframe) return;

    let lastWidth = iframe.clientWidth;
    let lastHeight = iframe.clientHeight;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        const roundedWidth = Math.round(width);
        const roundedHeight = Math.round(height);

        if (roundedWidth !== lastWidth || roundedHeight !== lastHeight) {
          lastWidth = roundedWidth;
          lastHeight = roundedHeight;
          sendToSandbox(
            createHostContextChangedNotification({
              viewport: {
                width: roundedWidth,
                height: roundedHeight,
                maxWidth: window.innerWidth,
                maxHeight: window.innerHeight,
              },
            })
          );
        }
      }
    });

    resizeObserver.observe(iframe);

    return () => {
      resizeObserver.disconnect();
    };
  }, [isGuestInitialized, sendToSandbox]);

  void isSandboxReady;

  return {
    iframeRef,
    proxyUrl,
    iframeHeight,
  };
}
