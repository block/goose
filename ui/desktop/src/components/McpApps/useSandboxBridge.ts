import { useRef, useEffect, useState, useCallback } from 'react';
import { JsonRpcMessage, IncomingGuestMessage, ToolInput, ToolResult, HostContext } from './types';
import {
  fetchMcpAppProxyUrl,
  createSandboxResourceReadyMessage,
  createInitializeResponse,
  createHostContextChangedNotification,
  createToolInputNotification,
  createToolResultNotification,
} from './utils';
import { useTheme } from '../../contexts/ThemeContext';

interface SandboxBridgeOptions {
  resourceHtml: string;
  resourceCsp: Record<string, string[]> | null;
  resourceUri: string;
  toolInput?: ToolInput;
  toolResult?: ToolResult;
  appendMessage?: (value: string) => void;
}

interface SandboxBridgeResult {
  iframeRef: React.RefObject<HTMLIFrameElement | null>;
  proxyUrl: string | null;
  iframeHeight: number;
}

export function useSandboxBridge(options: SandboxBridgeOptions): SandboxBridgeResult {
  const { resourceHtml, resourceCsp, resourceUri, toolInput, toolResult, appendMessage } = options;

  const { resolvedTheme } = useTheme();

  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const pendingMessagesRef = useRef<JsonRpcMessage[]>([]);

  const [proxyUrl, setProxyUrl] = useState<string | null>(null);
  const [iframeHeight, setIframeHeight] = useState(200);
  const [isSandboxReady, setIsSandboxReady] = useState(false);
  const [isGuestInitialized, setIsGuestInitialized] = useState(false);

  useEffect(() => {
    fetchMcpAppProxyUrl(resourceCsp).then(setProxyUrl);
  }, [resourceCsp]);

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
    (data: unknown) => {
      if (!data || typeof data !== 'object' || !('method' in data)) return;
      const msg = data as IncomingGuestMessage;

      switch (msg.method) {
        case 'ui/notifications/sandbox-ready':
          console.log('[Sandbox Bridge] Sandbox ready');
          setIsSandboxReady(true);
          sendToSandbox(createSandboxResourceReadyMessage(resourceHtml, resourceCsp));
          return;

        case 'ui/initialize': {
          console.log('[Sandbox Bridge] Guest UI requesting initialization');
          const iframe = iframeRef.current;
          const hostContext: HostContext = {
            // TODO: Populate toolInfo when we have tool call context
            toolInfo: undefined,
            theme: resolvedTheme,
            displayMode: 'inline',
            availableDisplayModes: ['inline'],
            viewport: {
              width: iframe?.clientWidth ?? 0,
              height: iframe?.clientHeight ?? 0,
              maxWidth: window.innerWidth,
              maxHeight: window.innerHeight,
            },
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
          sendToSandbox(createInitializeResponse(msg.id, hostContext));
          return;
        }

        case 'ui/notifications/initialized':
          console.log('[Sandbox Bridge] Guest UI initialized');
          setIsGuestInitialized(true);
          flushPendingMessages();
          return;

        case 'ui/notifications/size-changed': {
          const newHeight = Math.max(200, msg.params.height);
          setIframeHeight(newHeight);

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
          return;
        }

        case 'ui/open-link': {
          window.electron.openExternal(msg.params.url).catch(console.error);
          return;
        }

        case 'ui/message': {
          const { content } = msg.params;
          if (content.type === 'text' && typeof content.text === 'string') {
            if (appendMessage) {
              appendMessage(content.text);
              window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
              if (msg.id !== undefined) {
                sendToSandbox({
                  jsonrpc: '2.0',
                  id: msg.id,
                  result: {},
                });
              }
            } else if (msg.id !== undefined) {
              sendToSandbox({
                jsonrpc: '2.0',
                id: msg.id,
                error: {
                  code: -32603,
                  message: 'Message handling not available',
                },
              });
            }
          }
          return;
        }
      }
    },
    [resourceHtml, resourceCsp, resolvedTheme, sendToSandbox, flushPendingMessages, appendMessage]
  );

  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      const iframe = iframeRef.current;
      if (!iframe || event.source !== iframe.contentWindow) {
        return;
      }
      handleMessage(event.data);
    };

    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, [handleMessage]);

  // Send tool input when guest is initialized
  useEffect(() => {
    if (!isGuestInitialized || !toolInput) return;
    console.log('[Sandbox Bridge] Sending tool input', toolInput);
    sendToSandbox(createToolInputNotification(toolInput));
  }, [isGuestInitialized, toolInput, sendToSandbox]);

  // Send tool result when guest is initialized and result is available
  useEffect(() => {
    if (!isGuestInitialized || !toolResult) return;
    console.log('[Sandbox Bridge] Sending tool result', toolResult);
    sendToSandbox(createToolResultNotification(toolResult));
  }, [isGuestInitialized, toolResult, sendToSandbox]);

  // Send theme changes to sandbox when resolvedTheme changes
  useEffect(() => {
    if (!isGuestInitialized) return;
    sendToSandbox(createHostContextChangedNotification({ theme: resolvedTheme }));
  }, [isGuestInitialized, resolvedTheme, sendToSandbox]);

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
