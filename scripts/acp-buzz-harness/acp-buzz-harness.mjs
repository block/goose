// Proves goose -> Buzz tools over MCP-over-ACP: spawns `goose acp`, declares
// the Buzz tools as an `acp` MCP server, and answers goose's `mcp/message`
// requests with Buzz MCP's real `respond()` against an in-memory community.
//
//   node acp-buzz-harness.mjs <goose-binary> <provider> <model>
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { respond } from "./buzz-rpc.mjs";

const [gooseBin, provider, model] = process.argv.slice(2);
if (!gooseBin || !provider || !model) {
  console.error("usage: node acp-buzz-harness.mjs <goose> <provider> <model>");
  process.exit(2);
}

const SERVER_ID = `buzz:${Math.random().toString(16).slice(2, 8)}`;
const AGENT = "a".repeat(64);
const OWNER = "b".repeat(64);
const CHANNEL = "c".repeat(64);

// A tiny community: one channel the agent belongs to, with one message in it.
let nextId = 1;
const id = () => (nextId++).toString(16).padStart(64, "0");
const events = [];
const add = (event) => {
  const full = { id: id(), sig: "0".repeat(128), created_at: Math.floor(Date.now() / 1000), ...event };
  events.push(full);
  return full;
};
add({ kind: 39000, pubkey: OWNER, content: "", tags: [["d", CHANNEL], ["name", "goose-lab"]] });
add({ kind: 39002, pubkey: OWNER, content: "", tags: [["d", CHANNEL], ["p", AGENT], ["p", OWNER]] });
add({ kind: 0, pubkey: OWNER, content: JSON.stringify({ name: "salman" }), tags: [] });
add({
  kind: 9,
  pubkey: OWNER,
  content: "The goose ACP prototype demo is at 3pm today. Bring snacks.",
  tags: [["h", CHANNEL]],
});

// Models sometimes pass the channel's name where the tools expect its id.
const channelRef = (value) => (value === "goose-lab" || value === "#goose-lab" ? CHANNEL : value);
const normalize = (filter) =>
  Object.fromEntries(Object.entries(filter).map(([key, value]) =>
    key === "#h" || key === "#d" ? [key, value.map(channelRef)] : [key, value]));

const matches = (filter, event) =>
  Object.entries(filter).every(([key, value]) => {
    if (key === "limit" || key === "search") return true;
    if (key === "kinds") return value.includes(event.kind);
    if (key === "ids") return value.includes(event.id);
    if (key === "authors") return value.includes(event.pubkey);
    if (key.startsWith("#"))
      return event.tags.some((tag) => tag[0] === key.slice(1) && value.includes(tag[1]));
    return true;
  });

const published = [];
const client = {
  pubkey: AGENT,
  async query(filters) {
    return events.filter((event) => filters.some((filter) => matches(normalize(filter), event)));
  },
  async publish(template) {
    const tags = template.tags.map((tag) => (tag[0] === "h" ? ["h", channelRef(tag[1])] : tag));
    const event = add({ ...template, tags, pubkey: AGENT });
    published.push(event);
    return event;
  },
  async upload() { throw new Error("uploads are not part of this test"); },
  async media() { throw new Error("media is not part of this test"); },
  async read() { throw new Error("files are not part of this test"); },
  async memories() { return []; },
  async remember() {},
};

// --- ACP over stdio ---------------------------------------------------------
const goose = spawn(gooseBin, ["acp"], {
  env: { ...process.env, GOOSE_PROVIDER: provider, GOOSE_MODEL: model },
  stdio: ["pipe", "pipe", "pipe"],
});
let stderr = "";
goose.stderr.on("data", (chunk) => { stderr += chunk; });
goose.on("exit", (code) => {
  if (!finished) {
    console.error(`goose exited early (${code})\n${stderr.slice(-4000)}`);
    process.exit(1);
  }
});

const pending = new Map();
let nextRequest = 1;
const write = (message) => goose.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", ...message })}\n`);
const request = (method, params) =>
  new Promise((resolve, reject) => {
    const requestId = nextRequest++;
    pending.set(requestId, { resolve, reject });
    write({ id: requestId, method, params });
  });

const stats = { mcpMessages: 0, toolsList: 0, toolCalls: [] };
let agentText = "";

async function onRequest(message) {
  const { id: requestId, method, params } = message;
  if (method === "mcp/message") {
    stats.mcpMessages++;
    if (params.serverId !== SERVER_ID) {
      write({ id: requestId, error: { code: -33001, message: "Server registration unavailable" } });
      return;
    }
    if (params.method === "tools/list") stats.toolsList++;
    if (params.method === "tools/call") stats.toolCalls.push(params.params?.name);
    // Buzz MCP answers plain JSON-RPC; the inner id is the logical request id.
    const answer = await respond(client, { channel: CHANNEL }, {
      id: params.requestId,
      method: params.method,
      params: params.params ?? {},
    });
    const outcome = "error" in answer ? { error: answer.error } : { result: answer.result };
    if (params.method === "tools/call")
      console.log(`  [buzz] ${params.params?.name}(${JSON.stringify(params.params?.arguments)}) -> ${JSON.stringify(outcome.result?.content?.[0]?.text ?? outcome.error).slice(0, 160)}`);
    write({ id: requestId, result: outcome });
    return;
  }
  if (method === "session/request_permission") {
    const allow = params.options.find((option) => option.kind === "allow_once") ?? params.options[0];
    write({ id: requestId, result: { outcome: { outcome: "selected", optionId: allow.optionId } } });
    return;
  }
  write({ id: requestId, error: { code: -32601, message: `No method ${method}` } });
}

function onNotification({ method, params }) {
  if (method !== "session/update") return;
  const update = params.update;
  if (update.sessionUpdate === "agent_message_chunk" && update.content?.type === "text")
    agentText += update.content.text;
  if (update.sessionUpdate === "tool_call") console.log(`  [goose] tool_call ${update.title ?? update.toolCallId}`);
}

createInterface({ input: goose.stdout }).on("line", (line) => {
  if (!line.trim()) return;
  let message;
  try { message = JSON.parse(line); } catch { return; }
  if (message.method && message.id !== undefined) onRequest(message).catch((error) => {
    write({ id: message.id, error: { code: -32603, message: String(error) } });
  });
  else if (message.method) onNotification(message);
  else if (pending.has(message.id)) {
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    message.error ? reject(new Error(JSON.stringify(message.error))) : resolve(message.result);
  }
});

let finished = false;
const fail = (why) => {
  console.error(`FAIL: ${why}`);
  finished = true;
  goose.kill();
  process.exit(1);
};

const init = await request("initialize", {
  protocolVersion: 1,
  clientCapabilities: {},
  clientInfo: { name: "buzz-acp-harness", version: "0" },
});
const acp = init.agentCapabilities?.mcpCapabilities?.acp;
console.log(`initialize: mcpCapabilities.acp = ${acp}`);
if (acp !== true) fail("goose does not advertise MCP-over-ACP");

// `mcp-servers` declares Buzz in `mcpServers` next to goose's defaults;
// `enabled-extensions` sends the explicit list a Buzz agent uses instead.
const mode = process.env.HARNESS_MODE ?? "mcp-servers";
const buzzServer = { type: "acp", name: "buzz", serverId: SERVER_ID };
const session = await request(
  "session/new",
  mode === "enabled-extensions"
    ? {
        cwd: process.cwd(),
        mcpServers: [],
        _meta: {
          enabledExtensions: [
            { type: "builtin", name: "developer" },
            { type: "mcp", server: buzzServer },
          ],
        },
      }
    : { cwd: process.cwd(), mcpServers: [buzzServer] },
);
console.log(`session/new: ${session.sessionId} (${provider}/${model}, ${mode})`);
const started = (session._meta?.extensionResults ?? []).map((result) => `${result.name}${result.success ? "" : " (failed)"}`);
console.log(`extensions: ${started.join(", ")}`);
if (!started.includes("buzz")) fail("the session did not start the buzz extension");
if (mode === "enabled-extensions" && started.length !== 2)
  fail(`expected only developer and buzz, got ${started.join(", ")}`);

const prompt =
  "You are an agent in the Buzz chat app. Use your buzz tools only. " +
  "First list your channels, then read the recent messages in the goose-lab channel, " +
  "then send one short message to that channel (audience: everyone) that restates " +
  "when the demo is. Then stop.";
const turn = await request("session/prompt", {
  sessionId: session.sessionId,
  prompt: [{ type: "text", text: prompt }],
});

finished = true;
goose.kill();
console.log(`stopReason: ${turn.stopReason}`);
console.log(`mcp/message requests: ${stats.mcpMessages} (tools/list ${stats.toolsList}, tools/call ${stats.toolCalls.join(", ") || "none"})`);
console.log(`agent said: ${agentText.trim().slice(0, 400)}`);
for (const event of published) console.log(`published kind ${event.kind}: ${event.content}`);

const sent = published.find((event) => event.kind === 9 && event.tags.some((tag) => tag[0] === "h" && tag[1] === CHANNEL));
if (!stats.toolsList) fail("goose never listed the Buzz tools");
if (!sent) fail("the agent did not send a message to the channel");
if (!/3\s*pm/i.test(sent.content)) fail("the sent message does not mention the demo time");
console.log("PASS");
process.exit(0);
