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
import jsQR from "jsqr";
import { Button } from "@desktop/components/ui/button";
import { Camera, ChevronLeft, ChevronRight, Menu } from "lucide-react";
import { SessionMatrix } from "./SessionMatrix";
import MarkdownContent from "@desktop/components/MarkdownContent";
import { Goose } from "@desktop/components/icons/Goose";
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

function ToolRow({ item }: { item: Extract<Item, { kind: "tool" }> }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="tool w-full">
      <Button
        onClick={() => setOpen((v) => !v)}
        variant="ghost"
        className="group w-full flex justify-between items-center pr-2 transition-colors rounded-none h-8 px-2"
      >
        <span className="flex items-center gap-2 font-sans text-sm truncate flex-1 min-w-0 text-text-secondary">
          <span className="relative inline-block w-2.5 shrink-0">
            <ToolCallStatusIndicator status={item.status} className="static" />
          </span>
          <span className="truncate">{item.title}</span>
        </span>
        <ChevronRight
          className={`w-4 h-4 shrink-0 opacity-70 group-hover:opacity-100 transition-transform ${open ? "rotate-90" : ""}`}
        />
      </Button>
      {open && (
        <div className="border-t border-border-primary px-2 py-2">
          {item.output ? (
            <pre className="bg-background-secondary rounded-lg p-2.5 overflow-x-auto max-h-80 overflow-y-auto font-mono text-xs whitespace-pre-wrap break-words">
              {item.output}
            </pre>
          ) : (
            <div className="text-xs text-text-tertiary px-1">no output yet</div>
          )}
        </div>
      )}
    </div>
  );
}

function contentText(content: unknown): string {
  const c = content as { type?: string; text?: string } | undefined;
  if (!c) return "";
  return c.type === "text" ? (c.text ?? "") : `[${c.type}]`;
}

// goose surfaces model selection via ACP's generic session config options.
// Find the "model" select option and resolve its currentValue to a name.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function modelFromConfigOptions(opts: any[] | null | undefined): string | null {
  const opt = opts?.find((o) => o.type === "select" && /model/i.test(o.id));
  if (!opt) return null;
  const flat = (opt.options ?? []).flatMap((x: any) => (x.options ? x.options : [x]));
  return flat.find((x: any) => x.id === opt.currentValue)?.name ?? opt.currentValue ?? null;
}

const HOST_CARD_KEY = "goose-roam-last-host-card";
const HOSTS_KEY = "goose-roam-hosts";

type SavedHost = { name: string; card: string; endpointId: string; lastUsed: number };

function relayRegion(cardText: string): string | null {
  try {
    const b64 = cardText.trim().replace(/^goose\+roam:\/\//, "");
    const json = JSON.parse(atob(b64.replace(/-/g, "+").replace(/_/g, "/")));
    const url: string | undefined = json.relay_urls?.[0];
    const m = url?.match(/^https?:\/\/([a-z0-9-]+)\./i);
    return m ? m[1] : null;
  } catch {
    return null;
  }
}

function loadHosts(): SavedHost[] {
  try {
    return JSON.parse(localStorage.getItem(HOSTS_KEY) ?? "[]");
  } catch {
    return [];
  }
}
const SESSION_KEY = "goose-roam-last-session";

declare const __BUILD_STAMP__: string;
const BUILD = typeof __BUILD_STAMP__ !== "undefined" ? __BUILD_STAMP__ : "dev";

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
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [modelName, setModelName] = useState<string | null>(null);
  const [relay, setRelay] = useState<string | null>(null);
  const [card, setCard] = useState("");
  const agentRef = useRef<GooseClient | null>(null);
  const connRef = useRef<RoamConnection | null>(null);
  const sessionRef = useRef<string | null>(null);
  const streamRole = useRef<"user" | "agent" | "thought" | null>(null);
  const logRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const [scanning, setScanning] = useState(false);
  const [hosts, setHosts] = useState<SavedHost[]>(loadHosts);
  const [addingHost, setAddingHost] = useState(false);
  const [hostName, setHostName] = useState("");
  const reconnectAttempt = useRef(0);
  const resumeAfterDrop = useRef<string | null>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const scanStop = useRef<(() => void) | null>(null);
  // BarcodeDetector: Chrome/Android today; feature-detected so the button
  // simply doesn't render where unsupported (iOS Safari needs a lib later).
  const canScan =
    typeof navigator !== "undefined" && !!navigator.mediaDevices?.getUserMedia;

  const startScan = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment" },
      });
      setScanning(true);
      requestAnimationFrame(() => {
        const video = videoRef.current;
        if (video) {
          video.srcObject = stream;
          void video.play();
        }
      });
      // BarcodeDetector on Chrome/Android; jsQR canvas fallback elsewhere (iOS Safari).
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const native = "BarcodeDetector" in window ? new (window as any).BarcodeDetector({ formats: ["qr_code"] }) : null;
      const canvas = document.createElement("canvas");
      const detect = async (video: HTMLVideoElement): Promise<string | null> => {
        if (native) {
          const codes = await native.detect(video);
          return codes.find((c: { rawValue: string }) => c.rawValue.startsWith("goose+roam://"))?.rawValue ?? null;
        }
        const w = video.videoWidth, h = video.videoHeight;
        if (!w || !h) return null;
        canvas.width = w; canvas.height = h;
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (!ctx) return null;
        ctx.drawImage(video, 0, 0, w, h);
        const img = ctx.getImageData(0, 0, w, h);
        const hit = jsQR(img.data, w, h);
        return hit?.data.startsWith("goose+roam://") ? hit.data : null;
      };
      let active = true;
      scanStop.current = () => {
        active = false;
        stream.getTracks().forEach((t) => t.stop());
        setScanning(false);
      };
      const tick = async () => {
        if (!active) return;
        const video = videoRef.current;
        if (video && video.readyState >= 2) {
          try {
            const hit = await detect(video);
            if (hit) {
              setCard(hit);
              scanStop.current?.();
              return;
            }
          } catch {
            // keep scanning
          }
        }
        setTimeout(() => void tick(), 250);
      };
      void tick();
    } catch (err) {
      setScanning(false);
      setStatus(`camera unavailable: ${err}`);
      setStatusKind("err");
    }
  }, []);

  const myCard = roam.myCard();
  const myId = roam.endpointId();

  // Autoscroll only when the user is already near the bottom, so reading
  // back through history isn't yanked away by streaming updates.
  const atBottom = useRef(true);
  useEffect(() => {
    const el = logRef.current;
    if (!el) return;
    const onScroll = () => {
      atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [connected]);
  useEffect(() => {
    if (atBottom.current) logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [items]);

  // Remember the last host across refreshes: prefill and reconnect once.
  const bootTried = useRef(false);
  useEffect(() => {
    if (bootTried.current) return;
    bootTried.current = true;
    const saved = localStorage.getItem(HOST_CARD_KEY);
    if (saved) {
      setCard(saved);
      void connect(saved);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
            setStatus(`tool: ${(t.title ?? "running").slice(0, 28)}`);
            setStatusKind("busy");
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
          case "config_option_update": {
            const m = modelFromConfigOptions((u as { configOptions?: unknown[] }).configOptions);
            if (m) setModelName(m);
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
      const m = modelFromConfigOptions((res as { configOptions?: unknown[] }).configOptions);
      if (m) setModelName(m);
      localStorage.setItem(SESSION_KEY, res.sessionId);
      sessionRef.current = res.sessionId;
      setSessionId(res.sessionId);
      setItems([{ kind: "system", id: nextId++, text: "New session — say hello" }]);
      void refreshSessions();
    } catch (err) {
      push({ kind: "system", text: `could not start session: ${err}` } as Omit<Item, "id">);
    } finally {
      setBusy(false);
    }
  }, [push, refreshSessions]);

  const openSession = useCallback(
    async (id: string, force = false) => {
      const agent = agentRef.current;
      if (!agent || (id === sessionRef.current && !force)) return;
      setBusy(true);
      setStatus("loading session…");
      setStatusKind("busy");
      try {
        setItems([]);
        const info = sessions.find((x) => x.sessionId === id);
        document.title = info?.title ? `${info.title} · goose remote` : "goose remote";
        lastSeenUpdate.current = null;
        localStorage.setItem(SESSION_KEY, id);
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

  // Follow mode: the ACP server only streams updates to the connection that
  // sent the prompt (no multi-viewer broadcast yet). If someone else drives
  // this session (desktop, another device), poll updatedAt while idle and
  // re-load to catch up. Coarse, but keeps a "joined" session scrolling.
  const lastSeenUpdate = useRef<string | null>(null);
  useEffect(() => {
    if (!connected) return;
    const t = setInterval(async () => {
      const agent = agentRef.current;
      if (!agent || busy) return;
      try {
        const res = await agent.listSessions({});
        setSessions(res.sessions ?? []);
        const sid = sessionRef.current;
        if (!sid) return;
        const mine = (res.sessions ?? []).find((x) => x.sessionId === sid);
        const stamp = (mine as { updatedAt?: string } | undefined)?.updatedAt ?? null;
        if (stamp && lastSeenUpdate.current && stamp !== lastSeenUpdate.current) {
          await openSession(sid, true);
        }
        if (stamp) lastSeenUpdate.current = stamp;
      } catch {
        // transient; next tick
      }
    }, 6000);
    return () => clearInterval(t);
  }, [connected, busy, openSession]);


  const connect = useCallback(async (cardText?: string) => {
    const text = (cardText ?? card).trim();
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
      localStorage.setItem(HOST_CARD_KEY, text);
      // Surface unexpected drops (phone sleep, network switch): back to the
      // connect panel with the card prefilled — one tap to reconnect.
      void agent.closed.then(() => {
        if (agentRef.current === agent) {
          agentRef.current = null;
          resumeAfterDrop.current = sessionRef.current;
          sessionRef.current = null;
          setConnected(false);
          setBusy(false);
          const attempt = reconnectAttempt.current++;
          if (attempt < 8) {
            const delay = Math.min(20000, 1500 * 2 ** attempt);
            setStatus("connection lost — reconnecting…");
            setStatusKind("err");
            setTimeout(() => void connect(text), delay);
          } else {
            setStatus("connection lost — press connect");
            setStatusKind("err");
          }
        }
      });
      reconnectAttempt.current = 0;
      setRelay(relayRegion(text));
      {
        const eid = conn.agentId();
        const next = loadHosts().filter((h) => h.endpointId !== eid);
        const prior = loadHosts().find((h) => h.endpointId === eid);
        next.unshift({
          name: hostName.trim() || prior?.name || `host ${eid.slice(0, 8)}`,
          card: text,
          endpointId: eid,
          lastUsed: Date.now(),
        });
        localStorage.setItem(HOSTS_KEY, JSON.stringify(next.slice(0, 12)));
        setHosts(next.slice(0, 12));
        setHostName("");
        setAddingHost(false);
      }
      setAgentId(conn.agentId());
      setConnected(true);
      setStatus("connected");
      setStatusKind("ok");
      await refreshSessions();
      const resume = resumeAfterDrop.current;
      resumeAfterDrop.current = null;
      if (resume) {
        // recovering from a dropped connection mid-conversation: go back to it
        await openSession(resume, true);
        inputRef.current?.focus();
      }
      // otherwise land on the session matrix (front page)
      setBusy(false);
    } catch (err) {
      console.error(err);
      setBusy(false);
      if (reconnectAttempt.current > 0 && reconnectAttempt.current < 8) {
        const delay = Math.min(20000, 1500 * 2 ** reconnectAttempt.current++);
        setStatus("reconnecting…");
        setStatusKind("err");
        setTimeout(() => void connect(text), delay);
      } else {
        setStatus(`connect failed: ${err}`);
        setStatusKind("err");
      }
    }
  }, [card, hostName, roam, makeClient, refreshSessions, newSession, openSession]);

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
    <div className="h-[100dvh] flex flex-col bg-background-primary text-text-primary pb-[env(safe-area-inset-bottom)]">
      <div className="flex items-center justify-between gap-2 px-3 md:px-4 py-2.5 border-b border-border-primary bg-background-secondary shrink-0">
        <div className="flex items-center gap-2 shrink-0">
          {connected && (
            <button
              id="sidebar-toggle"
              className="md:hidden text-text-secondary px-1"
              aria-label="toggle sessions"
              onClick={() => setSidebarOpen((v) => !v)}
            >
              <Menu className="w-5 h-5" />
            </button>
          )}
          <Goose className="w-5 h-5" />
          <span className="font-bold text-[15px] whitespace-nowrap">goose remote</span>
          <span id="build-stamp" className="hidden md:inline text-[10px] text-text-tertiary font-mono self-end pb-0.5">{BUILD}</span>
        </div>
        <div className="flex items-center gap-2.5 min-w-0 shrink">
          {connected && modelName && (
            <span
              id="model-badge"
              className="hidden md:inline text-[11px] text-text-secondary bg-background-secondary border border-border-primary rounded-full px-2.5 py-0.5"
            >
              {modelName}
            </span>
          )}
          {connected && (
            <span
              id="agent-badge"
              className="hidden md:inline font-mono text-[11px] text-text-secondary bg-background-secondary border border-border-primary rounded-full px-2.5 py-0.5"
            >
              agent {agentId.slice(0, 12)}…
            </span>
          )}
          <span id="status" className={`flex items-center gap-1.5 text-xs whitespace-nowrap ${statusColor}`}>
            <span aria-hidden className="inline-block w-2 h-2 rounded-full bg-current shrink-0" />
            <span className={connected ? "hidden md:inline" : "truncate max-w-[180px] md:max-w-none"}>{status}</span>
          </span>
          {connected && (
            <button
              id="switch-host"
              className="shrink-0 text-xs text-text-secondary border border-border-secondary rounded-lg px-2.5 py-1 hover:border-border-info transition-colors"
              title="disconnect and connect to a different host (keeps this browser's identity)"
              onClick={() => {
                localStorage.removeItem(HOST_CARD_KEY);
                localStorage.removeItem(SESSION_KEY);
                location.reload();
              }}
            >
              switch
            </button>
          )}
        </div>
      </div>

      {!connected ? (
        <section id="connect-panel" className="flex-1 grid place-items-center p-3 md:p-6 overflow-auto">
          <div className="w-full max-w-[480px] min-w-0 overflow-hidden bg-background-primary border rounded-xl shadow-sm p-5 md:p-7">
            {hosts.length > 0 && !addingHost ? (
              <>
                <h2 className="text-lg font-semibold mb-1">Your hosts</h2>
                <p className="text-xs text-text-tertiary mb-4">tap to connect</p>
                <div className="flex flex-col gap-1.5">
                  {hosts.map((h) => (
                    <button
                      key={h.endpointId}
                      className="host-row w-full text-left rounded-lg px-3 py-2.5 hover:bg-background-secondary hover:shadow-default transition-all flex items-center gap-3 disabled:opacity-50"
                      disabled={busy}
                      onClick={() => {
                        setCard(h.card);
                        void connect(h.card);
                      }}
                    >
                      <Goose className="w-4 h-4 shrink-0 opacity-70" />
                      <span className="flex-1 min-w-0">
                        <span className="block text-sm font-medium truncate">{h.name}</span>
                        <span className="block text-[10px] text-text-tertiary font-mono truncate">
                          {h.endpointId.slice(0, 16)}
                        </span>
                      </span>
                      <ChevronRight className="w-4 h-4 shrink-0 text-text-tertiary" />
                    </button>
                  ))}
                </div>
                <div className="mt-4 flex justify-center">
                  <button
                    id="add-host"
                    className="text-xs text-text-secondary border border-border-secondary rounded-lg px-3 py-1.5 hover:border-border-info transition-colors"
                    onClick={() => setAddingHost(true)}
                  >
                    add another host
                  </button>
                </div>
              </>
            ) : (
              <>
                <div className="flex items-center gap-2 mb-1">
                  {hosts.length > 0 && (
                    <button
                      aria-label="back to hosts"
                      className="text-text-secondary hover:text-text-primary -ml-1 p-0.5"
                      onClick={() => setAddingHost(false)}
                    >
                      <ChevronLeft className="w-4 h-4" />
                    </button>
                  )}
                  <h2 className="text-lg font-semibold">Add a host</h2>
                </div>
                <p className="text-xs text-text-tertiary mb-4">
                  a machine running <code className="font-mono">goose roam share</code>
                </p>
                <input
                  id="host-name"
                  type="text"
                  className="w-full bg-background-primary border border-border-primary rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-border-info mb-2"
                  placeholder="name (optional) — e.g. laptop"
                  value={hostName}
                  onChange={(e) => setHostName(e.target.value)}
                />
                <textarea
                  id="card-input"
                  rows={3}
                  className="w-full bg-background-primary border border-border-primary rounded-lg px-3 py-2.5 text-sm font-mono focus:outline-none focus:border-border-info resize-none"
                  placeholder="goose+roam://…  (the host's card)"
                  value={card}
                  onChange={(e) => setCard(e.target.value)}
                />
                <div className="mt-2.5 flex items-center gap-2">
                  {canScan && (
                    <button
                      id="scan-card"
                      type="button"
                      className="inline-flex items-center gap-1.5 text-text-secondary border border-border-secondary rounded-lg px-3 py-1.5 text-xs hover:border-border-info"
                      onClick={() => void startScan()}
                    >
                      <Camera className="w-3.5 h-3.5" /> scan QR
                    </button>
                  )}
                  <span className="flex-1" />
                  <button
                    id="connect-btn"
                    disabled={busy}
                    onClick={() => void connect()}
                    className="bg-background-inverse text-text-inverse text-sm font-medium rounded-lg px-4 py-1.5 hover:brightness-110 disabled:opacity-50"
                  >
                    connect
                  </button>
                </div>
                <details className="mt-5 border-t border-border-primary pt-3">
                  <summary className="text-xs text-text-secondary cursor-pointer select-none">
                    first time? pair this browser with the host
                  </summary>
                  <div className="mt-2.5 text-xs text-text-secondary leading-relaxed">
                    Send this browser's card to the host and accept it once:
                    <div className="flex gap-2 items-center my-2 min-w-0">
                      <code
                        id="my-card"
                        className="flex-1 min-w-0 font-mono text-[11px] bg-background-secondary rounded-lg px-2.5 py-1.5 overflow-hidden text-ellipsis whitespace-nowrap"
                      >
                        {myCard}
                      </code>
                      <button
                        id="copy-card"
                        className="shrink-0 text-text-secondary border border-border-secondary rounded-lg px-2.5 py-1 text-xs hover:border-border-info"
                        onClick={() => navigator.clipboard?.writeText(myCard)}
                      >
                        copy
                      </button>
                      {"share" in navigator && (
                        <button
                          id="share-card"
                          className="shrink-0 text-text-secondary border border-border-secondary rounded-lg px-2.5 py-1 text-xs hover:border-border-info"
                          onClick={() => void navigator.share({ text: myCard }).catch(() => {})}
                        >
                          share
                        </button>
                      )}
                    </div>
                    <code className="block font-mono text-[11px] text-text-info bg-background-secondary rounded-lg px-2.5 py-2 break-all">
                      goose roam peers accept '&lt;card&gt;'
                    </code>
                    <div className="text-[10px] text-text-tertiary mt-1.5">
                      key <code id="my-endpoint-id" className="font-mono break-all">{myId}</code>
                    </div>
                  </div>
                </details>
              </>
            )}
          </div>
          {scanning && (
            <div
              className="fixed inset-0 z-50 bg-black/80 flex flex-col items-center justify-center gap-3"
              onClick={() => scanStop.current?.()}
            >
              <video
                ref={videoRef}
                className="w-[90vw] max-w-[480px] rounded-xl"
                playsInline
                muted
              />
              <div className="text-white text-sm">point at the host QR — tap to cancel</div>
            </div>
          )}
        </section>
      ) : (
        <section id="workspace" className="flex-1 relative md:grid md:grid-cols-[240px_1fr] flex min-h-0">
          <aside
            className={`${sidebarOpen ? "flex" : "hidden"} md:flex absolute md:static inset-y-0 left-0 z-20 w-[240px] shadow-lg md:shadow-none border-r border-border-primary bg-background-secondary p-3 flex-col gap-2.5 min-h-0`}
          >
            <button
              id="new-session"
              disabled={busy}
              onClick={() => { setSidebarOpen(false); void newSession(); }}
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
                  onClick={() => { setSidebarOpen(false); void openSession(s.sessionId); }}
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
            <div className="mt-auto pt-2 border-t border-border-primary text-[10px] text-text-tertiary font-mono truncate">
              {modelName ? `${modelName} · ` : ""}{relay ? `via relay ${relay} · ` : ""}agent {agentId.slice(0, 8)}
            </div>
          </aside>

          <main id="chat" className="flex flex-col min-h-0 flex-1 min-w-0">
            {sessionId === null ? (
              <SessionMatrix
                sessions={sessions}
                selectedId={sessionId}
                onOpen={(id) => void openSession(id)}
                onNew={() => void newSession()}
                busy={busy}
              />
            ) : (
            <>
            <div className="shrink-0 px-3 md:px-6 pt-2 flex items-center gap-2 min-w-0">
              <button
                id="back-to-matrix"
                disabled={busy}
                onClick={() => {
                  sessionRef.current = null;
                  lastSeenUpdate.current = null;
                  setSessionId(null);
                  setItems([]);
                  document.title = "goose remote";
                }}
                className="inline-flex items-center gap-1 text-xs text-text-secondary hover:text-text-primary disabled:opacity-40"
              >
                <ChevronLeft className="w-3.5 h-3.5" /> sessions
              </button>
              <span className="text-xs text-text-tertiary truncate">
                {sessions.find((x) => x.sessionId === sessionId)?.title ?? ""}
              </span>
            </div>
            <div ref={logRef} id="log" className="flex-1 overflow-y-auto px-3 md:px-6 py-4 md:py-5">
              <div className="max-w-3xl mx-auto w-full flex flex-col gap-4 pb-2">
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
                          <div className="body user-message-bubble max-w-[92%] md:max-w-[85%] w-fit min-w-0 bg-text-primary text-background-primary rounded-xl py-2.5 px-4 whitespace-pre-wrap leading-relaxed [overflow-wrap:anywhere]">
                            {it.text}
                          </div>
                        </div>
                      );
                    return (
                      <div key={it.id} className="msg agent goose-message flex w-full md:w-[90%] justify-start min-w-0">
                        <div className="body min-w-0 flex-1 leading-relaxed">
                          <MarkdownContent content={it.text} />
                        </div>
                      </div>
                    );
                  case "tool":
                    return <ToolRow key={it.id} item={it} />;
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
              {busy && (
                <div className="msg system self-center flex items-center gap-2 text-text-secondary text-xs">
                  <span className="inline-block w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
                  goose is working…
                </div>
              )}
              </div>
            </div>
            <form
              id="prompt-form"
              className="px-3 md:px-6 pb-3 pt-1"
              onSubmit={(e) => {
                e.preventDefault();
                void send();
              }}
            >
              <div className="max-w-3xl mx-auto w-full flex gap-2.5 items-end border border-border-primary hover:border-border-secondary focus-within:border-border-secondary rounded-xl bg-background-primary px-1.5 py-1 transition-colors">
              <textarea
                ref={inputRef}
                id="prompt-input"
                rows={1}
                disabled={busy}
                className="flex-1 outline-none border-none focus:ring-0 bg-transparent px-3 pt-2.5 pb-2 text-sm resize-none max-h-52 text-text-primary placeholder:text-text-secondary"
                placeholder="Message goose…"
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
                className="bg-background-inverse text-text-inverse text-sm font-medium rounded-lg px-3.5 py-1.5 mb-0.5 mr-0.5 hover:brightness-110 disabled:opacity-50"
              >
                send
              </button>
              </div>
            </form>
            </>
            )}
          </main>
        </section>
      )}
    </div>
  );
}
