/**
 * MCP App Bridge
 *
 * This hook provides communication between the Host and an MCP App loaded
 * in an iframe with a real URL. This gives the MCP App a proper origin and
 * secure context, which is required for Web Payments SDK, WebAuthn, and
 * other APIs that check window.isSecureContext.
 *
 * How it works:
 * - HTML is served from /mcp-app-proxy/{token} endpoint
 * - The iframe has a real localhost origin (secure context)
 * - postMessage is used for JSON-RPC communication
 */

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
  PermissionsMetadata,
} from './types';
import { createMcpAppProxyUrl } from './utils';
import { useTheme } from '../../contexts/ThemeContext';
import packageJson from '../../../package.json';
import { errorMessage } from '../../utils/conversionUtils';

interface SandboxBridgeOptions {
  resourceHtml: string;
  resourceCsp: CspMetadata | null;
  resourcePermissions: PermissionsMetadata | null;
  resourceUri: string;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  onMcpRequest: (
    method: string,
    params?: Record<string, unknown>,
    id?: string | number
  ) => Promise<unknown>;
  onSizeChanged?: (height: number, width?: number) => void;
}

interface SandboxBridgeResult {
  iframeRef: React.RefObject<HTMLIFrameElement | null>;
  proxyUrl: string | null;
  isLoading: boolean;
}

export function useSandboxBridge(options: SandboxBridgeOptions): SandboxBridgeResult {
  const {
    resourceHtml,
    resourceCsp,
    resourcePermissions,
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
  const [isLoading, setIsLoading] = useState(true);

  // Create the proxy URL when HTML changes
  useEffect(() => {
    if (!resourceHtml) {
      setProxyUrl(null);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    createMcpAppProxyUrl(resourceHtml, resourceCsp, resourcePermissions)
      .then((url) => {
        setProxyUrl(url);
        setIsLoading(false);
      })
      .catch((err) => {
        console.error('Failed to create proxy URL:', err);
        setProxyUrl(null);
        setIsLoading(false);
      });
  }, [resourceHtml, resourceCsp, resourcePermissions]);

  // Reset initialization state when resource changes
  useEffect(() => {
    isGuestInitializedRef.current = false;
  }, [resourceUri]);

  const sendToView = useCallback((message: JsonRpcMessage) => {
    iframeRef.current?.contentWindow?.postMessage(message, '*');
  }, []);

  const handleJsonRpcMessage = useCallback(
    async (data: unknown) => {
      if (!data || typeof data !== 'object') return;

      // Handle notifications (no id)
      if ('method' in data && !('id' in data)) {
        const msg = data as JsonRpcNotification;

        switch (msg.method) {
          case 'ui/notifications/initialized':
            isGuestInitializedRef.current = true;
            // Send any pending tool data that arrived before initialization
            if (toolInput) {
              sendToView({
                jsonrpc: '2.0',
                method: 'ui/notifications/tool-input',
                params: { arguments: toolInput.arguments },
              });
            }
            if (toolResult) {
              sendToView({
                jsonrpc: '2.0',
                method: 'ui/notifications/tool-result',
                params: toolResult,
              });
            }
            break;

          case 'ui/notifications/size-changed': {
            const params = msg.params as { height: number; width?: number };
            onSizeChanged?.(params.height, params.width);
            break;
          }
        }
        return;
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

            sendToView({
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

          const result = await onMcpRequest(msg.method, msg.params, msg.id);
          if (msg.id !== undefined) {
            sendToView({ jsonrpc: '2.0', id: msg.id, result });
          }
        } catch (error) {
          console.error(`[Secure Context Bridge] Error handling ${msg.method}:`, error);
          if (msg.id !== undefined) {
            sendToView({
              jsonrpc: '2.0',
              id: msg.id,
              error: {
                code: -32603,
                message: errorMessage(error),
              },
            });
          }
        }
      }
    },
    [
      resolvedTheme,
      sendToView,
      onMcpRequest,
      onSizeChanged,
      toolInput,
      toolResult,
    ]
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
    if (!isGuestInitializedRef.current || !toolInput) return;
    sendToView({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-input',
      params: { arguments: toolInput.arguments },
    });
  }, [toolInput, sendToView]);

  useEffect(() => {
    if (!isGuestInitializedRef.current || !toolInputPartial) return;
    sendToView({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-input-partial',
      params: { arguments: toolInputPartial.arguments },
    });
  }, [toolInputPartial, sendToView]);

  useEffect(() => {
    if (!isGuestInitializedRef.current || !toolResult) return;
    sendToView({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-result',
      params: toolResult,
    });
  }, [toolResult, sendToView]);

  useEffect(() => {
    if (!isGuestInitializedRef.current || !toolCancelled) return;
    sendToView({
      jsonrpc: '2.0',
      method: 'ui/notifications/tool-cancelled',
      params: toolCancelled.reason ? { reason: toolCancelled.reason } : {},
    });
  }, [toolCancelled, sendToView]);

  useEffect(() => {
    if (!isGuestInitializedRef.current) return;
    sendToView({
      jsonrpc: '2.0',
      method: 'ui/notifications/host-context-changed',
      params: { theme: resolvedTheme },
    });
  }, [resolvedTheme, sendToView]);

  useEffect(() => {
    if (!isGuestInitializedRef.current || !iframeRef.current) return;

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
        sendToView({
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
  }, [sendToView]);

  useEffect(() => {
    return () => {
      if (isGuestInitializedRef.current) {
        sendToView({
          jsonrpc: '2.0',
          id: Date.now(),
          method: 'ui/resource-teardown',
          params: { reason: 'Component unmounting' },
        });
      }
    };
  }, [sendToView]);

  return { iframeRef, proxyUrl, isLoading };
}
