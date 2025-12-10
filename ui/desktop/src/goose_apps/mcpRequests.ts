import { app, shell } from 'electron';
import { callTool, GooseApp, readResource } from '../api';
import { Client } from '../api/client';

interface JSONRPCRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: string;
  params?: Record<string, unknown>;
}

interface JSONRPCResult {
  [key: string]: unknown;
}

interface HostContext {
  theme?: 'light' | 'dark';
  displayMode?: 'inline' | 'fullscreen' | 'standalone';
  viewport?: {
    width: number;
    height: number;
  };
}

interface InitializeResult {
  protocolVersion: string;
  hostCapabilities: Record<string, unknown>;
  hostInfo: {
    name: string;
    version: string;
  };
  hostContext: HostContext;
}

export async function handleMCPRequest(
  msg: JSONRPCRequest,
  gapp: GooseApp,
  sessionId: string,
  client: Client
): Promise<JSONRPCResult|InitializeResult> {
  const { method, params = {} } = msg;

  switch (method) {
    case 'ui/initialize':
      return {
        protocolVersion: '2025-06-18',
        hostCapabilities: {},
        hostInfo: { name: 'goose', version: app.getVersion() },
        hostContext: {
          theme: 'dark',
          displayMode: 'standalone',
          viewport: {
            width: gapp.width || 800,
            height: gapp.height || 600,
          },
        },
      } as InitializeResult;

    case 'tools/call': {
      if (!params.name || typeof params.name !== 'string') {
        throw new Error('Invalid tool name');
      }
      if (!gapp.mcpServer) {
        throw new Error('need an mcp server to call');
      }

      const fullToolName = `${gapp.mcpServer}__${params.name}`;

      const response = await callTool({
        client,
        body: {
          session_id: sessionId,
          name: fullToolName,
          arguments: (params.arguments as Record<string, unknown>) || {},
        },
        throwOnError:true
      });

      if (!response.data) {
        throw new Error('Tool call failed');
      }

      return {
        content: response.data.content,
        structuredContent: response.data.structured_content,
        isError: response.data.is_error,
      };
    }

    case 'resources/read': {
      if (!params.uri || typeof params.uri !== 'string') {
        throw new Error('Invalid resource URI');
      }
      if (!sessionId) {
        throw new Error('sessionId required for resource reads');
      }

      const response = await readResource({
        client,
        body: {
          session_id: sessionId,
          uri: params.uri,
          extension_name: gapp.name,
        },
        throwOnError:true
      });

      if (!response.data) {
        throw new Error('Resource read failed');
      }

      return {
        content: response.data.html,
      };
    }

    case 'ui/message':
      console.log('ui/message not yet implemented:', params);
      return {};

    case 'ui/open-link':
      if (!params.url || typeof params.url !== 'string') {
        throw new Error('Invalid URL');
      }
      await shell.openExternal(params.url);
      return {};

    default:
      throw new Error(`Unknown method: ${method}`);
  }
}
