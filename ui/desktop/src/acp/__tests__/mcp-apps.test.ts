import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getAcpClient } from '../acpConnection';
import { callMcpAppTool, listMcpAppTools, readMcpAppResource } from '../mcp-apps';

vi.mock('../acpConnection', () => ({
  getAcpClient: vi.fn(),
}));

function createClient() {
  return {
    gosling: {
      resourcesRead_unstable: vi.fn(),
      toolsCall_unstable: vi.fn(),
      toolsList_unstable: vi.fn(),
    },
  };
}

describe('ACP MCP app helpers', () => {
  let client: ReturnType<typeof createClient>;

  beforeEach(() => {
    vi.clearAllMocks();
    client = createClient();
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );
  });

  it('flattens ACP resource reads into the renderer resource shape', async () => {
    client.gosling.resourcesRead_unstable.mockResolvedValue({
      result: {
        contents: [
          {
            uri: 'ui://weather/panel',
            mimeType: 'text/html;profile=mcp-app',
            text: '<main>Weather</main>',
            _meta: {
              ui: {
                csp: {
                  connectDomains: ['https://api.example.com'],
                },
                prefersBorder: false,
              },
            },
          },
        ],
      },
    });

    const resource = await readMcpAppResource('session-1', 'weather', 'ui://weather/panel');

    expect(client.gosling.resourcesRead_unstable).toHaveBeenCalledWith({
      sessionId: 'session-1',
      extensionName: 'weather',
      uri: 'ui://weather/panel',
    });
    expect(resource).toEqual({
      uri: 'ui://weather/panel',
      mimeType: 'text/html;profile=mcp-app',
      text: '<main>Weather</main>',
      _meta: {
        ui: {
          csp: {
            connectDomains: ['https://api.example.com'],
          },
          prefersBorder: false,
        },
      },
    });
  });

  it('decodes blob resources as UTF-8 text', async () => {
    client.gosling.resourcesRead_unstable.mockResolvedValue({
      result: {
        contents: [
          {
            uri: 'ui://weather/panel',
            mimeType: 'text/html;profile=mcp-app',
            blob: Buffer.from('<main>São Paulo 東京</main>', 'utf8').toString('base64'),
          },
        ],
      },
    });

    const resource = await readMcpAppResource('session-1', 'weather', 'ui://weather/panel');

    expect(resource.text).toBe('<main>São Paulo 東京</main>');
  });

  it('prefixes app tool calls before sending them over ACP', async () => {
    client.gosling.toolsCall_unstable.mockResolvedValue({
      content: [{ type: 'text', text: 'done' }],
      structuredContent: { ok: true },
      isError: false,
      _meta: { traceId: 'trace-1' },
    });

    const result = await callMcpAppTool('session-1', 'weather', 'refresh', { city: 'Amsterdam' });

    expect(client.gosling.toolsCall_unstable).toHaveBeenCalledWith({
      sessionId: 'session-1',
      name: 'weather__refresh',
      arguments: { city: 'Amsterdam' },
    });
    expect(result).toEqual({
      content: [{ type: 'text', text: 'done' }],
      structuredContent: { ok: true },
      isError: false,
      _meta: { traceId: 'trace-1' },
    });
  });

  it('maps and filters ACP tools for app host context', async () => {
    client.gosling.toolsList_unstable.mockResolvedValue({
      tools: [
        {
          name: 'weather__refresh',
          description: 'Refresh weather',
          parameters: [],
          inputSchema: {
            type: 'object',
            properties: {
              city: { type: 'string' },
            },
          },
        },
        {
          name: 'calendar__refresh',
          description: 'Refresh calendar',
          parameters: [],
          inputSchema: { type: 'object' },
        },
      ],
    });

    const tools = await listMcpAppTools('session-1', 'weather');

    expect(client.gosling.toolsList_unstable).toHaveBeenCalledWith({ sessionId: 'session-1' });
    expect(tools).toEqual([
      {
        name: 'weather__refresh',
        description: 'Refresh weather',
        parameters: [],
        inputSchema: {
          type: 'object',
          properties: {
            city: { type: 'string' },
          },
        },
      },
    ]);
  });
});
