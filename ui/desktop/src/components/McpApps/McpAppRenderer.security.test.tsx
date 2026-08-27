import { render } from '@testing-library/react';
import { useLayoutEffect } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
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

describe('MCP App host-action gating', () => {
  beforeEach(() => {
    bridgeInstances.length = 0;
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
