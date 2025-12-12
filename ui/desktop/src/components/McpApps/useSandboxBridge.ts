import { useRef, useEffect, useState, useCallback } from 'react';
import { JsonRpcMessage, JsonRpcRequest } from './types';
import {
  fetchMcpAppProxyUrl,
  createSandboxResourceReadyMessage,
  createInitializeResponse,
} from './utils';

interface SandboxBridgeOptions {
  resourceHtml: string;
  resourceCsp: Record<string, string[]> | null;
  resourceUri: string;
}

interface SandboxBridgeResult {
  iframeRef: React.RefObject<HTMLIFrameElement | null>;
  proxyUrl: string | null;
  iframeHeight: number;
}

export function useSandboxBridge(options: SandboxBridgeOptions): SandboxBridgeResult {
  const { resourceHtml, resourceCsp, resourceUri } = options;

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
          sendToSandbox(createInitializeResponse(request.id));
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
              setIframeHeight(Math.max(200, height));
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

        case 'ui/message':
          // TODO: Send message content to chat
          console.log('🐛 McpAppRenderer: ui/message request', data);
          return;

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
    [resourceHtml, resourceCsp, sendToSandbox, flushPendingMessages]
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

  // Suppress unused variable warnings - these track state for future use
  void isSandboxReady;
  void isGuestInitialized;

  return {
    iframeRef,
    proxyUrl,
    iframeHeight,
  };
}
