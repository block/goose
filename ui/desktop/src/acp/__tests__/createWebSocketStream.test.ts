import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createWebSocketStream } from '../createWebSocketStream';

class FakeWebSocket extends window.EventTarget {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  readonly sent: string[] = [];
  readyState = FakeWebSocket.CONNECTING;

  constructor(readonly url: string) {
    super();
    fakeWebSockets.push(this);
  }

  open(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.dispatchEvent(new Event('open'));
  }

  send(message: string): void {
    this.sent.push(message);
  }

  close(): void {
    if (this.readyState === FakeWebSocket.CLOSED) {
      return;
    }
    this.readyState = FakeWebSocket.CLOSED;
    this.dispatchEvent(new Event('close'));
  }

  fail(): void {
    this.dispatchEvent(new Event('error'));
    this.close();
  }
}

const fakeWebSockets: FakeWebSocket[] = [];

function latestWebSocket(): FakeWebSocket {
  const ws = fakeWebSockets[fakeWebSockets.length - 1];
  if (!ws) {
    throw new Error('Expected a WebSocket to be created');
  }
  return ws;
}

function testRequest() {
  return {
    jsonrpc: '2.0' as const,
    id: 1,
    method: 'test',
  };
}

describe('createWebSocketStream', () => {
  beforeEach(() => {
    fakeWebSockets.length = 0;
    vi.stubGlobal('WebSocket', FakeWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('waits for the socket to open before sending JSON', async () => {
    const stream = createWebSocketStream('ws://localhost/acp');
    const writer = stream.writable.getWriter();
    const write = writer.write(testRequest());
    const ws = latestWebSocket();

    expect(ws.sent).toEqual([]);

    ws.open();
    await write;

    expect(ws.sent).toEqual(['{"jsonrpc":"2.0","id":1,"method":"test"}']);
  });

  it('closes the readable stream when the socket closes', async () => {
    const stream = createWebSocketStream('ws://localhost/acp');
    const reader = stream.readable.getReader();
    const ws = latestWebSocket();

    ws.open();
    ws.close();

    await expect(reader.read()).resolves.toEqual({ done: true, value: undefined });
  });

  it.each([
    {
      event: 'closes',
      trigger: (ws: FakeWebSocket) => ws.close(),
      error: 'ACP WebSocket closed before connection opened',
    },
    {
      event: 'errors',
      trigger: (ws: FakeWebSocket) => ws.fail(),
      error: 'ACP WebSocket connection failed',
    },
  ])(
    'rejects a pending write when the socket $event before opening',
    async ({ trigger, error }) => {
      const stream = createWebSocketStream('ws://localhost/acp');
      const writer = stream.writable.getWriter();
      const write = writer.write(testRequest());

      trigger(latestWebSocket());

      await expect(write).rejects.toThrow(error);
    }
  );

  it('rejects a write when the socket has closed', async () => {
    const stream = createWebSocketStream('ws://localhost/acp');
    const writer = stream.writable.getWriter();
    const ws = latestWebSocket();

    ws.open();
    ws.close();

    await expect(writer.write(testRequest())).rejects.toThrow('ACP WebSocket connection lost');
  });
});
