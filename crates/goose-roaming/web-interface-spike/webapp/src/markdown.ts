// Minimal, dependency-free, XSS-safe markdown → DOM renderer.
//
// Safe *by construction*: every piece of text reaches the DOM via
// `textContent` / `createTextNode`, never `innerHTML`. There is no HTML
// parsing path, so agent output cannot inject markup or scripts. Link hrefs
// are scheme-allowlisted (http/https/mailto only). This is the whole reason we
// don't need DOMPurify — untrusted agent text is never treated as HTML.
//
// Scope is deliberately "good enough for an agent chat": fenced code blocks
// (the important one), inline code, bold/italic, links, headings, blockquotes,
// and ordered/unordered lists. Not a CommonMark implementation.

const SAFE_LINK = /^(https?:|mailto:)/i;

/** Render markdown source into a fresh element tree appended to `parent`. */
export function renderMarkdown(parent: HTMLElement, src: string): void {
  parent.replaceChildren();
  for (const block of splitBlocks(src)) {
    parent.appendChild(renderBlock(block));
  }
}

type Block =
  | { kind: "code"; lang: string; text: string }
  | { kind: "heading"; level: number; text: string }
  | { kind: "quote"; text: string }
  | { kind: "ul"; items: string[] }
  | { kind: "ol"; items: string[] }
  | { kind: "p"; text: string };

function splitBlocks(src: string): Block[] {
  const lines = src.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // fenced code block
    const fence = line.match(/^```(.*)$/);
    if (fence) {
      const lang = fence[1].trim();
      const body: string[] = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) body.push(lines[i++]);
      i++; // closing fence
      blocks.push({ kind: "code", lang, text: body.join("\n") });
      continue;
    }

    // blank
    if (line.trim() === "") {
      i++;
      continue;
    }

    // heading
    const h = line.match(/^(#{1,6})\s+(.*)$/);
    if (h) {
      blocks.push({ kind: "heading", level: h[1].length, text: h[2] });
      i++;
      continue;
    }

    // blockquote (consecutive > lines)
    if (/^>\s?/.test(line)) {
      const body: string[] = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) {
        body.push(lines[i].replace(/^>\s?/, ""));
        i++;
      }
      blocks.push({ kind: "quote", text: body.join("\n") });
      continue;
    }

    // unordered list
    if (/^[-*+]\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*+]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*+]\s+/, ""));
        i++;
      }
      blocks.push({ kind: "ul", items });
      continue;
    }

    // ordered list
    if (/^\d+[.)]\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+[.)]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+[.)]\s+/, ""));
        i++;
      }
      blocks.push({ kind: "ol", items });
      continue;
    }

    // paragraph (consecutive non-blank, non-special lines)
    const body: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !/^```/.test(lines[i]) &&
      !/^#{1,6}\s/.test(lines[i]) &&
      !/^>\s?/.test(lines[i]) &&
      !/^[-*+]\s+/.test(lines[i]) &&
      !/^\d+[.)]\s+/.test(lines[i])
    ) {
      body.push(lines[i++]);
    }
    blocks.push({ kind: "p", text: body.join("\n") });
  }

  return blocks;
}

function renderBlock(block: Block): HTMLElement {
  switch (block.kind) {
    case "code": {
      const pre = document.createElement("pre");
      const code = document.createElement("code");
      if (block.lang) code.dataset.lang = block.lang;
      code.textContent = block.text;
      pre.appendChild(code);
      return pre;
    }
    case "heading": {
      const el = document.createElement(`h${Math.min(block.level, 6)}`);
      renderInline(el, block.text);
      return el;
    }
    case "quote": {
      const bq = document.createElement("blockquote");
      renderInline(bq, block.text);
      return bq;
    }
    case "ul":
    case "ol": {
      const list = document.createElement(block.kind === "ul" ? "ul" : "ol");
      for (const item of block.items) {
        const li = document.createElement("li");
        renderInline(li, item);
        list.appendChild(li);
      }
      return list;
    }
    case "p": {
      const p = document.createElement("p");
      renderInline(p, block.text);
      return p;
    }
  }
}

// Inline: `code`, **bold**/__bold__, *italic*/_italic_, [text](url), and \n → <br>.
// Tokenised with a single regex; anything not matched is a literal text node.
const INLINE =
  /(`[^`]+`)|(\*\*[^*]+\*\*|__[^_]+__)|(\*[^*]+\*|_[^_]+_)|(\[[^\]]+\]\([^)]+\))/g;

function renderInline(parent: HTMLElement, text: string): void {
  let last = 0;
  let m: RegExpExecArray | null;
  INLINE.lastIndex = 0;
  while ((m = INLINE.exec(text)) !== null) {
    if (m.index > last) appendText(parent, text.slice(last, m.index));
    const tok = m[0];
    if (m[1]) {
      const code = document.createElement("code");
      code.textContent = tok.slice(1, -1);
      parent.appendChild(code);
    } else if (m[2]) {
      const strong = document.createElement("strong");
      strong.textContent = tok.slice(2, -2);
      parent.appendChild(strong);
    } else if (m[3]) {
      const em = document.createElement("em");
      em.textContent = tok.slice(1, -1);
      parent.appendChild(em);
    } else if (m[4]) {
      const linkMatch = tok.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      if (linkMatch) appendLink(parent, linkMatch[1], linkMatch[2]);
      else appendText(parent, tok);
    }
    last = m.index + tok.length;
  }
  if (last < text.length) appendText(parent, text.slice(last));
}

function appendText(parent: HTMLElement, text: string): void {
  const parts = text.split("\n");
  parts.forEach((part, idx) => {
    if (part) parent.appendChild(document.createTextNode(part));
    if (idx < parts.length - 1) parent.appendChild(document.createElement("br"));
  });
}

function appendLink(parent: HTMLElement, label: string, href: string): void {
  if (SAFE_LINK.test(href.trim())) {
    const a = document.createElement("a");
    a.href = href.trim();
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    a.textContent = label;
    parent.appendChild(a);
  } else {
    // Unsafe scheme (javascript:, data:, …) → render as inert text.
    appendText(parent, `${label} (${href})`);
  }
}
