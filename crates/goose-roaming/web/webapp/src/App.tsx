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
import { Camera, ChevronDown, ChevronLeft, ChevronRight, Menu, SlidersHorizontal, Square } from "lucide-react";
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

// goose surfaces session configuration (provider, model, mode, thinking
// effort) via ACP's generic session config options — selects the client
// renders and sets back with session/set_config_option. Keep the raw array
// so the settings panel can render all of them, plus a helper to resolve
// the current model's display name for the header badge.
type ConfigSelectOption = { id: string; name: string; options?: ConfigSelectOption[] };
type ConfigOption = {
  type: string;
  id: string;
  name: string;
  description?: string | null;
  currentValue?: string | boolean;
  options?: ConfigSelectOption[];
};

// "just now", "4m", "2h", "3d" — compact relative time for session rows.
function relativeTime(iso: string | null | undefined): string {
  if (!iso) return "";
  const secs = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000));
  if (secs < 60) return "just now";
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
  return `${Math.floor(secs / 86400)}d ago`;
}

// Same liveness rule the constellation and hot chip use: updated <5 min ago.
function isSessionLive(s: { updatedAt?: string | null }): boolean {
  return !!s.updatedAt && Date.now() - new Date(s.updatedAt).getTime() < 5 * 60 * 1000;
}

// Ticking elapsed readout for the working indicator ("12s", "1m 04s").
function LiveElapsed({ startedAt }: { startedAt: number }) {
  const [, tick] = useState(0);
  useEffect(() => {
    const t = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, []);
  const secs = Math.max(0, Math.floor((Date.now() - startedAt) / 1000));
  const label =
    secs < 60 ? `${secs}s` : `${Math.floor(secs / 60)}m ${String(secs % 60).padStart(2, "0")}s`;
  return (
    <span className="font-mono tabular-nums text-text-tertiary" id="turn-elapsed">
      {label}
    </span>
  );
}

function flatSelectOptions(opt: ConfigOption): ConfigSelectOption[] {
  return (opt.options ?? []).flatMap((x) => (x.options ? x.options : [x]));
}

function modelFromConfigOptions(opts: ConfigOption[] | null | undefined): string | null {
  const opt = opts?.find((o) => o.type === "select" && /model/i.test(o.id));
  if (!opt) return null;
  const current = typeof opt.currentValue === "string" ? opt.currentValue : null;
  return flatSelectOptions(opt).find((x) => x.id === current)?.name ?? current;
}

const HOST_CARD_KEY = "goose-roam-last-host-card";
const HOSTS_KEY = "goose-roam-hosts";

type SavedHost = { name: string; card: string; endpointId: string; lastUsed: number };

// One live roam connection. The tab holds several at once — each saved host
// gets its own iroh duplex + GooseClient, and the session list is the merge.
type HostConn = {
  endpointId: string;
  name: string;
  card: string;
  agent: GooseClient;
  conn: RoamConnection;
  relay: string | null;
};

// A session tagged with the host it lives on. Session ids are only unique
// per host, so every lookup carries (_host, sessionId).
type TaggedSession = SessionInfo & { _host: string };

function groupByHost(sessions: TaggedSession[]): [string, TaggedSession[]][] {
  const order: string[] = [];
  const byHost = new Map<string, TaggedSession[]>();
  for (const s of sessions) {
    const g = byHost.get(s._host);
    if (g) g.push(s);
    else {
      byHost.set(s._host, [s]);
      order.push(s._host);
    }
  }
  return order.map((h) => [h, byHost.get(h) ?? []]);
}

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

// Endpoint id straight out of the card JSON — for keying reconnect attempts
// when the dial itself failed (no RoamConnection to ask).
function cardEndpointHint(cardText: string): string | null {
  try {
    const b64 = cardText.trim().replace(/^goose\+roam:\/\//, "");
    const json = JSON.parse(atob(b64.replace(/-/g, "+").replace(/_/g, "/")));
    return typeof json.endpoint_id === "string" ? json.endpoint_id : null;
  } catch {
    return null;
  }
}
// Last-open session is remembered per host: "<endpointId>|<sessionId>".
const SESSION_KEY = "goose-roam-last-session";

function sessionProjectId(s: SessionInfo): string | null {
  const meta = (s as { _meta?: Record<string, unknown> })._meta;
  const pid = meta?.["projectId"] ?? meta?.["project_id"];
  return typeof pid === "string" && pid ? pid : null;
}

// Bucket sessions under their project (server order — most recent first —
// preserved within each bucket). Projectless sessions form a global "chats"
// group that leads (recency matters most), then projects follow sorted
// alphabetically. Every group folds, including "chats".
// Project titles are looked up per host — slugs are only unique per machine.
type SessionGroup = { key: string; label: string | null; sessions: TaggedSession[] };

function groupSessions(
  sessions: TaggedSession[],
  projects: Record<string, string>,
): SessionGroup[] {
  const loose: TaggedSession[] = [];
  const byProject = new Map<string, TaggedSession[]>();
  for (const s of sessions) {
    const pid = sessionProjectId(s);
    if (!pid) {
      loose.push(s);
    } else {
      const key = `${s._host}:${pid}`;
      const g = byProject.get(key);
      if (g) g.push(s);
      else byProject.set(key, [s]);
    }
  }
  const out: SessionGroup[] = [];
  if (loose.length) out.push({ key: "~", label: "chats", sessions: loose });
  const named = [...byProject.entries()].map(([key, group]) => ({
    key,
    label: projects[key] ?? key.split(":").slice(1).join(":"),
    sessions: group,
  }));
  named.sort((a, b) => a.label.localeCompare(b.label));
  out.push(...named);
  return out;
}

// Collapsed project groups persist per device.
const COLLAPSED_KEY = "goose-roam-collapsed-projects";

function loadCollapsed(): Set<string> {
  try {
    const raw = localStorage.getItem(COLLAPSED_KEY);
    if (raw) return new Set(JSON.parse(raw) as string[]);
  } catch {
    // ignore malformed state
  }
  return new Set();
}

declare const __BUILD_STAMP__: string;
const BUILD = typeof __BUILD_STAMP__ !== "undefined" ? __BUILD_STAMP__ : "dev";

let nextId = 1;

export function App({ roam }: { roam: RoamClient }) {
  const [items, setItems] = useState<Item[]>([]);
  const [status, setStatus] = useState("not connected");
  const [statusKind, setStatusKind] = useState<"idle" | "busy" | "ok" | "err">("idle");
  const [connected, setConnected] = useState(false);
  const [agentId, setAgentId] = useState("");
  const [sessions, setSessions] = useState<TaggedSession[]>([]);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [activeHostId, setActiveHostId] = useState<string | null>(null);
  const [logWindow, setLogWindow] = useState(80);
  const [busy, setBusy] = useState(false);
  // True while re-dialing a known host (boot restore or dropped connection).
  // Shows a quiet "connecting…" screen instead of flashing the hosts page.
  const [reconnecting, setReconnecting] = useState(false);
  // When the current turn started (ours or a steered one) — drives the live
  // elapsed readout in the working indicator, paseo-style.
  const [turnStartedAt, setTurnStartedAt] = useState<number | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [modelName, setModelName] = useState<string | null>(null);
  // Full config option set for the open session (provider/model/mode/…),
  // rendered in the session settings sheet and set back over ACP.
  const [configOptions, setConfigOptions] = useState<ConfigOption[]>([]);
  const [showConfig, setShowConfig] = useState(false);
  const [relay, setRelay] = useState<string | null>(null);
  const [projects, setProjects] = useState<Record<string, string>>({});
  const [collapsed, setCollapsed] = useState<Set<string>>(loadCollapsed);

  const toggleGroup = useCallback((key: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      try {
        localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
      } catch {
        // best effort
      }
      return next;
    });
  }, []);
  const [activeRun, setActiveRun] = useState<string | null>(null);
  const activeRunRef = useRef<string | null>(null);
  // Loop-boundary guard (L0): the open session is advancing but our
  // connection holds no activeRunId — a loop in another process (desktop,
  // CLI, scheduler) is driving it. Watch, but warn before sending: a plain
  // prompt would start a second loop against the same session. The composer
  // stays usable only after an explicit "send anyway".
  const [externalActive, setExternalActive] = useState(false);
  const [externalOverride, setExternalOverride] = useState(false);
  const [card, setCard] = useState("");
  const hostsRef = useRef<Map<string, HostConn>>(new Map());
  const activeHostRef = useRef<string | null>(null);
  const sessionRef = useRef<string | null>(null);
  const streamRole = useRef<"user" | "agent" | "thought" | null>(null);
  const logRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const [scanning, setScanning] = useState(false);
  const [hosts, setHosts] = useState<SavedHost[]>(loadHosts);
  const [addingHost, setAddingHost] = useState(false);
  // Auto-expanded when a dial is rejected as not-paired (e.g. first scan of
  // the desktop QR): the very next thing the user needs is their own card.
  const [pairOpen, setPairOpen] = useState(false);
  // Reopens the connect panel while connected, to dial additional hosts into
  // the same consolidated workspace.
  const [showHosts, setShowHosts] = useState(false);
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

  // Remember where the user was across refreshes: reconnect the last host
  // and reopen the last session. Without the session half, every reload
  // dumped the user back on the front page (issue #10906 feedback).
  const bootTried = useRef(false);
  useEffect(() => {
    if (bootTried.current) return;
    bootTried.current = true;
    // Deep link: the desktop QR encodes <app-url>#card=goose+roam://… so
    // scanning it opens this app with the host already filled in. The card
    // rides the fragment (never sent to the server) and is scrubbed from the
    // address bar immediately.
    const hash = window.location.hash;
    if (hash.includes("goose+roam://")) {
      const linked = decodeURIComponent(
        hash.slice(hash.indexOf("goose+roam://")),
      ).trim();
      window.history.replaceState(null, "", window.location.pathname + window.location.search);
      setCard(linked);
      setAddingHost(true);
      // Try connecting right away: if this browser is already paired we land
      // in sessions; if not, the host rejects us and the connect panel (with
      // the "first time? pair this browser" card) is already on screen.
      void connect(linked);
      return;
    }
    const saved = localStorage.getItem(HOST_CARD_KEY);
    if (!saved) return;
    setCard(saved);
    // Known host: this is a resume, not a first visit — show "connecting"
    // instead of flashing the hosts page while the dial lands.
    setReconnecting(true);
    const last = localStorage.getItem(SESSION_KEY);
    if (last) {
      const [hostId, sessionId] = last.split("|");
      if (hostId && sessionId && cardEndpointHint(saved) === hostId) {
        // connect() resumes this session once the dial lands (same path a
        // mid-conversation reconnect takes).
        resumeAfterDrop.current = sessionId;
      }
    }
    void connect(saved);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // During loadSession the server replays the entire history as individual
  // notifications; applying each via setState re-renders hundreds of times
  // (the "chonky open"). While replayBuf is set, handlers mutate it
  // synchronously instead, and openSession flushes once.
  const replayBuf = useRef<Item[] | null>(null);
  const mutateItems = useCallback((fn: (xs: Item[]) => Item[]) => {
    if (replayBuf.current) {
      replayBuf.current = fn(replayBuf.current);
    } else {
      setItems(fn);
    }
  }, []);

  const push = useCallback((item: Omit<Item, "id">) => {
    streamRole.current = null;
    mutateItems((xs) => [...xs, { ...item, id: nextId++ } as Item]);
  }, [mutateItems]);

  const chunk = useCallback((role: "user" | "agent" | "thought", text: string) => {
    mutateItems((xs) => {
      const last = xs[xs.length - 1];
      if (streamRole.current === role && last?.kind === "msg" && last.role === role) {
        return [...xs.slice(0, -1), { ...last, text: last.text + text }];
      }
      streamRole.current = role;
      return [...xs, { kind: "msg", id: nextId++, role, text }];
    });
  }, [mutateItems]);

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
            mutateItems((xs) => [
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
            mutateItems((xs) =>
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
            const opts = ((u as { configOptions?: unknown[] }).configOptions ?? []) as ConfigOption[];
            setConfigOptions(opts);
            const m = modelFromConfigOptions(opts);
            if (m) setModelName(m);
            break;
          }
          case "session_info_update": {
            // The server advertises the active prompt run (or its end) via
            // _meta.goose.activeRunId; this is the real "agent is running"
            // signal, and it tells send() to steer instead of prompt.
            const meta = (u as { _meta?: { goose?: { activeRunId?: string | null } } })._meta;
            if (meta?.goose && "activeRunId" in meta.goose) {
              const run = meta.goose.activeRunId ?? null;
              activeRunRef.current = run;
              setActiveRun(run);
            }
            break;
          }
          case "plan": {
            streamRole.current = null;
            const entries = u.entries;
            mutateItems((xs) => {
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
  }, [chunk, mutateItems]);

  // The host whose session is open (or the only/last-connected one). Chat
  // actions go here; list actions fan out over all hosts.
  const activeAgent = useCallback((): GooseClient | null => {
    const hid = activeHostRef.current;
    if (hid) return hostsRef.current.get(hid)?.agent ?? null;
    const first = hostsRef.current.values().next();
    return first.done ? null : first.value.agent;
  }, []);

  // Consolidated list: every connected host contributes its sessions, tagged
  // with the host they live on. One slow host can't hide the others.
  const refreshSessions = useCallback(async () => {
    const conns = [...hostsRef.current.values()];
    if (conns.length === 0) return;
    const lists = await Promise.all(
      conns.map(async (h) => {
        try {
          const res = await h.agent.listSessions({});
          return (res.sessions ?? []).map((s) => ({ ...s, _host: h.endpointId }));
        } catch (err) {
          console.warn(`listSessions failed for ${h.name}:`, err);
          return [] as TaggedSession[];
        }
      }),
    );
    setSessions(lists.flat());
  }, []);

  // Projects are ordinary ACP: sources/list with type "project" returns
  // slug + title; sessions carry projectId in _meta. Keyed "<host>:<slug>"
  // because slugs are only unique per machine.
  const refreshProjects = useCallback(async () => {
    const conns = [...hostsRef.current.values()];
    if (conns.length === 0) return;
    const maps = await Promise.all(
      conns.map(async (h) => {
        try {
          const res = (await h.agent.extMethod("_goose/unstable/sources/list", {
            type: "project",
          })) as { sources?: { name: string; properties?: Record<string, unknown> }[] };
          const map: Record<string, string> = {};
          for (const s of res.sources ?? []) {
            const title = s.properties?.["title"];
            map[`${h.endpointId}:${s.name}`] =
              typeof title === "string" && title ? title : s.name;
          }
          return map;
        } catch (err) {
          console.warn(`sources/list failed for ${h.name}:`, err);
          return {};
        }
      }),
    );
    setProjects(Object.assign({}, ...maps));
  }, []);

  const newSession = useCallback(async (hostId?: string) => {
    const hid = hostId ?? activeHostRef.current ?? hostsRef.current.keys().next().value ?? null;
    const host = hid ? hostsRef.current.get(hid) : null;
    if (!host) return;
    setBusy(true);
    try {
      const res = await host.agent.newSession({ cwd: "/", mcpServers: [] });
      const opts = ((res as { configOptions?: unknown[] }).configOptions ?? []) as ConfigOption[];
      setConfigOptions(opts);
      const m = modelFromConfigOptions(opts);
      if (m) setModelName(m);
      localStorage.setItem(SESSION_KEY, `${host.endpointId}|${res.sessionId}`);
      activeHostRef.current = host.endpointId;
      setActiveHostId(host.endpointId);
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
    async (hostId: string, id: string, force = false) => {
      const host = hostsRef.current.get(hostId);
      if (!host) return;
      if (id === sessionRef.current && hostId === activeHostRef.current && !force) return;
      setBusy(true);
      setStatus("loading session…");
      setStatusKind("busy");
      try {
        setItems([]);
        setLogWindow(80);
        activeRunRef.current = null;
        setActiveRun(null);
        setExternalActive(false);
        setExternalOverride(false);
        setConfigOptions([]);
        setShowConfig(false);
        const info = sessions.find((x) => x._host === hostId && x.sessionId === id);
        document.title = info?.title ? `${info.title} · goose remote` : "goose remote";
        lastSeenUpdate.current = null;
        localStorage.setItem(SESSION_KEY, `${hostId}|${id}`);
        activeHostRef.current = hostId;
        setActiveHostId(hostId);
        sessionRef.current = id;
        setSessionId(id);
        replayBuf.current = [];
        await host.agent.loadSession({ sessionId: id, cwd: "/", mcpServers: [] });
        const buf = replayBuf.current ?? [];
        replayBuf.current = null;
        setItems(buf);
        streamRole.current = null;
        setStatus("connected");
        setStatusKind("ok");
      } catch (err) {
        push({ kind: "system", text: `could not load session: ${err}` } as Omit<Item, "id">);
        setStatus("connected");
        setStatusKind("ok");
      } finally {
        replayBuf.current = null;
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
  //
  // The same signal powers the loop-boundary guard: a session advancing while
  // our connection holds no activeRunId means a loop we cannot see is driving
  // it (another process on the host — desktop, CLI, scheduler). We can watch
  // but not steer that loop, and a plain prompt would start a second one, so
  // the composer warns and asks for an explicit "send anyway".
  const lastSeenUpdate = useRef<string | null>(null);
  useEffect(() => {
    if (!connected) return;
    const t = setInterval(async () => {
      if (hostsRef.current.size === 0 || busy) return;
      try {
        await refreshSessions();
        const sid = sessionRef.current;
        const hid = activeHostRef.current;
        if (!sid || !hid) return;
        const host = hostsRef.current.get(hid);
        if (!host) return;
        const res = await host.agent.listSessions({});
        const mine = (res.sessions ?? []).find((x) => x.sessionId === sid);
        const stamp = (mine as { updatedAt?: string } | undefined)?.updatedAt ?? null;
        const advanced =
          stamp !== null && lastSeenUpdate.current !== null && stamp !== lastSeenUpdate.current;
        setExternalActive(advanced && !activeRunRef.current);
        if (advanced) {
          await openSession(hid, sid, true);
        }
        if (stamp) lastSeenUpdate.current = stamp;
      } catch {
        // transient; next tick
      }
    }, 6000);
    return () => clearInterval(t);
  }, [connected, busy, openSession, refreshSessions]);


  // Dial one host and add it to the live set. The tab holds a connection per
  // saved host — the session list is the merge — so connecting a second host
  // extends the workspace rather than replacing it.
  const reconnectAttempts = useRef<Map<string, number>>(new Map());
  const connect = useCallback(async (cardText?: string) => {
    const text = (cardText ?? card).trim();
    if (!text) return;
    setStatus("dialing host over relay…");
    setStatusKind("busy");
    setBusy(true);
    try {
      const conn = await roam.connect(text, "web");
      const eid = conn.agentId();
      const bytes = roamByteStreams(conn);
      const stream = ndJsonStream(bytes.writable, bytes.readable);
      const agent = new GooseClient(() => makeClient(), stream);
      await agent.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
      });
      localStorage.setItem(HOST_CARD_KEY, text);
      const prior = loadHosts().find((h) => h.endpointId === eid);
      const name = hostName.trim() || prior?.name || `host ${eid.slice(0, 8)}`;
      const entry: HostConn = { endpointId: eid, name, card: text, agent, conn, relay: relayRegion(text) };
      hostsRef.current.set(eid, entry);
      if (!activeHostRef.current) {
        activeHostRef.current = eid;
        setActiveHostId(eid);
      }
      // Per-host drop watch: reconnect this host with backoff. Other hosts'
      // connections are untouched; if the open session lived here, resume it
      // once the redial lands.
      void agent.closed.then(() => {
        if (hostsRef.current.get(eid)?.agent !== agent) return;
        hostsRef.current.delete(eid);
        if (activeHostRef.current === eid) {
          resumeAfterDrop.current = sessionRef.current;
          sessionRef.current = null;
        }
        if (hostsRef.current.size === 0) setConnected(false);
        setBusy(false);
        setSessions((xs) => xs.filter((s) => s._host !== eid));
        const attempt = reconnectAttempts.current.get(eid) ?? 0;
        reconnectAttempts.current.set(eid, attempt + 1);
        if (attempt < 8) {
          const delay = Math.min(20000, 1500 * 2 ** attempt);
          if (hostsRef.current.size === 0) setReconnecting(true);
          setStatus(`${name}: connection lost — reconnecting…`);
          setStatusKind("err");
          setTimeout(() => void connect(text), delay);
        } else {
          setReconnecting(false);
          setStatus(`${name}: connection lost — press connect`);
          setStatusKind("err");
        }
      });
      reconnectAttempts.current.delete(eid);
      setRelay(entry.relay);
      {
        const next = loadHosts().filter((h) => h.endpointId !== eid);
        next.unshift({ name, card: text, endpointId: eid, lastUsed: Date.now() });
        localStorage.setItem(HOSTS_KEY, JSON.stringify(next.slice(0, 12)));
        setHosts(next.slice(0, 12));
        setHostName("");
        setAddingHost(false);
        setShowHosts(false);
      }
      setAgentId(eid);
      setConnected(true);
      setReconnecting(false);
      setStatus("connected");
      setStatusKind("ok");
      await refreshSessions();
      void refreshProjects();
      const resume = resumeAfterDrop.current;
      resumeAfterDrop.current = null;
      if (resume && activeHostRef.current === eid) {
        // recovering from a dropped connection mid-conversation: go back to it
        await openSession(eid, resume, true);
        inputRef.current?.focus();
      }
      // otherwise land on the session matrix (front page)
      setBusy(false);
    } catch (err) {
      console.error(err);
      setBusy(false);
      const eid = cardEndpointHint(text);
      const attempt = eid ? (reconnectAttempts.current.get(eid) ?? 0) : 0;
      if (eid && attempt > 0 && attempt < 8) {
        const delay = Math.min(20000, 1500 * 2 ** attempt);
        reconnectAttempts.current.set(eid, attempt + 1);
        setStatus("reconnecting…");
        setStatusKind("err");
        setTimeout(() => void connect(text), delay);
      } else {
        // Out of retries (or a first-time connect failed): fall back to the
        // hosts page so the user can act.
        setReconnecting(false);
        setStatus(`connect failed: ${err}`);
        setStatusKind("err");
        // Not paired yet (fresh QR scan): open the pairing card — the user's
        // next step is sending their own card to the host.
        if (String(err).includes("not_allowlisted")) {
          setAddingHost(true);
          setPairOpen(true);
        }
      }
    }
  }, [card, hostName, roam, makeClient, refreshSessions, refreshProjects, openSession]);

  // A send during an active run is a steer: the message is queued into the
  // running loop rather than starting a second one. Two ways to know a run is
  // live: our own connection saw activeRunId (session_info_update), or a plain
  // prompt bounces with "already has active run `<id>`" — e.g. a loop started
  // by another device on this same share process.
  // Change a session config option (provider/model/mode/…) over ACP. The
  // response carries the full refreshed option set — mirror it locally so the
  // sheet and the model badge update immediately.
  const setConfigOption = useCallback(async (configId: string, value: string) => {
    const agent = activeAgent();
    const sid = sessionRef.current;
    if (!agent || !sid) return;
    try {
      const res = await agent.setSessionConfigOption({
        sessionId: sid,
        configId,
        value,
      });
      const opts = (res.configOptions ?? []) as ConfigOption[];
      setConfigOptions(opts);
      const m = modelFromConfigOptions(opts);
      if (m) setModelName(m);
    } catch (err) {
      push({ kind: "system", text: `could not set ${configId}: ${err}` } as Omit<Item, "id">);
    }
  }, [activeAgent, push]);

  const steer = useCallback(async (sid: string, text: string, runId: string) => {
    const agent = activeAgent();
    if (!agent) return;
    await agent.extMethod("_goose/unstable/session/steer", {
      sessionId: sid,
      prompt: [{ type: "text", text }],
      expectedRunId: runId,
    });
  }, [activeAgent]);

  const send = useCallback(async () => {
    const agent = activeAgent();
    const sid = sessionRef.current;
    const el = inputRef.current;
    const text = el?.value.trim();
    if (!agent || !sid || !text) return;
    // Loop-boundary guard: a foreign process is driving this session and the
    // user hasn't explicitly overridden — don't start a second loop.
    if (externalActive && !externalOverride) return;
    const runId = activeRunRef.current;
    if (busy && !runId) return;

    if (runId) {
      if (el) el.value = "";
      try {
        await steer(sid, text, runId);
        // pickup echoes back as a user_message_chunk; no local append needed
      } catch (err) {
        push({ kind: "system", text: `steer failed: ${err}` } as Omit<Item, "id">);
      }
      inputRef.current?.focus();
      return;
    }

    if (el) el.value = "";
    streamRole.current = null;
    setItems((xs) => [...xs, { kind: "msg", id: nextId++, role: "user", text }]);
    streamRole.current = null;
    setBusy(true);
    setTurnStartedAt(Date.now());
    setCancelling(false);
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
      const bounce = String(err).match(/already has active run `([^`]+)`/);
      if (bounce) {
        // A loop driven elsewhere owns this session — queue into it instead.
        try {
          await steer(sid, text, bounce[1]);
          push({ kind: "system", text: "queued into the running turn" } as Omit<Item, "id">);
        } catch (err2) {
          push({ kind: "system", text: `steer failed: ${err2}` } as Omit<Item, "id">);
        }
      } else {
        push({ kind: "system", text: `error: ${err}` } as Omit<Item, "id">);
      }
    } finally {
      activeRunRef.current = null;
      setActiveRun(null);
      setBusy(false);
      setTurnStartedAt(null);
      setCancelling(false);
      setStatus("connected");
      setStatusKind("ok");
      inputRef.current?.focus();
    }
  }, [busy, push, refreshSessions, steer, activeAgent, externalActive, externalOverride]);

  // Stop the running turn. ACP cancel is a notification: the prompt call
  // itself returns (stopReason: cancelled), which runs send()'s cleanup.
  const cancelTurn = useCallback(async () => {
    const agent = activeAgent();
    const sid = sessionRef.current;
    if (!agent || !sid) return;
    setCancelling(true);
    try {
      await agent.cancel({ sessionId: sid });
    } catch {
      setCancelling(false);
    }
  }, [activeAgent]);

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
              title="connect more hosts — sessions from every connected host share one list"
              onClick={() => setShowHosts((v) => !v)}
            >
              hosts{hostsRef.current.size > 1 ? ` · ${hostsRef.current.size}` : ""}
            </button>
          )}
        </div>
      </div>

      {!connected && reconnecting ? (
        // Re-dialing a known host (boot restore / dropped connection): a
        // quiet holding screen instead of flashing the hosts page.
        <section id="reconnect-panel" className="flex-1 grid place-items-center p-6">
          <div className="flex flex-col items-center gap-3 text-text-secondary">
            <span className="inline-block w-2 h-2 rounded-full bg-blue-400 animate-pulse" />
            <div className="text-sm">connecting…</div>
            <button
              className="text-xs underline underline-offset-2 hover:text-text-primary"
              onClick={() => setReconnecting(false)}
            >
              choose a different host
            </button>
          </div>
        </section>
      ) : !connected || showHosts ? (
        <section id="connect-panel" className="flex-1 grid place-items-center p-3 md:p-6 overflow-auto">
          <div className="w-full max-w-[480px] min-w-0 overflow-hidden bg-background-primary border rounded-xl shadow-sm p-5 md:p-7">
            {hosts.length > 0 && !addingHost ? (
              <>
                <div className="flex items-center gap-2 mb-1">
                  {connected && (
                    <button
                      aria-label="back to sessions"
                      className="text-text-secondary hover:text-text-primary -ml-1 p-0.5"
                      onClick={() => setShowHosts(false)}
                    >
                      <ChevronLeft className="w-4 h-4" />
                    </button>
                  )}
                  <h2 className="text-lg font-semibold">Your hosts</h2>
                </div>
                <p className="text-xs text-text-tertiary mb-4">
                  {connected
                    ? "connected hosts share one session list — tap to add another"
                    : "tap to connect"}
                </p>
                <div className="flex flex-col gap-1.5">
                  {hosts.map((h) => {
                    const live = hostsRef.current.has(h.endpointId);
                    return (
                    <button
                      key={h.endpointId}
                      className="host-row w-full text-left rounded-lg px-3 py-2.5 hover:bg-background-secondary hover:shadow-default transition-all flex items-center gap-3 disabled:opacity-50"
                      disabled={busy || live}
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
                      {live ? (
                        <span className="shrink-0 inline-flex items-center gap-1 text-[10px] text-text-success">
                          <span className="inline-block w-1.5 h-1.5 rounded-full bg-current" />
                          connected
                        </span>
                      ) : (
                        <ChevronRight className="w-4 h-4 shrink-0 text-text-tertiary" />
                      )}
                    </button>
                  );})}
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
                  <h2 className="text-lg font-semibold">
                    {hosts.length === 0 ? "goose remote" : "Add a host"}
                  </h2>
                </div>
                {hosts.length === 0 ? (
                  <div className="text-xs text-text-secondary leading-relaxed mb-4 space-y-1.5">
                    <p>
                      Chat with a goose agent on your own machine, from this browser —
                      peer to peer, no accounts, nothing in between.
                    </p>
                    <ol className="list-decimal ml-4 space-y-1 text-text-tertiary">
                      <li>
                        on your machine: <code className="font-mono">goose roam share --qr</code>
                      </li>
                      <li>scan or paste its card below</li>
                      <li>
                        accept this browser once:{" "}
                        <code className="font-mono">goose roam pair</code> (or{" "}
                        <code className="font-mono">peers accept</code>) with the card under
                        “first time?”
                      </li>
                    </ol>
                  </div>
                ) : (
                  <p className="text-xs text-text-tertiary mb-4">
                    a machine running <code className="font-mono">goose roam share</code>
                  </p>
                )}
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
                <details
                  className="mt-5 border-t border-border-primary pt-3"
                  open={pairOpen}
                  onToggle={(e) => setPairOpen((e.target as HTMLDetailsElement).open)}
                >
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
              {groupSessions(sessions, projects).map(({ key, label, sessions: group }) => (
                <div key={key} className="flex flex-col gap-1">
                  {label !== null && (
                    <button
                      className="project-label flex items-center gap-1 text-[10px] uppercase tracking-wide text-text-tertiary px-2.5 pt-2 hover:text-text-secondary"
                      onClick={() => toggleGroup(key)}
                      aria-expanded={!collapsed.has(key)}
                    >
                      <ChevronDown
                        size={12}
                        className={`shrink-0 transition-transform duration-150 ${collapsed.has(key) ? "-rotate-90" : ""}`}
                      />
                      <span className="truncate">{label}</span>
                      <span className="font-normal normal-case tracking-normal">({group.length})</span>
                    </button>
                  )}
                  {(label === null || !collapsed.has(key)) && group.map((s) => (
                    <button
                      key={`${s._host}|${s.sessionId}`}
                      className={`session-item text-left rounded-lg px-2.5 py-2 transition-all duration-150 ${
                        s.sessionId === sessionId && s._host === activeHostId
                          ? "bg-background-tertiary"
                          : "hover:bg-background-secondary hover:shadow-default"
                      }`}
                      onClick={() => { setSidebarOpen(false); void openSession(s._host, s.sessionId); }}
                    >
                      <div className="flex items-center gap-1.5 min-w-0">
                        {isSessionLive(s) && (
                          <span
                            className="shrink-0 inline-block w-1.5 h-1.5 rounded-full bg-text-info animate-pulse"
                            title="active in the last 5 minutes"
                          />
                        )}
                        <span className="text-[13px] whitespace-nowrap overflow-hidden text-ellipsis">
                          {s.title || "(untitled session)"}
                        </span>
                      </div>
                      <div className="text-[11px] text-text-tertiary mt-0.5">
                        {relativeTime(s.updatedAt)}
                        {hostsRef.current.size > 1 && (
                          <span className="text-text-info"> · {hostsRef.current.get(s._host)?.name ?? s._host.slice(0, 6)}</span>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
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
                onOpen={(s) => { if (s._host) void openSession(s._host, s.sessionId); }}
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
                  setExternalActive(false);
                  setExternalOverride(false);
                  document.title = "goose remote";
                }}
                className="inline-flex items-center gap-1 text-xs text-text-secondary hover:text-text-primary disabled:opacity-40"
              >
                <ChevronLeft className="w-3.5 h-3.5" /> sessions
              </button>
              <span className="text-xs text-text-tertiary truncate">
                {sessions.find((x) => x._host === activeHostId && x.sessionId === sessionId)?.title ?? ""}
                {hostsRef.current.size > 1 && activeHostId && (
                  <span className="text-text-info"> · {hostsRef.current.get(activeHostId)?.name ?? ""}</span>
                )}
              </span>
              {(() => {
                const cur = sessions.find((x) => x._host === activeHostId && x.sessionId === sessionId);
                const hot =
                  cur?.updatedAt &&
                  Date.now() - new Date(cur.updatedAt).getTime() < 5 * 60 * 1000;
                return hot ? (
                  <span
                    id="hot-chip"
                    title="active in the last 5 min — checked every 6s"
                    className="shrink-0 inline-flex items-center gap-1 text-[10px] text-text-info"
                  >
                    <span className="inline-block w-1.5 h-1.5 rounded-full bg-current animate-pulse" />
                    live
                  </span>
                ) : null;
              })()}
              <span className="flex-1" />
              {configOptions.length > 0 && (
                <button
                  id="session-config-btn"
                  aria-label="session settings"
                  title="session settings — model, mode, provider"
                  className={`shrink-0 p-1 rounded-md transition-colors ${
                    showConfig
                      ? "text-text-primary bg-background-tertiary"
                      : "text-text-secondary hover:text-text-primary"
                  }`}
                  onClick={() => setShowConfig((v) => !v)}
                >
                  <SlidersHorizontal className="w-3.5 h-3.5" />
                </button>
              )}
            </div>
            {showConfig && (
              <div
                id="session-config"
                className="shrink-0 border-b border-border-primary bg-background-secondary px-3 md:px-6 py-3"
              >
                <div className="max-w-3xl mx-auto w-full grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-2.5">
                  {configOptions
                    .filter((o) => o.type === "select")
                    .map((o) => (
                      <label key={o.id} className="flex flex-col gap-1 min-w-0">
                        <span className="text-[10px] uppercase tracking-wide text-text-tertiary">
                          {o.name}
                        </span>
                        <select
                          className="w-full bg-background-primary border border-border-primary rounded-lg px-2 py-1.5 text-xs text-text-primary focus:outline-none focus:border-border-info disabled:opacity-50"
                          value={typeof o.currentValue === "string" ? o.currentValue : ""}
                          disabled={busy && !activeRun}
                          onChange={(e) => void setConfigOption(o.id, e.target.value)}
                        >
                          {flatSelectOptions(o).map((v) => (
                            <option key={v.id} value={v.id}>
                              {v.name}
                            </option>
                          ))}
                        </select>
                      </label>
                    ))}
                </div>
              </div>
            )}
            <div ref={logRef} id="log" className="flex-1 overflow-y-auto px-3 md:px-6 py-4 md:py-5">
              <div className="max-w-3xl mx-auto w-full flex flex-col gap-4 pb-2">
              {items.length > logWindow && (
                <button
                  id="show-earlier"
                  className="self-center text-xs text-text-secondary border border-border-secondary rounded-lg px-3 py-1.5 hover:border-border-info"
                  onClick={() => {
                    const el = logRef.current;
                    const prevH = el?.scrollHeight ?? 0;
                    setLogWindow((w) => w + 200);
                    requestAnimationFrame(() => {
                      if (el) el.scrollTop += el.scrollHeight - prevH;
                    });
                  }}
                >
                  show earlier · {items.length - logWindow} more
                </button>
              )}
              {items.slice(-logWindow).map((it) => {
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
                <div className="msg system self-center flex items-center gap-2.5 text-text-secondary text-xs">
                  <span className="inline-block w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
                  <span>{activeRun ? "goose is working — you can steer" : "goose is working…"}</span>
                  {turnStartedAt && <LiveElapsed startedAt={turnStartedAt} />}
                  {turnStartedAt && (
                    <button
                      id="stop-turn"
                      type="button"
                      disabled={cancelling}
                      onClick={() => void cancelTurn()}
                      className="inline-flex items-center gap-1 border border-border-secondary rounded-md px-2 py-0.5 text-[11px] hover:border-border-warning hover:text-text-warning transition-colors disabled:opacity-50"
                    >
                      <Square className="w-2.5 h-2.5 fill-current" />
                      {cancelling ? "stopping…" : "stop"}
                    </button>
                  )}
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
              {externalActive && !externalOverride && (
                <div
                  id="external-run-guard"
                  className="max-w-3xl mx-auto w-full mb-1.5 flex items-center gap-2 text-[11px] text-text-warning border border-border-warning rounded-lg bg-background-secondary px-3 py-1.5"
                >
                  <span className="inline-block w-1.5 h-1.5 rounded-full bg-current animate-pulse shrink-0" />
                  <span className="flex-1 min-w-0">
                    active in another goose (desktop or CLI) — sending from here may start a second loop
                  </span>
                  <button
                    id="external-run-override"
                    type="button"
                    className="shrink-0 text-[11px] underline underline-offset-2 hover:text-text-primary"
                    onClick={() => setExternalOverride(true)}
                  >
                    send anyway
                  </button>
                </div>
              )}
              <div className="max-w-3xl mx-auto w-full flex gap-2.5 items-end border border-border-primary hover:border-border-secondary focus-within:border-border-secondary rounded-xl bg-background-primary px-1.5 py-1 transition-colors">
              <textarea
                ref={inputRef}
                id="prompt-input"
                rows={1}
                disabled={(busy && !activeRun) || (externalActive && !externalOverride)}
                className="flex-1 outline-none border-none focus:ring-0 bg-transparent px-3 pt-2.5 pb-2 text-sm resize-none max-h-52 text-text-primary placeholder:text-text-secondary"
                placeholder={
                  externalActive && !externalOverride
                    ? "Running in another goose — watching…"
                    : activeRun
                      ? "Steer the running turn…"
                      : "Message goose…"
                }
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
                disabled={(busy && !activeRun) || (externalActive && !externalOverride)}
                className="bg-background-inverse text-text-inverse text-sm font-medium rounded-lg px-3.5 py-1.5 mb-0.5 mr-0.5 hover:brightness-110 disabled:opacity-50"
              >
                {activeRun ? "steer" : "send"}
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
