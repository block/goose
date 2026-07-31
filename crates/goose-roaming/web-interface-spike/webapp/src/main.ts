// Lean browser chat client for a goose roam agent.
//
// The whole stack, entirely in the browser tab (no Tauri, no local server):
//
//   iroh (wasm, relay-only) ── roam handshake ──► authorized ACP byte duplex
//        │
//        ▼  roamByteStreams()
//   Web Streams <Uint8Array>
//        │
//        ▼  ndJsonStream()            (from @agentclientprotocol/sdk)
//   Stream<AnyMessage>
//        │
//        ▼  new ClientSideConnection(client, stream)
//   typed ACP: initialize / newSession / prompt / sessionUpdate
//
import {
  ClientSideConnection,
  ndJsonStream,
  PROTOCOL_VERSION,
  type Client,
  type Agent,
  type SessionNotification,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type ContentBlock,
} from "@agentclientprotocol/sdk";
import initWasm, { RoamClient, type RoamConnection } from "./wasm/goose_roaming_web.js";
import { roamByteStreams } from "./roam-stream.js";

const SECRET_STORAGE_KEY = "goose-roam-secret-hex";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const els = {
  myId: $("my-endpoint-id"),
  myCard: $("my-card"),
  copyCard: $<HTMLButtonElement>("copy-card"),
  cardInput: $<HTMLTextAreaElement>("card-input"),
  connectBtn: $<HTMLButtonElement>("connect-btn"),
  status: $("status"),
  connectPanel: $("connect-panel"),
  chat: $("chat"),
  log: $("log"),
  promptForm: $<HTMLFormElement>("prompt-form"),
  promptInput: $<HTMLInputElement>("prompt-input"),
};

function setStatus(text: string, kind: "idle" | "busy" | "ok" | "err" = "idle") {
  els.status.textContent = text;
  els.status.className = `status ${kind}`;
}

// --- chat log rendering -------------------------------------------------

function addLine(role: string, text: string): HTMLDivElement {
  const line = document.createElement("div");
  line.className = `line ${role}`;
  const who = document.createElement("span");
  who.className = "who";
  who.textContent = role;
  const body = document.createElement("span");
  body.className = "body";
  body.textContent = text;
  line.append(who, body);
  els.log.appendChild(line);
  els.log.scrollTop = els.log.scrollHeight;
  return line;
}

function contentText(content: ContentBlock): string {
  return content?.type === "text" ? content.text : `[${content?.type ?? "content"}]`;
}

// A single agent turn streams as many chunks; coalesce them into one line.
let currentAgentLine: HTMLSpanElement | null = null;
function appendAgentChunk(text: string) {
  if (!currentAgentLine) {
    const line = addLine("agent", "");
    currentAgentLine = line.querySelector(".body");
  }
  if (currentAgentLine) {
    currentAgentLine.textContent += text;
    els.log.scrollTop = els.log.scrollHeight;
  }
}
function endAgentTurn() {
  currentAgentLine = null;
}

// --- ACP client callbacks (host drives these) --------------------------

function makeClient(): Client {
  return {
    async sessionUpdate(params: SessionNotification): Promise<void> {
      const u = params.update;
      switch (u.sessionUpdate) {
        case "agent_message_chunk":
          appendAgentChunk(contentText(u.content));
          break;
        case "agent_thought_chunk":
          // keep thoughts visible but muted
          addLine("thought", contentText(u.content));
          break;
        case "tool_call":
          addLine("tool", `▶ ${u.title ?? u.toolCallId ?? "tool call"}`);
          break;
        case "tool_call_update":
          if (u.status) addLine("tool", `  ${u.status}`);
          break;
        case "plan":
          addLine("plan", "📋 plan updated");
          break;
        default:
          // ignore the rest for this lean client
          break;
      }
    },

    async requestPermission(
      params: RequestPermissionRequest,
    ): Promise<RequestPermissionResponse> {
      const title = params.toolCall?.title ?? "the agent";
      const labels = params.options.map((o) => o.name).join(" / ");
      const ok = window.confirm(
        `${title} requests permission.\n\nAllow?\n(options: ${labels})`,
      );
      // Prefer an explicit allow/reject option by kind; else fall back to
      // the first / cancel.
      const pick = (kinds: string[]) =>
        params.options.find((o) => kinds.includes(o.kind))?.optionId;
      if (ok) {
        const id = pick(["allow_once", "allow_always"]) ?? params.options[0]?.optionId;
        if (id) return { outcome: { outcome: "selected", optionId: id } };
      } else {
        const id = pick(["reject_once", "reject_always"]);
        if (id) return { outcome: { outcome: "selected", optionId: id } };
      }
      return { outcome: { outcome: "cancelled" } };
    },
  };
}

// --- wiring -------------------------------------------------------------

let agent: Agent | null = null;
let sessionId: string | null = null;
let roamConn: RoamConnection | null = null;

async function connect(cardText: string, client: RoamClient) {
  setStatus("dialing host over relay…", "busy");
  els.connectBtn.disabled = true;
  try {
    roamConn = await client.connect(cardText.trim(), "web");
    setStatus(`connected to ${roamConn.agentId()}`, "ok");

    const bytes = roamByteStreams(roamConn);
    const stream = ndJsonStream(bytes.writable, bytes.readable);
    const conn = new ClientSideConnection(() => makeClient(), stream);
    agent = conn;

    await conn.initialize({
      protocolVersion: PROTOCOL_VERSION,
      // The host imposes its own cwd/tools; the browser advertises no fs or
      // terminal capabilities.
      clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
    });

    // Host ignores our cwd (it uses the share's working dir), but ACP requires
    // a syntactically-absolute path.
    const session = await conn.newSession({ cwd: "/", mcpServers: [] });
    sessionId = session.sessionId;

    els.connectPanel.hidden = true;
    els.chat.hidden = false;
    els.promptInput.focus();
    addLine("system", `session ${sessionId.slice(0, 8)}… ready — say hello`);
  } catch (err) {
    console.error(err);
    setStatus(`connect failed: ${err}`, "err");
    els.connectBtn.disabled = false;
  }
}

async function sendPrompt(text: string) {
  if (!agent || !sessionId) return;
  addLine("you", text);
  endAgentTurn();
  els.promptInput.value = "";
  els.promptInput.disabled = true;
  try {
    const res = await agent.prompt({
      sessionId,
      prompt: [{ type: "text", text }],
    });
    endAgentTurn();
    if (res.stopReason && res.stopReason !== "end_turn") {
      addLine("system", `· ${res.stopReason}`);
    }
  } catch (err) {
    console.error(err);
    addLine("system", `error: ${err}`);
  } finally {
    els.promptInput.disabled = false;
    els.promptInput.focus();
  }
}

async function main() {
  setStatus("loading…", "busy");
  await initWasm();

  // Stable per-browser identity: reuse the persisted secret so the host only
  // has to `accept` this key once.
  const saved = localStorage.getItem(SECRET_STORAGE_KEY) ?? undefined;
  const client = new RoamClient(saved);
  if (!saved) localStorage.setItem(SECRET_STORAGE_KEY, client.secretHex());

  const id = client.endpointId();
  const card = client.myCard();
  els.myId.textContent = id;
  els.myCard.textContent = card;
  els.copyCard.onclick = () => navigator.clipboard?.writeText(card);
  setStatus("not connected");

  els.connectBtn.onclick = () => {
    const card = els.cardInput.value.trim();
    if (card) void connect(card, client);
  };
  els.promptForm.onsubmit = (e) => {
    e.preventDefault();
    const text = els.promptInput.value.trim();
    if (text) void sendPrompt(text);
  };
}

void main();
