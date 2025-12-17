import { useRef, useEffect, useState, useCallback } from 'react';
import type {
  JsonRpcMessage,
  JsonRpcResponse,
  IncomingGuestMessage,
  ToolInput,
  ToolInputPartial,
  ToolResult,
  ToolCancelled,
  HostContext,
  SizeChangedNotification,
  MessageRequest,
  OpenLinkRequest,
  LoggingMessageRequest,
  CallToolRequest,
  ListResourcesRequest,
  ListResourceTemplatesRequest,
  ReadResourceRequest,
  ListPromptsRequest,
  PingRequest,
  CspMetadata,
} from './types';
import {
  fetchMcpAppProxyUrl,
  createSandboxResourceReadyMessage,
  createInitializeResponse,
  createHostContextChangedNotification,
  createToolInputNotification,
  createToolInputPartialNotification,
  createToolResultNotification,
  createToolCancelledNotification,
  createResourceTeardownRequest,
} from './utils';
import { useTheme } from '../../contexts/ThemeContext';

/** Handler function type that may return a response to send back to the guest */
type MessageHandler<T> = (msg: T) => JsonRpcResponse | null;

interface SandboxBridgeOptions {
  resourceHtml: string;
  resourceCsp: CspMetadata | null;
  resourceUri: string;
  toolInput?: ToolInput;
  toolInputPartial?: ToolInputPartial;
  toolResult?: ToolResult;
  toolCancelled?: ToolCancelled;
  onMessage?: MessageHandler<MessageRequest>;
  onOpenLink?: MessageHandler<OpenLinkRequest>;
  onNotificationMessage?: MessageHandler<LoggingMessageRequest>;
  onToolsCall?: MessageHandler<CallToolRequest>;
  onResourcesList?: MessageHandler<ListResourcesRequest>;
  onResourceTemplatesList?: MessageHandler<ListResourceTemplatesRequest>;
  onResourcesRead?: MessageHandler<ReadResourceRequest>;
  onPromptsList?: MessageHandler<ListPromptsRequest>;
  onPing?: MessageHandler<PingRequest>;
  onSizeChanged?: (msg: SizeChangedNotification) => null;
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
    onMessage,
    onOpenLink,
    onNotificationMessage,
    onToolsCall,
    onResourcesList,
    onResourceTemplatesList,
    onResourcesRead,
    onPromptsList,
    onPing,
    onSizeChanged,
  } = options;

  const { resolvedTheme } = useTheme();

  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const pendingMessagesRef = useRef<JsonRpcMessage[]>([]);

  const [proxyUrl, setProxyUrl] = useState<string | null>(null);
  const [isGuestInitialized, setIsGuestInitialized] = useState(false);

  useEffect(() => {
    fetchMcpAppProxyUrl(resourceCsp).then(setProxyUrl);
  }, [resourceCsp]);

  useEffect(() => {
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

  const handleJsonRpcMessage = useCallback(
    (data: unknown) => {
      if (!data || typeof data !== 'object' || !('method' in data)) return;
      const msg = data as IncomingGuestMessage;

      console.log(`[Sandbox Bridge] Incoming message: ${msg.method}`, { msg });

      switch (msg.method) {
        case 'ui/notifications/sandbox-ready': {
          sendToSandbox(createSandboxResourceReadyMessage(resourceHtml, resourceCsp));
          return;
        }

        case 'ui/initialize': {
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

        case 'ui/notifications/initialized': {
          setIsGuestInitialized(true);
          flushPendingMessages();
          return;
        }

        case 'ui/notifications/size-changed': {
          onSizeChanged?.(msg);
          return;
        }

        case 'ui/open-link': {
          const response = onOpenLink?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'ui/message': {
          const response = onMessage?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'notifications/message': {
          const response = onNotificationMessage?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'tools/call': {
          const response = onToolsCall?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'resources/list': {
          const response = onResourcesList?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'resources/templates/list': {
          const response = onResourceTemplatesList?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'resources/read': {
          const response = onResourcesRead?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'prompts/list': {
          const response = onPromptsList?.(msg);
          if (response) sendToSandbox(response);
          return;
        }

        case 'ping': {
          const response = onPing?.(msg);
          if (response) sendToSandbox(response);
          return;
        }
      }
    },
    [
      resourceHtml,
      resourceCsp,
      resolvedTheme,
      sendToSandbox,
      flushPendingMessages,
      onToolsCall,
      onNotificationMessage,
      onOpenLink,
      onMessage,
      onResourcesList,
      onResourceTemplatesList,
      onResourcesRead,
      onPromptsList,
      onPing,
      onSizeChanged,
    ]
  );

  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      const iframe = iframeRef.current;
      if (!iframe || event.source !== iframe.contentWindow) {
        return;
      }
      handleJsonRpcMessage(event.data);
    };

    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, [handleJsonRpcMessage]);

  // Send tool input when guest is initialized
  useEffect(() => {
    if (!isGuestInitialized || !toolInput) return;
    sendToSandbox(createToolInputNotification(toolInput));
  }, [isGuestInitialized, toolInput, sendToSandbox]);

  // Send partial tool input (streaming) when guest is initialized
  useEffect(() => {
    if (!isGuestInitialized || !toolInputPartial) return;
    sendToSandbox(createToolInputPartialNotification(toolInputPartial));
  }, [isGuestInitialized, toolInputPartial, sendToSandbox]);

  // Send tool result when guest is initialized and result is available
  useEffect(() => {
    if (!isGuestInitialized || !toolResult) return;
    sendToSandbox(createToolResultNotification(toolResult));
  }, [isGuestInitialized, toolResult, sendToSandbox]);

  // Send tool cancelled notification when toolCancelled changes
  useEffect(() => {
    if (!isGuestInitialized || !toolCancelled) return;
    sendToSandbox(createToolCancelledNotification(toolCancelled));
  }, [isGuestInitialized, toolCancelled, sendToSandbox]);

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

  // Send resource teardown request when component unmounts
  useEffect(() => {
    const currentSendToSandbox = sendToSandbox;
    const checkInitialized = () => isGuestInitialized;

    return () => {
      if (checkInitialized()) {
        const { message } = createResourceTeardownRequest('Component unmounting');
        currentSendToSandbox(message);
      }
    };
  }, [sendToSandbox, isGuestInitialized]);

  return {
    iframeRef,
    proxyUrl,
  };
}
