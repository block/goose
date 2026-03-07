/**
 * Lightweight Chrome DevTools Protocol client for Electron.
 *
 * Connects to Electron's remote-debugging endpoint over raw WebSocket
 * (no Puppeteer dependency). Subscribes to Runtime and Log domains to
 * collect console messages, exceptions, and log entries in real time.
 */

// ── CDP target as returned by /json/list ────────────────────────────
export interface CDPTarget {
  id: string;
  type: string;
  title: string;
  url: string;
  webSocketDebuggerUrl?: string;
  devtoolsFrontendUrl?: string;
  description?: string;
  faviconUrl?: string;
}

// ── Collected console entry ─────────────────────────────────────────
export interface ConsoleEntry {
  id: number;
  timestamp: number;
  source: "console-api" | "exception" | "log-entry";
  level: string;
  text: string;
  url?: string;
  lineNumber?: number;
  columnNumber?: number;
  stackTrace?: string;
  targetId: string;
  targetTitle: string;
}

// ── Minimal WebSocket interface (works with both native and 'ws') ───
interface MinimalWebSocket {
  addEventListener(type: "open", listener: () => void): void;
  addEventListener(type: "close", listener: () => void): void;
  addEventListener(type: "error", listener: (e: unknown) => void): void;
  addEventListener(type: "message", listener: (e: { data: unknown }) => void): void;
  send(data: string): void;
  close(): void;
}

// ── Per-target CDP session ──────────────────────────────────────────
interface CDPSession {
  ws: MinimalWebSocket;
  nextId: number;
  pending: Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>;
  targetId: string;
  targetTitle: string;
}

type WsConstructor = new (url: string) => MinimalWebSocket;

const KEY_CODES: Record<string, number> = {
  Enter: 13, Tab: 9, Escape: 27, Backspace: 8, Delete: 46,
  ArrowUp: 38, ArrowDown: 40, ArrowLeft: 37, ArrowRight: 39,
  Home: 36, End: 35, PageUp: 33, PageDown: 34,
  Space: 32, F1: 112, F2: 113, F3: 114, F4: 115, F5: 116,
  F6: 117, F7: 118, F8: 119, F9: 120, F10: 121, F11: 122, F12: 123,
  a: 65, b: 66, c: 67, d: 68, e: 69, f: 70, g: 71, h: 72, i: 73,
  j: 74, k: 75, l: 76, m: 77, n: 78, o: 79, p: 80, q: 81, r: 82,
  s: 83, t: 84, u: 85, v: 86, w: 87, x: 88, y: 89, z: 90,
};

export class CDPClient {
  private port: number;
  private host: string;
  private entries: ConsoleEntry[] = [];
  private entryId = 0;
  private maxEntries: number;
  private sessions = new Map<string, CDPSession>();
  private WsClass: WsConstructor | null = null;

  constructor(port: number, host = "127.0.0.1", maxEntries = 5000) {
    this.port = port;
    this.host = host;
    this.maxEntries = maxEntries;
  }

  // ── Public API ──────────────────────────────────────────────────

  async listTargets(): Promise<CDPTarget[]> {
    const res = await fetch(`http://${this.host}:${this.port}/json/list`);
    if (!res.ok) {
      throw new Error(`Failed to list targets: ${res.status} ${res.statusText}`);
    }
    return (await res.json()) as CDPTarget[];
  }

  async getVersion(): Promise<Record<string, string>> {
    const res = await fetch(`http://${this.host}:${this.port}/json/version`);
    if (!res.ok) {
      throw new Error(`Failed to get version: ${res.status} ${res.statusText}`);
    }
    return (await res.json()) as Record<string, string>;
  }

  async attach(target: CDPTarget): Promise<void> {
    if (this.sessions.has(target.id)) return;
    if (!target.webSocketDebuggerUrl) {
      throw new Error(`Target ${target.id} (${target.title}) has no webSocketDebuggerUrl`);
    }

    const WS = await this.getWsClass();
    const ws = new WS(target.webSocketDebuggerUrl);

    const session: CDPSession = {
      ws,
      nextId: 1,
      pending: new Map(),
      targetId: target.id,
      targetTitle: target.title,
    };

    await new Promise<void>((resolve, reject) => {
      ws.addEventListener("open", () => resolve());
      ws.addEventListener("error", (e) => reject(new Error(`WebSocket error: ${e}`)));
    });

    ws.addEventListener("message", (event) => {
      const data = JSON.parse(String(event.data));
      if (data.id !== undefined) {
        const p = session.pending.get(data.id);
        if (p) {
          session.pending.delete(data.id);
          if (data.error) {
            p.reject(new Error(data.error.message));
          } else {
            p.resolve(data.result);
          }
        }
        return;
      }
      this.handleCDPEvent(session, data.method, data.params);
    });

    ws.addEventListener("close", () => {
      this.sessions.delete(target.id);
    });

    this.sessions.set(target.id, session);

    await this.send(session, "Runtime.enable");
    await this.send(session, "Log.enable");
  }

  async attachAll(): Promise<CDPTarget[]> {
    const targets = await this.listTargets();
    const attached: CDPTarget[] = [];
    for (const t of targets) {
      if (!t.webSocketDebuggerUrl) continue;
      try {
        await this.attach(t);
        attached.push(t);
      } catch {
        // Skip targets we can't attach to
      }
    }
    return attached;
  }

  async disconnect(targetId: string): Promise<void> {
    const session = this.sessions.get(targetId);
    if (session) {
      this.sessions.delete(targetId);
      // Reject any pending requests
      for (const [, p] of session.pending) {
        p.reject(new Error("Session disconnected"));
      }
      session.pending.clear();

      await new Promise<void>((resolve) => {
        const timeout = setTimeout(() => resolve(), 2000);
        session.ws.addEventListener("close", () => {
          clearTimeout(timeout);
          resolve();
        });
        session.ws.close();
      });
    }
  }

  async disconnectAll(): Promise<void> {
    const ids = [...this.sessions.keys()];
    await Promise.all(ids.map((id) => this.disconnect(id)));
  }

  getEntries(opts?: {
    targetId?: string;
    level?: string;
    since?: number;
    limit?: number;
    search?: string;
  }): ConsoleEntry[] {
    let result = this.entries;

    if (opts?.targetId) {
      result = result.filter((e) => e.targetId === opts.targetId);
    }
    if (opts?.level) {
      const levels = opts.level.split(",").map((l) => l.trim().toLowerCase());
      result = result.filter((e) => levels.includes(e.level.toLowerCase()));
    }
    if (opts?.since !== undefined) {
      const sinceId = opts.since;
      result = result.filter((e) => e.id > sinceId);
    }
    if (opts?.search) {
      const term = opts.search.toLowerCase();
      result = result.filter((e) => e.text.toLowerCase().includes(term));
    }
    if (opts?.limit) {
      result = result.slice(-opts.limit);
    }

    return result;
  }

  clearEntries(): void {
    this.entries = [];
  }

  getAttachedTargetIds(): string[] {
    return [...this.sessions.keys()];
  }

  async evaluate(
    targetId: string,
    expression: string,
    returnByValue = true
  ): Promise<{ result: unknown; exceptionDetails?: unknown }> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }
    return (await this.send(session, "Runtime.evaluate", {
      expression,
      returnByValue,
      awaitPromise: true,
      generatePreview: true,
      userGesture: true,
    })) as { result: unknown; exceptionDetails?: unknown };
  }

  // ── Screenshot & DOM inspection ────────────────────────────────

  async captureScreenshot(
    targetId: string,
    opts?: {
      format?: "png" | "jpeg" | "webp";
      quality?: number;
      clip?: { x: number; y: number; width: number; height: number; scale?: number };
      fullPage?: boolean;
    }
  ): Promise<string> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    // If fullPage, get the full document dimensions and set clip
    let clip = opts?.clip;
    if (opts?.fullPage) {
      const layoutMetrics = (await this.send(session, "Page.getLayoutMetrics")) as {
        contentSize: { width: number; height: number };
        cssContentSize?: { width: number; height: number };
      };
      const size = layoutMetrics.cssContentSize ?? layoutMetrics.contentSize;
      clip = { x: 0, y: 0, width: size.width, height: size.height, scale: 1 };
    }

    const params: Record<string, unknown> = {
      format: opts?.format ?? "png",
    };
    if (opts?.quality !== undefined) params.quality = opts.quality;
    if (clip) params.clip = { ...clip, scale: clip.scale ?? 1 };

    const result = (await this.send(session, "Page.captureScreenshot", params)) as { data: string };
    return result.data;
  }

  async captureElementScreenshot(
    targetId: string,
    selector: string,
    opts?: { format?: "png" | "jpeg" | "webp"; quality?: number; padding?: number }
  ): Promise<{ data: string; box: { x: number; y: number; width: number; height: number } }> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    // Get document root
    const doc = (await this.send(session, "DOM.getDocument", { depth: 0 })) as {
      root: { nodeId: number };
    };

    // Find the element
    const queryResult = (await this.send(session, "DOM.querySelector", {
      nodeId: doc.root.nodeId,
      selector,
    })) as { nodeId: number };

    if (!queryResult.nodeId || queryResult.nodeId === 0) {
      throw new Error(`No element found matching selector: ${selector}`);
    }

    // Get bounding box
    const boxModel = (await this.send(session, "DOM.getBoxModel", {
      nodeId: queryResult.nodeId,
    })) as {
      model: {
        content: number[];
        padding: number[];
        border: number[];
        margin: number[];
        width: number;
        height: number;
      };
    };

    // border quad: [x1,y1, x2,y2, x3,y3, x4,y4] — use border box for screenshot
    const quad = boxModel.model.border;
    const xs = [quad[0], quad[2], quad[4], quad[6]];
    const ys = [quad[1], quad[3], quad[5], quad[7]];
    const pad = opts?.padding ?? 0;
    const x = Math.max(0, Math.min(...xs) - pad);
    const y = Math.max(0, Math.min(...ys) - pad);
    const width = Math.max(...xs) - Math.min(...xs) + pad * 2;
    const height = Math.max(...ys) - Math.min(...ys) + pad * 2;

    const clip = { x, y, width, height, scale: 1 };

    const params: Record<string, unknown> = {
      format: opts?.format ?? "png",
      clip,
    };
    if (opts?.quality !== undefined) params.quality = opts.quality;

    const result = (await this.send(session, "Page.captureScreenshot", params)) as { data: string };

    // Disable DOM to avoid holding references
    await this.send(session, "DOM.disable").catch(() => {});

    return { data: result.data, box: { x, y, width, height } };
  }

  async getDomSnapshot(
    targetId: string,
    opts?: { computedStyles?: string[] }
  ): Promise<unknown> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    return await this.send(session, "DOMSnapshot.captureSnapshot", {
      computedStyles: opts?.computedStyles ?? [
        "display", "visibility", "opacity", "color", "background-color",
        "font-size", "font-weight", "width", "height", "overflow",
      ],
    });
  }

  async getDocumentOuterHTML(targetId: string, selector?: string): Promise<string> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    const doc = (await this.send(session, "DOM.getDocument", { depth: 0 })) as {
      root: { nodeId: number };
    };

    let nodeId = doc.root.nodeId;
    if (selector) {
      const queryResult = (await this.send(session, "DOM.querySelector", {
        nodeId: doc.root.nodeId,
        selector,
      })) as { nodeId: number };
      if (!queryResult.nodeId || queryResult.nodeId === 0) {
        throw new Error(`No element found matching selector: ${selector}`);
      }
      nodeId = queryResult.nodeId;
    }

    const result = (await this.send(session, "DOM.getOuterHTML", { nodeId })) as {
      outerHTML: string;
    };

    await this.send(session, "DOM.disable").catch(() => {});

    return result.outerHTML;
  }

  // ── Navigation & interaction ───────────────────────────────────

  async navigate(targetId: string, url: string): Promise<{ frameId: string; errorText?: string }> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }
    return (await this.send(session, "Page.navigate", { url })) as {
      frameId: string;
      errorText?: string;
    };
  }

  async waitForLoad(targetId: string, timeoutMs = 30000): Promise<void> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    await this.send(session, "Page.enable");

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        cleanup();
        reject(new Error(`Page load timed out after ${timeoutMs}ms`));
      }, timeoutMs);

      const handler = (event: { data: unknown }) => {
        const data = JSON.parse(String(event.data));
        if (data.method === "Page.loadEventFired") {
          cleanup();
          resolve();
        }
      };

      const cleanup = () => {
        clearTimeout(timeout);
        session.ws.addEventListener("message", () => {});
      };

      session.ws.addEventListener("message", handler);
    });
  }

  async clickAtPoint(
    targetId: string,
    x: number,
    y: number,
    opts?: { button?: "left" | "right" | "middle"; clickCount?: number }
  ): Promise<void> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    const button = opts?.button ?? "left";
    const clickCount = opts?.clickCount ?? 1;

    await this.send(session, "Input.dispatchMouseEvent", {
      type: "mousePressed",
      x,
      y,
      button,
      clickCount,
    });
    await this.send(session, "Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x,
      y,
      button,
      clickCount,
    });
  }

  async clickSelector(
    targetId: string,
    selector: string,
    opts?: { button?: "left" | "right" | "middle"; clickCount?: number }
  ): Promise<{ x: number; y: number }> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    const doc = (await this.send(session, "DOM.getDocument", { depth: 0 })) as {
      root: { nodeId: number };
    };

    const queryResult = (await this.send(session, "DOM.querySelector", {
      nodeId: doc.root.nodeId,
      selector,
    })) as { nodeId: number };

    if (!queryResult.nodeId || queryResult.nodeId === 0) {
      await this.send(session, "DOM.disable").catch(() => {});
      throw new Error(`No element found matching selector: ${selector}`);
    }

    const boxModel = (await this.send(session, "DOM.getBoxModel", {
      nodeId: queryResult.nodeId,
    })) as {
      model: { content: number[] };
    };

    await this.send(session, "DOM.disable").catch(() => {});

    // Click center of content box
    const quad = boxModel.model.content;
    const cx = (quad[0] + quad[2] + quad[4] + quad[6]) / 4;
    const cy = (quad[1] + quad[3] + quad[5] + quad[7]) / 4;

    await this.clickAtPoint(targetId, cx, cy, opts);
    return { x: cx, y: cy };
  }

  async typeText(targetId: string, text: string): Promise<void> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    for (const char of text) {
      await this.send(session, "Input.dispatchKeyEvent", {
        type: "keyDown",
        text: char,
        key: char,
        unmodifiedText: char,
      });
      await this.send(session, "Input.dispatchKeyEvent", {
        type: "keyUp",
        key: char,
      });
    }
  }

  async pressKey(
    targetId: string,
    key: string,
    opts?: { modifiers?: number }
  ): Promise<void> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    const modifiers = opts?.modifiers ?? 0;

    await this.send(session, "Input.dispatchKeyEvent", {
      type: "keyDown",
      key,
      windowsVirtualKeyCode: KEY_CODES[key] ?? 0,
      modifiers,
    });
    await this.send(session, "Input.dispatchKeyEvent", {
      type: "keyUp",
      key,
      windowsVirtualKeyCode: KEY_CODES[key] ?? 0,
      modifiers,
    });
  }

  async focus(targetId: string, selector: string): Promise<void> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    const doc = (await this.send(session, "DOM.getDocument", { depth: 0 })) as {
      root: { nodeId: number };
    };

    const queryResult = (await this.send(session, "DOM.querySelector", {
      nodeId: doc.root.nodeId,
      selector,
    })) as { nodeId: number };

    if (!queryResult.nodeId || queryResult.nodeId === 0) {
      await this.send(session, "DOM.disable").catch(() => {});
      throw new Error(`No element found matching selector: ${selector}`);
    }

    await this.send(session, "DOM.focus", { nodeId: queryResult.nodeId });
    await this.send(session, "DOM.disable").catch(() => {});
  }

  async waitForSelector(
    targetId: string,
    selector: string,
    opts?: { timeoutMs?: number; pollIntervalMs?: number; visible?: boolean }
  ): Promise<boolean> {
    const timeout = opts?.timeoutMs ?? 10000;
    const interval = opts?.pollIntervalMs ?? 200;
    const checkVisible = opts?.visible ?? false;
    const deadline = Date.now() + timeout;

    while (Date.now() < deadline) {
      try {
        const session = this.sessions.get(targetId);
        if (!session) throw new Error(`Not attached to target ${targetId}`);

        const result = (await this.send(session, "Runtime.evaluate", {
          expression: checkVisible
            ? `(() => { const el = document.querySelector(${JSON.stringify(selector)}); return el && el.offsetParent !== null; })()`
            : `!!document.querySelector(${JSON.stringify(selector)})`,
          returnByValue: true,
        })) as { result: { value: boolean } };

        if (result.result.value) return true;
      } catch {
        // target might have navigated, keep polling
      }

      await new Promise((r) => setTimeout(r, interval));
    }

    return false;
  }

  async scrollTo(
    targetId: string,
    opts: { x?: number; y?: number; selector?: string }
  ): Promise<void> {
    const session = this.sessions.get(targetId);
    if (!session) {
      throw new Error(`Not attached to target ${targetId}`);
    }

    if (opts.selector) {
      await this.send(session, "Runtime.evaluate", {
        expression: `document.querySelector(${JSON.stringify(opts.selector)})?.scrollIntoView({ behavior: 'smooth', block: 'center' })`,
        returnByValue: true,
      });
    } else {
      await this.send(session, "Runtime.evaluate", {
        expression: `window.scrollTo(${opts.x ?? 0}, ${opts.y ?? 0})`,
        returnByValue: true,
      });
    }
  }

  // ── Private helpers ─────────────────────────────────────────────

  private async getWsClass(): Promise<WsConstructor> {
    if (this.WsClass) return this.WsClass;
    if (typeof globalThis.WebSocket !== "undefined") {
      this.WsClass = globalThis.WebSocket as unknown as WsConstructor;
    } else {
      const ws = await import("ws");
      this.WsClass = ws.default as unknown as WsConstructor;
    }
    return this.WsClass;
  }

  private send(session: CDPSession, method: string, params: Record<string, unknown> = {}): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const id = session.nextId++;
      const timeout = setTimeout(() => {
        session.pending.delete(id);
        reject(new Error(`CDP request ${method} timed out after 10s`));
      }, 10000);
      session.pending.set(id, {
        resolve: (v) => { clearTimeout(timeout); resolve(v); },
        reject: (e) => { clearTimeout(timeout); reject(e); },
      });
      session.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  private handleCDPEvent(session: CDPSession, method: string, params: Record<string, unknown>): void {
    switch (method) {
      case "Runtime.consoleAPICalled":
        this.onConsoleAPI(session, params);
        break;
      case "Runtime.exceptionThrown":
        this.onException(session, params);
        break;
      case "Log.entryAdded":
        this.onLogEntry(session, params);
        break;
    }
  }

  private onConsoleAPI(session: CDPSession, params: Record<string, unknown>): void {
    const type = params.type as string;
    const args = params.args as Array<{ type: string; value?: unknown; description?: string; preview?: unknown }>;
    const stackTrace = params.stackTrace as {
      callFrames?: Array<{ url: string; lineNumber: number; columnNumber: number }>;
    } | undefined;

    const textParts = args.map((arg) => {
      if (arg.value !== undefined) return String(arg.value);
      if (arg.description) return arg.description;
      if (arg.preview) return JSON.stringify(arg.preview);
      return `[${arg.type}]`;
    });

    const topFrame = stackTrace?.callFrames?.[0];

    this.addEntry({
      source: "console-api",
      level: type,
      text: textParts.join(" "),
      url: topFrame?.url,
      lineNumber: topFrame?.lineNumber,
      columnNumber: topFrame?.columnNumber,
      stackTrace: stackTrace?.callFrames
        ?.map((f) => `  at ${f.url}:${f.lineNumber}:${f.columnNumber}`)
        .join("\n"),
      targetId: session.targetId,
      targetTitle: session.targetTitle,
    });
  }

  private onException(session: CDPSession, params: Record<string, unknown>): void {
    const details = params.exceptionDetails as {
      text?: string;
      exception?: { description?: string; value?: unknown };
      url?: string;
      lineNumber?: number;
      columnNumber?: number;
      stackTrace?: { callFrames?: Array<{ url: string; lineNumber: number; columnNumber: number }> };
    };

    const text =
      details.exception?.description ||
      details.exception?.value?.toString() ||
      details.text ||
      "Unknown exception";

    this.addEntry({
      source: "exception",
      level: "error",
      text,
      url: details.url,
      lineNumber: details.lineNumber,
      columnNumber: details.columnNumber,
      stackTrace: details.stackTrace?.callFrames
        ?.map((f) => `  at ${f.url}:${f.lineNumber}:${f.columnNumber}`)
        .join("\n"),
      targetId: session.targetId,
      targetTitle: session.targetTitle,
    });
  }

  private onLogEntry(session: CDPSession, params: Record<string, unknown>): void {
    const entry = params.entry as {
      source: string;
      level: string;
      text: string;
      url?: string;
      lineNumber?: number;
    };

    this.addEntry({
      source: "log-entry",
      level: entry.level,
      text: `[${entry.source}] ${entry.text}`,
      url: entry.url,
      lineNumber: entry.lineNumber,
      targetId: session.targetId,
      targetTitle: session.targetTitle,
    });
  }

  private addEntry(entry: Omit<ConsoleEntry, "id" | "timestamp">): void {
    const full: ConsoleEntry = {
      ...entry,
      id: ++this.entryId,
      timestamp: Date.now(),
    };
    this.entries.push(full);

    if (this.entries.length > this.maxEntries) {
      this.entries = this.entries.slice(-Math.floor(this.maxEntries * 0.8));
    }
  }
}
