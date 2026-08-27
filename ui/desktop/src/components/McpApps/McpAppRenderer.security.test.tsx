import { act, render, screen, waitFor } from '@testing-library/react';
import { useLayoutEffect } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../i18n/test-utils';
import McpAppRenderer, {
  appendMcpAppMessage,
  assertMcpAppHostActionAllowed,
  GooseAppFrame,
} from './McpAppRenderer';

interface MockBridge {
  oncalltool?: (params: { name: string; arguments?: Record<string, unknown> }) => Promise<unknown>;
  onmessage?: (params: { content: Array<{ type: string; text?: string }> }) => Promise<unknown>;
  onopenlink?: (params: { url: string }) => Promise<unknown>;
  onreadresource?: (params: { uri: string }) => Promise<unknown>;
  fallbackRequestHandler?: (request: { method: string }, extra: unknown) => Promise<unknown>;
}

const bridgeInstances = vi.hoisted(() => [] as MockBridge[]);
const rendererMocks = vi.hoisted(() => ({
  readMcpAppResource: vi.fn(),
  getCachedTools: vi.fn(),
  getAcpUrl: vi.fn(),
  getSecretKey: vi.fn(),
}));
const disabledError = new Error('MCP App host actions are disabled until the recipe is trusted');

vi.mock('@mcp-ui/client', () => ({
  AppBridge: class {
    constructor() {
      bridgeInstances.push(this as MockBridge);
    }

    close() {}
  },
  PostMessageTransport: class {},
}));

vi.mock('../../acp/mcp-apps', () => ({
  callMcpAppTool: vi.fn(),
  readMcpAppResource: rendererMocks.readMcpAppResource,
}));

vi.mock('./toolsCache', () => ({
  getCachedTools: rendererMocks.getCachedTools,
}));

vi.mock('../../contexts/ThemeContext', () => ({
  useTheme: () => ({ resolvedTheme: 'light', mcpHostStyles: {} }),
}));

vi.mock('./useDisplayMode', () => ({
  AVAILABLE_DISPLAY_MODES: ['inline'],
  PIP_WIDTH: 400,
  PIP_HEIGHT: 600,
  PIP_MARGIN_RIGHT: 24,
  PIP_MARGIN_BOTTOM: 24,
  useDisplayMode: () => ({
    activeDisplayMode: 'inline',
    effectiveDisplayModes: ['inline'],
    isStandalone: false,
    isFullscreen: false,
    isPip: false,
    isFillsViewport: false,
    isInline: true,
    appSupportsFullscreen: false,
    appSupportsPip: false,
    appTitle: undefined,
    changeDisplayMode: vi.fn(),
    inlineHeight: 200,
    pipPosition: { x: 0, y: 0 },
    pipHandlers: {},
    fullscreenCloseRef: { current: null },
  }),
}));

vi.mock('../FlyingBird', () => ({ default: () => null }));

vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: false })));
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    disconnect() {}
  }
);

describe('MCP App host-action gating', () => {
  beforeEach(() => {
    bridgeInstances.length = 0;
    vi.clearAllMocks();
    rendererMocks.getCachedTools.mockResolvedValue(null);
    rendererMocks.getAcpUrl.mockResolvedValue('ws://127.0.0.1:3000');
    rendererMocks.getSecretKey.mockResolvedValue('secret');
    rendererMocks.readMcpAppResource.mockResolvedValue({
      uri: 'ui://recipe/app',
      text: '<html><body>app</body></html>',
      mimeType: 'text/html',
      _meta: {},
    });
    Object.assign(window.electron, {
      getAcpUrl: rendererMocks.getAcpUrl,
      getSecretKey: rendererMocks.getSecretKey,
    });
  });

  it('rejects host actions while recipe trust is unresolved', () => {
    expect(() => assertMcpAppHostActionAllowed(true)).toThrow(disabledError);
    expect(() => assertMcpAppHostActionAllowed(false)).not.toThrow();
  });

  it('blocks every privileged bridge handler while recipe trust is unresolved', () => {
    const onMessage = vi.fn().mockResolvedValue({});
    const onOpenLink = vi.fn().mockResolvedValue({ status: 'success' });
    const onCallTool = vi.fn().mockResolvedValue({ content: [] });
    const onReadResource = vi.fn().mockResolvedValue({ contents: [] });
    const onFallbackRequest = vi.fn().mockResolvedValue({});

    render(
      <GooseAppFrame
        html=""
        sandbox={{ url: new URL('about:blank') }}
        hostContext={{}}
        onMessage={onMessage}
        onOpenLink={onOpenLink}
        onCallTool={onCallTool}
        onReadResource={onReadResource}
        onLoggingMessage={() => {}}
        onFallbackRequest={onFallbackRequest}
        hostActionsDisabled
      />
    );

    const bridge = bridgeInstances[0];
    expect(bridgeInstances).toHaveLength(1);
    expect(() => bridge.onmessage?.({ content: [{ type: 'text', text: 'draft' }] })).toThrow(
      disabledError
    );
    expect(() => bridge.onopenlink?.({ url: 'https://example.com' })).toThrow(disabledError);
    expect(() => bridge.oncalltool?.({ name: 'delete_everything' })).toThrow(disabledError);
    expect(() => bridge.onreadresource?.({ uri: 'file:///secret' })).toThrow(disabledError);
    expect(() => bridge.fallbackRequestHandler?.({ method: 'custom/action' }, {})).toThrow(
      disabledError
    );
    expect(onMessage).not.toHaveBeenCalled();
    expect(onOpenLink).not.toHaveBeenCalled();
    expect(onCallTool).not.toHaveBeenCalled();
    expect(onReadResource).not.toHaveBeenCalled();
    expect(onFallbackRequest).not.toHaveBeenCalled();
  });

  it('blocks a bridge action in the same commit that trust is revoked', () => {
    const onOpenLink = vi.fn().mockResolvedValue({ status: 'success' });
    let transitionError: unknown;

    function Harness({ hostActionsDisabled }: { hostActionsDisabled: boolean }) {
      useLayoutEffect(() => {
        if (!hostActionsDisabled) return;
        try {
          void bridgeInstances[0]?.onopenlink?.({ url: 'https://example.com' });
        } catch (error) {
          transitionError = error;
        }
      }, [hostActionsDisabled]);

      return (
        <GooseAppFrame
          html=""
          sandbox={{ url: new URL('about:blank') }}
          hostContext={{}}
          onMessage={async () => ({})}
          onOpenLink={onOpenLink}
          onCallTool={async () => ({ content: [] })}
          onReadResource={async () => ({ contents: [] })}
          onLoggingMessage={() => {}}
          onFallbackRequest={async () => ({})}
          hostActionsDisabled={hostActionsDisabled}
        />
      );
    }

    const { rerender } = render(<Harness hostActionsDisabled={false} />);
    rerender(<Harness hostActionsDisabled />);

    expect(transitionError).toEqual(disabledError);
    expect(onOpenLink).not.toHaveBeenCalled();
  });

  it('defers app loading and removes the iframe whenever recipe trust is blocked', async () => {
    const props = {
      resourceUri: 'ui://recipe/app',
      extensionName: 'recipe-extension',
      toolName: 'render-app',
      sessionId: 'session-1',
      cachedHtml: '<html><body>cached app</body></html>',
    };
    const { rerender } = render(<McpAppRenderer {...props} hostActionsDisabled />, {
      wrapper: IntlTestWrapper,
    });

    expect(screen.getByTestId('mcp-app-trust-placeholder')).toBeInTheDocument();
    expect(rendererMocks.readMcpAppResource).not.toHaveBeenCalled();
    expect(rendererMocks.getCachedTools).not.toHaveBeenCalled();
    expect(rendererMocks.getAcpUrl).not.toHaveBeenCalled();
    expect(bridgeInstances).toHaveLength(0);
    expect(document.querySelector('iframe')).not.toBeInTheDocument();

    rerender(<McpAppRenderer {...props} hostActionsDisabled={false} />);

    await waitFor(() => expect(rendererMocks.readMcpAppResource).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(rendererMocks.getCachedTools).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(rendererMocks.getAcpUrl).toHaveBeenCalled());
    await waitFor(() => expect(bridgeInstances).toHaveLength(1));
    expect(document.querySelector('iframe')).toBeInTheDocument();
    const trustedBridge = bridgeInstances[0];

    rerender(<McpAppRenderer {...props} hostActionsDisabled />);

    expect(screen.getByTestId('mcp-app-trust-placeholder')).toBeInTheDocument();
    expect(document.querySelector('iframe')).not.toBeInTheDocument();
    expect(() => trustedBridge.onopenlink?.({ url: 'https://example.com' })).toThrow(disabledError);
  });

  it('abandons an in-flight sandbox lookup when recipe trust is revoked', async () => {
    let resolveAcpUrl: ((url: string) => void) | undefined;
    rendererMocks.getAcpUrl.mockReturnValueOnce(
      new Promise<string>((resolve) => {
        resolveAcpUrl = resolve;
      })
    );
    const props = {
      resourceUri: 'ui://recipe/app',
      extensionName: 'recipe-extension',
      cachedHtml: '<html><body>cached app</body></html>',
    };
    function Harness({ hostActionsDisabled }: { hostActionsDisabled: boolean }) {
      useLayoutEffect(() => {
        if (hostActionsDisabled) {
          resolveAcpUrl?.('ws://127.0.0.1:3000');
        }
      }, [hostActionsDisabled]);

      return <McpAppRenderer {...props} hostActionsDisabled={hostActionsDisabled} />;
    }
    const { rerender } = render(<Harness hostActionsDisabled={false} />, {
      wrapper: IntlTestWrapper,
    });

    await waitFor(() => expect(rendererMocks.getAcpUrl).toHaveBeenCalledTimes(1));
    rerender(<Harness hostActionsDisabled />);

    await waitFor(() => expect(rendererMocks.getSecretKey).not.toHaveBeenCalled());
    expect(bridgeInstances).toHaveLength(0);
    expect(screen.getByTestId('mcp-app-trust-placeholder')).toBeInTheDocument();
  });

  it('does not cache tool or resource results that settle during trust revocation', async () => {
    let resolveTools: ((tools: Array<{ name: string; inputSchema: { type: string } }>) => void) |
      undefined;
    let resolveResource:
      | ((resource: { uri: string; text: string; mimeType: string; _meta: object }) => void)
      | undefined;
    rendererMocks.getCachedTools
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveTools = resolve;
        })
      )
      .mockReturnValueOnce(new Promise(() => undefined));
    rendererMocks.readMcpAppResource
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveResource = resolve;
        })
      )
      .mockReturnValueOnce(new Promise(() => undefined));
    const props = {
      resourceUri: 'ui://recipe/app',
      extensionName: 'recipe-extension',
      toolName: 'render-app',
      sessionId: 'session-1',
    };
    function Harness({ hostActionsDisabled }: { hostActionsDisabled: boolean }) {
      useLayoutEffect(() => {
        if (!hostActionsDisabled) return;
        resolveTools?.([
          {
            name: 'recipe-extension__render-app',
            inputSchema: { type: 'object' },
          },
        ]);
        resolveResource?.({
          uri: 'ui://recipe/app',
          text: '<html><body>app</body></html>',
          mimeType: 'text/html',
          _meta: {},
        });
      }, [hostActionsDisabled]);

      return <McpAppRenderer {...props} hostActionsDisabled={hostActionsDisabled} />;
    }
    const { rerender } = render(<Harness hostActionsDisabled={false} />, {
      wrapper: IntlTestWrapper,
    });

    await waitFor(() => expect(rendererMocks.getCachedTools).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(rendererMocks.readMcpAppResource).toHaveBeenCalledTimes(1));
    rerender(<Harness hostActionsDisabled />);
    await act(async () => undefined);
    rerender(<Harness hostActionsDisabled={false} />);

    await waitFor(() => expect(rendererMocks.getCachedTools).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(rendererMocks.readMcpAppResource).toHaveBeenCalledTimes(2));
    expect(bridgeInstances).toHaveLength(0);
  });
});

describe('MCP App message delivery', () => {
  it('reports when the host rejects a message', async () => {
    const append = vi.fn().mockReturnValue(false);

    await expect(
      appendMcpAppMessage(append, [{ type: 'text', text: 'keep this draft' }])
    ).rejects.toThrow('MCP App message was not submitted');
    expect(append).toHaveBeenCalledWith('keep this draft');
  });

  it('reports success only after the host accepts a message', async () => {
    const append = vi.fn().mockResolvedValue(true);

    await expect(
      appendMcpAppMessage(append, [{ type: 'text', text: 'send this draft' }])
    ).resolves.toBeUndefined();
    expect(append).toHaveBeenCalledWith('send this draft');
  });
});
