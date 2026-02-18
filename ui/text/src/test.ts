// Minimal test - bypass the SDK, just test raw HTTP + SSE

async function main() {
  const serverUrl = "http://127.0.0.1:3284";

  // Step 1: Initialize - POST and keep SSE stream open
  console.log("1. Sending initialize...");
  const initResponse = await fetch(`${serverUrl}/acp`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json, text/event-stream",
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "0",
        clientInfo: { name: "test", version: "0.1.0" },
        clientCapabilities: {},
      },
    }),
  });

  const sessionId = initResponse.headers.get("Acp-Session-Id");
  console.log("   Session ID:", sessionId);
  console.log("   Status:", initResponse.status);
  console.log("   Content-Type:", initResponse.headers.get("content-type"));

  if (!initResponse.body) {
    console.error("No response body!");
    process.exit(1);
  }

  // Start reading SSE in background
  const reader = initResponse.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  const messages: any[] = [];
  const waiters: Array<() => void> = [];

  function pushMsg(msg: any) {
    messages.push(msg);
    const w = waiters.shift();
    if (w) w();
  }

  function waitForMessage(): Promise<void> {
    if (messages.length > 0) return Promise.resolve();
    return new Promise<void>((r) => waiters.push(r));
  }

  // Background SSE consumer
  (async () => {
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) {
          console.log("   [SSE] stream ended");
          break;
        }
        buffer += decoder.decode(value, { stream: true });
        const parts = buffer.split("\n\n");
        buffer = parts.pop() || "";

        for (const part of parts) {
          for (const line of part.split("\n")) {
            if (line.startsWith("data: ")) {
              try {
                const msg = JSON.parse(line.slice(6));
                console.log("   [SSE] received:", JSON.stringify(msg).slice(0, 150));
                pushMsg(msg);
              } catch (_e) {
                console.log("   [SSE] non-JSON:", line.slice(0, 80));
              }
            } else if (line.startsWith(":")) {
              // keep-alive comment
            } else if (line.trim()) {
              console.log("   [SSE] other:", line.slice(0, 80));
            }
          }
        }
      }
    } catch (e) {
      console.log("   [SSE] error:", e);
    }
  })();

  // Read initialize response
  await waitForMessage();
  const initMsg = messages.shift();
  console.log("   Init response:", JSON.stringify(initMsg).slice(0, 200));

  // Step 2: Send session/new (fire and forget - server will block on SSE mutex)
  console.log("\n2. Sending session/new...");
  const sessionAbort = new AbortController();
  fetch(`${serverUrl}/acp`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json, text/event-stream",
      "Acp-Session-Id": sessionId!,
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 2,
      method: "session/new",
      params: {
        cwd: process.cwd(),
        mcpServers: [],
      },
    }),
    signal: sessionAbort.signal,
  }).catch(() => {});

  // Wait for response on the first SSE stream
  console.log("   Waiting for session/new response on SSE stream...");
  await waitForMessage();
  const sessionMsg = messages.shift();
  console.log("   Session response:", JSON.stringify(sessionMsg).slice(0, 300));
  sessionAbort.abort();

  const newSessionId = sessionMsg?.result?.sessionId;
  console.log("   New session ID:", newSessionId);

  // Step 3: Send prompt
  console.log("\n3. Sending prompt...");
  const promptAbort = new AbortController();
  fetch(`${serverUrl}/acp`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json, text/event-stream",
      "Acp-Session-Id": sessionId!,
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 3,
      method: "session/prompt",
      params: {
        sessionId: newSessionId,
        prompt: [{ type: "text", text: "Say hello in exactly one word" }],
      },
    }),
    signal: promptAbort.signal,
  }).catch(() => {});

  console.log("   Waiting for prompt responses...");
  for (let i = 0; i < 30; i++) {
    await waitForMessage();
    const msg = messages.shift();
    console.log(`   msg[${i}]:`, JSON.stringify(msg).slice(0, 200));
    if (msg?.id === 3) {
      console.log("   Got prompt response!");
      break;
    }
  }

  promptAbort.abort();
  console.log("\nDone!");
  process.exit(0);
}

main().catch((e) => {
  console.error("Fatal:", e);
  process.exit(1);
});
