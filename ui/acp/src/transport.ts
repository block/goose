import type { AnyMessage, Stream } from "@agentclientprotocol/sdk";

const ACP_SESSION_HEADER = "Acp-Session-Id";

export function createHttpStream(serverUrl: string): Stream {
  let sessionId: string | null = null;
  const incoming: AnyMessage[] = [];
  const waiters: Array<() => void> = [];
  const sseAbort = new AbortController();

  function pushMessage(msg: AnyMessage) {
    incoming.push(msg);
    const w = waiters.shift();
    if (w) w();
  }

  function waitForMessage(): Promise<void> {
    if (incoming.length > 0) return Promise.resolve();
    return new Promise<void>((r) => waiters.push(r));
  }

  async function consumeSSE(response: Response) {
    if (!response.body) return;
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        const parts = buffer.split("\n\n");
        buffer = parts.pop() || "";

        for (const part of parts) {
          for (const line of part.split("\n")) {
            if (line.startsWith("data: ")) {
              try {
                const msg = JSON.parse(line.slice(6)) as AnyMessage;
                pushMessage(msg);
              } catch {
                // skip non-JSON data lines
              }
            }
          }
        }
      }
    } catch (e: unknown) {
      if (e instanceof DOMException && e.name === "AbortError") return;
      // Connection closed - expected on shutdown
    }
  }

  // Protocol:
  // 1. POST initialize (no session header) -> SSE stream with response + session ID header.
  //    This SSE stream stays open and receives ALL subsequent responses/notifications.
  // 2. POST subsequent requests with session header -> fire-and-forget (server delivers
  //    the message before blocking on SSE mutex). Responses arrive on stream #1.
  // 3. POST notifications/responses with session header -> server returns 202 immediately.

  let isFirstRequest = true;

  const readable = new ReadableStream<AnyMessage>({
    async pull(controller) {
      await waitForMessage();
      while (incoming.length > 0) {
        controller.enqueue(incoming.shift()!);
      }
    },
  });

  const writable = new WritableStream<AnyMessage>({
    async write(msg) {
      const isRequest =
        "method" in msg && "id" in msg && msg.id !== undefined && msg.id !== null;

      const headers: Record<string, string> = {
        "Content-Type": "application/json",
        Accept: "application/json, text/event-stream",
      };
      if (sessionId) {
        headers[ACP_SESSION_HEADER] = sessionId;
      }

      if (isFirstRequest && isRequest) {
        // Initialize: open the long-lived SSE stream
        isFirstRequest = false;

        const response = await fetch(`${serverUrl}/acp`, {
          method: "POST",
          headers,
          body: JSON.stringify(msg),
          signal: sseAbort.signal,
        });

        const sid = response.headers.get(ACP_SESSION_HEADER);
        if (sid) sessionId = sid;

        // Consume SSE in background for the session lifetime
        consumeSSE(response);
      } else if (isRequest) {
        // Subsequent requests: fire-and-forget POST.
        // The server processes the message before blocking on the SSE mutex,
        // so the response arrives on the first SSE stream.
        const abort = new AbortController();
        fetch(`${serverUrl}/acp`, {
          method: "POST",
          headers,
          body: JSON.stringify(msg),
          signal: abort.signal,
        }).catch(() => {});
        // Allow time for the server to receive and process the request
        setTimeout(() => abort.abort(), 200);
      } else {
        // Notifications/responses: server returns 202 immediately
        await fetch(`${serverUrl}/acp`, {
          method: "POST",
          headers,
          body: JSON.stringify(msg),
        });
      }
    },

    close() {
      sseAbort.abort();
    },
  });

  return { readable, writable };
}
