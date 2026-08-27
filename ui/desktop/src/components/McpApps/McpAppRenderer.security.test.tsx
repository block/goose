import { render } from '@testing-library/react';
import { useLayoutEffect } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { assertMcpAppToolCallAllowed, GooseAppFrame } from './McpAppRenderer';

interface MockBridge {
  oncalltool?: (params: { name: string; arguments?: Record<string, unknown> }) => Promise<unknown>;
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
