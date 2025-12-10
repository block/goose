import { useState, useEffect, useRef } from 'react';
import { FlaskConical } from 'lucide-react';
import { readResource } from '../api';
import { injectMCPClient } from '../goose_apps/injectMcpClient';

interface MCPAppsRendererProps {
  resourceUri: string;
  structuredContent?: Record<string, unknown>;
  toolInput?: Record<string, unknown>;
  extensionName: string;
  sessionId: string;
  appendPromptToChat?: (value: string) => void;
}

export default function MCPAppsRenderer({
                                          resourceUri,
                                          structuredContent,
                                          toolInput,
                                          extensionName,
                                          sessionId,
                                        }: MCPAppsRendererProps) {
  const [html, setHtml] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const iframeRef = useRef<React.ComponentRef<'iframe'>>(null);

  useEffect(() => {
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
          const appWithMCP = injectMCPClient({
            name: extensionName,
            html: response.data.html,
            width: null,
            height: null,
            resizable: true,
            prd: '',
            description: null,
          });
          setHtml(appWithMCP);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load resource');
      }
    };

    fetchResource();
  }, [resourceUri, extensionName, sessionId]);

// Handle MCP protocol messages from iframe
  useEffect(() => {
    const handleMessage = async (event: globalThis.MessageEvent) => {
      if (event.source !== iframeRef.current?.contentWindow) return;

      const msg = event.data;
      if (!msg || msg.jsonrpc !== '2.0') return;

      console.log('MCPAppsRenderer received MCP request:', msg);

      try {
        const result = await window.__gooseMCP.handleRequest(msg, {
          sessionId,
          extensionName,
        });

        if (msg.id) {
          iframeRef.current?.contentWindow?.postMessage(
            {
              jsonrpc: '2.0',
              id: msg.id,
              result,
            },
            '*'
          );
        }

        if (msg.method === 'ui/initialize') {
          console.log('ui/initialize complete, sending notifications');

          if (toolInput) {
            console.log('Sending tool-input notification:', toolInput);
            iframeRef.current?.contentWindow?.postMessage(
              {
                jsonrpc: '2.0',
                method: 'ui/notifications/tool-input',
                params: {
                  arguments: toolInput,
                },
              },
              '*'
            );
          }

          if (structuredContent) {
            console.log('Sending tool-result notification:', structuredContent);
            iframeRef.current?.contentWindow?.postMessage(
              {
                jsonrpc: '2.0',
                method: 'ui/notifications/tool-result',
                params: {
                  structuredContent,
                },
              },
              '*'
            );
          }
        }

      } catch (error) {
        console.error('MCP request failed:', error);
        if (msg.id) {
          iframeRef.current?.contentWindow?.postMessage(
            {
              jsonrpc: '2.0',
              id: msg.id,
              error: {
                code: -32000,
                message: error instanceof Error ? error.message : 'Request failed',
              },
            },
            '*'
          );
        }
      }
    };

    window.addEventListener('message', handleMessage);

    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [extensionName, sessionId, toolInput, structuredContent]);

  if (error) {
    return (
      <div className="mt-3 p-4 border border-red-500 rounded-lg bg-red-50">
        <div className="text-sm text-red-700">Error loading MCP App: {error}</div>
      </div>
    );
  }

  if (!html) {
    return (
      <div className="mt-3 p-4 border border-borderSubtle rounded-lg bg-background-muted">
        <div className="text-sm">Loading MCP App...</div>
      </div>
    );
  }

  return (
    <>
      <div className="mt-3 border border-borderSubtle rounded-lg bg-background-muted overflow-hidden">
        <iframe
          ref={iframeRef}
          srcDoc={html}
          sandbox="allow-scripts allow-same-origin"
          className="w-full border-0"
          style={{ minHeight: '400px' }}
        />
      </div>
      <div className="mt-3 p-4 py-3 border border-borderSubtle rounded-lg bg-background-muted flex items-center">
        <FlaskConical className="mr-2" size={20} />
        <div className="text-sm font-sans">
          MCP Apps is experimental and may change at any time.
        </div>
      </div>
    </>
  );
}