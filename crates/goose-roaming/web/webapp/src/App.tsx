// The roam web client UI, built on goose's real desktop componentry:
//   - MarkdownContent (react-markdown + katex + syntax highlighting)
//   - ToolCallStatusIndicator (live status dot)
// wired to a roaming ACP connection (GooseClient over the iroh wasm duplex).
//
// DOM ids/classes (#my-card, #connect-btn, .msg.agent, …) are kept identical
// to the previous vanilla UI so tests/e2e.mjs and tests/visual.mjs still work.
import { useCallback, useEffect, useRef, useState } from "react";
import {
  ndJsonStream,
  PROTOCOL_VERSION,
  type Client,
  type SessionNotification,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionInfo,
  type PlanEntry,
} from "@agentclientprotocol/sdk";
import { GooseClient } from "@aaif/goose-sdk";
import MarkdownContent from "@desktop/components/MarkdownContent";
import { ToolCallStatusIndicator, type ToolCallStatus } from "@desktop/components/ToolCallStatusIndicator";
import type { RoamClient, RoamConnection } from "./wasm/goose_roaming_web.js";
import { roamByteStreams } from "./roam-stream.js";

type PermOption = { optionId: string; name: string; kind: string };

type Item =
  | { kind: "msg"; id: number; role: "user" | "agent" | "thought"; text: string }
  | { kind: "system"; id: number; text: string }
  | {
      kind: "tool";
      id: number;
      toolCallId: string;
      title: string;
      status: ToolCallStatus;
      output: string;
    }
  | { kind: "plan"; id: number; entries: PlanEntry[] }
  | {
      kind: "perm";
      id: number;
      title: string;
      options: PermOption[];
      chosen: string | null;
      resolve: (optionId: string | null) => void;
    };

const ACP_TOOL_STATUS: Record<string, ToolCallStatus> = {
  pending: "pending",
  in_progress: "loading",
  completed: "success",
  failed: "error",
};

function contentText(content: unknown): string {
  const c = content as { type?: string; text?: string } | undefined;
  if (!c) return "";
  return c.type === "text" ? (c.text ?? "") : `[${c.type}]`;
}

let nextId = 1;

export function App({ roam }: { roam: RoamClient }) {
  const [items, setItems] = useState<Item[]>([]);
  const [status, setStatus] = useState("not connected");
  const [statusKind, setStatusKind] = useState<"idle" | "busy" | "ok" | "err">("idle");
  const [connected, setConnected] = useState(false);
  const [agentId, setAgentId] = useState("");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [card, setCard] = useState("");
  const agentRef = useRef<GooseClient | null>(null);
  const connRef = useRef<RoamConnection | null>(null);
  const sessionRef = useRef<string | null>(null);
  const streamRole = useRef<"user" | "agent" | "thought" | null>(null);
  const logRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const myCard = roam.myCard();
  const myId = roam.endpointId();

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [items]);

  const push = useCallback((item: Omit<Item, "id">) => {
    streamRole.current = null;
    setItems((xs) => [...xs, { ...item, id: nextId++ } as Item]);
  }, []);

  const chunk = useCallback((role: "user" | "agent" | "thought", text: string) => {
    setItems((xs) => {
      const last = xs[xs.length - 1];
      if (streamRole.current === role && last?.kind === "msg" && last.role === role) {
        return [...xs.slice(0, -1), { ...last, text: last.text + text }];
      }
      streamRole.current = role;
      return [...xs, { kind: "msg", id: nextId++, role, text }];
    });
  }, []);

  const makeClient = useCallback((): Client => {
    return {
      async sessionUpdate(params: SessionNotification): Promise<void> {
        const u = params.update;
        switch (u.sessionUpdate) {
          case "user_message_chunk":
            chunk("user", contentText(u.content));
            break;
          case "agent_message_chunk":
            chunk("agent", contentText(u.content));
            break;
          case "agent_thought_chunk":
            chunk("thought", contentText(u.content));
            break;
          case "tool_call": {
            streamRole.current = null;
            const t = u;
            setItems((xs) => [
              ...xs,
              {
                kind: "tool",
                id: nextId++,
                toolCallId: t.toolCallId,
                title: t.title ?? "tool",
                status: ACP_TOOL_STATUS[t.status ?? "pending"] ?? "pending",
                output: "",
              },
            ]);
            break;
          }
          case "tool_call_update": {
            const t = u;
            setItems((xs) =>
              xs.map((it) =>
                it.kind === "tool" && it.toolCallId === t.toolCallId
                  ? {
                      ...it,
                      title: t.title ?? it.title,
                      status: t.status ? (ACP_TOOL_STATUS[t.status] ?? it.status) : it.status,
                      output:
                        t.content
                          ?.map((c) => (c.type === "content" ? contentText(c.content) : ""))
                          .join("\n")
                          .trim() || it.output,
                    }
                  : it,
              ),
            );
            break;
          }
          case "plan": {
            streamRole.current = null;
            const entries = u.entries;
            setItems((xs) => {
              const i = xs.findIndex((it) => it.kind === "plan");
              if (i >= 0) {
                const copy = [...xs];
                copy[i] = { ...(copy[i] as Extract<Item, { kind: "plan" }>), entries };
                return copy;
              }
              return [...xs, { kind: "plan", id: nextId++, entries }];
            });
            break;
          }
        }
      },
      async requestPermission(
        params: RequestPermissionRequest,
      ): Promise<RequestPermissionResponse> {
        const optionId = await new Promise<string | null>((resolve) => {
          streamRole.current = null;
          setItems((xs) => [
            ...xs,
            {
              kind: "perm",
              id: nextId++,
              title: params.toolCall?.title ?? "the agent",
              options: params.options.map((o) => ({
                optionId: o.optionId,
                name: o.name,
                kind: o.kind,
              })),
              chosen: null,
              resolve,
            },
          ]);
        });
        if (optionId) return { outcome: { outcome: "selected", optionId } };
        return { outcome: { outcome: "cancelled" } };
      },
    };
  }, [chunk]);

  const refreshSessions = useCallback(async () => {
    const agent = agentRef.current;
    if (!agent) return;
    try {
      const res = await agent.listSessions({});
      setSessions(res.sessions ?? []);
    } catch (err) {
      console.warn("listSessions unavailable:", err);
    }
  }, []);

  const newSession = useCallback(async () => {
    const agent = agentRef.current;
    if (!agent) return;
    setBusy(true);
    try {
      const res = await agent.newSession({ cwd: "/", mcpServers: [] });
      sessionRef.current = res.sessionId;
      setSessionId(res.sessionId);
      setItems([{ kind: "system", id: nextId++, text: "New session — say hello 👋" }]);
      void refreshSessions();
    } catch (err) {
      push({ kind: "system", text: `could not start session: ${err}` } as Omit<Item, "id">);
    } finally {
      setBusy(false);
    }
  }, [push, refreshSessions]);

  const openSession = useCallback(
    async (id: string) => {
      const agent = agentRef.current;
      if (!agent || id === sessionRef.current) return;
      setBusy(true);
      setStatus("loading session…");
      setStatusKind("busy");
      try {
        setItems([]);
        sessionRef.current = id;
        setSessionId(id);
        await agent.loadSession({ sessionId: id, cwd: "/", mcpServers: [] });
        streamRole.current = null;
        setStatus("connected");
        setStatusKind("ok");
      } catch (err) {
        push({ kind: "system", text: `could not load session: ${err}` } as Omit<Item, "id">);
        setStatus("connected");
        setStatusKind("ok");
      } finally {
        setBusy(false);
        inputRef.current?.focus();
      }
    },
    [push],
  );

  const connect = useCallback(async () => {
    const text = card.trim();
    if (!text) return;
    setStatus("dialing host over relay…");
    setStatusKind("busy");
    setBusy(true);
    try {
      const conn = await roam.connect(text, "web");
      connRef.current = conn;
      const bytes = roamByteStreams(conn);
      const stream = ndJsonStream(bytes.writable, bytes.readable);
      const agent = new GooseClient(() => makeClient(), stream);
      agentRef.current = agent;
      await agent.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
      });
      setAgentId(conn.agentId());
      setConnected(true);
      setStatus("connected");
      setStatusKind("ok");
      await refreshSessions();
      await newSession();
      inputRef.current?.focus();
    } catch (err) {
      console.error(err);
      setStatus(`connect failed: ${err}`);
      setStatusKind("err");
      setBusy(false);
    }
  }, [card, roam, makeClient, refreshSessions, newSession]);

  const send = useCallback(async () => {
    const agent = agentRef.current;
    const sid = sessionRef.current;
    const el = inputRef.current;
    const text = el?.value.trim();
    if (!agent || !sid || !text || busy) return;
    if (el) el.value = "";
    streamRole.current = null;
    setItems((xs) => [...xs, { kind: "msg", id: nextId++, role: "user", text }]);
    streamRole.current = null;
    setBusy(true);
    setStatus("thinking…");
    setStatusKind("busy");
    try {
      const res = await agent.prompt({ sessionId: sid, prompt: [{ type: "text", text }] });
      streamRole.current = null;
      if (res.stopReason && res.stopReason !== "end_turn") {
        push({ kind: "system", text: `· ${res.stopReason}` } as Omit<Item, "id">);
      }
      void refreshSessions();
    } catch (err) {
      push({ kind: "system", text: `error: ${err}` } as Omit<Item, "id">);
    } finally {
      setBusy(false);
      setStatus("connected");
      setStatusKind("ok");
      inputRef.current?.focus();
    }
  }, [busy, push, refreshSessions]);

  const statusColor =
    statusKind === "ok"
      ? "text-text-success"
      : statusKind === "err"
        ? "text-text-danger"
        : statusKind === "busy"
          ? "text-text-info"
          : "text-text-secondary";

  return (
    <div className="h-screen flex flex-col bg-background-primary text-text-primary">
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-border-primary bg-background-secondary shrink-0">
        <div className="font-bold text-[15px]">
          goose roam <span className="text-text-secondary font-normal">· web</span>
        </div>
        <div className="flex items-center gap-2.5">
          {connected && (
            <span
              id="agent-badge"
              className="font-mono text-[11px] text-text-secondary bg-background-secondary border border-border-primary rounded-full px-2.5 py-0.5"
            >
              agent {agentId.slice(0, 12)}…
            </span>
          )}
          <span id="status" className={`text-xs ${statusColor}`}>
            {status}
          </span>
        </div>
      </div>

      {!connected ? (
        <section id="connect-panel" className="flex-1 grid place-items-center p-6 overflow-auto">
          <div className="w-full max-w-[560px] bg-background-primary border rounded-xl shadow-sm p-7">
            <h2 className="text-lg font-semibold mb-4">Connect to your roaming agent</h2>
            <ol className="list-decimal pl-5 flex flex-col gap-4 leading-relaxed text-sm">
              <li>
                On the host, accept this browser once:
                <div className="flex gap-2 items-center my-2">
                  <code
                    id="my-card"
                    className="flex-1 font-mono text-[11px] bg-background-primary border border-border-primary rounded-lg px-2.5 py-1.5 overflow-hidden text-ellipsis whitespace-nowrap"
                  >
                    {myCard}
                  </code>
                  <button
                    id="copy-card"
                    className="text-text-secondary border border-border-secondary rounded-lg px-2.5 py-1 text-xs hover:border-border-info"
                    onClick={() => navigator.clipboard?.writeText(myCard)}
                  >
                    copy
                  </button>
                </div>
                <code className="block font-mono text-xs text-text-info bg-background-primary border border-border-primary rounded-lg px-2.5 py-2">
                  goose roam peers accept '&lt;card&gt;'
                </code>
                <div className="text-xs text-text-tertiary mt-1.5">
                  key <code id="my-endpoint-id" className="font-mono text-[11px]">{myId}</code>
                </div>
              </li>
              <li>
                Start sharing (<code className="font-mono text-xs">goose roam share</code>) and
                paste its card here:
                <textarea
                  id="card-input"
                  rows={3}
                  className="w-full mt-2 bg-background-primary border border-border-primary rounded-xl px-3 py-2.5 text-sm font-mono focus:outline-none focus:border-border-info resize-none"
                  placeholder="goose+roam://…  (the host's card)"
                  value={card}
                  onChange={(e) => setCard(e.target.value)}
                />
              </li>
            </ol>
            <div className="mt-5 flex justify-end">
              <button
                id="connect-btn"
                disabled={busy}
                onClick={() => void connect()}
                className="bg-background-inverse text-text-inverse font-semibold rounded-lg px-4 py-1.5 hover:brightness-110 disabled:opacity-50"
              >
                connect
              </button>
            </div>
          </div>
        </section>
      ) : (
        <section id="workspace" className="flex-1 grid grid-cols-[240px_1fr] min-h-0">
          <aside className="border-r border-border-primary bg-background-secondary p-3 flex flex-col gap-2.5 min-h-0">
            <button
              id="new-session"
              disabled={busy}
              onClick={() => void newSession()}
              className="w-full bg-background-inverse text-text-inverse font-semibold rounded-lg px-3 py-1.5 hover:brightness-110 disabled:opacity-50"
            >
              + New session
            </button>
            <div id="session-list" className="overflow-y-auto flex flex-col gap-1">
              {sessions.map((s) => (
                <button
                  key={s.sessionId}
                  className={`session-item text-left rounded-lg px-2.5 py-2 transition-all duration-150 ${
                    s.sessionId === sessionId
                      ? "bg-background-tertiary"
                      : "hover:bg-background-secondary hover:shadow-default"
                  }`}
                  onClick={() => void openSession(s.sessionId)}
                >
                  <div className="text-[13px] whitespace-nowrap overflow-hidden text-ellipsis">
                    {s.title || "(untitled session)"}
                  </div>
                  <div className="text-[11px] text-text-tertiary font-mono mt-0.5">
                    {s.sessionId.slice(0, 8)}
                  </div>
                </button>
              ))}
            </div>
          </aside>

          <main id="chat" className="flex flex-col min-h-0">
            <div ref={logRef} id="log" className="flex-1 overflow-y-auto px-6 py-5">
              <div className="max-w-3xl mx-auto w-full flex flex-col gap-4">
              {items.map((it) => {
                switch (it.kind) {
                  case "system":
                    return (
                      <div key={it.id} className="msg system self-center text-text-secondary text-xs">
                        {it.text}
                      </div>
                    );
                  case "msg":
                    if (it.role === "thought")
                      return (
                        <details
                          key={it.id}
                          className="msg thought bg-background-secondary rounded-lg px-3 py-1.5 text-[13px] text-text-secondary"
                        >
                          <summary className="cursor-pointer italic text-text-tertiary">
                            thinking
                          </summary>
                          <div className="body mt-1.5 whitespace-pre-wrap">{it.text}</div>
                        </details>
                      );
                    // Mirror the desktop's presentation (UserMessage /
                    // GooseMessage): user prompts are right-aligned inverse
                    // bubbles; goose replies are plain left-aligned content.
                    if (it.role === "user")
                      return (
                        <div key={it.id} className="msg user flex justify-end w-full">
                          <div className="body user-message-bubble max-w-[85%] w-fit bg-text-primary text-background-primary rounded-xl py-2.5 px-4 whitespace-pre-wrap leading-relaxed">
                            {it.text}
                          </div>
                        </div>
                      );
                    return (
                      <div key={it.id} className="msg agent goose-message flex w-[90%] justify-start min-w-0">
                        <div className="body min-w-0 flex-1 leading-relaxed">
                          <MarkdownContent content={it.text} />
                        </div>
                      </div>
                    );
                  case "tool":
                    return (
                      <div
                        key={it.id}
                        className="tool mt-1 rounded-lg hover:bg-background-secondary transition-colors"
                      >
                        <div className="flex items-center gap-2 px-2 py-1.5 text-sm text-text-secondary">
                          <span className="relative inline-block w-2.5">
                            <ToolCallStatusIndicator status={it.status} className="static" />
                          </span>
                          <span className="flex-1 font-mono text-[12.5px] text-text-warning overflow-hidden text-ellipsis whitespace-nowrap">
                            {it.title}
                          </span>
                          <span className="text-[11px] text-text-secondary">{it.status}</span>
                        </div>
                        {it.output && (
                          <details className="border-t border-border-primary px-2 py-1.5">
                            <summary className="cursor-pointer text-xs text-text-secondary">
                              output
                            </summary>
                            <pre className="mt-2 bg-background-secondary rounded-lg p-2.5 overflow-x-auto max-h-80 overflow-y-auto font-mono text-xs">
                              {it.output}
                            </pre>
                          </details>
                        )}
                      </div>
                    );
                  case "plan":
                    return (
                      <div
                        key={it.id}
                        className="plan mt-1 rounded-lg bg-background-secondary px-3.5 py-2.5"
                      >
                        <div className="text-xs text-text-secondary uppercase tracking-wider mb-1.5">
                          Plan
                        </div>
                        {it.entries.map((e, i) => (
                          <div key={i} className="flex gap-2 py-0.5 text-[13px]">
                            <span className="text-text-secondary">
                              {e.status === "completed" ? "☑" : e.status === "in_progress" ? "▸" : "☐"}
                            </span>
                            <span
                              className={
                                e.status === "completed"
                                  ? "text-text-tertiary line-through"
                                  : "text-text-primary"
                              }
                            >
                              {e.content}
                            </span>
                          </div>
                        ))}
                      </div>
                    );
                  case "perm":
                    return (
                      <div
                        key={it.id}
                        className="perm mt-1 rounded-lg bg-background-warning px-3.5 py-2.5"
                      >
                        <div className="text-[13px] text-text-warning mb-2">
                          🔐 {it.title} needs permission
                        </div>
                        {it.chosen ? (
                          <span className="text-xs text-text-secondary">→ {it.chosen}</span>
                        ) : (
                          <div className="perm-actions flex gap-2 flex-wrap">
                            {it.options.map((o) => (
                              <button
                                key={o.optionId}
                                className={
                                  o.kind.startsWith("allow")
                                    ? "primary bg-background-inverse text-text-inverse font-semibold rounded-lg px-3 py-1 text-[13px]"
                                    : "border border-border-secondary text-text-secondary rounded-lg px-3 py-1 text-[13px]"
                                }
                                onClick={() => {
                                  it.resolve(o.optionId);
                                  setItems((xs) =>
                                    xs.map((x) =>
                                      x.id === it.id && x.kind === "perm"
                                        ? { ...x, chosen: o.name }
                                        : x,
                                    ),
                                  );
                                }}
                              >
                                {o.name}
                              </button>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                }
              })}
              </div>
            </div>
            <form
              id="prompt-form"
              className="border-t border-border-primary px-6 py-3"
              onSubmit={(e) => {
                e.preventDefault();
                void send();
              }}
            >
              <div className="max-w-3xl mx-auto w-full flex gap-2.5 items-end">
              <textarea
                ref={inputRef}
                id="prompt-input"
                rows={1}
                disabled={busy}
                className="flex-1 bg-background-secondary rounded-xl px-3 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none max-h-52"
                placeholder="Message the agent…  (Enter to send, Shift+Enter for newline)"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    void send();
                  }
                }}
              />
              <button
                id="send-btn"
                type="submit"
                disabled={busy}
                className="bg-background-inverse text-text-inverse font-semibold rounded-lg px-4 py-2 hover:brightness-110 disabled:opacity-50"
              >
                send
              </button>
              </div>
            </form>
          </main>
        </section>
      )}
    </div>
  );
}
