import path from "node:path";
import { spawn } from "node:child_process";

const repoRoot = process.cwd();
const protocolVersion = "2025-03-26";

function writeFrame(stream, payload) {
  const body = JSON.stringify(payload);
  stream.write(`Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`);
}

function createMcpClient(serverPath) {
  const child = spawn("node", [serverPath], {
    cwd: repoRoot,
    stdio: ["pipe", "pipe", "pipe"],
  });

  let stdoutBuffer = Buffer.alloc(0);
  let stderrBuffer = "";
  let nextId = 1;
  const pending = new Map();

  child.stdout.on("data", (chunk) => {
    stdoutBuffer = Buffer.concat([stdoutBuffer, chunk]);

    while (true) {
      const headerEnd = stdoutBuffer.indexOf("\r\n\r\n");
      if (headerEnd === -1) {
        return;
      }

      const header = stdoutBuffer.slice(0, headerEnd).toString("utf8");
      const contentLengthMatch = header.match(/Content-Length:\s*(\d+)/i);
      if (!contentLengthMatch) {
        throw new Error(`Invalid MCP response header from ${serverPath}`);
      }

      const contentLength = Number.parseInt(contentLengthMatch[1], 10);
      const frameEnd = headerEnd + 4 + contentLength;
      if (stdoutBuffer.length < frameEnd) {
        return;
      }

      const body = stdoutBuffer.slice(headerEnd + 4, frameEnd).toString("utf8");
      stdoutBuffer = stdoutBuffer.slice(frameEnd);

      const message = JSON.parse(body);
      if (!Object.prototype.hasOwnProperty.call(message, "id")) {
        continue;
      }

      const waiter = pending.get(message.id);
      if (!waiter) {
        continue;
      }
      pending.delete(message.id);

      if (message.error) {
        waiter.reject(new Error(`${message.error.message}: ${JSON.stringify(message.error.data ?? {})}`));
        continue;
      }

      waiter.resolve(message.result);
    }
  });

  child.stderr.on("data", (chunk) => {
    stderrBuffer += chunk.toString("utf8");
  });

  child.on("exit", (code) => {
    for (const waiter of pending.values()) {
      waiter.reject(
        new Error(
          `${serverPath} exited before reply (code=${code ?? "null"}) stderr=${stderrBuffer.trim()}`,
        ),
      );
    }
    pending.clear();
  });

  return {
    async request(method, params = {}) {
      const id = nextId++;
      const result = await new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        writeFrame(child.stdin, { jsonrpc: "2.0", id, method, params });
      });
      return result;
    },
    notify(method, params = {}) {
      writeFrame(child.stdin, { jsonrpc: "2.0", method, params });
    },
    async close() {
      child.kill("SIGTERM");
      await new Promise((resolve) => child.once("exit", resolve));
    },
  };
}

async function smokeBrowserAssist() {
  const client = createMcpClient(
    path.join(repoRoot, "distro/security-cn/extensions/browser-assist-mcp/server.mjs"),
  );

  try {
    const init = await client.request("initialize", {
      protocolVersion,
      capabilities: {},
      clientInfo: { name: "security-goose-smoke", version: "0.1.0" },
    });
    if (init.protocolVersion !== protocolVersion) {
      throw new Error("browser-assist initialize returned unexpected protocolVersion");
    }
    client.notify("notifications/initialized");

    const toolList = await client.request("tools/list", {});
    const toolNames = toolList.tools.map((tool) => tool.name);
    for (const requiredName of ["summarize_web_page", "extract_page_observables"]) {
      if (!toolNames.includes(requiredName)) {
        throw new Error(`browser-assist missing tool ${requiredName}`);
      }
    }

    const html = `
      <html>
        <head>
          <title>Security Goose Login</title>
          <meta name="description" content="preview login page">
        </head>
        <body>
          <form action="/signin" method="post">
            <input type="email" name="email" />
            <input type="password" name="password" />
          </form>
          <a href="https://malicious.example/login?token=abc">Continue</a>
          <script src="https://cdn.example/app.js"></script>
        </body>
      </html>
    `;

    const summary = await client.request("tools/call", {
      name: "summarize_web_page",
      arguments: {
        html,
        source_url: "https://portal.example/login",
      },
    });
    const structured = summary.structuredContent;
    if (structured.title !== "Security Goose Login") {
      throw new Error("browser-assist summary did not extract title");
    }
    if (structured.counts.forms < 1) {
      throw new Error("browser-assist summary did not detect forms");
    }

    const observables = await client.request("tools/call", {
      name: "extract_page_observables",
      arguments: {
        html,
        source_url: "https://portal.example/login",
      },
    });
    if (!observables.structuredContent.observables.urls.includes("https://malicious.example/login?token=abc")) {
      throw new Error("browser-assist observable extraction missed URL");
    }

    console.log("browser-assist-mcp smoke passed");
  } finally {
    await client.close();
  }
}

async function smokeThreatIntel() {
  const client = createMcpClient(
    path.join(repoRoot, "distro/security-cn/extensions/threat-intel-mcp/server.mjs"),
  );

  try {
    const init = await client.request("initialize", {
      protocolVersion,
      capabilities: {},
      clientInfo: { name: "security-goose-smoke", version: "0.1.0" },
    });
    if (init.protocolVersion !== protocolVersion) {
      throw new Error("threat-intel initialize returned unexpected protocolVersion");
    }
    client.notify("notifications/initialized");

    const toolList = await client.request("tools/list", {});
    const toolNames = toolList.tools.map((tool) => tool.name);
    for (const requiredName of [
      "extract_observables_from_text",
      "analyze_observable",
      "enrich_domain_dns",
    ]) {
      if (!toolNames.includes(requiredName)) {
        throw new Error(`threat-intel missing tool ${requiredName}`);
      }
    }

    const extracted = await client.request("tools/call", {
      name: "extract_observables_from_text",
      arguments: {
        text: "IOC hit from https://evil.example/reset?token=1 and IP 8.8.8.8 mail admin@corp.example",
      },
    });
    const extractedContent = extracted.structuredContent.observables;
    if (!extractedContent.urls.includes("https://evil.example/reset?token=1")) {
      throw new Error("threat-intel extraction missed URL");
    }
    if (!extractedContent.ipv4.includes("8.8.8.8")) {
      throw new Error("threat-intel extraction missed IPv4");
    }

    const analyzed = await client.request("tools/call", {
      name: "analyze_observable",
      arguments: {
        observable: "https://evil.example/reset?token=1",
      },
    });
    if (analyzed.structuredContent.analysis.type !== "url") {
      throw new Error("threat-intel analysis misclassified URL");
    }

    const enriched = await client.request("tools/call", {
      name: "enrich_domain_dns",
      arguments: {
        domain: "example.com",
      },
    });
    if (!Array.isArray(enriched.structuredContent.enrichment.a)) {
      throw new Error("threat-intel DNS enrichment returned unexpected shape");
    }

    console.log("threat-intel-mcp smoke passed");
  } finally {
    await client.close();
  }
}

await smokeBrowserAssist();
await smokeThreatIntel();
