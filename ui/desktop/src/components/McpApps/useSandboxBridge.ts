import { useRef, useEffect, useState, useCallback } from 'react';
import type {
  JsonRpcMessage,
  JsonRpcRequest,
  JsonRpcNotification,
  ToolInput,
  ToolInputPartial,
  ToolResult,
  ToolCancelled,
  HostContext,
  CspMetadata,
} from './types';
import { fetchMcpAppProxyUrl } from './utils';
import { useTheme } from '../../contexts/ThemeContext';
import packageJson from '../../../package.json';

interface SandboxBridgeOptions {
  resourceHtml: string;
  resourceCsp: CspMetadata | null;
  resourceUri: string;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  onMcpRequest: (method: string, params: unknown, id?: string | number) => Promise<unknown>;
  onSizeChanged?: (height: number, width?: number) => void;
}

interface SandboxBridgeResult {
  iframeRef: React.RefObject<HTMLIFrameElement | null>;
  proxyUrl: string | null;
}

export function useSandboxBridge(options: SandboxBridgeOptions): SandboxBridgeResult {
  const {
    resourceHtml,
    resourceCsp,
    resourceUri,
    toolInput,
    toolInputPartial,
    toolResult,
    toolCancelled,
    onMcpRequest,
    onSizeChanged,
  } = options;

  const { resolvedTheme } = useTheme();
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const isGuestInitializedRef = useRef(false);
  const [proxyUrl, setProxyUrl] = useState<string | null>(null);
  const [isGuestInitialized, setIsGuestInitialized] = useState(false);

  useEffect(() => {
    fetchMcpAppProxyUrl(resourceCsp).then(setProxyUrl);
  }, [resourceCsp]);

  useEffect(() => {
    setIsGuestInitialized(false);
    isGuestInitializedRef.current = false;
  }, [resourceUri]);

  const sendToSandbox = useCallback((message: JsonRpcMessage) => {
    iframeRef.current?.contentWindow?.postMessage(message, '*');
  }, []);

  const handleJsonRpcMessage = useCallback(
    async (data: unknown) => {
      if (!data || typeof data !== 'object') return;

      // Handle notifications (no id)
      if ('method' in data && !('id' in data)) {
        const msg = data as JsonRpcNotification;

        if (msg.method === 'ui/notifications/sandbox-ready') {
          sendToSandbox({
            jsonrpc: '2.0',
            method: 'ui/notifications/sandbox-resource-ready',
            params: { html: resourceHtml, csp: resourceCsp },
          });
          return;
        }

        if (msg.method === 'ui/notifications/initialized') {
          setIsGuestInitialized(true);
          isGuestInitializedRef.current = true;
          return;
        }

        if (msg.method === 'ui/notifications/size-changed') {
          const params = msg.params as { height: number; width?: number };
          onSizeChanged?.(params.height, params.width);
          return;
        }
      }

      // Handle requests (with id)
      if ('method' in data && 'id' in data) {
        const msg = data as JsonRpcRequest;

        try {
          if (msg.method === 'ui/initialize') {
            if (msg.id === undefined) return;

            const iframe = iframeRef.current;
            const hostContext: HostContext = {
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
              safeAreaInsets: { top: 0, right: 0, bottom: 0, left: 0 },
            };

            sendToSandbox({
              jsonrpc: '2.0',
              id: msg.id,
              result: {
                protocolVersion: '2025-06-18',
                hostCapabilities: { links: true, messages: true },
                hostInfo: {
                  name: packageJson.productName,
                  version: packageJson.version,
                },
                hostContext,
              },
            });
            return;
          }

          // Delegate other requests to handler
          const result = await onMcpRequest(msg.method, msg.params, msg.id);
          if (msg.id !== undefined) {
            sendToSandbox({ jsonrpc: '2.0', id: msg.id, result });
          }
        } catch (error) {
          console.error(`[Sandbox Bridge] Error handling ${msg.method}:`, error);
          if (msg.id !== undefined) {
            sendToSandbox({
              jsonrpc: '2.0',
              id: msg.id,
              error: {
                code: -32603,
                message: error instanceof Error ? error.message : 'Unknown error',
              },
            });
          }
        }
      }
    },
    [resourceHtml, resourceCsp, resolvedTheme, sendToSandbox, onMcpRequest, onSizeChanged]
  );

  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      if (event.source !== iframeRef.current?.contentWindow) return;
      handleJsonRpcMessage(event.data);
    };
    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, [handleJsonRpcMessage]);

  // Send tool input notification when it changes
  useEffect(() => {
    if (!isGuestInitialized || !toolInput) return;
    sendToSandbox({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-input',
      params: { arguments: toolInput.arguments },
    });
  }, [isGuestInitialized, toolInput, sendToSandbox]);

  // Send partial tool input (streaming) notification when it changes
  useEffect(() => {
    if (!isGuestInitialized || !toolInputPartial) return;
    sendToSandbox({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-input-partial',
      params: { arguments: toolInputPartial.arguments },
    });
  }, [isGuestInitialized, toolInputPartial, sendToSandbox]);

  // Send tool result notification when it changes
  useEffect(() => {
    if (!isGuestInitialized || !toolResult) return;
    sendToSandbox({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-result',
      params: toolResult,
    });
  }, [isGuestInitialized, toolResult, sendToSandbox]);

  // Send tool cancelled notification when it changes
  useEffect(() => {
    if (!isGuestInitialized || !toolCancelled) return;
    sendToSandbox({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-cancelled',
      params: toolCancelled.reason ? { reason: toolCancelled.reason } : {},
    });
  }, [isGuestInitialized, toolCancelled, sendToSandbox]);

  // Send theme changes when it changes
  useEffect(() => {
    if (!isGuestInitialized) return;
    sendToSandbox({
      jsonrpc: '2.0',
      method: 'ui/notifications/host-context-changed',
      params: { theme: resolvedTheme },
    });
  }, [isGuestInitialized, resolvedTheme, sendToSandbox]);

  useEffect(() => {
    if (!isGuestInitialized || !iframeRef.current) return;

    const iframe = iframeRef.current;
    let lastWidth = iframe.clientWidth;
    let lastHeight = iframe.clientHeight;

    const observer = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      const w = Math.round(width);
      const h = Math.round(height);

      if (w !== lastWidth || h !== lastHeight) {
        lastWidth = w;
        lastHeight = h;
        sendToSandbox({
          jsonrpc: '2.0',
          method: 'ui/notifications/host-context-changed',
          params: {
            viewport: {
              width: w,
              height: h,
              maxWidth: window.innerWidth,
              maxHeight: window.innerHeight,
            },
          },
        });
      }
    });

    observer.observe(iframe);
    return () => observer.disconnect();
  }, [isGuestInitialized, sendToSandbox]);

  // Cleanup on unmount - use ref to capture latest initialized state
  useEffect(() => {
    return () => {
      if (isGuestInitializedRef.current) {
        sendToSandbox({
          jsonrpc: '2.0',
          id: Date.now(),
          method: 'ui/resource-teardown',
          params: { reason: 'Component unmounting' },
        });
      }
    };
  }, [sendToSandbox]);

  return { iframeRef, proxyUrl };
}
