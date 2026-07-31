// Chat log renderer: turns ACP session updates into a real chat UI —
// markdown agent messages, collapsible thinking blocks, tool-call widgets that
// update in place, and a plan checklist. Bound to one log element.
//
// All agent text goes through the XSS-safe markdown renderer (textContent-only,
// never innerHTML), so untrusted agent output can't inject markup.
import { renderMarkdown } from "./markdown.js";
import type {
  ContentBlock,
  ToolCall,
  ToolCallUpdate,
  ToolCallContent,
  PlanEntry,
} from "@agentclientprotocol/sdk";

type TurnRole = "user" | "agent" | "thought";

function blockText(content: ContentBlock | undefined | null): string {
  if (!content) return "";
  return content.type === "text" ? content.text : `[${content.type}]`;
}

const TOOL_ICON: Record<string, string> = {
  read: "📖", edit: "✏️", delete: "🗑️", move: "📦", search: "🔍",
  execute: "⚡", think: "💭", fetch: "🌐", switch_mode: "🔀", other: "🔧",
};
const STATUS_GLYPH: Record<string, string> = {
  pending: "○", in_progress: "◐", completed: "✓", failed: "✗",
};

function toolContentText(content: ToolCallContent[] | null | undefined): string {
  if (!content) return "";
  const parts: string[] = [];
  for (const it of content) {
    if (it.type === "content") {
      parts.push(blockText((it as { content: ContentBlock }).content));
    } else if (it.type === "diff") {
      const d = it as { path?: string; newText?: string };
      parts.push(`${d.path ?? "diff"}\n${d.newText ?? ""}`);
    } else if (it.type === "terminal") {
      parts.push("[terminal output]");
    }
  }
  return parts.join("\n").trim();
}

export class ChatRenderer {
  private cur: { role: TurnRole; raw: string; body: HTMLElement } | null = null;
  private tools = new Map<string, HTMLElement>();
  private planEl: HTMLElement | null = null;

  constructor(private log: HTMLElement) {}

  clear(): void {
    this.log.replaceChildren();
    this.cur = null;
    this.tools.clear();
    this.planEl = null;
  }

  get isEmpty(): boolean {
    return this.log.childElementCount === 0;
  }

  private scroll(): void {
    this.log.scrollTop = this.log.scrollHeight;
  }

  finalizeTurn(): void {
    this.cur = null;
  }

  private turnBody(role: TurnRole): HTMLElement {
    if (this.cur && this.cur.role === role) return this.cur.body;
    const body = document.createElement("div");
    body.className = "body";

    if (role === "thought") {
      const details = document.createElement("details");
      details.className = "msg thought";
      const summary = document.createElement("summary");
      summary.textContent = "thinking";
      details.append(summary, body);
      this.log.appendChild(details);
    } else {
      const msg = document.createElement("div");
      msg.className = `msg ${role}`;
      const avatar = document.createElement("div");
      avatar.className = "avatar";
      avatar.textContent = role === "user" ? "you" : "goose";
      msg.append(avatar, body);
      this.log.appendChild(msg);
    }
    this.cur = { role, raw: "", body };
    return body;
  }

  chunk(role: TurnRole, content: ContentBlock | undefined | null): void {
    const body = this.turnBody(role);
    this.cur!.raw += blockText(content);
    if (role === "agent") renderMarkdown(body, this.cur!.raw);
    else body.textContent = this.cur!.raw;
    this.scroll();
  }

  /** A live user message (always starts a fresh turn). */
  userMessage(text: string): void {
    this.finalizeTurn();
    this.chunk("user", { type: "text", text });
    this.finalizeTurn();
  }

  system(text: string): void {
    this.finalizeTurn();
    const el = document.createElement("div");
    el.className = "msg system";
    el.textContent = text;
    this.log.appendChild(el);
    this.scroll();
  }

  toolCall(tc: ToolCall): void {
    this.finalizeTurn();
    const el = document.createElement("div");
    el.className = "tool";
    el.innerHTML =
      '<div class="tool-head"><span class="tool-icon"></span>' +
      '<span class="tool-title"></span><span class="tool-status"></span></div>';
    this.tools.set(tc.toolCallId, el);
    this.log.appendChild(el);
    this.fillTool(el, tc.kind, tc.title, tc.status, tc.content);
    this.scroll();
  }

  toolUpdate(tc: ToolCallUpdate): void {
    const el = this.tools.get(tc.toolCallId);
    if (!el) {
      this.toolCall({ ...tc, title: tc.title ?? "tool" } as ToolCall);
      return;
    }
    this.fillTool(el, tc.kind, tc.title, tc.status, tc.content);
    this.scroll();
  }

  private fillTool(
    el: HTMLElement,
    kind: string | null | undefined,
    title: string | null | undefined,
    status: string | null | undefined,
    content: ToolCallContent[] | null | undefined,
  ): void {
    if (kind) (el.querySelector(".tool-icon") as HTMLElement).textContent = TOOL_ICON[kind] ?? "🔧";
    if (title) (el.querySelector(".tool-title") as HTMLElement).textContent = title;
    if (status) {
      const s = el.querySelector(".tool-status") as HTMLElement;
      s.textContent = `${STATUS_GLYPH[status] ?? ""} ${status.replace("_", " ")}`;
      s.className = `tool-status ${status}`;
      el.classList.toggle("running", status === "in_progress" || status === "pending");
    }
    const text = toolContentText(content);
    if (text) {
      let details = el.querySelector("details.tool-body") as HTMLDetailsElement | null;
      if (!details) {
        details = document.createElement("details");
        details.className = "tool-body";
        const summary = document.createElement("summary");
        summary.textContent = "output";
        const pre = document.createElement("pre");
        details.append(summary, pre);
        el.appendChild(details);
      }
      (details.querySelector("pre") as HTMLElement).textContent = text;
    }
  }

  /**
   * Inline permission request. Appends a card with the options as buttons and
   * resolves with the chosen optionId (or null if cancelled). Promise-based, so
   * unlike window.confirm it does NOT block the JS thread / ACP message pump —
   * tool output keeps streaming while the card is shown.
   */
  permission(
    title: string,
    options: { optionId: string; name: string; kind: string }[],
  ): Promise<string | null> {
    this.finalizeTurn();
    const card = document.createElement("div");
    card.className = "perm";
    const head = document.createElement("div");
    head.className = "perm-head";
    head.textContent = `🔐 ${title} needs permission`;
    const row = document.createElement("div");
    row.className = "perm-actions";
    card.append(head, row);
    this.log.appendChild(card);
    this.scroll();

    return new Promise<string | null>((resolve) => {
      const finish = (id: string | null, label: string) => {
        row.replaceChildren();
        const chosen = document.createElement("span");
        chosen.className = "perm-chosen";
        chosen.textContent = `→ ${label}`;
        card.appendChild(chosen);
        resolve(id);
      };
      for (const o of options) {
        const btn = document.createElement("button");
        const allow = o.kind.startsWith("allow");
        btn.className = allow ? "primary" : "ghost";
        btn.textContent = o.name;
        btn.onclick = () => finish(o.optionId, o.name);
        row.appendChild(btn);
      }
      const cancel = document.createElement("button");
      cancel.className = "ghost";
      cancel.textContent = "cancel";
      cancel.onclick = () => finish(null, "cancelled");
      row.appendChild(cancel);
    });
  }

  plan(entries: PlanEntry[]): void {
    if (!this.planEl) {
      this.planEl = document.createElement("div");
      this.planEl.className = "plan";
      this.log.appendChild(this.planEl);
    }
    this.planEl.replaceChildren();
    const head = document.createElement("div");
    head.className = "plan-head";
    head.textContent = "Plan";
    this.planEl.appendChild(head);
    for (const e of entries) {
      const row = document.createElement("div");
      row.className = `plan-row ${e.status}`;
      const box = document.createElement("span");
      box.className = "plan-box";
      box.textContent =
        e.status === "completed" ? "☑" : e.status === "in_progress" ? "▸" : "☐";
      const txt = document.createElement("span");
      txt.className = "plan-text";
      txt.textContent = e.content;
      row.append(box, txt);
      this.planEl.appendChild(row);
    }
    this.finalizeTurn();
    this.scroll();
  }
}
