import type { Stream } from '@aaif/goose-sdk';

export type ClosableAcpStream = Stream & {
  close: () => void;
};

export function createWebSocketStream(wsUrl: string): ClosableAcpStream {
  const ws = new window.WebSocket(wsUrl);

  const incoming: unknown[] = [];
  const waiters: Array<() => void> = [];
  let closed = false;
  let opened = false;
  let closeRequested = false;
  let hadError = false;

  function pushMessage(message: unknown): void {
    incoming.push(message);
    waiters.shift()?.();
  }

  function waitForMessage(): Promise<void> {
    if (incoming.length > 0 || closed) {
      return Promise.resolve();
    }
    return new Promise<void>((resolve) => waiters.push(resolve));
  }

  const openPromise = new Promise<void>((resolve, reject) => {
    ws.addEventListener(
      'open',
      () => {
        opened = true;
        resolve();
      },
      { once: true }
    );
    ws.addEventListener('error', () => reject(new Error('ACP WebSocket connection failed')), {
      once: true,
    });
    ws.addEventListener(
      'close',
      () => reject(new Error('ACP WebSocket closed before connection opened')),
      { once: true }
    );
  });

  ws.addEventListener('message', (event) => {
    if (typeof event.data !== 'string') {
      return;
    }
    try {
      pushMessage(JSON.parse(event.data));
    } catch {
      // Ignore malformed messages from the transport.
    }
  });

  const closeWaiters = () => {
    closed = true;
    for (const waiter of waiters) {
      waiter();
    }
    waiters.length = 0;
  };

  function phase(): 'before-open' | 'after-open' {
    return opened ? 'after-open' : 'before-open';
  }

  function logClose(event: CloseEvent): void {
    const details = {
      phase: phase(),
      code: event.code,
      reason: event.reason,
      wasClean: event.wasClean,
      readyState: ws.readyState,
      initiatedByClient: closeRequested,
      hadError,
    };

    if (closeRequested || event.wasClean) {
      console.debug('ACP WebSocket closed', details);
      return;
    }

    console.warn('ACP WebSocket closed', details);
  }

  function closeSocket(): void {
    closeRequested = true;
    ws.close();
  }

  ws.addEventListener('close', (event) => {
    logClose(event);
    closeWaiters();
  });
  ws.addEventListener('error', () => {
    hadError = true;
    closeWaiters();
  });

  const readable = new window.ReadableStream({
    async pull(controller) {
      await waitForMessage();
      while (incoming.length > 0) {
        controller.enqueue(incoming.shift());
      }
      if (closed && incoming.length === 0) {
        controller.close();
      }
    },
  });

  const writable = new window.WritableStream({
    async write(message) {
      await openPromise;
      if (closed || ws.readyState !== window.WebSocket.OPEN) {
        throw new Error('ACP WebSocket connection lost');
      }
      ws.send(JSON.stringify(message));
    },
    close() {
      closeSocket();
    },
    abort() {
      closeSocket();
    },
  });

  return {
    readable,
    writable,
    close: closeSocket,
  } as ClosableAcpStream;
}
