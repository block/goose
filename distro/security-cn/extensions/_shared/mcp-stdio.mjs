export const MCP_PROTOCOL_VERSION = "2025-03-26";

function jsonRpcError(code, message, data) {
  return {
    code,
    message,
    ...(data === undefined ? {} : { data }),
  };
}

function writeFrame(payload) {
  const body = JSON.stringify(payload);
  const frame = `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
  process.stdout.write(frame);
}

function createToolResult(payload, options = {}) {
  if (typeof payload === "string") {
    return {
      content: [{ type: "text", text: payload }],
      ...(options.isError ? { isError: true } : {}),
    };
  }

  return {
    content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
    structuredContent: payload,
    ...(options.isError ? { isError: true } : {}),
  };
}

async function resolveRequest(request, server) {
  const { id, method, params } = request;

  if (method === "initialize") {
    return {
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: MCP_PROTOCOL_VERSION,
        capabilities: {
          tools: {
            listChanged: false,
          },
        },
        serverInfo: {
          name: server.name,
          version: server.version,
        },
        ...(server.instructions ? { instructions: server.instructions } : {}),
      },
    };
  }

  if (method === "tools/list") {
    return {
      jsonrpc: "2.0",
      id,
      result: {
        tools: server.tools.map(({ handler, ...tool }) => tool),
      },
    };
  }

  if (method === "tools/call") {
    const tool = server.tools.find((entry) => entry.name === params?.name);
    if (!tool) {
      return {
        jsonrpc: "2.0",
        id,
        error: jsonRpcError(-32601, `Unknown tool: ${params?.name ?? "<empty>"}`),
      };
    }

    try {
      const payload = await tool.handler(params?.arguments ?? {});
      return {
        jsonrpc: "2.0",
        id,
        result: createToolResult(payload),
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        jsonrpc: "2.0",
        id,
        result: createToolResult(
          {
            error: message,
            tool: tool.name,
          },
          { isError: true },
        ),
      };
    }
  }

  if (method === "ping") {
    return {
      jsonrpc: "2.0",
      id,
      result: {},
    };
  }

  return {
    jsonrpc: "2.0",
    id,
    error: jsonRpcError(-32601, `Method not found: ${method}`),
  };
}

function handleMessage(rawBody, server) {
  let request;
  try {
    request = JSON.parse(rawBody);
  } catch (error) {
    writeFrame({
      jsonrpc: "2.0",
      id: null,
      error: jsonRpcError(-32700, "Invalid JSON", error instanceof Error ? error.message : undefined),
    });
    return;
  }

  if (!request || typeof request !== "object") {
    writeFrame({
      jsonrpc: "2.0",
      id: null,
      error: jsonRpcError(-32600, "Invalid request"),
    });
    return;
  }

  if (!Object.prototype.hasOwnProperty.call(request, "id")) {
    if (request.method !== "notifications/initialized") {
      console.error(`[${server.name}] ignored notification ${request.method ?? "<unknown>"}`);
    }
    return;
  }

  resolveRequest(request, server)
    .then((response) => {
      writeFrame(response);
    })
    .catch((error) => {
      writeFrame({
        jsonrpc: "2.0",
        id: request.id ?? null,
        error: jsonRpcError(
          -32000,
          error instanceof Error ? error.message : "Unhandled server error",
        ),
      });
    });
}

export function startStdioMcpServer(server) {
  let buffer = Buffer.alloc(0);

  process.stdin.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);

    while (true) {
      const headerEnd = buffer.indexOf("\r\n\r\n");
      if (headerEnd === -1) {
        return;
      }

      const header = buffer.slice(0, headerEnd).toString("utf8");
      const contentLengthMatch = header.match(/Content-Length:\s*(\d+)/i);
      if (!contentLengthMatch) {
        console.error(`[${server.name}] invalid MCP frame header`);
        process.exit(1);
      }

      const contentLength = Number.parseInt(contentLengthMatch[1], 10);
      const frameEnd = headerEnd + 4 + contentLength;
      if (buffer.length < frameEnd) {
        return;
      }

      const body = buffer.slice(headerEnd + 4, frameEnd).toString("utf8");
      buffer = buffer.slice(frameEnd);
      handleMessage(body, server);
    }
  });

  process.stdin.resume();
}
