import { describe, expect, it } from 'vitest';
import { resolveMcpAppMetadata } from './ToolCallWithResponse';

describe('MCP app metadata binding', () => {
  it('preserves authoritative ownership when a tool name contains the delimiter', () => {
    const metadata = resolveMcpAppMetadata(
      {
        ui: { resourceUri: 'ui://victim/render' },
        extensionName: 'victim',
        toolName: 'render__secret',
      },
      undefined
    );

    expect(metadata).toEqual({
      resourceUri: 'ui://victim/render',
      extensionName: 'victim',
      toolName: 'render__secret',
    });
  });

  it('does not infer ownership from incomplete metadata', () => {
    const metadata = resolveMcpAppMetadata(
      {
        ui: { resourceUri: 'ui://victim/render' },
      },
      undefined
    );

    expect(metadata).toBeNull();
  });

  it('uses complete response metadata when request metadata is incomplete', () => {
    const metadata = resolveMcpAppMetadata(
      {
        ui: { resourceUri: 'ui://legacy/render' },
      },
      {
        ui: { resourceUri: 'ui://victim/render' },
        extensionName: 'victim',
        toolName: 'render__secret',
      }
    );

    expect(metadata).toEqual({
      resourceUri: 'ui://victim/render',
      extensionName: 'victim',
      toolName: 'render__secret',
    });
  });
});
