/**
 * MCP Apps Renderer (SEP-1865)
 *
 * This component renders MCP Apps - interactive UI resources from MCP servers.
 * It implements the SEP-1865 specification for host-side rendering using the
 * official @modelcontextprotocol/ext-apps AppBridge.
 *
 * This is a temporary local implementation based on the mcp-ui PR #147.
 * Once that PR merges and @mcp-ui/client is updated, this can be replaced
 * with the official AppRenderer component.
 *
 * @see https://github.com/MCP-UI-Org/mcp-ui/pull/147
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import { toast } from 'react-toastify';
import { EmbeddedResource, ResourceContents } from '../../api';

// MCP Apps MIME type from SEP-1865
export const MCP_APPS_MIME_TYPE = 'text/html;profile=mcp-app';
export const MCP_APPS_URI_SCHEME = 'ui://';

// SEP-1865 JSON-RPC notification method names
export const MCP_APPS_METHODS = {
  TOOL_INPUT: 'ui/notifications/tool-input',
  TOOL_INPUT_PARTIAL: 'ui/notifications/tool-input-partial',
  TOOL_RESULT: 'ui/notifications/tool-result',
  TOOL_CANCELLED: 'ui/tool-cancelled',
} as const;

/** Tool arguments for tool-input notification */
export interface ToolInputData {
  arguments: Record<string, unknown>;
}

/** Tool result for tool-result notification */
export interface ToolResultData {
  isError?: boolean;
  content: unknown[];
}

/** Tool cancelled data */
export interface ToolCancelledData {
  reason?: string;
}

interface McpAppRendererProps {
  /** The embedded resource containing the MCP App HTML */
  content: EmbeddedResource;
  /** Callback to append a prompt to the chat */
  appendPromptToChat?: (value: string) => void;
  /** Session ID for message context */
  sessionId?: string;
  /** Tool call ID if this UI is linked to a tool */
  toolCallId?: string;
  /** Tool arguments - when provided, sends ui/notifications/tool-input */
  toolArguments?: Record<string, unknown>;
  /** Tool result - when provided, sends ui/notifications/tool-result */
  toolResult?: { isError?: boolean; content: unknown[] };
  /** Whether the tool was cancelled */
  toolCancelled?: boolean;
  /** Cancellation reason */
  toolCancelledReason?: string;
  /** Callback invoked when an error occurs */
  onError?: (error: Error) => void;
}

/**
 * Check if a resource is an MCP App (SEP-1865)
 *
 * MCP Apps are identified by BOTH:
 * 1. The specific MIME type "text/html;profile=mcp-app"
 * 2. The ui:// URI scheme
 *
 * Both conditions must be met to distinguish from legacy MCP-UI resources
 * which use ui:// URIs but with plain "text/html" MIME type.
 */
export function isMcpApp(resource: ResourceContents): boolean {
  const hasMcpAppMimeType = resource.mimeType === MCP_APPS_MIME_TYPE;
  const hasUiScheme = resource.uri?.startsWith(MCP_APPS_URI_SCHEME);
  return hasMcpAppMimeType && Boolean(hasUiScheme);
}

/**
 * Extract HTML content from an embedded resource
 */
function getHtmlFromResource(resource: EmbeddedResource['resource']): string | null {
  if ('text' in resource && typeof resource.text === 'string') {
    return resource.text;
  }
  if ('blob' in resource && typeof resource.blob === 'string') {
    try {
      return globalThis.atob(resource.blob);
    } catch {
      return null;
    }
  }
  return null;
}

// Toast notification component
const ToastNotification = ({
  title,
  message,
  isSupported = true,
}: {
  title: string;
  message?: string;
  isSupported?: boolean;
}) => (
  <div className="flex flex-col gap-0 py-2 pr-4">
    <p className="font-bold">{title}</p>
    {isSupported ? (
      <p>{message}</p>
    ) : (
      <p>
        {message}
        <br />
        <span className="text-sm text-textSubtle">This action is not yet supported.</span>
      </p>
    )}
  </div>
);

// SEP-1865 sandbox proxy method names
const SANDBOX_READY_METHOD = 'ui/notifications/sandbox-ready';
const SANDBOX_RESOURCE_READY_METHOD = 'ui/notifications/sandbox-resource-ready';

/**
 * Setup a sandboxed iframe for the MCP App proxy
 *
 * Note: The iframe must be appended to the DOM before the src is set,
 * otherwise the browser won't load it and we'll never receive the ready message.
 * This function creates the iframe and returns it along with a promise that
 * resolves when the proxy sends its ready message. The caller must:
 * 1. Append the iframe to the DOM
 * 2. Then set iframe.src to trigger loading
 *
 * Per SEP-1865:
 * - Host and Sandbox MUST have different origins
 * - Sandbox MUST have permissions: allow-scripts, allow-same-origin
 * - Sandbox sends ui/notifications/sandbox-ready when ready
 */
function createSandboxProxyIframe(): {
  iframe: globalThis.HTMLIFrameElement;
  onReady: Promise<void>;
} {
  const iframe = document.createElement('iframe');
  iframe.style.width = '100%';
  iframe.style.height = '400px';
  iframe.style.border = 'none';
  iframe.style.backgroundColor = 'transparent';
  // Per SEP-1865: Sandbox MUST have allow-scripts, allow-same-origin
  iframe.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-forms');

  const onReady = new Promise<void>((resolve) => {
    const initialListener = (event: globalThis.MessageEvent) => {
      if (event.source === iframe.contentWindow) {
        // Check for sandbox proxy ready notification (SEP-1865: ui/notifications/sandbox-ready)
        const isSandboxReady = event.data?.method === SANDBOX_READY_METHOD;

        console.log(
          '[MCP Apps] Received message from iframe:',
          event.data?.method || 'unknown',
          'isSandboxReady:',
          isSandboxReady
        );

        if (isSandboxReady) {
          window.removeEventListener('message', initialListener);
          resolve();
        }
      }
    };
    window.addEventListener('message', initialListener);
  });

  // Note: Don't set src here - caller must append to DOM first, then set src

  return { iframe, onReady };
}

/**
 * MCP App Renderer Component
 *
 * Renders MCP Apps using the official AppBridge from @modelcontextprotocol/ext-apps.
 * This provides proper SEP-1865 protocol compliance including:
 * - Sandbox proxy iframe setup
 * - MCP protocol initialization
 * - Tool input/result notifications
 * - UI action handling (prompts, links, notifications)
 */
export default function McpAppRenderer({
  content,
  appendPromptToChat,
  sessionId: _sessionId, // Reserved for future use with full AppBridge integration
  toolCallId: _toolCallId, // Reserved for future use with full AppBridge integration
  toolArguments,
  toolResult,
  toolCancelled,
  toolCancelledReason,
  onError,
}: McpAppRendererProps) {
  const [error, setError] = useState<Error | null>(null);
  const [_iframeReady, setIframeReady] = useState(false); // Proxy iframe is ready (kept for potential future use)
  const [appInitialized, setAppInitialized] = useState(false); // App has sent ui/notifications/initialized
  const [isLoading, setIsLoading] = useState(true);
  const [proxyUrl, setProxyUrl] = useState<string | null>(null);
  const [cspString, setCspString] = useState<string | null>(null); // CSP to inject into inner iframe

  const containerRef = useRef<HTMLDivElement>(null);
  const iframeRef = useRef<globalThis.HTMLIFrameElement | null>(null);

  // Track what we've sent to avoid duplicates
  const sentToolInput = useRef(false);
  const sentToolResult = useRef(false);
  const sentToolCancelled = useRef(false);

  // Get theme
  const theme = (typeof localStorage !== 'undefined' && localStorage.getItem('theme')) || 'light';

  // Handle errors
  const handleError = useCallback(
    (err: Error) => {
      setError(err);
      onError?.(err);
      console.error('[MCP Apps] Error:', err);
    },
    [onError]
  );

  // Helper function to build CSP string from domains (matches Rust implementation)
  const buildCspString = (connectDomains: string[], resourceDomains: string[]): string => {
    const connectSrc = connectDomains.length > 0 ? ` ${connectDomains.join(' ')}` : '';
    const resourceSrc = resourceDomains.length > 0 ? ` ${resourceDomains.join(' ')}` : '';

    return [
      "default-src 'none'",
      `script-src 'self' 'unsafe-inline'${resourceSrc}`,
      `style-src 'self' 'unsafe-inline'${resourceSrc}`,
      `connect-src 'self'${connectSrc}`,
      `img-src 'self' data:${resourceSrc}`,
      `font-src 'self'${resourceSrc}`,
      `media-src 'self' data:${resourceSrc}`,
      "frame-src 'none'",
      "object-src 'none'",
      "base-uri 'self'",
    ].join('; ') + ';';
  };

  // Effect 0: Get the proxy URL from electron and extract CSP
  useEffect(() => {
    const getProxyUrl = async () => {
      try {
        const baseUrl = await window.electron.getGoosedHostPort();
        const secretKey = await window.electron.getSecretKey();
        if (baseUrl && secretKey) {
          // Build proxy URL with secret and contentType for rawhtml mode
          const params = new URLSearchParams({
            secret: secretKey,
            contentType: 'rawhtml',
          });

          // Extract CSP domains from resource metadata per SEP-1865
          // The CSP can be at different levels depending on how the server returns it:
          // - content._meta.ui.csp (on the EmbeddedResource)
          // - content.resource._meta.ui.csp (on the inner resource)
          // We check both locations
          console.log('[MCP Apps] Full content object:', JSON.stringify(content, null, 2));
          console.log('[MCP Apps] content._meta:', content._meta);
          console.log('[MCP Apps] content.resource:', content.resource);

          // Try content._meta first (EmbeddedResource level)
          let meta = content._meta as Record<string, unknown> | undefined;
          let uiMeta = meta?.ui as Record<string, unknown> | undefined;
          let csp = uiMeta?.csp as Record<string, unknown> | undefined;

          // If not found, try content.resource._meta (inner resource level)
          if (!csp) {
            const resourceMeta = (content.resource as Record<string, unknown>)?._meta as Record<string, unknown> | undefined;
            console.log('[MCP Apps] content.resource._meta:', resourceMeta);
            uiMeta = resourceMeta?.ui as Record<string, unknown> | undefined;
            csp = uiMeta?.csp as Record<string, unknown> | undefined;
          }

          console.log('[MCP Apps] Found CSP config:', csp);

          let connectDomains: string[] = [];
          let resourceDomains: string[] = [];

          if (csp) {
            connectDomains = (csp.connectDomains as string[] | undefined) || [];
            resourceDomains = (csp.resourceDomains as string[] | undefined) || [];

            console.log('[MCP Apps] CSP connect domains:', connectDomains);
            console.log('[MCP Apps] CSP resource domains:', resourceDomains);

            if (connectDomains.length > 0) {
              params.set('connect_domains', connectDomains.join(','));
            }
            if (resourceDomains.length > 0) {
              params.set('resource_domains', resourceDomains.join(','));
            }
          } else {
            console.log('[MCP Apps] No CSP config found in metadata');
          }

          // Build CSP string to inject into inner iframe
          const cspStr = buildCspString(connectDomains, resourceDomains);
          console.log('[MCP Apps] Built CSP string:', cspStr);
          setCspString(cspStr);

          setProxyUrl(`${baseUrl}/mcp-apps-proxy?${params.toString()}`);
        }
      } catch (err) {
        handleError(err instanceof Error ? err : new Error('Failed to get proxy URL'));
      }
    };
    getProxyUrl();
  }, [handleError, content]);

  // Effect 1: Setup iframe when proxy URL is ready
  useEffect(() => {
    if (!proxyUrl || !containerRef.current) return;

    let mounted = true;
    // Capture container ref for cleanup (React hooks lint rule)
    const container = containerRef.current;

    const setup = async () => {
      try {
        setIsLoading(true);

        // Create the sandbox proxy iframe (but don't set src yet)
        console.log('[MCP Apps] Creating sandbox proxy iframe');
        const { iframe, onReady } = createSandboxProxyIframe();

        if (!mounted) return;

        iframeRef.current = iframe;

        // IMPORTANT: Append to DOM first, THEN set src
        // The browser won't load the iframe until it's in the DOM
        container.appendChild(iframe);
        console.log('[MCP Apps] Iframe appended to DOM, now setting src:', proxyUrl);
        iframe.src = proxyUrl;

        // Wait for proxy to be ready
        console.log('[MCP Apps] Waiting for proxy ready...');
        await onReady;
        console.log('[MCP Apps] Proxy ready!');

        if (!mounted) return;

        // Get the HTML content from the resource
        const html = getHtmlFromResource(content.resource);
        if (!html) {
          throw new Error('No HTML content found in resource');
        }

        // Send HTML content and CSP to the proxy iframe via SEP-1865 sandbox-resource-ready
        // The proxy will inject the CSP into the inner iframe's HTML
        console.log(
          '[MCP Apps] Sending sandbox-resource-ready to proxy, html length:',
          html.length,
          'with CSP:',
          cspString ? 'yes' : 'no'
        );
        iframe.contentWindow?.postMessage(
          {
            jsonrpc: '2.0',
            method: SANDBOX_RESOURCE_READY_METHOD,
            params: {
              html,
              csp: cspString, // CSP string to inject into inner iframe
              // sandbox: optional override for inner iframe sandbox attribute
            },
          },
          '*'
        );

        setIframeReady(true);
        setIsLoading(false);
      } catch (err) {
        if (!mounted) return;
        handleError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      }
    };

    setup();

    return () => {
      mounted = false;
      // Cleanup iframe
      if (iframeRef.current && container.contains(iframeRef.current)) {
        container.removeChild(iframeRef.current);
      }
      iframeRef.current = null;
    };
  }, [proxyUrl, content.resource, cspString, handleError]);

  // Effect 2: Send tool input when app is initialized
  useEffect(() => {
    // Wait for app to send ui/notifications/initialized before sending tool data
    if (!appInitialized || !toolArguments || sentToolInput.current) return;
    if (!iframeRef.current?.contentWindow) return;

    console.log('[MCP Apps] Sending tool input (app initialized):', toolArguments);
    iframeRef.current.contentWindow.postMessage(
      {
        jsonrpc: '2.0',
        method: MCP_APPS_METHODS.TOOL_INPUT,
        params: { arguments: toolArguments },
      },
      '*'
    );
    sentToolInput.current = true;
  }, [appInitialized, toolArguments]);

  // Effect 3: Send tool result when app is initialized
  useEffect(() => {
    // Wait for app to send ui/notifications/initialized before sending tool data
    if (!appInitialized || !toolResult || sentToolResult.current) return;
    if (!iframeRef.current?.contentWindow) return;

    console.log('[MCP Apps] Sending tool result (app initialized):', toolResult);
    iframeRef.current.contentWindow.postMessage(
      {
        jsonrpc: '2.0',
        method: MCP_APPS_METHODS.TOOL_RESULT,
        params: toolResult,
      },
      '*'
    );
    sentToolResult.current = true;
  }, [appInitialized, toolResult]);

  // Effect 4: Send tool cancelled when app is initialized
  useEffect(() => {
    // Wait for app to send ui/notifications/initialized before sending tool data
    if (!appInitialized || !toolCancelled || sentToolCancelled.current) return;
    if (!iframeRef.current?.contentWindow) return;

    console.log('[MCP Apps] Sending tool cancelled (app initialized)');
    iframeRef.current.contentWindow.postMessage(
      {
        jsonrpc: '2.0',
        method: MCP_APPS_METHODS.TOOL_CANCELLED,
        params: { reason: toolCancelledReason },
      },
      '*'
    );
    sentToolCancelled.current = true;
  }, [appInitialized, toolCancelled, toolCancelledReason]);

  // Effect 5: Listen for messages from the iframe
  useEffect(() => {
    const handleMessage = async (event: globalThis.MessageEvent) => {
      if (!iframeRef.current || event.source !== iframeRef.current.contentWindow) {
        return;
      }

      const data = event.data;
      if (!data) return;

      console.log('[MCP Apps] Received message from iframe:', data);

      // Handle size change (legacy format)
      if (data.type === 'ui-size-change' && data.payload) {
        const { height } = data.payload;
        if (iframeRef.current) {
          if (height !== undefined) {
            iframeRef.current.style.height = `${height}px`;
          }
          // Don't set width from content - keep iframe at 100% width to fill container
          // The content should adapt to the available width
        }
        return;
      }

      // Handle UI actions (from @mcp-ui/client format)
      if (data.type) {
        switch (data.type) {
          case 'link': {
            const url = data.payload?.url;
            if (url) {
              try {
                await window.electron.openExternal(url);
                toast.success(
                  <ToastNotification title="Link Opened" message={`Opened ${url}`} />,
                  { theme }
                );
              } catch (err) {
                console.error('[MCP Apps] Failed to open link:', err);
              }
            }
            break;
          }
          case 'prompt': {
            const prompt = data.payload?.prompt;
            if (prompt && appendPromptToChat) {
              appendPromptToChat(prompt);
              window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
            }
            break;
          }
          case 'notify': {
            const message = data.payload?.message;
            if (message) {
              toast.info(<ToastNotification title="Notification" message={message} />, { theme });
            }
            break;
          }
        }
      }

      // Handle JSON-RPC notifications and requests (SEP-1865 format)
      if (data.jsonrpc === '2.0' && data.method) {
        const requestId = data.id;

        switch (data.method) {
          case 'ui/notifications/initialized': {
            // App has finished initializing and is ready to receive tool data
            console.log('[MCP Apps] App initialized, ready to receive tool data');
            setAppInitialized(true);
            break;
          }
          case 'ui/notifications/size-changed': {
            // Handle size change notification from app
            // Only adjust height - width should stay at 100% to fill container
            const { height } = data.params || {};
            if (iframeRef.current && height !== undefined) {
              iframeRef.current.style.height = `${height}px`;
            }
            break;
          }
          case 'ui/initialize': {
            // Respond with host context (required for @mcp-ui/client initialization)
            console.log('[MCP Apps] Responding to ui/initialize request');
            const hostContext = {
              protocolVersion: '2025-03-26',
              hostInfo: {
                name: 'Goose Desktop',
                version: '1.0.0',
              },
              capabilities: {
                ui: {
                  openLink: true,
                  message: true,
                },
              },
              context: {
                theme: theme === 'dark' ? 'dark' : 'light',
                locale: navigator.language || 'en-US',
                timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                display: {
                  mode: 'inline',
                  viewport: {
                    width: iframeRef.current?.clientWidth || 800,
                    height: iframeRef.current?.clientHeight || 600,
                  },
                },
                platform: {
                  os: 'desktop',
                },
              },
            };

            iframeRef.current?.contentWindow?.postMessage(
              {
                jsonrpc: '2.0',
                id: requestId,
                result: hostContext,
              },
              '*'
            );
            break;
          }
          case 'ui/open-link': {
            const url = data.params?.url;
            if (url) {
              try {
                await window.electron.openExternal(url);
                // Send success response if request has an id
                if (requestId !== undefined) {
                  iframeRef.current?.contentWindow?.postMessage(
                    {
                      jsonrpc: '2.0',
                      id: requestId,
                      result: { success: true },
                    },
                    '*'
                  );
                }
              } catch (err) {
                console.error('[MCP Apps] Failed to open link:', err);
                if (requestId !== undefined) {
                  iframeRef.current?.contentWindow?.postMessage(
                    {
                      jsonrpc: '2.0',
                      id: requestId,
                      error: { code: -32000, message: 'Failed to open link' },
                    },
                    '*'
                  );
                }
              }
            }
            break;
          }
          case 'ui/message': {
            const text = data.params?.content?.text || data.params?.content;
            if (text && appendPromptToChat) {
              appendPromptToChat(String(text));
              window.dispatchEvent(new CustomEvent('scroll-chat-to-bottom'));
            }
            // Send success response if request has an id
            if (requestId !== undefined) {
              iframeRef.current?.contentWindow?.postMessage(
                {
                  jsonrpc: '2.0',
                  id: requestId,
                  result: { success: true },
                },
                '*'
              );
            }
            break;
          }
        }
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [appendPromptToChat, theme]);

  // Render - always render the container so the ref is available for the iframe
  // The iframe will be appended to containerRef by the effect
  return (
    <div className="mt-3 border border-borderSubtle rounded-lg bg-bgApp overflow-hidden">
      {error && (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm">
          Error: {error.message}
        </div>
      )}
      {isLoading && (
        <div className="flex items-center justify-center h-32">
          <span className="text-textSubtle">Loading MCP App...</span>
        </div>
      )}
      {/* Container for the iframe - always rendered so ref is available */}
      <div
        ref={containerRef}
        style={{
          width: '100%',
          minHeight: '200px',
        }}
      />
    </div>
  );
}
