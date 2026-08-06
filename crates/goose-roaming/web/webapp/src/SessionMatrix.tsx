// Session constellation — a port of the goose-mobile iOS NodeMatrix view.
// Sessions are dots (size = message count) placed at deterministic
// pseudo-random positions seeded from the session id, threaded in
// chronological order by a faint polyline. One day per screen, chevrons to
// page. Live sessions (updated < 5 min) pulse. Tap a dot to open it.
import { useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, Plus } from "lucide-react";

export type MatrixSession = {
  sessionId: string;
  title?: string | null;
  updatedAt?: string | null;
  _meta?: Record<string, unknown> | null;
};

function hashId(str: string): number {
  let h = 0;
  for (let i = 0; i < str.length; i++) h = (h * 31 + str.charCodeAt(i)) | 0;
  return Math.abs(h);
}

// Mirrors NodeMatrix.swift: sin/cos of the hashed id, clamped into the field.
function nodePos(id: string): { x: number; y: number } {
  const seed = hashId(id);
  const r1 = Math.sin(seed);
  const r2 = Math.cos(seed * 1.5);
  const r3 = Math.sin(seed * 2.3);
  const x = 50 + r1 * 38 + r3 * 3;
  const y = 50 + r2 * 33 + Math.sin(r3 * 3) * 3;
  return { x: Math.min(92, Math.max(8, x)), y: Math.min(88, Math.max(12, y)) };
}

function messageCount(s: MatrixSession): number {
  const n = s._meta?.["message_count"];
  return typeof n === "number" ? n : 0;
}

function isLive(s: MatrixSession): boolean {
  if (!s.updatedAt) return false;
  return Date.now() - new Date(s.updatedAt).getTime() < 5 * 60 * 1000;
}

export function SessionMatrix({
  sessions,
  selectedId,
  onOpen,
  onNew,
  busy,
}: {
  sessions: MatrixSession[];
  selectedId: string | null;
  onOpen: (id: string) => void;
  onNew: () => void;
  busy: boolean;
}) {
  const [daysOffset, setDaysOffset] = useState(0);

  const { label, daySessions } = useMemo(() => {
    const target = new Date();
    target.setHours(0, 0, 0, 0);
    target.setDate(target.getDate() - daysOffset);
    const next = new Date(target);
    next.setDate(next.getDate() + 1);
    const filtered = sessions
      .filter((s) => {
        if (!s.updatedAt) return false;
        const d = new Date(s.updatedAt);
        return d >= target && d < next;
      })
      .sort(
        (a, b) =>
          new Date(a.updatedAt ?? 0).getTime() - new Date(b.updatedAt ?? 0).getTime(),
      )
      .slice(-30);
    const lbl =
      daysOffset === 0
        ? "Today"
        : daysOffset === 1
          ? "Yesterday"
          : target.toLocaleDateString(undefined, { month: "short", day: "numeric" });
    return { label: lbl, daySessions: filtered };
  }, [sessions, daysOffset]);

  const points = daySessions.map((s) => ({ s, ...nodePos(s.sessionId) }));

  return (
    <div id="session-matrix" className="flex-1 flex flex-col min-h-0">
      <div className="flex items-center justify-center gap-3 pt-3 shrink-0">
        <button
          aria-label="previous day"
          className="text-text-secondary hover:text-text-primary disabled:opacity-30 p-1"
          disabled={daysOffset >= 60}
          onClick={() => setDaysOffset((d) => Math.min(60, d + 1))}
        >
          <ChevronLeft className="w-4 h-4" />
        </button>
        <span className="text-sm text-text-secondary min-w-[90px] text-center">
          {label}
          <span className="text-text-tertiary"> · {daySessions.length}</span>
        </span>
        <button
          aria-label="next day"
          className="text-text-secondary hover:text-text-primary disabled:opacity-30 p-1"
          disabled={daysOffset === 0}
          onClick={() => setDaysOffset((d) => Math.max(0, d - 1))}
        >
          <ChevronRight className="w-4 h-4" />
        </button>
      </div>

      <div className="relative flex-1 min-h-0 mx-3 md:mx-8">
        {points.length > 1 && (
          <svg
            className="absolute inset-0 w-full h-full pointer-events-none"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
          >
            <polyline
              points={points.map((p) => `${p.x},${p.y}`).join(" ")}
              fill="none"
              stroke="var(--color-border-secondary)"
              strokeOpacity="0.7"
              strokeWidth="1"
              vectorEffect="non-scaling-stroke"
              strokeLinecap="round"
            />
          </svg>
        )}

        {points.map(({ s, x, y }) => {
          const count = messageCount(s);
          const size = 10 + Math.min(1, count / 60) * 8;
          const selected = s.sessionId === selectedId;
          const live = isLive(s);
          return (
            <button
              key={s.sessionId}
              className="matrix-node absolute w-9 h-9 -translate-x-1/2 -translate-y-1/2 grid place-items-center group"
              style={{ left: `${x}%`, top: `${y}%` }}
              title={s.title ?? s.sessionId}
              aria-label={s.title ?? s.sessionId}
              onClick={() => onOpen(s.sessionId)}
            >
              {live && (
                <span
                  className="absolute rounded-full bg-text-info opacity-40 animate-ping"
                  style={{ width: size + 6, height: size + 6 }}
                />
              )}
              <span
                className={`rounded-full transition-transform group-hover:scale-125 ${
                  selected
                    ? "bg-background-inverse ring-2 ring-border-info"
                    : live
                      ? "bg-text-info"
                      : "bg-text-secondary group-hover:bg-text-primary"
                }`}
                style={{ width: size, height: size }}
              />
              <span className="pointer-events-none absolute top-full mt-1 max-w-[130px] truncate text-[10px] text-text-tertiary opacity-0 group-hover:opacity-100 transition-opacity">
                {s.title ?? ""}
              </span>
            </button>
          );
        })}

        {points.length === 0 && (
          <div className="absolute inset-0 grid place-items-center">
            <div className="text-center text-sm text-text-secondary">
              No sessions {label === "Today" ? "yet today" : `on ${label.toLowerCase()}`}.
              <div className="text-xs text-text-tertiary mt-1">
                Start one below — it becomes a dot here.
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="shrink-0 pb-4 pt-1 grid place-items-center">
        <button
          id="matrix-new-session"
          disabled={busy}
          onClick={onNew}
          className="inline-flex items-center gap-1.5 bg-background-inverse text-text-inverse text-sm font-medium rounded-lg px-4 py-2 hover:brightness-110 disabled:opacity-50"
        >
          <Plus className="w-4 h-4" /> new session
        </button>
      </div>
    </div>
  );
}
