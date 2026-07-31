// Lean browser chat client for a goose roam agent.
//
// The whole stack, entirely in the browser tab (no Tauri, no local server):
//
//   iroh (wasm, relay-only) ── roam handshake ──► authorized ACP byte duplex
//        ▼  roamByteStreams()  →  ndJsonStream()  →  ClientSideConnection
//   typed ACP: initialize / listSessions / newSession / loadSession / prompt
//
// Session list/new/load all work because goose's ACP server advertises them
// and replays history on loadSession through the same sessionUpdate callback.
import {
  ClientSideConnection,
  ndJsonStream,
  PROTOCOL_VERSION,
  type Client,
  type Agent,
  type SessionNotification,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionInfo,
} from "@agentclientprotocol/sdk";
import initWasm, { RoamClient, type RoamConnection } from "./wasm/goose_roaming_web.js";
import { roamByteStreams } from "./roam-stream.js";
import { ChatRenderer } from "./render.js";

const SECRET_STORAGE_KEY = "goose-roam-secret-hex";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const els = {
  myId: $("my-endpoint-id"),
  myCard: $("my-card"),
  copyCard: $<HTMLButtonElement>("copy-card"),
  cardInput: $<HTMLTextAreaElement>("card-input"),
  connectBtn: $<HTMLButtonElement>("connect-btn"),
  status: $("status"),
  agentBadge: $("agent-badge"),
  connectPanel: $("connect-panel"),
  workspace: $("workspace"),
  sidebar: $("sidebar"),
  sessionList: $("session-list"),
  newSession: $<HTMLButtonElement>("new-session"),
  log: $("log"),
  promptForm: $<HTMLFormElement>("prompt-form"),
  promptInput: $<HTMLTextAreaElement>("prompt-input"),
  sendBtn: $<HTMLButtonElement>("send-btn"),
};

function setStatus(text: string, kind: "idle" | "busy" | "ok" | "err" = "idle") {
  els.status.textContent = text;
  els.status.className = `status ${kind}`;
}

const chat = new ChatRenderer(els.log);

let agent: Agent | null = null;
let sessionId: string | null = null;
let roamConn: RoamConnection | null = null;
let busy = false;

// --- ACP client callbacks (the host drives these) ----------------------

function makeClient(): Client {
  return {
    async sessionUpdate(params: SessionNotification): Promise<void> {
      const u = params.update;
      switch (u.sessionUpdate) {
        case "user_message_chunk":
          chat.chunk("user", u.content);
          break;
        case "agent_message_chunk":
          chat.chunk("agent", u.content);
          break;
        case "agent_thought_chunk":
          chat.chunk("thought", u.content);
          break;
        case "tool_call":
          chat.toolCall(u);
          break;
        case "tool_call_update":
          chat.toolUpdate(u);
          break;
        case "plan":
          chat.plan(u.entries);
          break;
      }
    },

    async requestPermission(
      params: RequestPermissionRequest,
    ): Promise<RequestPermissionResponse> {
      const title = params.toolCall?.title ?? "the agent";
      // Inline, non-blocking permission card (never window.confirm, which would
      // freeze the JS thread and stall the ACP message pump mid-turn).
      const optionId = await chat.permission(
        title,
        params.options.map((o) => ({ optionId: o.optionId, name: o.name, kind: o.kind })),
      );
      if (optionId) return { outcome: { outcome: "selected", optionId } };
      return { outcome: { outcome: "cancelled" } };
    },
  };
}

// --- connect + handshake ------------------------------------------------

async function connect(cardText: string, client: RoamClient) {
  setStatus("dialing host over relay…", "busy");
  els.connectBtn.disabled = true;
  try {
    roamConn = await client.connect(cardText.trim(), "web");
    const agentId = roamConn.agentId();

    const bytes = roamByteStreams(roamConn);
    const stream = ndJsonStream(bytes.writable, bytes.readable);
    const conn = new ClientSideConnection(() => makeClient(), stream);
    agent = conn;

    await conn.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
    });

    els.connectPanel.hidden = true;
    els.workspace.hidden = false;
    els.agentBadge.hidden = false;
    els.agentBadge.textContent = `agent ${agentId.slice(0, 12)}…`;
    setStatus("connected", "ok");

    // Keep the composer disabled until a session is actually ready, so a send
    // can't be silently dropped in the gap.
    setBusy(true);
    await refreshSessions();
    await startNewSession();
    els.promptInput.focus();
  } catch (err) {
    console.error(err);
    setStatus(`connect failed: ${err}`, "err");
    els.connectBtn.disabled = false;
  }
}

// --- sessions -----------------------------------------------------------

let sessions: SessionInfo[] = [];

async function refreshSessions() {
  if (!agent?.listSessions) return;
  try {
    const res = await agent.listSessions({});
    sessions = res.sessions ?? [];
    renderSessionList();
  } catch (err) {
    // listSessions is optional; a host without it just shows the current one.
    console.warn("listSessions unavailable:", err);
  }
}

function renderSessionList() {
  els.sessionList.replaceChildren();
  for (const s of sessions) {
    const btn = document.createElement("button");
    btn.className = "session-item" + (s.sessionId === sessionId ? " active" : "");
    const title = document.createElement("div");
    title.className = "s-title";
    title.textContent = s.title || "(untitled session)";
    const sub = document.createElement("div");
    sub.className = "s-sub";
    sub.textContent = s.sessionId.slice(0, 8);
    btn.append(title, sub);
    btn.onclick = () => void openSession(s.sessionId);
    els.sessionList.appendChild(btn);
  }
}

async function startNewSession() {
  if (!agent) return;
  setBusy(true);
  try {
    // Host ignores our cwd (uses the share's dir) but ACP wants an absolute path.
    const res = await agent.newSession({ cwd: "/", mcpServers: [] });
    sessionId = res.sessionId;
    chat.clear();
    chat.system("New session — say hello 👋");
    await refreshSessions();
    renderSessionList();
  } catch (err) {
    console.error(err);
    chat.system(`could not start session: ${err}`);
  } finally {
    setBusy(false);
  }
}

async function openSession(id: string) {
  if (!agent?.loadSession || id === sessionId) {
    if (id === sessionId) return;
    chat.system("this host doesn't support loading past sessions");
    return;
  }
  setBusy(true);
  setStatus("loading session…", "busy");
  try {
    chat.clear();
    sessionId = id;
    renderSessionList();
    // Replays the session's history back through sessionUpdate → chat renders it.
    await agent.loadSession({ sessionId: id, cwd: "/", mcpServers: [] });
    chat.finalizeTurn();
    if (chat.isEmpty) chat.system("(empty session)");
    setStatus("connected", "ok");
  } catch (err) {
    console.error(err);
    chat.system(`could not load session: ${err}`);
    setStatus("connected", "ok");
  } finally {
    setBusy(false);
    els.promptInput.focus();
  }
}

// --- prompting ----------------------------------------------------------

function setBusy(b: boolean) {
  busy = b;
  els.promptInput.disabled = b;
  els.sendBtn.disabled = b;
  els.newSession.disabled = b;
}

async function sendPrompt(text: string) {
  if (!agent || !sessionId || busy) return;
  chat.userMessage(text);
  els.promptInput.value = "";
  autosize();
  setBusy(true);
  setStatus("thinking…", "busy");
  try {
    const res = await agent.prompt({ sessionId, prompt: [{ type: "text", text }] });
    chat.finalizeTurn();
    if (res.stopReason && res.stopReason !== "end_turn") {
      chat.system(`· ${res.stopReason}`);
    }
    setStatus("connected", "ok");
    // A first prompt often names the session — refresh titles.
    void refreshSessions();
  } catch (err) {
    console.error(err);
    chat.system(`error: ${err}`);
    setStatus("connected", "ok");
  } finally {
    setBusy(false);
    els.promptInput.focus();
  }
}

function autosize() {
  els.promptInput.style.height = "auto";
  els.promptInput.style.height = Math.min(els.promptInput.scrollHeight, 200) + "px";
}

// --- boot ---------------------------------------------------------------

async function main() {
  setStatus("loading…", "busy");
  await initWasm();

  const saved = localStorage.getItem(SECRET_STORAGE_KEY) ?? undefined;
  const client = new RoamClient(saved);
  if (!saved) localStorage.setItem(SECRET_STORAGE_KEY, client.secretHex());

  els.myId.textContent = client.endpointId();
  els.myCard.textContent = client.myCard();
  els.copyCard.onclick = () => navigator.clipboard?.writeText(client.myCard());
  setStatus("not connected");

  els.connectBtn.onclick = () => {
    const card = els.cardInput.value.trim();
    if (card) void connect(card, client);
  };
  els.newSession.onclick = () => void startNewSession();
  els.promptForm.onsubmit = (e) => {
    e.preventDefault();
    const text = els.promptInput.value.trim();
    if (text) void sendPrompt(text);
  };
  els.promptInput.addEventListener("input", autosize);
  els.promptInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      const text = els.promptInput.value.trim();
      if (text) void sendPrompt(text);
    }
  });
}

void main();
