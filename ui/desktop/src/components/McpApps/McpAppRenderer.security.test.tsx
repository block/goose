import { render } from '@testing-library/react';
import { useLayoutEffect } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  appendMcpAppMessage,
  assertMcpAppMessageAllowed,
  assertMcpAppToolCallAllowed,
  GooseAppFrame,
} from './McpAppRenderer';

interface MockBridge {
  oncalltool?: (params: { name: string; arguments?: Record<string, unknown> }) => Promise<unknown>;
  onmessage?: (params: { content: Array<{ type: string; text?: string }> }) => Promise<unknown>;
}

const bridgeInstances = vi.hoisted(() => [] as MockBridge[]);

vi.mock('@mcp-ui/client', () => ({
  AppBridge: class {
    constructor() {
      bridgeInstances.push(this as MockBridge);
    }

    close() {}
  },
  PostMessageTransport: class {},
}));

describe('MCP App tool-call gating', () => {
  beforeEach(() => {
    bridgeInstances.length = 0;
  });

  it('rejects tool calls while recipe trust is unresolved', () => {
    expect(() => assertMcpAppToolCallAllowed(true)).toThrow(
      'MCP App tool calls are disabled until the recipe is trusted'
    );
  });

  it('allows tool calls after recipe trust resolves', () => {
    expect(() => assertMcpAppToolCallAllowed(false)).not.toThrow();
  });

  it('blocks a bridge call in the same commit that trust is revoked', () => {
    const onCallTool = vi.fn().mockResolvedValue({ content: [] });
    let transitionError: unknown;

    function Harness({ toolCallDisabled }: { toolCallDisabled: boolean }) {
      useLayoutEffect(() => {
        if (!toolCallDisabled) return;
        try {
          void bridgeInstances[0]?.oncalltool?.({ name: 'delete_everything' });
        } catch (error) {
          transitionError = error;
        }
      }, [toolCallDisabled]);

      return (
        <GooseAppFrame
          html=""
          sandbox={{ url: new URL('about:blank') }}
          hostContext={{}}
          onMessage={async () => ({})}
          onOpenLink={async () => ({ status: 'success' })}
          onCallTool={onCallTool}
          onReadResource={async () => ({ contents: [] })}
          onLoggingMessage={() => {}}
          onFallbackRequest={async () => ({})}
          toolCallDisabled={toolCallDisabled}
          messageDisabled={false}
        />
      );
    }

    const { rerender } = render(<Harness toolCallDisabled={false} />);

    expect(bridgeInstances).toHaveLength(1);
    rerender(<Harness toolCallDisabled />);

    expect(transitionError).toEqual(
      new Error('MCP App tool calls are disabled until the recipe is trusted')
    );
    expect(onCallTool).not.toHaveBeenCalled();
  });
});

describe('MCP App message gating', () => {
  beforeEach(() => {
    bridgeInstances.length = 0;
  });

  it('blocks a bridge message in the same commit that trust is revoked', () => {
    const onMessage = vi.fn().mockResolvedValue({});
    let transitionError: unknown;

    function Harness({ messageDisabled }: { messageDisabled: boolean }) {
      useLayoutEffect(() => {
        if (!messageDisabled) return;
        try {
          void bridgeInstances[0]?.onmessage?.({
            content: [{ type: 'text', text: 'keep this draft' }],
          });
        } catch (error) {
          transitionError = error;
        }
      }, [messageDisabled]);

      return (
        <GooseAppFrame
          html=""
          sandbox={{ url: new URL('about:blank') }}
          hostContext={{}}
          onMessage={onMessage}
          onOpenLink={async () => ({ status: 'success' })}
          onCallTool={vi.fn()}
          onReadResource={async () => ({ contents: [] })}
          onLoggingMessage={() => {}}
          onFallbackRequest={async () => ({})}
          toolCallDisabled={false}
          messageDisabled={messageDisabled}
        />
      );
    }

    const { rerender } = render(<Harness messageDisabled={false} />);

    expect(bridgeInstances).toHaveLength(1);
    rerender(<Harness messageDisabled />);

    expect(transitionError).toEqual(
      new Error('MCP App messages are disabled until the recipe is trusted')
    );
    expect(onMessage).not.toHaveBeenCalled();
  });

  it('rejects messages while recipe trust is unresolved', () => {
    expect(() => assertMcpAppMessageAllowed(true)).toThrow(
      'MCP App messages are disabled until the recipe is trusted'
    );
  });

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
