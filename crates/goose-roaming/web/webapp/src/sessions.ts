// Session tagging and project grouping for the consolidated multi-host list.
import type { SessionInfo } from "@agentclientprotocol/sdk";

// A session tagged with the host it lives on. Session ids are only unique
// per host, so every lookup carries (_host, sessionId).
export type TaggedSession = SessionInfo & { _host: string };

export function sessionProjectId(s: SessionInfo): string | null {
  const meta = (s as { _meta?: Record<string, unknown> })._meta;
  const pid = meta?.["projectId"] ?? meta?.["project_id"];
  return typeof pid === "string" && pid ? pid : null;
}

// Bucket sessions under their project (server order — most recent first —
// preserved within each bucket). Projectless sessions form a global "chats"
// group that leads (recency matters most), then projects follow sorted
// alphabetically. Every group folds, including "chats".
// Project titles are looked up per host — slugs are only unique per machine.
export type SessionGroup = { key: string; label: string | null; sessions: TaggedSession[] };

export function groupSessions(
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
